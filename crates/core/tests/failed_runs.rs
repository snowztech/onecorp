use onecorp::store::Company;
use onecorp::work::{self, Job};
use onecorp::templates;
use std::process::Command;

#[test]
fn a_run_that_errors_before_the_agent_starts_ends_failed_with_why() {
    let root = std::env::temp_dir().join(format!("onecorp-failed-runs-{}", std::process::id()));
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

    let t = c.add_ticket("Search", "maya").unwrap();
    c.set_status(t, "queued").unwrap();
    c.fire("maya").unwrap();
    assert!(work::execute(&c, t, &Job::Start).is_err());

    assert_eq!(c.ticket(t).unwrap().unwrap().status, "failed", "not stuck in Up next");
    let events = c.events(t).unwrap();
    let (_, kind, why) = events.last().unwrap();
    assert_eq!(kind, "error");
    assert!(why.contains("no longer works"), "the Inbox card says why: {why}");

    let _ = std::fs::remove_dir_all(&root);
}
