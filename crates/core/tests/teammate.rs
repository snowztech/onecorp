//! An employee asks a teammate during a ticket and gets the answer back, posted in chat and kept
//! on the ticket. A mention in a plain post says how to get an answer instead.

use onecorp::store::Company;
use onecorp::{mcp, templates};
use serde_json::json;
use std::process::Command;

#[test]
fn asking_a_teammate_gets_an_answer() {
    let root = std::env::temp_dir().join(format!("onecorp-teammate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    let git = |args: &[&str]| assert!(Command::new("git").arg("-C").arg(&repo).args(["-c", "user.name=F", "-c", "user.email=f@x"]).args(args).status().unwrap().success());
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# notes\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "init"]);
    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();

    // Lina, in product, asks Maya, in engineering, during her design ticket.
    let ticket = c.add_ticket("Design the watch view", "lina").unwrap();
    let call = |name: &str, args| {
        let msg = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": name, "arguments": args}});
        let r = mcp::handle(&c, &mcp::Ctx { ticket, employee: "lina".into() }, &msg).unwrap();
        r["result"]["content"][0]["text"].as_str().unwrap().to_string()
    };
    let answer = call("ask_teammate", json!({"who": "Maya", "question": "Does each country carry its own link?"}));
    assert!(answer.starts_with("Maya answered in #general:"), "{answer}");
    assert!(answer.contains("Does each country carry its own link?"), "the stand-in quotes what it was asked: {answer}");

    // Both sides are in chat, and the exchange is on the ticket.
    let said: Vec<(String, String)> = c.messages(Some("general")).unwrap().into_iter().map(|m| (m.author, m.text)).collect();
    assert_eq!(said[said.len() - 2], ("lina".into(), "@maya Does each country carry its own link?".into()));
    assert_eq!(said[said.len() - 1].0, "maya");
    assert!(c.events(ticket).unwrap().iter().any(|(_, k, s)| k == "teammate" && s.starts_with("asked Maya:")));

    // Nobody by that name, yourself, and a paused company each say what to do instead.
    assert!(call("ask_teammate", json!({"who": "Zed", "question": "?"})).contains("Your teammates:"));
    assert!(call("ask_teammate", json!({"who": "lina", "question": "?"})).contains("that's you"));
    c.set_paused(true).unwrap();
    assert!(call("ask_teammate", json!({"who": "maya", "question": "?"})).contains("is paused"));
    c.set_paused(false).unwrap();

    // At most five a ticket.
    for _ in 0..4 {
        call("ask_teammate", json!({"who": "maya", "question": "one more?"}));
    }
    assert!(call("ask_teammate", json!({"who": "maya", "question": "and another?"})).contains("5 times"));

    // A mention in a plain post isn't answered now, and the post says so.
    let posted = call("post_message", json!({"text": "@maya the spec is on my branch"}));
    assert!(posted.contains("Maya Okafor will read it with their next ticket") && posted.contains("ask_teammate"), "{posted}");

    let _ = std::fs::remove_dir_all(&root);
}
