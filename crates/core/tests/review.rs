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

    // Leads review first by default: work reaches the founder with its verdict, nobody asked.
    assert!(work::leads_review(&c).unwrap());
    let auto = c.add_ticket("add the share button", "maya").unwrap();
    work::dispatch(&c, auto, Job::Start).unwrap();
    assert_eq!(card(&c, &c.key(auto))["lead_review"]["ok"], true);

    // Turned off, work comes straight to the founder, who can still ask from the Inbox.
    work::set_leads_review(&c, false).unwrap();
    let a = c.add_ticket("add the export button", "maya").unwrap();
    work::dispatch(&c, a, Job::Start).unwrap();
    let key_a = c.key(a);
    assert!(card(&c, &key_a)["lead_review"].is_null());
    assert_eq!(card(&c, &key_a)["lead"]["name"], "Odile Martin");
    assert_eq!(work::ask_review(&c, &c.ticket(a).unwrap().unwrap(), None).unwrap(), "Odile Martin");
    assert_eq!(status(a), "review");
    let review = card(&c, &key_a);
    assert_eq!((review["lead_review"]["name"].as_str(), review["lead_review"]["ok"].as_bool()), (Some("Odile Martin"), Some(true)));
    assert!(review["reviewing"].is_null());
    let board = view::snapshot(&c).unwrap()["tickets"].as_array().unwrap().iter().find(|t| t["key"] == key_a).cloned().unwrap();
    assert_eq!(board["reviewed"], json!({"name": "Odile Martin", "state": "good"}), "the Board card shows the badge");
    let said = chat::home(&c, &c.employee("maya").unwrap().unwrap()).unwrap();
    assert!(c.messages(Some(&said)).unwrap().iter().any(|m| m.author == "odile" && m.text.contains("looks good")));

    // Nobody reviews their own work, and only work in review can be reviewed.
    let err = work::ask_review(&c, &c.ticket(a).unwrap().unwrap(), Some("Maya")).unwrap_err().to_string();
    assert!(err.contains("own work"), "{err}");
    let later = c.add_ticket("someday", "maya").unwrap();
    assert!(work::ask_review(&c, &c.ticket(later).unwrap().unwrap(), None).is_err());

    // Asked by the author while working: the review starts once the work is in review. Changes go
    // straight back to the author, as a comment on the ticket and in their memory, and they redo it.
    let b = c.add_ticket("add the import button [changes]", "maya").unwrap();
    let ctx = mcp::Ctx { ticket: b, employee: "maya".into() };
    let call = |args: Value| mcp::handle(&c, &ctx, &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "request_review", "arguments": args}})).unwrap();
    assert_eq!(call(json!({"reviewer": "Zed"}))["result"]["isError"], true, "a name nobody has fails while the agent can fix it");
    call(json!({}));
    work::dispatch(&c, b, Job::Start).unwrap();
    assert_eq!(status(b), "review", "sent back, redone, and back in review");
    let kinds = |id: i64, kind: &str| c.events(id).unwrap().iter().filter(|(_, k, _)| k == kind).count();
    assert_eq!(kinds(b, "review-sendback"), 1);
    assert!(c.events(b).unwrap().iter().any(|(_, k, s)| k == "comment" && s.starts_with("Odile Martin: ") && s.contains("Add a test")));
    assert!(c.memory("maya").unwrap().contains("Add a test"));
    // Reviews are off here and nobody asked again, so the redone work comes to the founder as it is.
    assert!(card(&c, &c.key(b))["lead_review"].is_null());

    // With leads reviewing first, a lead who keeps asking for changes sends the work back twice,
    // then it's the founder's call, with the verdict and how many rounds it took.
    work::set_leads_review(&c, true).unwrap();
    let d = c.add_ticket("rewrite the sync engine [changes]", "maya").unwrap();
    work::dispatch(&c, d, Job::Start).unwrap();
    assert_eq!(status(d), "review");
    assert_eq!(kinds(d, "review-sendback"), work::MAX_REVIEW_ROUNDS);
    let review = card(&c, &c.key(d));
    assert_eq!((review["lead_review"]["ok"].as_bool(), review["lead_review"]["rounds"].as_u64()), (Some(false), Some(2)));
    assert!(review["lead_review"]["notes"].as_str().unwrap().contains("Add a test"));

    // The founder asking for changes starts the count again.
    work::set_leads_review(&c, false).unwrap();
    work::request_changes(&c, &c.ticket(b).unwrap().unwrap(), "add the test Odile asked for").unwrap();
    assert_eq!(status(b), "review");
    assert_eq!(work::sent_back(&c, b).unwrap(), 0);

    // From chat: the lead asks to review it, does, and sends it back.
    let channel = chat::home(&c, &c.employee("maya").unwrap().unwrap()).unwrap();
    let slots = chat::say(&c, &channel, &format!("@odile [review] {} please", c.key(b))).unwrap();
    chat::reply(&c, slots[0]).unwrap();
    assert!(!c.message(slots[0]).unwrap().unwrap().text.contains("REVIEW:"), "the line is an action, not part of the reply");
    assert_eq!(kinds(b, "review-sendback"), 2);
    assert_eq!(status(b), "review");

    let _ = std::fs::remove_dir_all(&root);
}
