//! The whole ticket loop, end to end, on a real git repo, with the fake runtime standing in for
//! an AI agent. If this passes, starting, committing under the employee's name, review, changes,
//! conflicts and merging all still work together.

use onecorp::store::{Company, Employee};
use onecorp::work::{self, Job};
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(dir).args(["-c", "user.name=Founder", "-c", "user.email=founder@example.com"]).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn hire(c: &Company, id: &str, name: &str) {
    let e = Employee {
        id: id.into(),
        name: name.into(),
        role: "Engineer".into(),
        runtime: "fake".into(),
        model: None,
        bio: String::new(),
        department: None,
    };
    c.hire(&e).unwrap();
}

#[test]
fn a_ticket_goes_from_start_to_merged() {
    let root = std::env::temp_dir().join(format!("onecorp-flow-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    // Integration tests run in their own process, so pointing the home somewhere else is safe here.
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };

    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# acme\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "init"]);

    let c = Company::create("acme", &repo, "Make notes simple").unwrap();
    hire(&c, "maya", "Maya Okafor");
    hire(&c, "tomas", "Tomás Reis");
    let status = |id: i64| c.ticket(id).unwrap().unwrap().status;

    // Two tickets from the same main. Both write WORK.md, so the second one will conflict.
    let a = c.add_ticket("add the first line", "maya").unwrap();
    let b = c.add_ticket("add the second line", "tomas").unwrap();
    work::dispatch(&c, a, Job::Start).unwrap();
    work::dispatch(&c, b, Job::Start).unwrap();
    assert_eq!((status(a), status(b)), ("review".to_string(), "review".to_string()));

    // The work is committed on the ticket's branch, under the employee's name.
    let branch_a = c.ticket(a).unwrap().unwrap().branch.unwrap();
    assert_eq!(git(&repo, &["log", "-1", "--format=%an <%ae>", &branch_a]), "Maya Okafor <maya@acme.onecorp>");
    assert!(c.events(a).unwrap().iter().any(|(_, kind, _)| kind == "commit"));

    // Approving lands it on main and cleans up the worktree.
    let wt_a = c.ticket(a).unwrap().unwrap().worktree.unwrap();
    work::approve(&c, &c.ticket(a).unwrap().unwrap()).unwrap();
    assert_eq!(status(a), "done");
    assert!(!Path::new(&wt_a).exists());
    assert!(std::fs::read_to_string(repo.join("WORK.md")).unwrap().contains("add the first line"));

    // Asking for changes resumes the same work, and the feedback goes into Tomás's memory.
    work::request_changes(&c, &c.ticket(b).unwrap().unwrap(), "mention the release date").unwrap();
    assert_eq!(status(b), "review");
    assert!(c.memory("tomas").unwrap().contains("mention the release date"));

    // main moved, so approving finds the conflict and sends it back. The fix lands on the next approve.
    work::approve(&c, &c.ticket(b).unwrap().unwrap()).unwrap();
    assert_eq!(status(b), "review", "the conflict went back and came back resolved");
    assert!(c.events(b).unwrap().iter().any(|(_, kind, s)| kind == "merge" && s.contains("conflicts")));
    work::approve(&c, &c.ticket(b).unwrap().unwrap()).unwrap();
    assert_eq!(status(b), "done");
    let work_md = std::fs::read_to_string(repo.join("WORK.md")).unwrap();
    assert!(work_md.contains("add the first line") && work_md.contains("add the second line"), "{work_md}");
    assert!(!work_md.contains("<<<<<<<"), "{work_md}");

    // A run that fails is marked failed, not left hanging.
    let f = c.add_ticket("this one breaks [fail]", "maya").unwrap();
    work::dispatch(&c, f, Job::Start).unwrap();
    assert_eq!(status(f), "failed");

    let _ = std::fs::remove_dir_all(&root);
}
