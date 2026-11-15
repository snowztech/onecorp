//! Copies of a company: taken by hand, every hour while it changes, and before an update, kept two
//! weeks, and put back without losing what was there.

use onecorp::backup;
use onecorp::store::Company;
use onecorp::work::Job;
use onecorp::{templates, wiki};
use std::process::Command;

#[test]
fn copies_are_taken_kept_and_restored() {
    let root = std::env::temp_dir().join(format!("onecorp-backup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    assert!(Command::new("git").arg("-C").arg(&repo).args(["init", "-q"]).status().unwrap().success());
    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();
    assert!(backup::list(&c).unwrap().is_empty(), "a new company has nothing to copy yet");

    let first = c.add_ticket("Search", "maya").unwrap();
    wiki::write(&c, wiki::DECISIONS, "We use Postgres.", "founder", "").unwrap();
    let copy = backup::take(&c, "").unwrap();
    assert!(copy.join("company.db").is_file() && copy.join("wiki").is_dir());
    let taken = backup::list(&c).unwrap();
    assert_eq!(taken.len(), 1);

    // An hour hasn't passed: nothing new.
    assert_eq!(backup::maybe(&c).unwrap(), None);

    // Things change, then the copy goes back.
    let second = c.add_ticket("Export", "maya").unwrap();
    wiki::write(&c, wiki::DECISIONS, "We use SQLite.", "founder", "").unwrap();
    backup::restore(&c, &taken[0].name).unwrap();
    assert!(c.ticket(first).unwrap().is_some());
    assert!(c.ticket(second).unwrap().is_none(), "the later ticket is gone");
    assert_eq!(wiki::read(&c, wiki::DECISIONS).unwrap().trim(), "We use Postgres.");

    // What was there before is a copy too, so a restore can be undone.
    let before = backup::list(&c).unwrap().into_iter().find(|x| x.why == "before-restore").unwrap();
    backup::restore(&c, &before.name).unwrap();
    assert!(c.ticket(second).unwrap().is_some());
    assert_eq!(wiki::read(&c, wiki::DECISIONS).unwrap().trim(), "We use SQLite.");

    // Older than a day, one copy a day stays: the newest of that day. Older than two weeks, none.
    let backups = root.join("home/companies/acme/backups");
    let at = |offset: &str| c.local_time("%Y-%m-%d_%H-%M-%S", offset).unwrap();
    let fake = |name: &str| {
        std::fs::create_dir_all(backups.join(name)).unwrap();
        std::fs::write(backups.join(name).join("company.db"), "").unwrap();
    };
    let (morning, evening, ancient) = (at("-3 days"), at("-3 days"), at("-20 days"));
    let morning = format!("{}_08-00-00", &morning[..10]);
    let evening = format!("{}_20-00-00", &evening[..10]);
    for name in [&morning, &evening, &ancient] {
        fake(name);
    }
    backup::maybe(&c).unwrap();
    assert!(backups.join(&evening).exists(), "the newest of that day stays");
    assert!(!backups.join(&morning).exists(), "the rest of that day goes");
    assert!(!backups.join(&ancient).exists(), "older than two weeks goes");

    // Several copies at the same second: asked for by time, OneCorp asks which.
    let second = at("");
    fake(&second);
    fake(&format!("{second}-before-restore"));
    let shown = format!("{} {}", &second[..10], second[11..].replace('-', ":"));
    assert!(backup::restore(&c, &shown).unwrap_err().to_string().contains("copies were taken at"));

    // Running work holds a restore back.
    let job = c.run_now(first, &Job::Start).unwrap();
    assert!(backup::restore(&c, &taken[0].name).unwrap_err().to_string().contains("work is running"));
    c.finish_job(job, None).unwrap();

    // A new version copies the company before it changes anything.
    c.set_setting("version", "0.0.1").unwrap();
    drop(c);
    let c = Company::open("acme").unwrap();
    let ver = env!("CARGO_PKG_VERSION");
    assert!(backup::list(&c).unwrap().iter().any(|x| x.why == format!("before-{ver}")), "copied before {ver}");
    assert_eq!(c.setting("version").unwrap().as_deref(), Some(ver));

    let _ = std::fs::remove_dir_all(&root);
}
