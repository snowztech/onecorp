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

    // You, as git knows you in that repo.
    for (k, v) in [("user.name", "Lucas Founder"), ("user.email", "lucas@example.com")] {
        assert!(Command::new("git").arg("-C").arg(&api).args(["config", k, v]).status().unwrap().success());
    }
    let last = |branch: &str| String::from_utf8(Command::new("git").arg("-C").arg(&api).args(["log", "-1", "--format=%an|%cn|%B", branch]).output().unwrap().stdout).unwrap();

    // A ticket in the api repo branches from it and merges into it, leaving the app untouched.
    c.set_setting("commits", "team").unwrap();
    let t = c.new_ticket(&NewTicket { title: "Add search".into(), assignee: Some("maya".into()), repo: Some("api".into()), ..Default::default() }, "todo").unwrap();
    work::start(&c, &c.ticket(t).unwrap().unwrap()).unwrap();
    let ticket = c.ticket(t).unwrap().unwrap();
    assert_eq!(ticket.status, "review");
    assert_eq!(c.repo_of(&ticket).unwrap(), added.path);
    // As your team: Maya wrote it, you committed it and share the credit.
    let log = last(ticket.branch.as_deref().unwrap());
    assert!(log.starts_with("Maya Okafor|Lucas Founder|"), "{log}");
    assert!(log.contains("Co-authored-by: Lucas Founder <lucas@example.com>"), "{log}");
    ship::approve(&c, &ticket).unwrap();
    assert!(api.join("WORK.md").exists(), "merged into the api repo");
    assert!(!app.join("WORK.md").exists(), "the app repo is untouched");

    // Being on a feature branch in your checkout doesn't change where new work starts.
    assert!(Command::new("git").arg("-C").arg(&api).args(["checkout", "-q", "-b", "feat/wip"]).status().unwrap().success());
    c.delete_setting("commits").unwrap();
    let t2 = c.new_ticket(&NewTicket { title: "Add tags".into(), assignee: Some("maya".into()), repo: Some("api".into()), ..Default::default() }, "todo").unwrap();
    work::start(&c, &c.ticket(t2).unwrap().unwrap()).unwrap();
    assert_eq!(c.ticket(t2).unwrap().unwrap().base.as_deref(), Some("main"), "starts from main, not feat/wip");
    // As you, the default once git knows you: every commit is yours alone.
    let log = last(c.ticket(t2).unwrap().unwrap().branch.as_deref().unwrap());
    assert!(log.starts_with("Lucas Founder|Lucas Founder|") && !log.contains("Co-authored-by"), "{log}");

    // A pull request ends with who did it and where, unless you want it plain.
    let signed = ship::signoff(&c, &c.ticket(t2).unwrap().unwrap()).unwrap();
    assert!(signed.contains(&format!("Maya Okafor, Engineer at {}", c.name)) && signed.contains("[OneCorp](https://onecorp.dev)"), "{signed}");
    c.set_setting("pr_sign", "off").unwrap();
    assert_eq!(ship::signoff(&c, &c.ticket(t2).unwrap().unwrap()).unwrap(), "");

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
    assert!(c.remove_repo("api").is_err(), "Add tags is still in review there");
    // Approving lands on main even while the checkout sits on feat/wip.
    ship::approve(&c, &c.ticket(t2).unwrap().unwrap()).unwrap();
    let on_main = Command::new("git").arg("-C").arg(&api).args(["show", "main:WORK.md"]).output().unwrap();
    assert!(on_main.status.success(), "the work is on main");
    assert!(c.remove_repo("api").is_ok());
    // What the team can run. New companies run anything. A list never keeps what can't run.
    use onecorp::commands;
    assert!(commands::run_any(&c).unwrap(), "new companies run any command");
    let main = c.repos().unwrap().remove(0);
    commands::set(&c, &main, "make test\ngit rebase\nsudo ls\nbash tools/check.sh").unwrap();
    assert_eq!(commands::list(&c, &main).unwrap(), ["make test", "bash tools/check.sh"], "git and sudo are never on a list");
    std::fs::write(app.join("package.json"), r#"{"scripts":{"test":"node t.js"}}"#).unwrap();
    commands::rescan(&c, &main).unwrap();
    let after = commands::list(&c, &main).unwrap();
    assert!(after.starts_with(&["make test".to_string(), "bash tools/check.sh".to_string()]) && after.len() > 2, "rescanning adds and keeps yours: {after:?}");

    // Several commands in one request, and none at all once the team runs anything.
    commands::set_run_any(&c, false).unwrap();
    let t = c.add_ticket("needs tools", "maya").unwrap();
    let ask = |args| mcp::handle(&c, &mcp::Ctx { ticket: t, employee: "maya".into() }, &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "ask_to_run", "arguments": args}})).unwrap();
    ask(json!({"commands": ["docker compose up", "make test -v", "go vet ./..."], "why": "integration tests"}));
    let q = c.inbox().unwrap().into_iter().find(|q| q.ticket_id == Some(t) && q.kind == "command").unwrap();
    assert_eq!(q.question, "docker compose up\ngo vet ./...", "one request, without what's already allowed");
    commands::allow(&c, &c.ticket(t).unwrap().unwrap(), &q.question).unwrap();
    assert!(commands::list(&c, &main).unwrap().iter().any(|x| x == "go vet ./..."), "allowing it adds each one");
    commands::set_run_any(&c, true).unwrap();
    let t2 = c.add_ticket("more tools", "maya").unwrap();
    let out = mcp::handle(&c, &mcp::Ctx { ticket: t2, employee: "maya".into() }, &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "ask_to_run", "arguments": {"command": "npm ci", "why": "deps"}}})).unwrap();
    assert!(out.to_string().contains("already run any command"), "{out}");

    // Making another repo primary keeps every ticket and list with its own repo.
    if c.repo_named("api").is_err() {
        c.add_repo(&api, Some("api")).unwrap();
    }
    let on_app = c.add_ticket("app work", "maya").unwrap();
    let api_list = commands::list(&c, &c.repo_named("api").unwrap()).unwrap();
    c.make_main("api").unwrap();
    let names: Vec<String> = c.repos().unwrap().into_iter().map(|r| r.name).collect();
    assert_eq!(names, ["api", "notes-app"]);
    assert_eq!(c.ticket(on_app).unwrap().unwrap().repo.as_deref(), Some("notes-app"));
    assert_eq!(c.repo_of(&c.ticket(on_app).unwrap().unwrap()).unwrap(), app.canonicalize().unwrap());
    assert_eq!(commands::list(&c, &c.repos().unwrap()[0]).unwrap(), api_list, "the api keeps its own commands");
    assert!(commands::list(&c, &c.repo_named("notes-app").unwrap()).unwrap().contains(&"bash tools/check.sh".to_string()));
    let _ = std::fs::remove_dir_all(&root);
}
