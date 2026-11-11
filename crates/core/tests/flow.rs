//! The whole ticket loop, end to end, on a real git repo, with the fake runtime standing in for
//! an AI agent. If this passes, starting, committing under the employee's name, review, changes,
//! conflicts and merging all still work together.

use onecorp::mcp;
use onecorp::store::{Company, Employee, NewTicket};
use onecorp::work::{self, Job};
use std::path::Path;
use serde_json::{Value, json};
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
    // The employee signs their own commits here, so this test doesn't depend on your git config.
    c.set_setting("commits", "team").unwrap();
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
    onecorp::ship::approve(&c, &c.ticket(a).unwrap().unwrap()).unwrap();
    assert_eq!(status(a), "done");
    assert!(!Path::new(&wt_a).exists());
    assert!(std::fs::read_to_string(repo.join("WORK.md")).unwrap().contains("add the first line"));

    // Asking for changes resumes the same work, and the feedback goes into Tomás's memory.
    work::request_changes(&c, &c.ticket(b).unwrap().unwrap(), "mention the release date").unwrap();
    assert_eq!(status(b), "review");
    assert!(c.memory("tomas").unwrap().contains("mention the release date"));

    // main moved, so approving finds the conflict and sends it back. The fix lands on the next approve.
    onecorp::ship::approve(&c, &c.ticket(b).unwrap().unwrap()).unwrap();
    assert_eq!(status(b), "review", "the conflict went back and came back resolved");
    assert!(c.events(b).unwrap().iter().any(|(_, kind, s)| kind == "merge" && s.contains("conflicts")));
    onecorp::ship::approve(&c, &c.ticket(b).unwrap().unwrap()).unwrap();
    assert_eq!(status(b), "done");
    let work_md = std::fs::read_to_string(repo.join("WORK.md")).unwrap();
    assert!(work_md.contains("add the first line") && work_md.contains("add the second line"), "{work_md}");
    assert!(!work_md.contains("<<<<<<<"), "{work_md}");

    // A run that fails is marked failed, not left hanging.
    let f = c.add_ticket("this one breaks [fail]", "maya").unwrap();
    work::dispatch(&c, f, Job::Start).unwrap();
    assert_eq!(status(f), "failed");

    // Retrying picks the same work back up and finishes it.
    let r = c.add_ticket("flaky one [fail once]", "maya").unwrap();
    work::dispatch(&c, r, Job::Start).unwrap();
    assert_eq!(status(r), "failed");
    work::retry(&c, &c.ticket(r).unwrap().unwrap()).unwrap();
    assert_eq!(status(r), "review");
    assert!(work::retry(&c, &c.ticket(r).unwrap().unwrap()).is_err(), "only failed tickets can be retried");

    // Tickets saved for later, chained: the second starts by itself once the first is merged.
    let saved = |title: &str, who: &str, after: Option<i64>| {
        let new = NewTicket { title: title.into(), description: format!("Details for {title}"), assignee: Some(who.into()), after, ..Default::default() };
        c.new_ticket(&new, "backlog").unwrap()
    };
    let one = saved("plan step one", "maya", None);
    let two = saved("plan step two", "tomas", Some(one));
    assert_eq!((status(one), status(two)), ("backlog".to_string(), "backlog".to_string()));
    work::start(&c, &c.ticket(two).unwrap().unwrap()).unwrap();
    assert_eq!(status(two), "waiting", "it waits for step one");
    work::start(&c, &c.ticket(one).unwrap().unwrap()).unwrap();
    assert_eq!(status(one), "review");
    onecorp::ship::approve(&c, &c.ticket(one).unwrap().unwrap()).unwrap();
    assert_eq!(status(two), "review", "merging step one started step two");
    let nobody = c.new_ticket(&NewTicket { title: "someday".into(), ..Default::default() }, "backlog").unwrap();
    let err = work::start(&c, &c.ticket(nobody).unwrap().unwrap()).unwrap_err().to_string();
    assert!(err.contains("nobody on it"), "{err}");

    // Questions go to the department lead before they reach the founder.
    hire(&c, "odile", "Odile Martin");
    let eng = c.add_department("Engineering").unwrap();
    c.set_department("odile", eng.id).unwrap(); // first one in leads it
    c.set_department("maya", eng.id).unwrap();
    let q = c.add_ticket("pick a token expiry", "maya").unwrap();
    work::dispatch(&c, q, Job::Start).unwrap();
    let ask = |args: Value| {
        let ctx = mcp::Ctx { ticket: q, employee: "maya".into() };
        let call = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "ask_human", "arguments": args}});
        mcp::handle(&c, &ctx, &call).unwrap()
    };

    // The lead settles it. It never reaches the founder, and the work carries on.
    ask(json!({"question": "15 or 60 minutes?", "options": ["15", "60"], "default": "15"}));
    assert!(c.inbox().unwrap().is_empty());
    assert_eq!(c.with_leads().unwrap().len(), 1);
    work::drain_consults(&c).unwrap();
    assert!(c.with_leads().unwrap().is_empty() && c.inbox().unwrap().is_empty());
    assert_eq!(status(q), "review");

    // A founder call is passed up, with the lead's reason.
    ask(json!({"question": "Should we charge for this? [founder]"}));
    work::drain_consults(&c).unwrap();
    let mine = c.inbox().unwrap();
    assert_eq!((mine.len(), mine[0].via.as_deref()), (1, Some("odile")));
    assert!(mine[0].note.as_deref().unwrap_or("").contains("product"));

    // A blocking question the lead answers resumes the work with the lead's answer.
    ask(json!({"question": "Which hash for the tokens?", "default": "sha2", "blocking": true}));
    assert_eq!(status(q), "blocked");
    work::drain_consults(&c).unwrap();
    assert_eq!(status(q), "review");
    let wt = c.ticket(q).unwrap().unwrap().worktree.unwrap();
    assert!(std::fs::read_to_string(Path::new(&wt).join("WORK.md")).unwrap().contains("sha2"));

    // The lead's own questions go straight to the founder.
    assert!(c.lead_of("odile").unwrap().is_none());

    let _ = std::fs::remove_dir_all(&root);
}
