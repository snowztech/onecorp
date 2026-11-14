//! A run stops at what's left of the day's budget, and its ticket waits in line with its work kept.

use onecorp::store::Company;
use onecorp::{spend, templates, work};
use std::process::Command;

#[test]
fn a_run_stops_at_the_budget_and_waits() {
    let root = std::env::temp_dir().join(format!("onecorp-budget-stop-{}", std::process::id()));
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

    // No budget, no ceiling. With one, a run gets what's left, and a person's cap counts when it's less.
    assert_eq!(spend::left(&c, "maya").unwrap(), None);
    spend::set_budget(&c, 2.0).unwrap();
    spend::record(&c, "chat", "odile", None, "claude", None, 0.5, 1).unwrap();
    assert_eq!(spend::left(&c, "maya").unwrap(), Some(1.5));
    c.set_daily_cap("maya", 1.0).unwrap();
    assert_eq!(spend::left(&c, "maya").unwrap(), Some(1.0));
    c.set_daily_cap("maya", 0.0).unwrap();

    let t = c.add_ticket("Search [budget]", "maya").unwrap();
    work::start(&c, &c.ticket(t).unwrap().unwrap()).unwrap();
    assert!((spend::spent_today(&c).unwrap() - 2.0).abs() < 1e-9, "the run spent what was left and no more");
    assert!(spend::paused_by_budget(&c).unwrap());
    assert_eq!(c.ticket(t).unwrap().unwrap().status, "queued", "waits instead of failing");
    assert!(c.events(t).unwrap().iter().any(|(_, kind, s)| kind == "wait" && s.contains("daily budget")));

    // Going on for today lifts the ceiling with the pause.
    spend::resume_for_today(&c).unwrap();
    assert_eq!(spend::left(&c, "maya").unwrap(), None);
    assert!(c.claim_job().unwrap().is_some());

    let _ = std::fs::remove_dir_all(&root);
}
