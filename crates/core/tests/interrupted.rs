//! Work that OneCorp itself cut short, by quitting or restarting to update, picks back up when
//! serve starts again. A paused company, and work that can't go on where it stopped, stay stopped.

use onecorp::store::Company;
use onecorp::{templates, work};
use std::process::Command;

#[test]
fn interrupted_work_picks_back_up() {
    let root = std::env::temp_dir().join(format!("onecorp-interrupted-{}", std::process::id()));
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
    let status = |id| c.ticket(id).unwrap().unwrap().status;

    // A ticket with a session and a worktree, then cut short the way recovery marks it.
    let cut = |title: &str, who: &str| {
        let t = c.add_ticket(title, who).unwrap();
        work::start(&c, &c.ticket(t).unwrap().unwrap()).unwrap();
        c.set_status(t, "failed").unwrap();
        c.add_event(t, "interrupted", "stopped before it finished", "").unwrap();
        t
    };
    let a = cut("Search", "maya");
    // A run that failed on its own is yours to retry, not picked up behind your back.
    let failed = c.add_ticket("Broken [fail]", "odile").unwrap();
    work::start(&c, &c.ticket(failed).unwrap().unwrap()).unwrap();
    assert_eq!(status(failed), "failed");

    // Paused: nothing moves.
    c.set_paused(true).unwrap();
    assert!(work::resume_interrupted(&c).unwrap().is_empty());
    assert_eq!(status(a), "failed");

    c.set_paused(false).unwrap();
    assert_eq!(work::resume_interrupted(&c).unwrap(), [c.key(a)]);
    assert_eq!(status(a), "review", "same session, finished");
    assert!(c.events(a).unwrap().iter().any(|(_, k, s)| k == "retry" && s.contains("restarted")));
    assert_eq!(status(failed), "failed");
    // Only once: it's no longer interrupted.
    assert!(work::resume_interrupted(&c).unwrap().is_empty());

    let _ = std::fs::remove_dir_all(&root);
}
