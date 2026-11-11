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

    // A reply knows its own tickets, and only its own.
    let mine = chat::own_tickets(&c, "maya").unwrap();
    assert!(mine.contains(&format!("{} Login (", c.key(ticket))), "{mine}");
    assert_eq!(chat::own_tickets(&c, "odile").unwrap(), "");

    // A ticket named in chat comes with its details, whoever has it, and in any case.
    let key = c.key(ticket);
    let named = chat::mentioned_tickets(&c, &format!("is {}? still current", key.to_lowercase())).unwrap();
    assert!(named.contains(&format!("{key} Login")) && named.contains("Assigned to Maya"), "{named}");
    assert_eq!(chat::mentioned_tickets(&c, "nothing here, and no XYZ-1 either").unwrap(), "");

    // Everyone can see who leads which team and who's free.
    let know = work::company_knowledge(&c).unwrap();
    assert!(know.contains("Odile Martin, CTO, leads Engineering") && know.contains("Now: free"), "{know}");

    // @all calls the channel: the whole company in #general, the team in a department's channel.
    assert_eq!(chat::say(&c, "engineering", "@all standup in 5").unwrap().len(), 2, "odile and maya");
    assert_eq!(chat::say(&c, "general", "@everyone shipping Friday, @maya too").unwrap().len(), 4, "everyone once, even when named twice");

    // A question to the lead happens out loud in the team channel, and the founder's answer closes it there.
    let ask = json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {"name": "ask_human", "arguments": {"question": "15 or 60 minute links?", "default": "15"}}});
    let said = mcp::handle(&c, &mcp::Ctx { ticket, employee: "maya".into() }, &ask).unwrap()["result"]["content"][0]["text"].as_str().unwrap().to_string();
    let n: i64 = said.split('#').nth(1).unwrap().split_whitespace().next().unwrap().parse().unwrap();
    let eng = c.messages(Some("engineering")).unwrap();
    let asked = eng.iter().rev().find(|m| m.author == "maya").unwrap();
    assert!(asked.text.contains("15 or 60 minute links?") && asked.text.contains("@odile"), "{}", asked.text);
    work::answer(&c, n, "60 minutes").unwrap();
    let last = c.messages(Some("engineering")).unwrap().pop().unwrap();
    assert_eq!((last.author.as_str(), last.text.as_str()), ("founder", "@maya 60 minutes"));

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

    // Talk runs on a lighter model by default. Tickets keep each person's own.
    let odile = onecorp::store::Employee { runtime: "claude".into(), model: Some("claude-opus-5-5".into()), ..c.employee("odile").unwrap().unwrap() };
    assert_eq!(work::talk_model(&c, &odile).unwrap().as_deref(), Some("claude-sonnet-5-5"));
    c.set_setting("talk", "cheapest").unwrap();
    assert_eq!(work::talk_model(&c, &odile).unwrap().as_deref(), Some("claude-haiku-5-5"));
    c.set_setting("talk", "same").unwrap();
    assert_eq!(work::talk_model(&c, &odile).unwrap().as_deref(), Some("claude-opus-5-5"));

    // You can take back a message.
    let id = c.post("general", "founder", "oops", None).unwrap();
    c.delete_message(id).unwrap();
    assert!(c.message(id).unwrap().is_none());

    // A question that doesn't block can be set aside: nothing posted, nothing resumed. A blocking one can't.
    let ask = |args| {
        let call = json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {"name": "ask_human", "arguments": args}});
        let said = mcp::handle(&c, &mcp::Ctx { ticket, employee: "maya".into() }, &call).unwrap()["result"]["content"][0]["text"].as_str().unwrap().to_string();
        said.split('#').nth(1).unwrap().split_whitespace().next().unwrap().parse::<i64>().unwrap()
    };
    let (aside, stuck) = (ask(json!({"question": "Dark mode too?"})), ask(json!({"question": "Which provider?", "blocking": true})));
    let posted = c.messages(None).unwrap().len();
    work::dismiss(&c, aside).unwrap();
    assert_eq!(c.messages(None).unwrap().len(), posted, "dismissing says nothing in chat");
    assert!(c.inbox().unwrap().iter().chain(&c.with_leads().unwrap()).all(|q| q.id != aside));
    assert!(work::dismiss(&c, aside).is_err(), "only once");
    assert!(work::dismiss(&c, stuck).is_err(), "a blocking question needs an answer");

    let _ = std::fs::remove_dir_all(&root);
}
