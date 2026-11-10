//! Several repos in one company: a ticket works in its own repo, from its own worktree, and
//! approving merges there. The planner picks a repo per ticket.

use onecorp::store::{Company, NewTicket};
use onecorp::{mcp, ship, templates, work};
use serde_json::json;
use std::path::Path;
use std::process::Command;

fn repo(dir: &Path) {
    std::fs::create_dir_all(dir).unwrap();
    let git = |args: &[&str]| assert!(Command::new("git").arg("-C").arg(dir).args(["-c", "user.name=F", "-c", "user.email=f@x"]).args(args).status().unwrap().success());
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(dir.join("README.md"), "# hi\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "init"]);
}

#[test]
fn tickets_work_and_land_in_their_own_repo() {
    let root = std::env::temp_dir().join(format!("onecorp-repos-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let (app, api) = (root.join("notes-app"), root.join("notes-api"));
    repo(&app);
    repo(&api);
    // Integration tests run in their own process, so this can't leak into other tests.
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    let c = Company::create("acme", &app, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();

    let added = c.add_repo(&api, Some("api")).unwrap();
    assert_eq!(c.repos().unwrap().iter().map(|r| r.name.as_str()).collect::<Vec<_>>(), ["notes-app", "api"]);
    assert!(c.add_repo(&api, None).is_err(), "the same folder twice");
    assert!(c.remove_repo("notes-app").is_err(), "the main repo stays");

    // A ticket in the api repo branches from it and merges into it, leaving the app untouched.
    let t = c.new_ticket(&NewTicket { title: "Add search".into(), assignee: Some("maya".into()), repo: Some("api".into()), ..Default::default() }, "todo").unwrap();
    work::start(&c, &c.ticket(t).unwrap().unwrap()).unwrap();
    let ticket = c.ticket(t).unwrap().unwrap();
    assert_eq!(ticket.status, "review");
    assert_eq!(c.repo_of(&ticket).unwrap(), added.path);
    ship::approve(&c, &ticket).unwrap();
    assert!(api.join("WORK.md").exists(), "merged into the api repo");
    assert!(!app.join("WORK.md").exists(), "the app repo is untouched");

    // The planner names the repo for each ticket it creates.
    let plan = c.new_plan("Search", "Search across notes", "maya").unwrap();
    let call = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "create_ticket", "arguments": {"title": "Search endpoint", "assignee": "maya", "repo": "api"}}});
    mcp::handle(&c, &mcp::Ctx { ticket: plan, employee: "maya".into() }, &call).unwrap();
    let made = c.plan_tickets(plan).unwrap();
    assert_eq!(made[0].repo.as_deref(), Some("api"));
    let bad = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "create_ticket", "arguments": {"title": "Nope", "repo": "mobile"}}});
    let reply = mcp::handle(&c, &mcp::Ctx { ticket: plan, employee: "maya".into() }, &bad).unwrap();
    assert!(reply.to_string().contains("no repo called mobile"), "an unknown repo is refused with the list: {reply}");

    // A repo with work in progress can't be removed. Once it's done, it can.
    assert!(c.remove_repo("api").is_ok());
    let _ = std::fs::remove_dir_all(&root);
}
