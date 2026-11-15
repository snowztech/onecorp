//! Every chat reply knows what the whole team finished lately, not only its own tickets.

use onecorp::store::Company;
use onecorp::{chat, templates};
use std::process::Command;

#[test]
fn chat_knows_what_the_team_finished() {
    let root = std::env::temp_dir().join(format!("onecorp-recent-work-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    assert!(Command::new("git").arg("-C").arg(&repo).args(["init", "-q"]).status().unwrap().success());
    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();
    assert_eq!(chat::recent_work(&c).unwrap(), "", "nothing finished, nothing said");

    // Finished the way approving does it: done, with a merge event after the summary.
    let finish = |title: &str, who: &str, said: &str| {
        let t = c.add_ticket(title, who).unwrap();
        c.add_event(t, "say", said, "").unwrap();
        c.set_status(t, "done").unwrap();
        c.add_event(t, "merge", "merged", "").unwrap();
        t
    };
    let a = finish("Rate limit the API", "maya", "Added a token bucket per key. Tests cover the burst case.");
    let b = finish("Pricing page", "odile", "The page is live with Free and Pro.");
    let open = c.add_ticket("Still going", "maya").unwrap();

    let work = chat::recent_work(&c).unwrap();
    let lines: Vec<&str> = work.lines().filter(|l| l.starts_with("- ")).collect();
    assert_eq!(lines.len(), 2, "{work}");
    assert!(lines.iter().any(|l| l.starts_with(&format!("- {} Rate limit the API (Maya, ", c.key(a))) && l.ends_with("Added a token bucket per key.")), "only the first sentence: {work}");
    assert!(lines.iter().any(|l| l.contains(&c.key(b)) && l.contains("(Odile, ")), "{work}");
    assert!(!work.contains(&c.key(open)), "only what's finished");

    // A reply gets it, whoever asks and wherever.
    let slots = chat::say(&c, "general", "@rui what shipped this week?").unwrap();
    chat::reply(&c, slots[0]).unwrap();

    let _ = std::fs::remove_dir_all(&root);
}
