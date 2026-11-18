//! Plan limits. Claude Code and Codex on a subscription stop working when the plan's usage limit
//! is reached. OneCorp spots the message, holds that agent's work until the limit resets, and
//! then picks it back up, instead of failing every ticket in the meantime.
//!
//! A limit belongs to your Mac, not to one company, so it's kept in the OneCorp home folder.

use crate::store::Company;
use anyhow::Result;
use std::time::{SystemTime, UNIX_EPOCH};

/// When the agent can work again, from its error text. None when the text isn't about a limit.
#[derive(Debug, PartialEq)]
pub enum Reset {
    /// A unix time, like Claude's "usage limit reached|1736985600".
    At(i64),
    /// Minutes from now, like "try again in 2 hours 10 minutes".
    In(i64),
    /// A local clock time, like "resets 3pm" or "resets at 15:30".
    Clock(u32, u32),
    /// A local date and time, like a weekly limit's "resets Nov 20 at 1am": month, day, hour, minute.
    Date(u32, u32, u32, u32),
    /// A limit, but no time we could read. We try again in a while.
    Unknown,
}

pub fn detect(text: &str) -> Option<Reset> {
    let t = text.to_lowercase();
    let limit = t.contains("usage limit")
        || t.contains("usage_limit")
        || t.contains("limit reached")
        || t.contains("hit your limit")
        || (t.contains("limit") && (t.contains("resets") || t.contains("try again")));
    if !limit {
        return None;
    }
    if let Some(n) = t.split('|').nth(1).and_then(|s| s.trim().split(|c: char| !c.is_ascii_digit()).next()?.parse::<i64>().ok()) {
        return Some(Reset::At(n));
    }
    if let Some(rest) = t.split("try again in").nth(1) {
        let words: Vec<&str> = rest.split_whitespace().take(6).collect();
        let mut minutes = 0;
        for pair in words.windows(2) {
            if let Ok(n) = pair[0].parse::<i64>() {
                minutes += match pair[1] {
                    u if u.starts_with("day") => n * 1440,
                    u if u.starts_with("hour") || u.starts_with('h') => n * 60,
                    u if u.starts_with("min") || u.starts_with('m') => n,
                    _ => 0,
                };
            }
        }
        if minutes > 0 {
            return Some(Reset::In(minutes));
        }
    }
    if let Some(rest) = t.split("reset").nth(1) {
        let mut rest = rest.trim_start_matches(|c: char| c.is_alphabetic()).trim_start();
        // A weekly limit gives a date first: "nov 20 at 1am".
        let mut date = None;
        if let Some(month) = MONTHS.iter().position(|m| rest.starts_with(m)) {
            let after = rest.trim_start_matches(|c: char| c.is_alphabetic() || c == '.').trim_start();
            let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(day) = digits.parse::<u32>() {
                date = Some((month as u32 + 1, day));
                rest = after[digits.len()..].trim_start_matches(|c: char| c.is_alphabetic() || c == ',').trim_start();
            }
        }
        let rest = rest.trim_start_matches("at").trim_start();
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        let (mut h, mut m) = (None, 0);
        if let Ok(mut hour) = digits.parse::<u32>() {
            let after = &rest[digits.len()..];
            let minute = after.strip_prefix(':').and_then(|s| s.get(..2)?.parse::<u32>().ok()).unwrap_or(0);
            let tail = after.trim_start_matches(|c: char| c == ':' || c.is_ascii_digit()).trim_start();
            if tail.starts_with("pm") && hour < 12 {
                hour += 12;
            } else if tail.starts_with("am") && hour == 12 {
                hour = 0;
            }
            if hour < 24 && minute < 60 {
                (h, m) = (Some(hour), minute);
            }
        }
        match (date, h) {
            (Some((mo, d)), h) => return Some(Reset::Date(mo, d, h.unwrap_or(0), m)),
            (None, Some(h)) => return Some(Reset::Clock(h, m)),
            _ => {}
        }
    }
    Some(Reset::Unknown)
}

const MONTHS: [&str; 12] = ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"];

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn file(runtime: &str) -> std::path::PathBuf {
    crate::store::home().join("limits").join(runtime)
}

/// SQLite knows the local time zone, so it turns a clock time into a unix time for us.
fn next_clock(h: u32, m: u32) -> Result<i64> {
    let db = rusqlite::Connection::open_in_memory()?;
    let at: i64 = db.query_row(
        "SELECT CAST(strftime('%s', datetime(date('now','localtime') || printf(' %02d:%02d:00', ?1, ?2), 'utc')) AS INTEGER)",
        [h, m],
        |r| r.get(0),
    )?;
    Ok(if at <= now() { at + 86_400 } else { at })
}

/// The next time it's that local date and time: this year, or next year in late December for
/// "resets Jan 2". None for a date that doesn't exist.
fn next_date(mo: u32, d: u32, h: u32, m: u32) -> Result<Option<i64>> {
    let db = rusqlite::Connection::open_in_memory()?;
    let year: u32 = db.query_row("SELECT CAST(strftime('%Y','now','localtime') AS INTEGER)", [], |r| r.get(0))?;
    for y in [year, year + 1] {
        let at: Option<i64> = db.query_row(
            // SQLite rolls Feb 30 over to March, so a date that doesn't come back the same isn't one.
            "SELECT CASE WHEN date(d) = substr(d, 1, 10) THEN CAST(strftime('%s', datetime(d, 'utc')) AS INTEGER) END FROM (SELECT printf('%04d-%02d-%02d %02d:%02d:00', ?1, ?2, ?3, ?4, ?5) AS d)",
            [y, mo, d, h, m],
            |r| r.get(0),
        )?;
        match at {
            Some(at) if at > now() => return Ok(Some(at)),
            Some(_) => continue,
            None => return Ok(None),
        }
    }
    Ok(None)
}

/// Records a limit for this runtime if the text says it hit one. True when it did.
pub fn hit(runtime: &str, text: &str) -> Result<bool> {
    let Some(reset) = detect(text) else { return Ok(false) };
    let until = match reset {
        Reset::At(t) if t > now() => t,
        Reset::In(m) => now() + m * 60,
        Reset::Clock(h, m) => next_clock(h, m)?,
        Reset::Date(mo, d, h, m) => next_date(mo, d, h, m)?.unwrap_or_else(|| now() + 30 * 60),
        // A time already past, or none at all: look again in half an hour.
        _ => now() + 30 * 60,
    };
    std::fs::create_dir_all(file(runtime).parent().unwrap())?;
    std::fs::write(file(runtime), until.to_string())?;
    Ok(true)
}

/// When this runtime can work again, as a unix time. None when it isn't limited.
pub fn until(runtime: &str) -> Option<i64> {
    let at: i64 = std::fs::read_to_string(file(runtime)).ok()?.trim().parse().ok()?;
    (at > now()).then_some(at)
}

/// Forgets this runtime's limit. A plan can be reset or upgraded before the time it gave, and work
/// shouldn't sit waiting for nothing. If the limit is still there, the next run sets it again.
pub fn clear(runtime: &str) -> Result<()> {
    match std::fs::remove_file(file(runtime)) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.into()),
        _ => Ok(()),
    }
}

/// "15:00", "tomorrow 09:00", or "Nov 20 01:00" further out, in local time.
pub fn label(at: i64) -> String {
    let db = rusqlite::Connection::open_in_memory().ok();
    let q = |sql: &str| db.as_ref().and_then(|d| d.query_row(sql, [at], |r| r.get::<_, String>(0)).ok()).unwrap_or_default();
    let day = q("SELECT date(?1, 'unixepoch', 'localtime')");
    let time = q("SELECT strftime('%H:%M', ?1, 'unixepoch', 'localtime')");
    if day == q("SELECT date('now','localtime') WHERE ?1 IS NOT NULL") {
        time
    } else if day == q("SELECT date('now','localtime','+1 day') WHERE ?1 IS NOT NULL") {
        format!("tomorrow {time}")
    } else {
        let month = day.get(5..7).and_then(|s| s.parse::<usize>().ok()).and_then(|n| MONTHS.get(n.wrapping_sub(1))).unwrap_or(&"");
        let month = month.get(..1).unwrap_or("").to_uppercase() + month.get(1..).unwrap_or("");
        format!("{month} {} {time}", day.get(8..).unwrap_or("").trim_start_matches('0'))
    }
}

/// Everyone who can't take work right now: at their daily cap, or on an agent that hit its limit.
pub fn away(c: &Company) -> Result<Vec<String>> {
    let mut out = crate::spend::capped(c)?;
    for e in c.employees()? {
        if until(&e.runtime).is_some() && !out.contains(&e.id) {
            out.push(e.id);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_limit_messages() {
        assert_eq!(detect("Claude AI usage limit reached|1736985600"), Some(Reset::At(1_736_985_600)));
        assert_eq!(detect("You've hit your limit · resets 3pm (Europe/Paris)"), Some(Reset::Clock(15, 0)));
        assert_eq!(detect("5-hour limit reached ∙ resets at 9:30am"), Some(Reset::Clock(9, 30)));
        assert_eq!(detect("You've hit your usage limit. Try again in 2 hours 10 minutes."), Some(Reset::In(130)));
        assert_eq!(detect("You've hit your weekly limit · resets Nov 20 at 1am (Europe/Paris)"), Some(Reset::Date(11, 20, 1, 0)));
        assert_eq!(detect("weekly limit reached, resets Dec 3, 4:30pm"), Some(Reset::Date(12, 3, 16, 30)));
        assert_eq!(detect("usage_limit_reached"), Some(Reset::Unknown));
        assert_eq!(detect("cargo test failed: 3 tests"), None);
        assert_eq!(detect("Added rate limiting to the API"), None);
    }

    #[test]
    fn a_date_is_the_next_one_and_says_its_day() {
        let at = next_date(1, 1, 0, 0).unwrap().unwrap();
        assert!(at > now() && at <= now() + 366 * 86_400, "the coming New Year");
        assert_eq!(next_date(2, 30, 0, 0).unwrap(), None, "no Feb 30");
        assert!(label(now() + 3 * 86_400).starts_with(char::is_uppercase), "three days out: {}", label(now() + 3 * 86_400));
        assert!(label(now() + 86_400).starts_with("tomorrow"));
    }
}
