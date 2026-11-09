//! From an idea to running work: a brief, the planner's plan, and starting it. The stand-in
//! runtime plans through the real company tools, so this needs no AI.

use onecorp::store::Company;
use onecorp::{ship, templates, work};
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git").arg("-C").arg(dir).args(["-c", "user.name=Founder", "-c", "user.email=founder@example.com"]).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn an_idea_becomes_a_plan_becomes_work() {
    let root = std::env::temp_dir().join(format!("onecorp-kickoff-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    // Integration tests run in their own process, so this can't leak into other tests.
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# notes\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "init"]);

    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();
    // The demo team has a product manager, Rui, who plans.
    let status = |id: i64| c.ticket(id).unwrap().unwrap().status;

    // No brief, no kickoff.
    assert!(work::kickoff(&c, None).unwrap_err().to_string().contains("brief"));
    c.write_brief("A notes app for writers, with magic link login.").unwrap();

    // The product manager plans it, read-only, and the plan comes back for review.
    let plan = work::kickoff(&c, None).unwrap();
    let p = c.ticket(plan).unwrap().unwrap();
    assert_eq!((p.kind.as_str(), p.assignee.as_deref(), p.status.as_str()), ("plan", Some("rui"), "review"));
    let tickets = c.plan_tickets(plan).unwrap();
    assert_eq!(tickets.len(), 3);
    assert!(tickets.iter().all(|t| t.status == "backlog" && t.assignee.is_some() && t.created_by == "rui"));
    assert!(tickets[0].title.contains("A notes app for writers"), "the plan comes from the brief");
    assert_eq!((tickets[1].after, tickets[2].after), (Some(tickets[0].id), Some(tickets[0].id)));
    assert!(git_status_clean(&repo), "planning never touches your checkout");

    // Planning again replaces the tickets instead of piling up new ones.
    work::request_changes(&c, &p, "keep it smaller").unwrap();
    let redone = c.plan_tickets(plan).unwrap();
    assert_eq!(redone.len(), 3);
    assert!(redone.iter().all(|t| !tickets.iter().any(|old| old.id == t.id)));

    // Starting the plan starts what can start. The rest waits its turn.
    ship::approve(&c, &c.ticket(plan).unwrap().unwrap()).unwrap();
    assert_eq!(status(plan), "done");
    let (setup, core, docs) = (redone[0].id, redone[1].id, redone[2].id);
    assert_eq!((status(setup), status(core), status(docs)), ("review".into(), "waiting".into(), "waiting".into()));
    ship::approve(&c, &c.ticket(setup).unwrap().unwrap()).unwrap();
    assert_eq!((status(core), status(docs)), ("review".into(), "review".into()), "merging setup started the rest");

    // A dismissed plan takes its tickets with it, and retrying plans again from scratch.
    let second = work::kickoff(&c, Some("rui")).unwrap();
    let gone: Vec<i64> = c.plan_tickets(second).unwrap().iter().map(|t| t.id).collect();
    work::dismiss_plan(&c, &c.ticket(second).unwrap().unwrap()).unwrap();
    assert_eq!(status(second), "failed");
    assert!(gone.iter().all(|id| c.ticket(*id).unwrap().is_none()));
    work::retry(&c, &c.ticket(second).unwrap().unwrap()).unwrap();
    assert_eq!((status(second), c.plan_tickets(second).unwrap().len()), ("review".into(), 3));
    assert!(repo.join("README.md").exists());

    // Paused, nothing starts. Resuming lets queued work go.
    c.set_paused(true).unwrap();
    let waiting = work::plan_project(&c, "Search", "Full-text search over notes", Some("rui")).unwrap();
    assert_eq!(status(waiting), "queued");
    assert!(onecorp::chat::say(&c, "general", "@rui how is search going?").unwrap().is_empty(), "no paid replies while paused");
    c.set_paused(false).unwrap();
    work::delete(&c, &c.ticket(waiting).unwrap().unwrap()).unwrap_err();

    // A later project plans from its own goal, and shows up with its progress.
    let billing = work::plan_project(&c, "Stripe billing", "Pro plan at $6 a month, with a free tier", Some("rui")).unwrap();
    let b = c.ticket(billing).unwrap().unwrap();
    assert_eq!((b.title.as_str(), b.description.as_str(), b.status.as_str()), ("Stripe billing", "Pro plan at $6 a month, with a free tier", "review"));
    assert!(work::plan_project(&c, "  ", "", None).is_err());
    let snap = onecorp::view::snapshot(&c).unwrap();
    let p = snap["projects"].as_array().unwrap().iter().find(|p| p["title"] == "Stripe billing").unwrap().clone();
    assert_eq!((p["total"].as_u64(), p["done"].as_u64()), (Some(3), Some(0)));

    let _ = std::fs::remove_dir_all(&root);
}

fn git_status_clean(repo: &Path) -> bool {
    let out = Command::new("git").arg("-C").arg(repo).args(["status", "--porcelain"]).output().unwrap();
    out.stdout.is_empty()
}
