//! A run that shows nothing for too long is noticed, then stopped with its work kept. Keep waiting
//! gives it the whole time again, and the limit can be changed or turned off.

use onecorp::store::Company;
use onecorp::work::{self, Job};
use onecorp::{templates, view};
use std::process::Command;

#[test]
fn quiet_runs_are_noticed_then_stopped() {
    let root = std::env::temp_dir().join(format!("onecorp-watchdog-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    assert!(Command::new("git").arg("-C").arg(&repo).args(["init", "-q"]).status().unwrap().success());
    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();
    let db = rusqlite::Connection::open(root.join("home/companies/acme/company.db")).unwrap();

    // A ticket running for a while, its last step `minutes` ago.
    let running = |title: &str, who: &str, minutes: i64| {
        let t = c.add_ticket(title, who).unwrap();
        c.set_status(t, "doing").unwrap();
        c.run_now(t, &Job::Start).unwrap();
        c.add_event(t, "tool", "Bash npm run dev", "").unwrap();
        let ago = format!("-{minutes} minutes");
        db.execute("UPDATE jobs SET started_at = datetime('now', ?2) WHERE ticket_id = ?1", rusqlite::params![t, ago]).unwrap();
        db.execute("UPDATE events SET at = datetime('now', ?2) WHERE ticket_id = ?1", rusqlite::params![t, ago]).unwrap();
        t
    };
    assert_eq!(work::quiet_limit(&c).unwrap(), 30, "30 minutes unless changed");

    let busy = running("Busy", "maya", 2);
    let quiet = running("Quiet", "odile", 12);
    let stuck = running("Stuck", "rui", 40);
    let keys: Vec<String> = work::quiet_runs(&c).unwrap().iter().map(|q| c.key(q.ticket.id)).collect();
    assert_eq!(keys, [c.key(quiet), c.key(stuck)], "past a third of the limit, not before");

    // The Inbox shows it, with how long and the last step.
    let snap = view::snapshot(&c).unwrap();
    let card = snap["quiet"].as_array().unwrap().iter().find(|q| q["key"] == c.key(quiet)).unwrap().clone();
    assert_eq!((card["minutes"].as_i64(), card["last"].as_str()), (Some(12), Some("Bash npm run dev")));

    // Past the limit: stopped, and the card says why. The others carry on.
    assert_eq!(work::stop_quiet(&c).unwrap(), [c.key(stuck)]);
    assert_eq!(c.ticket(stuck).unwrap().unwrap().status, "failed");
    let why = view::snapshot(&c).unwrap()["failed"].as_array().unwrap().iter().find(|f| f["key"] == c.key(stuck)).unwrap()["why"].clone();
    assert!(why.as_str().unwrap().starts_with("stopped after 40 minutes with no output. Last step: Bash npm run dev"), "{why}");
    assert_eq!(c.ticket(quiet).unwrap().unwrap().status, "doing");
    assert_eq!(c.ticket(busy).unwrap().unwrap().status, "doing");

    // Keep waiting: it has the whole time again.
    work::keep_waiting(&c, &c.ticket(quiet).unwrap().unwrap()).unwrap();
    assert!(work::quiet_runs(&c).unwrap().is_empty());

    // Never: nothing is noticed or stopped.
    let again = running("Stuck again", "lina", 90);
    work::set_quiet_limit(&c, 0).unwrap();
    assert!(work::quiet_runs(&c).unwrap().is_empty());
    assert!(work::stop_quiet(&c).unwrap().is_empty());
    assert_eq!(c.ticket(again).unwrap().unwrap().status, "doing");

    let _ = std::fs::remove_dir_all(&root);
}
