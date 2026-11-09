//! Plan limits: a run that hits the agent's usage limit waits in the queue instead of failing,
//! nobody on that agent takes new work, and it picks up again once the limit resets.

use onecorp::store::Company;
use onecorp::{limits, templates, work};
use std::process::Command;

#[test]
fn a_usage_limit_queues_work_until_it_resets() {
    let root = std::env::temp_dir().join(format!("onecorp-limits-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    // Integration tests run in their own process, so this can't leak into other tests.
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    let git = |args: &[&str]| assert!(Command::new("git").arg("-C").arg(&repo).args(["-c", "user.name=F", "-c", "user.email=f@x"]).args(args).status().unwrap().success());
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# notes\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "init"]);
    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();

    let t = c.add_ticket("Search [limit]", "maya").unwrap();
    work::start(&c, &c.ticket(t).unwrap().unwrap()).unwrap();
    assert_eq!(c.ticket(t).unwrap().unwrap().status, "queued", "waits instead of failing");
    assert!(c.events(t).unwrap().iter().any(|(_, kind, s)| kind == "wait" && s.contains("usage limit")));
    assert!(limits::until("fake").is_some());
    assert!(limits::away(&c).unwrap().contains(&"odile".to_string()), "everyone on that agent waits");
    assert!(c.claim_job().unwrap().is_none());

    // The limit resets: the queued work is picked up again.
    std::fs::remove_file(root.join("home/limits/fake")).unwrap();
    assert!(c.claim_job().unwrap().is_some());

    let _ = std::fs::remove_dir_all(&root);
}
