//! A ticket that changes no files, like an audit, ends as a report: no branch to merge, its reply
//! is the work, and closing it starts the tickets it proposed.

use onecorp::store::Company;
use onecorp::work::{self, Job};
use onecorp::{ship, store::Employee};
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git").arg("-C").arg(dir).args(["-c", "user.name=Founder", "-c", "user.email=founder@example.com"]).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn an_audit_is_a_report_and_starts_what_it_proposed() {
    let root = std::env::temp_dir().join(format!("onecorp-reports-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# acme\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "init"]);

    let c = Company::create("acme", &repo, "").unwrap();
    let e = Employee { id: "ines".into(), name: "Inês Costa".into(), role: "Growth".into(), runtime: "fake".into(), model: None, bio: String::new(), department: None };
    c.hire(&e).unwrap();

    let id = c.add_ticket("Audit our SEO [report]", "ines").unwrap();
    work::dispatch(&c, id, Job::Start).unwrap();
    let t = c.ticket(id).unwrap().unwrap();
    assert_eq!((t.status.as_str(), t.kind.as_str()), ("review", "report"));
    assert!(t.pr_number.is_none());

    // The review shows the report and what it proposes.
    let snap = onecorp::view::snapshot(&c).unwrap();
    let review = snap["reviews"].as_array().unwrap().iter().find(|r| r["kind"] == "report").unwrap().clone();
    assert!(review["document"].as_str().unwrap().contains("# SEO audit"));
    assert_eq!(review["tickets"][0]["title"], "Add meta descriptions");

    // Closing it cleans up its worktree and starts the proposal.
    ship::approve(&c, &t).unwrap();
    assert_eq!(c.ticket(id).unwrap().unwrap().status, "done");
    assert!(!Path::new(t.worktree.as_deref().unwrap()).exists());
    let follow = c.plan_tickets(id).unwrap().remove(0);
    assert_eq!((follow.status.as_str(), follow.kind.as_str()), ("review", "work"));
    let _ = std::fs::remove_dir_all(&root);
}
