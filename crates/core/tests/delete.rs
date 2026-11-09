//! Deleting a company removes everything OneCorp kept for it, and nothing of yours.

use onecorp::store::{self, Company};
use onecorp::{setup, ship, templates, work};
use std::process::Command;

#[test]
fn deleting_a_company_leaves_your_repo_alone() {
    let root = std::env::temp_dir().join(format!("onecorp-delete-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    // Integration tests run in their own process, so this can't leak into other tests.
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    let git = |args: &[&str]| {
        let out = Command::new("git").arg("-C").arg(&repo).args(["-c", "user.name=F", "-c", "user.email=f@x"]).args(args).output().unwrap();
        assert!(out.status.success(), "git {args:?}");
        String::from_utf8_lossy(&out.stdout).into_owned()
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# notes\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "init"]);

    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();
    Company::create("other", &repo, "").unwrap();
    store::set_current("acme").unwrap();
    let id = c.add_ticket("Write the readme", "maya").unwrap();
    work::start(&c, &c.ticket(id).unwrap().unwrap()).unwrap();
    assert_eq!(git(&["worktree", "list"]).lines().count(), 2, "the ticket has its worktree");

    // A ticket waiting for review can't be deleted. Once merged, it can, and the work stays.
    let other = c.add_ticket("Add a changelog", "maya").unwrap();
    work::start(&c, &c.ticket(other).unwrap().unwrap()).unwrap();
    assert!(work::delete(&c, &c.ticket(other).unwrap().unwrap()).unwrap_err().to_string().contains("review"));
    ship::approve(&c, &c.ticket(other).unwrap().unwrap()).unwrap();
    work::delete(&c, &c.ticket(other).unwrap().unwrap()).unwrap();
    assert!(c.ticket(other).unwrap().is_none());
    assert!(git(&["log", "--oneline"]).contains("Add a changelog"), "merged work stays in the repo");
    drop(c);

    setup::delete_company("acme").unwrap();
    assert!(!root.join("home/companies/acme").exists());
    assert_eq!(store::list().unwrap(), ["other"]);
    assert_eq!(store::current().unwrap(), "other", "you land on a company that still exists");
    assert_eq!(git(&["worktree", "list"]).lines().count(), 1, "git forgets the worktree");
    assert!(git(&["branch", "--list", "onecorp/acme/*"]).contains("onecorp/acme/"), "branches stay, they may hold work");
    assert!(repo.join("README.md").exists());
    assert!(setup::delete_company("acme").is_err());

    let _ = std::fs::remove_dir_all(&root);
}
