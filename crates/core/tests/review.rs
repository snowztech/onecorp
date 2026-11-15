//! A lead reviews their team's work before it reaches the founder: asked from the Inbox, by the
//! author through `request_review`, or by the lead from chat. The work stays in review either way.

use onecorp::store::{Company, Employee};
use onecorp::work::{self, Job};
use onecorp::{chat, mcp, view};
use serde_json::{Value, json};
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

/// The verdict the Inbox shows on a ticket's review card.
fn card(c: &Company, key: &str) -> Value {
    let snap = view::snapshot(c).unwrap();
    snap["reviews"].as_array().unwrap().iter().find(|r| r["key"] == key).cloned().unwrap()
}

#[test]
fn a_lead_reviews_work_before_the_founder() {
    let root = std::env::temp_dir().join(format!("onecorp-review-{}", std::process::id()));
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
    c.set_setting("commits", "team").unwrap();
    hire(&c, "odile", "Odile Martin");
    hire(&c, "maya", "Maya Okafor");
    let eng = c.add_department("Engineering").unwrap();
    c.set_department("odile", eng.id).unwrap(); // first one in leads it
    c.set_department("maya", eng.id).unwrap();
    let status = |id: i64| c.ticket(id).unwrap().unwrap().status;

    // Asked from the Inbox: the lead reads the work and says it's good. It stays the founder's to merge.
    let a = c.add_ticket("add the export button", "maya").unwrap();
    work::dispatch(&c, a, Job::Start).unwrap();
    let key_a = c.key(a);
    assert_eq!(card(&c, &key_a)["lead"]["name"], "Odile Martin");
    assert_eq!(work::ask_review(&c, &c.ticket(a).unwrap().unwrap(), None).unwrap(), "Odile Martin");
    assert_eq!(status(a), "review");
    let review = card(&c, &key_a);
    assert_eq!((review["lead_review"]["name"].as_str(), review["lead_review"]["ok"].as_bool()), (Some("Odile Martin"), Some(true)));
    assert!(review["reviewing"].is_null());
    let said = chat::home(&c, &c.employee("maya").unwrap().unwrap()).unwrap();
    assert!(c.messages(Some(&said)).unwrap().iter().any(|m| m.author == "odile" && m.text.contains("looks good")));

    // Nobody reviews their own work, and only work in review can be reviewed.
    let err = work::ask_review(&c, &c.ticket(a).unwrap().unwrap(), Some("Maya")).unwrap_err().to_string();
    assert!(err.contains("own work"), "{err}");
    let later = c.add_ticket("someday", "maya").unwrap();
    assert!(work::ask_review(&c, &c.ticket(later).unwrap().unwrap(), None).is_err());

    // Asked by the author while working: the review starts once the work is in review, with the notes.
    let b = c.add_ticket("add the import button [changes]", "maya").unwrap();
    let ctx = mcp::Ctx { ticket: b, employee: "maya".into() };
    let call = |args: Value| mcp::handle(&c, &ctx, &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "request_review", "arguments": args}})).unwrap();
    assert_eq!(call(json!({"reviewer": "Zed"}))["result"]["isError"], true, "a name nobody has fails while the agent can fix it");
    call(json!({}));
    work::dispatch(&c, b, Job::Start).unwrap();
    assert_eq!(status(b), "review");
    let review = card(&c, &c.key(b));
    assert_eq!(review["lead_review"]["ok"], false);
    assert!(review["lead_review"]["notes"].as_str().unwrap().contains("Add a test"));

    // Sent back and redone, the old verdict no longer applies, and nobody reviews it again unasked.
    work::request_changes(&c, &c.ticket(b).unwrap().unwrap(), "add the test Odile asked for").unwrap();
    assert_eq!(status(b), "review");
    assert!(card(&c, &c.key(b))["lead_review"].is_null());

    // From chat: the lead asks to review it and does.
    let channel = chat::home(&c, &c.employee("maya").unwrap().unwrap()).unwrap();
    let slots = chat::say(&c, &channel, &format!("@odile [review] {} please", c.key(b))).unwrap();
    chat::reply(&c, slots[0]).unwrap();
    assert!(!c.message(slots[0]).unwrap().unwrap().text.contains("REVIEW:"), "the line is an action, not part of the reply");
    assert!(!card(&c, &c.key(b))["lead_review"].is_null());

    let _ = std::fs::remove_dir_all(&root);
}
