//! Team chat: channels follow departments, agents post to their own team by default, and the
//! founder's @mentions get replies. Runs on the stand-in runtime, so no AI is needed.

use onecorp::store::Company;
use onecorp::{chat, mcp, templates};
use serde_json::json;
use std::process::Command;

#[test]
fn channels_posts_and_replies() {
    let root = std::env::temp_dir().join(format!("onecorp-chat-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    // Integration tests run in their own process, so this can't leak into other tests.
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    let git = |args: &[&str]| assert!(Command::new("git").arg("-C").arg(&repo).args(["-c", "user.name=F", "-c", "user.email=f@x"]).args(args).status().unwrap().success());
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(repo.join("README.md"), "# notes\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-q", "-m", "init"]);

    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();

    // One channel per department, after #general.
    let names: Vec<String> = chat::channels(&c).unwrap().into_iter().map(|ch| ch.name).collect();
    assert_eq!(names, ["general", "engineering", "product"]);
    assert_eq!(chat::resolve(&c, "#Eng").unwrap(), "engineering");
    assert_eq!(chat::resolve(&c, "").unwrap(), "general");
    assert!(chat::resolve(&c, "sales").unwrap_err().to_string().contains("#general, #engineering, #product"));

    // An agent's post goes to its own team unless it names a channel.
    let ticket = c.add_ticket("Login", "maya").unwrap();
    let post = |args| {
        let call = json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "post_message", "arguments": args}});
        mcp::handle(&c, &mcp::Ctx { ticket, employee: "maya".into() }, &call).unwrap()["result"]["content"][0]["text"].as_str().unwrap().to_string()
    };
    assert_eq!(post(json!({"text": "Login uses magic links"})), "Posted in #engineering.");
    assert_eq!(post(json!({"text": "Shipping Friday", "channel": "general"})), "Posted in #general.");

    // The founder @mentions someone, and they answer in the same channel.
    let slots = chat::say(&c, "engineering", "@odile @Odile can you check Maya's login?").unwrap();
    assert_eq!(slots.len(), 1, "one reply per person, however often they're mentioned");
    assert!(c.message(slots[0]).unwrap().unwrap().pending);
    chat::reply(&c, slots[0]).unwrap();
    let r = c.message(slots[0]).unwrap().unwrap();
    assert!(!r.pending && r.author == "odile" && r.text.contains("can you check Maya's login?"), "{}", r.text);
    assert!(chat::say(&c, "general", "  ").is_err());

    // Before a ticket, an engineer reads #general and #engineering. Product's channel isn't theirs.
    c.post("product", "rui", "Pricing is $6", None).unwrap();
    let maya = c.employee("maya").unwrap().unwrap();
    let seen = chat::catch_up(&c, &maya).unwrap();
    assert!(seen.contains("Login uses magic links") && seen.contains("Shipping Friday") && seen.contains("can you check"));
    assert!(!seen.contains("Pricing is $6"));

    let _ = std::fs::remove_dir_all(&root);
}
