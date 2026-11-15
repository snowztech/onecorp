//! A review never sends back work the founder merged while it ran, and a reviewer counts as
//! working in the job queue: busy while reviewing, and held at their cap.

use onecorp::store::{Company, Employee};
use onecorp::work::{self, Job};
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git").arg("-C").arg(dir).args(["-c", "user.name=Founder", "-c", "user.email=founder@example.com"]).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

fn hire(c: &Company, id: &str, name: &str) {
    let e = Employee { id: id.into(), name: name.into(), role: "Engineer".into(), runtime: "fake".into(), model: None, bio: String::new(), department: None };
    c.hire(&e).unwrap();
}

#[test]
fn reviews_respect_the_founder_and_the_queue() {
    let root = std::env::temp_dir().join(format!("onecorp-review-queue-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# acme\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "init"]);
    let c = Company::create("acme", &repo, "").unwrap();
    hire(&c, "odile", "Odile Martin");
    hire(&c, "maya", "Maya Okafor");
    let eng = c.add_department("Engineering").unwrap();
    c.set_department("odile", eng.id).unwrap();
    c.set_department("maya", eng.id).unwrap();
    work::set_leads_review(&c, false).unwrap();

    // Merged while Odile reviewed it: her notes stay on the ticket, nothing goes back to Maya.
    let t = c.add_ticket("add the share button [changes] [merged meanwhile]", "maya").unwrap();
    work::start(&c, &c.ticket(t).unwrap().unwrap()).unwrap();
    work::ask_review(&c, &c.ticket(t).unwrap().unwrap(), None).unwrap();
    assert_eq!(c.ticket(t).unwrap().unwrap().status, "done", "still merged");
    let events = c.events(t).unwrap();
    assert!(events.iter().any(|(_, k, s)| k == "lead-review" && s.contains("changes needed")));
    assert!(!events.iter().any(|(_, k, _)| k == "review-sendback"), "not sent back");

    // In the queue, the reviewer is the one working: busy while reviewing, and held at their cap.
    let mine = c.add_ticket("Odile's own work", "odile").unwrap();
    let theirs = c.add_ticket("Maya's work to review", "maya").unwrap();
    c.enqueue(theirs, &Job::Review { reviewer: "odile".into() }).unwrap();
    c.enqueue(mine, &Job::Start).unwrap();
    let review = c.claim_job().unwrap().unwrap();
    assert_eq!(review.ticket, theirs);
    assert!(c.claim_job().unwrap().is_none(), "Odile is reviewing, so her own ticket waits");
    c.finish_job(review.id, None).unwrap();
    let start = c.claim_job().unwrap().unwrap();
    assert_eq!(start.ticket, mine);
    c.finish_job(start.id, None).unwrap();

    c.enqueue(theirs, &Job::Review { reviewer: "odile".into() }).unwrap();
    c.set_daily_cap("odile", 1.0).unwrap();
    onecorp::spend::record(&c, "ticket", "odile", None, "claude", None, 1.5, 1).unwrap();
    assert!(c.claim_job().unwrap().is_none(), "Odile is at her cap, so the review waits");

    let _ = std::fs::remove_dir_all(&root);
}
