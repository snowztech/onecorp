//! Routines make an ordinary ticket when they're due, once even after missing several times, never
//! a second while the last one is open, and not while paused.

use onecorp::routine::{self, Every};
use onecorp::store::Company;
use onecorp::templates;
use std::process::Command;

#[test]
fn routines_make_tickets_on_schedule() {
    let root = std::env::temp_dir().join(format!("onecorp-routines-{}", std::process::id()));
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
    let db = rusqlite::Connection::open(root.join("home/companies/acme/company.db")).unwrap();
    let last_run = |id: i64, ago: &str| db.execute("UPDATE routines SET last_run = datetime('now', 'localtime', ?2) WHERE id = ?1", rusqlite::params![id, ago]).unwrap();
    let tickets = || c.tickets().unwrap().len();

    // Every day at midnight, added just now: not due until tomorrow.
    let daily = routine::add(&c, "Check dependencies", "List what's outdated.", "maya", None, &Every::Day(0, 0)).unwrap();
    assert!(routine::fire_due(&c).unwrap().is_empty());

    // Last ran three days ago, the Mac slept through: one ticket, not three.
    last_run(daily, "-3 days");
    let said = routine::fire_due(&c).unwrap();
    assert_eq!((said.len(), tickets()), (1, 1), "{said:?}");
    let r = routine::get(&c, daily).unwrap();
    let made = c.ticket(r.last_ticket.unwrap()).unwrap().unwrap();
    assert_eq!((made.title.as_str(), made.description.as_str(), made.assignee.as_deref()), ("Check dependencies", "List what's outdated.", Some("maya")));
    assert_ne!(made.status, "backlog", "started like any ticket");
    assert!(routine::fire_due(&c).unwrap().is_empty(), "nothing more today");

    // Due again while its last ticket waits for review: skipped, and it says so.
    last_run(daily, "-1 day");
    let said = routine::fire_due(&c).unwrap();
    assert!(said[0].contains("skipped") && said[0].contains(&c.key(made.id)), "{said:?}");
    assert_eq!(tickets(), 1);

    // Paused, nothing. Resumed, every 1 hour after two: due.
    let hourly = routine::add(&c, "Triage issues", "", "odile", None, &Every::Hours(1)).unwrap();
    routine::set_on(&c, hourly, false).unwrap();
    last_run(hourly, "-2 hours");
    assert!(routine::fire_due(&c).unwrap().is_empty());
    routine::set_on(&c, hourly, true).unwrap();
    assert_eq!(routine::fire_due(&c).unwrap().len(), 1);
    assert_eq!(tickets(), 2);

    // Its person left: said on the routine, and nothing breaks.
    let gone = routine::add(&c, "Weekly notes", "", "rui", None, &Every::Day(0, 0)).unwrap();
    c.fire("rui").unwrap();
    last_run(gone, "-1 day");
    let said = routine::fire_due(&c).unwrap();
    assert!(said.iter().any(|l| l.contains("couldn't run")), "{said:?}");
    assert!(routine::get(&c, gone).unwrap().note.starts_with("couldn't run"));
    assert!(routine::fire_due(&c).unwrap().is_empty(), "not again until its next time");

    // Run now makes a ticket whatever the schedule.
    routine::run_now(&c, hourly).unwrap();
    assert_eq!(tickets(), 3);
    routine::remove(&c, hourly).unwrap();
    assert_eq!(routine::list(&c).unwrap().len(), 2);

    let _ = std::fs::remove_dir_all(&root);
}
