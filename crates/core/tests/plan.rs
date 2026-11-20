//! Plan usage: each rise of the session counts once, for the run that saw it, and a company can
//! show it in place of dollars.

use onecorp::plan::{self, Usage};
use onecorp::store::Company;
use onecorp::{spend, templates, view};
use serde_json::Value;
use std::process::Command;

#[test]
fn plan_usage_is_counted_per_ticket_and_shown_instead_of_dollars() {
    let root = std::env::temp_dir().join(format!("onecorp-plan-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    // Integration tests run in their own process, so this can't leak into other tests.
    unsafe { std::env::set_var("ONECORP_HOME", root.join("home")) };
    assert!(Command::new("git").arg("-C").arg(&repo).args(["init", "-q"]).status().unwrap().success());
    let c = Company::create("acme", &repo, "").unwrap();
    templates::apply(&c, templates::template("demo").unwrap(), "fake").unwrap();
    let t = c.add_ticket("Pricing page", "odile").unwrap();
    let at = |p: f64| Usage { session: p, session_resets: 4_000_000_000, week: p / 2.0, week_resets: 4_100_000_000 };

    // The first report ever has nothing to compare with.
    plan::start();
    plan::seen(at(10.0));
    spend::record(&c, "ticket", "odile", Some(t), "claude", None, 0.5, 1000).unwrap();
    // The next run saw the session go from 10% to 14%.
    plan::start();
    plan::seen(at(14.0));
    spend::record(&c, "ticket", "odile", Some(t), "claude", None, 0.5, 1000).unwrap();
    assert_eq!(c.plan_by_ticket().unwrap().get(&t), Some(&4.0));

    let key = c.key(t);
    let find = |s: &Value, list: &str, k: &str, v: &str| s[list].as_array().unwrap().iter().find(|x| x[k] == v).unwrap()["spend"].clone();
    assert!(find(&view::snapshot(&c).unwrap(), "tickets", "key", &key).as_str().unwrap().starts_with('$'), "dollars by default");
    plan::set_shown(&c, true).unwrap();
    let s = view::snapshot(&c).unwrap();
    assert_eq!(find(&s, "tickets", "key", &key), "4%");
    assert_eq!(find(&s, "employees", "id", "odile"), "4%");
    assert_eq!(s["spend"]["plan"]["session"], 14.0);
    assert_eq!(s["spend"]["plan"]["week"], 7.0);

    let _ = std::fs::remove_dir_all(&root);
}
