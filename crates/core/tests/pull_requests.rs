//! Shipping as pull requests, end to end: a bare repo stands in for GitHub's copy, and a small
//! script stands in for `gh`. No network, no account.

use onecorp::store::Employee;
use onecorp::work::{self, Job};
use onecorp::{setup, ship};
use std::path::Path;
use std::process::Command;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").arg("-C").arg(dir).args(["-c", "user.name=Founder", "-c", "user.email=founder@example.com"]).args(args).output().unwrap();
    assert!(out.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// Just enough of `gh` for OneCorp: auth status, pr create, pr view and pr merge.
/// A merge pushes the PR's branch onto its base on the bare "GitHub" repo.
const FAKE_GH: &str = r#"#!/bin/sh
state="$(dirname "$0")/gh-state"; mkdir -p "$state"
case "$1 $2" in
  "auth status") echo "Logged in to github.com account tester" ;;
  "pr create")
    shift 2
    while [ $# -gt 0 ]; do case "$1" in --head) head=$2; shift ;; --base) base=$2; shift ;; esac; shift; done
    cat > /dev/null
    n=$(( $(cat "$state/count" 2>/dev/null || echo 0) + 1 )); echo $n > "$state/count"
    echo "$head $base" > "$state/pr-$n"; echo OPEN > "$state/state-$n"
    echo "https://github.com/acme/notes/pull/$n" ;;
  "pr merge") read head base < "$state/pr-$3"; git push -q origin "$head:$base" && echo MERGED > "$state/state-$3" ;;
  "pr view") echo "{\"state\":\"$(cat "$state/state-$3")\"}" ;;
  "repo create")
    bare="$(dirname "$0")/github/$3.git"; mkdir -p "$(dirname "$bare")"; git init -q --bare -b main "$bare"
    git remote add origin "$bare" && git push -q -u origin main && echo "https://github.com/$3" ;;
  *) echo "fake gh: unexpected $*" >&2; exit 1 ;;
esac
"#;

#[test]
fn work_ships_as_pull_requests() {
    let root = std::env::temp_dir().join(format!("onecorp-prs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (repo, origin) = (root.join("repo"), root.join("origin.git"));
    std::fs::create_dir_all(&repo).unwrap();

    let gh = root.join("gh");
    std::fs::write(&gh, FAKE_GH).unwrap();
    Command::new("chmod").arg("+x").arg(&gh).status().unwrap();
    // Integration tests run in their own process, so these overrides can't leak into other tests.
    unsafe {
        std::env::set_var("ONECORP_HOME", root.join("home"));
        std::env::set_var("ONECORP_GH", &gh);
    }

    git(&root, &["init", "-q", "--bare", "-b", "main", &origin.to_string_lossy()]);
    git(&repo, &["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# notes\n").unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "init"]);
    git(&repo, &["remote", "add", "origin", &origin.to_string_lossy()]);
    git(&repo, &["push", "-q", "-u", "origin", "main"]);

    let (c, _) = setup::new_company("acme", &repo, "", &[], "").unwrap();
    assert_eq!(c.ship_mode().unwrap(), "local", "a remote that isn't on GitHub ships locally by default");
    ship::set_mode(&c, "github").unwrap();
    let maya = Employee { id: "maya".into(), name: "Maya Okafor".into(), role: "Engineer".into(), runtime: "fake".into(), model: None, bio: String::new(), department: None };
    c.hire(&maya).unwrap();
    let ticket = |id: i64| c.ticket(id).unwrap().unwrap();
    let on_github = |rev: &str| git(&origin, &["show", rev]);

    // Finished work is pushed and opened as a pull request.
    let a = c.add_ticket("first note", "maya").unwrap();
    work::dispatch(&c, a, Job::Start).unwrap();
    let t = ticket(a);
    assert_eq!((t.status.as_str(), t.pr_number), ("review", Some(1)));
    assert_eq!(t.pr_url.as_deref(), Some("https://github.com/acme/notes/pull/1"));
    assert!(on_github(&format!("{}:WORK.md", t.branch.clone().unwrap())).contains("first note"));

    // Sending it back updates the same pull request.
    work::request_changes(&c, &t, "say it louder").unwrap();
    assert_eq!(ticket(a).pr_number, Some(1));
    assert!(c.events(a).unwrap().iter().any(|(_, kind, s)| kind == "pr" && s == "updated #1"));

    // Approving merges the pull request, and your local main follows.
    ship::approve(&c, &ticket(a)).unwrap();
    assert_eq!(ticket(a).status, "done");
    assert!(on_github("main:WORK.md").contains("first note"));
    assert!(std::fs::read_to_string(repo.join("WORK.md")).unwrap().contains("first note"), "local main was fast-forwarded");

    // A pull request merged on GitHub itself is picked up.
    let b = c.add_ticket("second note", "maya").unwrap();
    work::dispatch(&c, b, Job::Start).unwrap();
    assert_eq!(ticket(b).pr_number, Some(2));
    let merged_on_github = Command::new(&gh).args(["pr", "merge", "2"]).current_dir(&repo).status().unwrap();
    assert!(merged_on_github.success());
    assert_eq!(ship::sync(&c).unwrap(), vec!["ACM-2 was merged on GitHub".to_string()]);
    assert_eq!(ticket(b).status, "done");

    // So is one closed without merging. It stops, and can be retried.
    let d = c.add_ticket("third note", "maya").unwrap();
    work::dispatch(&c, d, Job::Start).unwrap();
    std::fs::write(root.join("gh-state/state-3"), "CLOSED").unwrap();
    ship::sync(&c).unwrap();
    assert_eq!(ticket(d).status, "failed");

    // Starting from nothing: a new repo on your Mac, and the same repo on "GitHub".
    let fresh = setup::create_repo(&root.join("ideas"), "Ideas", "Notes for later", Some(setup::Remote { full_name: "snowztech/ideas", private: true })).unwrap();
    assert!(std::fs::read_to_string(fresh.join("README.md")).unwrap().contains("Notes for later"));
    let pushed = root.join("github/snowztech/ideas.git");
    assert!(git(&pushed, &["log", "--oneline", "main"]).contains("Start"));
    assert!(setup::create_repo(&fresh, "Again", "", None).is_err(), "never overwrites a folder with files in it");

    let _ = std::fs::remove_dir_all(&root);
}
