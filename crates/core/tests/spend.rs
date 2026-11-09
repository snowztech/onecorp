//! Budgets: spend lands in a ledger, Codex tokens are priced, the company pauses at its budget
//! and picks up again the next day, and someone at their own cap takes no new work.

use onecorp::store::{self, Company};
use onecorp::{spend, templates, view, work};
use std::process::Command;

#[test]
fn budgets_pause_the_company_and_caps_pause_a_person() {
    let root = std::env::temp_dir().join(format!("onecorp-spend-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    // Integration tests run in their own process, so this can't leak into other tests.
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    assert!(Command::new("git").arg("-C").arg(&repo).args(["init", "-q"]).status().unwrap().success());
    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();

    // Codex reports tokens only: priced at the company's rate, marked estimated. Claude reports dollars.
    assert_eq!(spend::record(&c, "maya", None, "codex", None, 0.0, 1_000_000).unwrap(), 5.0);
    assert_eq!(spend::record(&c, "odile", None, "claude", Some("opus"), 0.25, 40_000).unwrap(), 0.25);
    spend::set_price(&c, "codex", 2.0).unwrap();
    assert_eq!(spend::record(&c, "maya", None, "codex", None, 0.0, 500_000).unwrap(), 1.0);
    assert!((spend::spent_today(&c).unwrap() - 6.25).abs() < 1e-9);

    // 80% of the budget is a heads-up. Reaching it pauses the company, for budget reasons.
    spend::set_budget(&c, 7.5).unwrap();
    spend::record(&c, "odile", None, "claude", None, 0.0, 0).unwrap();
    assert!(spend::warned_today(&c).unwrap() && !c.paused().unwrap());
    spend::record(&c, "odile", None, "claude", None, 1.5, 1).unwrap();
    assert!(c.paused().unwrap() && spend::paused_by_budget(&c).unwrap());

    // Going on for today sticks, even as spend keeps coming in.
    spend::resume_for_today(&c).unwrap();
    spend::record(&c, "odile", None, "claude", None, 1.0, 1).unwrap();
    assert!(!c.paused().unwrap());

    // A budget pause from an earlier day lifts on its own.
    c.set_paused(true).unwrap();
    c.set_setting("paused.reason", "budget:2000-01-01").unwrap();
    spend::new_day(&c).unwrap();
    assert!(!c.paused().unwrap());

    // Raising the budget lifts today's budget pause at once.
    spend::set_budget(&c, 0.5).unwrap();
    spend::record(&c, "odile", None, "claude", None, 0.1, 1).unwrap();
    assert!(spend::paused_by_budget(&c).unwrap());
    spend::set_budget(&c, 100.0).unwrap();
    assert!(!c.paused().unwrap());

    // Someone at their own cap takes no new work. Everyone else carries on.
    c.set_daily_cap("maya", 2.0).unwrap();
    assert_eq!(spend::capped(&c).unwrap(), ["maya"]);
    let theirs = c.add_ticket("Search", "maya").unwrap();
    work::start(&c, &c.ticket(theirs).unwrap().unwrap()).unwrap();
    assert_eq!(c.ticket(theirs).unwrap().unwrap().status, "queued", "waits instead of running");
    assert!(c.claim_job().unwrap().is_none(), "serve skips them too");
    c.set_daily_cap("maya", 0.0).unwrap();
    assert!(c.claim_job().unwrap().is_some(), "and picks it up once the cap is lifted");

    // The report adds up, by person and by agent.
    let r = view::spend_report(&c).unwrap();
    assert!((r["totals"]["today"]["cost"].as_f64().unwrap() - 8.85).abs() < 1e-9);
    let maya = r["people"].as_array().unwrap().iter().find(|p| p["id"] == "maya").unwrap().clone();
    assert_eq!(maya["today"]["cost"].as_f64(), Some(6.0));
    let codex = r["agents"].as_array().unwrap().iter().find(|a| a["runtime"] == "codex").unwrap().clone();
    assert_eq!(codex["estimated"], true);
    assert_eq!(r["days"].as_array().unwrap().len(), 14);
    let _ = store::list();

    let _ = std::fs::remove_dir_all(&root);
}
