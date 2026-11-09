//! `onecorp serve`: runs queued work for every company, several employees at a time.
//!
//! The CLI finds out serve is up from a heartbeat file it rewrites every second. Each job runs on
//! its own thread with its own database connection. The main loop is the only thing that claims
//! jobs, so two threads never grab the same one.

use crate::store::{self, Company};
use crate::work;
use anyhow::{Result, bail};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn heartbeat_path() -> std::path::PathBuf {
    store::home().join("serve.heartbeat")
}

fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// True when a serve process wrote its heartbeat in the last few seconds.
pub fn running() -> bool {
    let Ok(s) = std::fs::read_to_string(heartbeat_path()) else { return false };
    s.trim().parse::<u64>().is_ok_and(|t| now().saturating_sub(t) < 5)
}

pub fn serve(max: usize) -> Result<()> {
    if running() {
        bail!("onecorp serve is already running");
    }
    std::fs::create_dir_all(store::home())?;
    let companies = store::list()?;
    for name in &companies {
        let Ok(c) = Company::open(name) else { continue };
        for key in c.recover_interrupted()? {
            println!("{key} was interrupted when serve last stopped. It's marked failed: onecorp retry {key}");
        }
    }
    println!(
        "Serving {} {}, up to {max} at a time. Each employee works one ticket at a time. Ctrl-C to stop.",
        companies.len(),
        if companies.len() == 1 { "company" } else { "companies" }
    );

    let mut workers: Vec<JoinHandle<()>> = vec![];
    // Asking GitHub or GitLab about open pull requests is slow and rate-limited, so about once a minute.
    let mut last_sync: Option<Instant> = None;
    loop {
        if last_sync.is_none_or(|t| t.elapsed() > Duration::from_secs(60)) {
            last_sync = Some(Instant::now());
            for name in store::list()? {
                let Ok(c) = Company::open(&name) else { continue };
                match crate::ship::sync(&c) {
                    Ok(changed) => changed.iter().for_each(|m| println!("{m}")),
                    Err(e) => println!("{name}: couldn't check pull requests: {e:#}"),
                }
            }
        }
        std::fs::write(heartbeat_path(), now().to_string())?;
        workers.retain(|w| !w.is_finished());

        // New companies show up without a restart.
        'claim: for name in store::list()? {
            // A company deleted from the app vanishes between the listing and here.
            let Ok(c) = Company::open(&name) else { continue };
            for key in c.recover_interrupted()? {
                println!("{key} stopped before it finished. It's marked failed: onecorp retry {key}");
            }
            crate::spend::new_day(&c)?;
            if c.paused()? {
                continue;
            }
            while workers.len() < max {
                let Some(job) = c.claim_job()? else { continue 'claim };
                let key = c.key(job.ticket);
                let who = c.ticket(job.ticket)?.and_then(|t| t.assignee).unwrap_or_default();
                let name = name.clone();
                workers.push(thread::spawn(move || {
                    work::set_tag(&format!("{key} {who:<7} "));
                    // Each thread gets its own connection: rusqlite connections can't be shared across threads.
                    let result = Company::open(&name).and_then(|c| {
                        let r = work::execute(&c, job.ticket, &job.job);
                        c.finish_job(job.id, r.as_ref().err().map(|e| format!("{e:#}")).as_deref())?;
                        r
                    });
                    if let Err(e) = result {
                        println!("{key} {who:<7} error: {e:#}");
                    }
                }));
            }
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }
}
