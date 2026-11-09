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
        let rest = rest.trim_start_matches(|c: char| c.is_alphabetic()).trim_start().trim_start_matches("at").trim_start();
        let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
        if let Ok(mut h) = digits.parse::<u32>() {
            let after = &rest[digits.len()..];
            let m = after.strip_prefix(':').and_then(|s| s.get(..2)?.parse::<u32>().ok()).unwrap_or(0);
            let tail = after.trim_start_matches(|c: char| c == ':' || c.is_ascii_digit()).trim_start();
            if tail.starts_with("pm") && h < 12 {
                h += 12;
            } else if tail.starts_with("am") && h == 12 {
                h = 0;
            }
            if h < 24 && m < 60 {
                return Some(Reset::Clock(h, m));
            }
        }
    }
    Some(Reset::Unknown)
}

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

/// Records a limit for this runtime if the text says it hit one. True when it did.
pub fn hit(runtime: &str, text: &str) -> Result<bool> {
    let Some(reset) = detect(text) else { return Ok(false) };
    let until = match reset {
        Reset::At(t) if t > now() => t,
        Reset::In(m) => now() + m * 60,
        Reset::Clock(h, m) => next_clock(h, m)?,
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

/// "15:00", or "tomorrow 09:00", in local time.
pub fn label(at: i64) -> String {
    let db = rusqlite::Connection::open_in_memory().ok();
    let q = |f: &str| db.as_ref().and_then(|d| d.query_row(&format!("SELECT strftime('{f}', ?1, 'unixepoch', 'localtime')"), [at], |r| r.get::<_, String>(0)).ok());
    let (day, today, time) = (q("%Y-%m-%d"), db.as_ref().and_then(|d| d.query_row("SELECT date('now','localtime')", [], |r| r.get::<_, String>(0)).ok()), q("%H:%M"));
    let time = time.unwrap_or_default();
    if day == today { time } else { format!("tomorrow {time}") }
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
        assert_eq!(detect("usage_limit_reached"), Some(Reset::Unknown));
        assert_eq!(detect("cargo test failed: 3 tests"), None);
        assert_eq!(detect("Added rate limiting to the API"), None);
    }
}
