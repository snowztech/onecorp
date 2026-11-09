//! Team chat: channels follow departments, agents post to their own team by default, and the
//! founder's @mentions get replies. Runs on the stand-in runtime, so no AI is needed.

use onecorp::store::Company;
use onecorp::{chat, mcp, templates, work};
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

    // @all calls the channel: the whole company in #general, the team in a department's channel.
    assert_eq!(chat::say(&c, "engineering", "@all standup in 5").unwrap().len(), 2, "odile and maya");
    assert_eq!(chat::say(&c, "general", "@everyone shipping Friday, @maya too").unwrap().len(), 4, "everyone once, even when named twice");

    // Before a ticket, an engineer reads #general and #engineering. Product's channel isn't theirs.
    c.post("product", "rui", "Pricing is $6", None).unwrap();
    let maya = c.employee("maya").unwrap().unwrap();
    let seen = chat::catch_up(&c, &maya).unwrap();
    assert!(seen.contains("Login uses magic links") && seen.contains("Shipping Friday") && seen.contains("can you check"));
    assert!(!seen.contains("Pricing is $6"));

    // A direct message: no @ needed, it always gets its answer, and stays out of the channel list.
    let slots = chat::say(&c, "dm:rui", "how is the roadmap looking?").unwrap();
    assert_eq!(slots.len(), 1);
    chat::reply(&c, slots[0]).unwrap();
    assert_eq!(c.message(slots[0]).unwrap().unwrap().author, "rui");
    assert!(chat::channels(&c).unwrap().iter().all(|ch| !ch.name.starts_with("dm:")));
    assert!(chat::catch_up(&c, &c.employee("rui").unwrap().unwrap()).unwrap().contains("(direct message)"));
    assert!(!chat::catch_up(&c, &maya).unwrap().contains("roadmap"), "a DM is private");
    assert!(chat::resolve(&c, "dm:nobody").is_err());

    // A chosen face and a photo, kept with the company.
    let photo = root.join("rui.png");
    std::fs::write(&photo, b"png").unwrap();
    c.set_face("rui", Some("Rui Alves 3")).unwrap();
    c.set_photo("rui", Some(&photo)).unwrap();
    let (seed, file) = c.avatar("rui").unwrap();
    assert_eq!(seed.as_deref(), Some("Rui Alves 3"));
    assert!(file.unwrap().ends_with("avatars/rui.png"));
    c.set_photo("rui", None).unwrap();
    assert!(c.avatar("rui").unwrap().1.is_none());

    // No brief yet: whoever writes the wiki drafts what we're building, so planning has it.
    assert!(c.brief().unwrap().is_empty());
    work::write_wiki(&c, None).unwrap();
    assert_eq!(c.brief().unwrap().trim(), "A notes app for writers, from the README.");

    // Everyone has a personality, kept until you change it, and it reaches their prompts.
    let maya = c.employee("maya").unwrap().unwrap();
    let voice = onecorp::voice::of(&c, &maya).unwrap();
    assert_eq!(onecorp::voice::of(&c, &maya).unwrap(), voice);
    onecorp::voice::set(&c, "maya", "thorough").unwrap();
    assert_eq!(onecorp::voice::of(&c, &maya).unwrap(), "thorough");
    onecorp::voice::set(&c, "maya", "terse").unwrap();
    assert!(onecorp::voice::intro(&c, &maya).unwrap().contains("You are terse"));
    assert!(onecorp::voice::set(&c, "maya", "rude").is_err());
    let cto = onecorp::store::Employee { role: "CTO".into(), id: "odile".into(), ..maya.clone() };
    assert_eq!(onecorp::voice::of(&c, &cto).unwrap(), "calm", "a lead starts calm");

    // You can take back a message.
    let id = c.post("general", "founder", "oops", None).unwrap();
    c.delete_message(id).unwrap();
    assert!(c.message(id).unwrap().is_none());

    let _ = std::fs::remove_dir_all(&root);
}
