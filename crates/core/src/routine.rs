//! Routines: recurring work on a schedule, like a dependency check every Monday or a nightly test
//! run. When one is due it makes an ordinary ticket from its template and starts it, so review,
//! the budget, pause and the run limit all apply. It never piles up: while its last ticket is still
//! open, the next one is skipped. A Mac that slept through a time runs it once when it wakes.

use crate::store::{Company, NewTicket};
use anyhow::{Result, bail};
use rusqlite::{OptionalExtension, params};

/// When a routine runs.
#[derive(Debug, Clone, PartialEq)]
pub enum Every {
    /// Every N hours.
    Hours(u32),
    /// Every day at HH:MM.
    Day(u32, u32),
    /// Monday to Friday at HH:MM.
    Weekday(u32, u32),
    /// One day of the week (0 is Sunday) at HH:MM.
    Week(u32, u32, u32),
}

const DAYS: [&str; 7] = ["sunday", "monday", "tuesday", "wednesday", "thursday", "friday", "saturday"];

impl Every {
    /// "every 6 hours", "daily 09:00", "weekdays 09:00", "monday 09:00".
    pub fn parse(text: &str) -> Result<Every> {
        let words: Vec<String> = text.split_whitespace().map(str::to_lowercase).collect();
        let time = |s: &str| -> Result<(u32, u32)> {
            let (h, m) = s.split_once(':').unwrap_or((s, "0"));
            match (h.parse::<u32>(), m.parse::<u32>()) {
                (Ok(h), Ok(m)) if h < 24 && m < 60 => Ok((h, m)),
                _ => bail!("a time is like 09:00"),
            }
        };
        let w: Vec<&str> = words.iter().map(String::as_str).collect();
        Ok(match w[..] {
            ["every", n, "hours" | "hour" | "h"] => Every::Hours(n.parse().ok().filter(|n| (1..=168).contains(n)).ok_or_else(|| anyhow::anyhow!("every 1 to 168 hours"))?),
            ["daily" | "everyday", t] => {
                let (h, m) = time(t)?;
                Every::Day(h, m)
            }
            ["weekdays", t] => {
                let (h, m) = time(t)?;
                Every::Weekday(h, m)
            }
            [day, t] if DAYS.iter().any(|d| d.starts_with(day) && day.len() >= 3) => {
                let (h, m) = time(t)?;
                Every::Week(DAYS.iter().position(|d| d.starts_with(day)).unwrap() as u32, h, m)
            }
            _ => bail!("say when like: every 6 hours, daily 09:00, weekdays 09:00, monday 09:00"),
        })
    }

    /// How it's stored, which `parse` reads back.
    pub fn code(&self) -> String {
        match self {
            Every::Hours(n) => format!("every {n} hours"),
            Every::Day(h, m) => format!("daily {h:02}:{m:02}"),
            Every::Weekday(h, m) => format!("weekdays {h:02}:{m:02}"),
            Every::Week(d, h, m) => format!("{} {h:02}:{m:02}", DAYS[*d as usize]),
        }
    }

    /// For people: "Every day at 09:00".
    pub fn words(&self) -> String {
        match self {
            Every::Hours(1) => "Every hour".into(),
            Every::Hours(n) => format!("Every {n} hours"),
            Every::Day(h, m) => format!("Every day at {h:02}:{m:02}"),
            Every::Weekday(h, m) => format!("Weekdays at {h:02}:{m:02}"),
            Every::Week(d, h, m) => {
                let day = DAYS[*d as usize];
                format!("Every {}{} at {h:02}:{m:02}", day[..1].to_uppercase(), &day[1..])
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Routine {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub assignee: String,
    pub repo: Option<String>,
    pub every: Every,
    pub on: bool,
    /// When it last ran or was skipped, local time.
    pub last_run: String,
    pub last_ticket: Option<i64>,
    /// What happened last time, when it didn't make a ticket.
    pub note: String,
}

/// Local time now, or `offset` from now, as "YYYY-MM-DD HH:MM:SS" (or another strftime format).
fn local(c: &Company, format: &str, offset: &str) -> Result<String> {
    Ok(c.db.query_row("SELECT strftime(?1, 'now', 'localtime', ?2)", params![format, if offset.is_empty() { "+0 seconds" } else { offset }], |r| r.get(0))?)
}

pub fn add(c: &Company, title: &str, description: &str, assignee: &str, repo: Option<&str>, every: &Every) -> Result<i64> {
    if title.trim().is_empty() {
        bail!("a routine needs a title: say what it does");
    }
    if c.employee(assignee)?.is_none() {
        bail!("nobody called {assignee} works at {}", c.name);
    }
    // Starting now: the first run is the next time on its schedule, not right away.
    c.db.execute(
        "INSERT INTO routines (title, description, assignee, repo, every, on_, last_run) VALUES (?1, ?2, ?3, ?4, ?5, 1, ?6)",
        params![title.trim(), description.trim(), assignee, repo.filter(|r| !r.trim().is_empty()), every.code(), local(c, "%Y-%m-%d %H:%M:%S", "")?],
    )?;
    Ok(c.db.last_insert_rowid())
}

pub fn list(c: &Company) -> Result<Vec<Routine>> {
    let mut st = c.db.prepare("SELECT id, title, description, assignee, repo, every, on_, last_run, last_ticket, note FROM routines ORDER BY id")?;
    let rows = st.query_map([], |r| {
        let every: String = r.get(5)?;
        Ok(Routine {
            id: r.get(0)?,
            title: r.get(1)?,
            description: r.get(2)?,
            assignee: r.get(3)?,
            repo: r.get(4)?,
            every: Every::parse(&every).unwrap_or(Every::Day(9, 0)),
            on: r.get(6)?,
            last_run: r.get(7)?,
            last_ticket: r.get(8)?,
            note: r.get(9)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn get(c: &Company, id: i64) -> Result<Routine> {
    list(c)?.into_iter().find(|r| r.id == id).ok_or_else(|| anyhow::anyhow!("no routine {id}. See them with: onecorp routine"))
}

pub fn set_on(c: &Company, id: i64, on: bool) -> Result<()> {
    get(c, id)?;
    c.db.execute("UPDATE routines SET on_ = ?2 WHERE id = ?1", params![id, on])?;
    Ok(())
}

pub fn remove(c: &Company, id: i64) -> Result<()> {
    get(c, id)?;
    c.db.execute("DELETE FROM routines WHERE id = ?1", [id])?;
    Ok(())
}

/// The latest time on the schedule that has come, local "YYYY-MM-DD HH:MM:SS". For a time of day,
/// the latest of the last 7 days that matches, so a Mac asleep through it still finds it.
fn latest_slot(c: &Company, every: &Every) -> Result<String> {
    let (days, h, m): (&dyn Fn(u32) -> bool, u32, u32) = match every {
        Every::Hours(n) => return local(c, "%Y-%m-%d %H:%M:%S", &format!("-{n} hours")),
        Every::Day(h, m) => (&|_| true, *h, *m),
        Every::Weekday(h, m) => (&|d| (1..=5).contains(&d), *h, *m),
        Every::Week(day, h, m) => (&move |d| d == *day, *h, *m),
    };
    let now = local(c, "%Y-%m-%d %H:%M:%S", "")?;
    for back in 0..8 {
        let at = local(c, "%Y-%m-%d %w", &format!("-{back} days"))?;
        let (date, wday) = at.split_once(' ').unwrap_or((&at, "0"));
        let slot = format!("{date} {h:02}:{m:02}:00");
        if days(wday.parse().unwrap_or(0)) && slot <= now {
            return Ok(slot);
        }
    }
    Ok(String::new())
}

/// Whether a routine is due: on, and a time on its schedule has come since it last ran.
pub fn due(c: &Company, r: &Routine) -> Result<bool> {
    if !r.on {
        return Ok(false);
    }
    let slot = latest_slot(c, &r.every)?;
    Ok(match r.every {
        // Every N hours: N hours since the last run.
        Every::Hours(_) => r.last_run <= slot,
        _ => !slot.is_empty() && r.last_run < slot,
    })
}

/// Makes the routine's ticket and starts it, now. Its review, budget and pause apply as usual.
pub fn run_now(c: &Company, id: i64) -> Result<i64> {
    let r = get(c, id)?;
    let ticket = c.new_ticket(
        &NewTicket { title: r.title.clone(), description: r.description.clone(), assignee: Some(r.assignee.clone()), repo: r.repo.clone(), ..Default::default() },
        "backlog",
    )?;
    c.add_event(ticket, "create", &format!("made by the routine \"{}\" ({})", r.title, r.every.words().to_lowercase()), "")?;
    c.db.execute("UPDATE routines SET last_run = ?2, last_ticket = ?3, note = '' WHERE id = ?1", params![id, local(c, "%Y-%m-%d %H:%M:%S", "")?, ticket])?;
    let t = c.ticket(ticket)?.ok_or_else(|| anyhow::anyhow!("the ticket vanished"))?;
    crate::work::start(c, &t)?;
    Ok(ticket)
}

/// Runs the routines that are due. One whose last ticket is still open is skipped this time, and
/// says so. Returns what it did, a line each. Called from serve's loop.
pub fn fire_due(c: &Company) -> Result<Vec<String>> {
    let mut said = vec![];
    for r in list(c)? {
        if !due(c, &r)? {
            continue;
        }
        let open = r.last_ticket.map(|t| c.ticket(t)).transpose()?.flatten().filter(|t| !["done", "failed"].contains(&t.status.as_str()));
        if let Some(t) = open {
            let note = format!("skipped at {}: {} was still {}", local(c, "%H:%M", "")?, c.key(t.id), t.status);
            c.db.execute("UPDATE routines SET last_run = ?2, note = ?3 WHERE id = ?1", params![r.id, local(c, "%Y-%m-%d %H:%M:%S", "")?, note])?;
            said.push(format!("Routine \"{}\" {note}", r.title));
            continue;
        }
        match run_now(c, r.id) {
            Ok(ticket) => said.push(format!("Routine \"{}\" made {}", r.title, c.key(ticket))),
            // Like someone who left the company: said on the routine, and tried again next time,
            // not every second until then.
            Err(e) => {
                let note = format!("couldn't run at {}: {e:#}", local(c, "%H:%M", "")?);
                c.db.execute("UPDATE routines SET last_run = ?2, note = ?3 WHERE id = ?1", params![r.id, local(c, "%Y-%m-%d %H:%M:%S", "")?, note])?;
                said.push(format!("Routine \"{}\" {note}", r.title));
            }
        }
    }
    Ok(said)
}

/// When it runs next, local "YYYY-MM-DD HH:MM", for showing. Empty when it's off.
pub fn next(c: &Company, r: &Routine) -> Result<String> {
    if !r.on {
        return Ok(String::new());
    }
    let at = |offset: &str| local(c, "%Y-%m-%d %H:%M", offset);
    match r.every {
        Every::Hours(n) => Ok(c.db.query_row("SELECT strftime('%Y-%m-%d %H:%M', ?1, ?2)", params![r.last_run, format!("+{n} hours")], |row| row.get(0)).optional()?.unwrap_or_default()),
        Every::Day(h, m) | Every::Weekday(h, m) | Every::Week(_, h, m) => {
            for ahead in 0..8 {
                let day = local(c, "%Y-%m-%d %w", &format!("+{ahead} days"))?;
                let (date, wday) = day.split_once(' ').unwrap_or((&day, "0"));
                let wday: u32 = wday.parse().unwrap_or(0);
                let fits = match r.every {
                    Every::Weekday(..) => (1..=5).contains(&wday),
                    Every::Week(d, ..) => d == wday,
                    _ => true,
                };
                let slot = format!("{date} {h:02}:{m:02}");
                if fits && slot > at("")? {
                    return Ok(slot);
                }
            }
            Ok(String::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Every;

    #[test]
    fn reads_and_writes_schedules() {
        for (text, every) in [("every 6 hours", Every::Hours(6)), ("daily 9:00", Every::Day(9, 0)), ("Weekdays 18:30", Every::Weekday(18, 30)), ("mon 09:00", Every::Week(1, 9, 0))] {
            assert_eq!(Every::parse(text).unwrap(), every, "{text}");
            assert_eq!(Every::parse(&every.code()).unwrap(), every, "round trip");
        }
        assert_eq!(Every::Week(1, 9, 0).words(), "Every Monday at 09:00");
        assert!(Every::parse("daily 25:00").is_err());
        assert!(Every::parse("every 0 hours").is_err());
        assert!(Every::parse("sometimes").is_err());
    }
}
