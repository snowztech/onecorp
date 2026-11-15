//! A ticket from a project plan knows the project: its goal and the other tickets in it, so the
//! pieces fit. A ticket on its own gets nothing extra.

use onecorp::store::{Company, NewTicket};
use onecorp::{chat, templates, work};
use std::process::Command;

#[test]
fn a_ticket_knows_its_project() {
    let root = std::env::temp_dir().join(format!("onecorp-project-goal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    assert!(Command::new("git").arg("-C").arg(&repo).args(["init", "-q"]).status().unwrap().success());
    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();

    let plan = c.new_plan("Where to watch", "Show every country's streaming services for a title, with one link each.", "rui").unwrap();
    let piece = |title: &str, who: &str| c.new_ticket(&NewTicket { title: title.into(), assignee: Some(who.into()), plan: Some(plan), ..Default::default() }, "backlog").unwrap();
    let api = piece("Endpoint for every country", "maya");
    let view = piece("The all-countries view", "lina");
    let alone = c.add_ticket("Fix the footer", "maya").unwrap();

    let t = c.ticket(view).unwrap().unwrap();
    let block = work::project_of(&c, &t).unwrap();
    assert!(block.starts_with(&format!("This ticket is part of the project Where to watch, {}. Its goal: Show every country's", c.key(plan))), "{block}");
    assert!(block.contains(&format!("- {} Endpoint for every country (Maya Okafor, backlog)", c.key(api))), "{block}");
    assert!(!block.contains("The all-countries view"), "not itself");
    assert_eq!(work::project_of(&c, &c.ticket(alone).unwrap().unwrap()).unwrap(), "", "on its own, nothing");

    // A plan with no goal of its own is the first version, from the brief.
    let first = c.new_plan("First version", "", "rui").unwrap();
    let t = c.ticket(c.new_ticket(&NewTicket { title: "Login".into(), assignee: Some("maya".into()), plan: Some(first), ..Default::default() }, "backlog").unwrap()).unwrap().unwrap();
    assert!(work::project_of(&c, &t).unwrap().contains("Its goal: the first version, from the brief"));

    // In chat, a ticket named in the conversation says which project it's in.
    let named = chat::mentioned_tickets(&c, &format!("how is {} going?", c.key(api))).unwrap();
    assert!(named.contains("Part of the project Where to watch: Show every country's"), "{named}");

    let _ = std::fs::remove_dir_all(&root);
}
