//! Firing someone frees the tickets they hadn't started, and what waited on them still moves.

use onecorp::store::Company;
use onecorp::{templates, work};
use std::process::Command;

#[test]
fn firing_frees_their_unstarted_tickets() {
    let root = std::env::temp_dir().join(format!("onecorp-fire-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    let git = |args: &[&str]| assert!(Command::new("git").arg("-C").arg(&repo).args(["-c", "user.name=F", "-c", "user.email=f@x"]).args(args).status().unwrap().success());
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# notes\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "init"]);
    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();
    let ticket = |id| c.ticket(id).unwrap().unwrap();

    // Maya's work is in review. Odile has one ticket waiting on it, one in the backlog, and one
    // queued behind a pause.
    let base = c.add_ticket("Base work", "maya").unwrap();
    work::start(&c, &ticket(base)).unwrap();
    assert_eq!(ticket(base).status, "review");
    let waits = c.add_ticket("Depends on base", "odile").unwrap();
    c.set_after(waits, Some(base)).unwrap();
    work::start(&c, &ticket(waits)).unwrap();
    assert_eq!(ticket(waits).status, "waiting");
    let later = c.add_ticket("Later", "odile").unwrap();
    c.set_paused(true).unwrap();
    let queued = c.add_ticket("Queued", "odile").unwrap();
    work::start(&c, &ticket(queued)).unwrap();
    assert_eq!(ticket(queued).status, "queued");

    let freed = work::fire(&c, "odile").unwrap();
    assert_eq!(freed, [c.key(waits), c.key(later), c.key(queued)]);
    for id in [waits, later, queued] {
        assert_eq!((ticket(id).status.as_str(), ticket(id).assignee), ("backlog", None), "{} is free to give to someone", c.key(id));
    }
    assert_eq!(ticket(base).assignee.as_deref(), Some("maya"), "other people's tickets are untouched");

    // Nothing of theirs is left in line, and merging what they waited on doesn't fail.
    c.set_paused(false).unwrap();
    assert!(c.claim_job().unwrap().is_none());
    onecorp::ship::approve(&c, &ticket(base)).unwrap();
    assert_eq!(ticket(base).status, "done");
    assert_eq!(ticket(waits).status, "backlog");

    let _ = std::fs::remove_dir_all(&root);
}
