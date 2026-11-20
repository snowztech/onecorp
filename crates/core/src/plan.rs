//! Plan usage. On a Claude subscription, dollars don't say much: what runs out is the share of the
//! 5-hour session and of the week. Claude Code reports both on its runs, so OneCorp keeps the
//! latest, and can show them in place of dollars.
//!
//! Like a limit, the plan belongs to your Mac, not to one company, so it's kept in the OneCorp home
//! folder. What one run used is an estimate: Claude Code reports the plan only when it changes, in
//! whole percents, and your own use of Claude counts too. Each rise is counted once, for the run
//! that saw it.

use crate::store::{Company, Ticket};
use anyhow::Result;
use serde_json::{Value, json};
use std::cell::Cell;
use std::collections::HashMap;

/// The plan as the last run saw it. Percents, and unix times for when each window starts over.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Usage {
    pub session: f64,
    pub session_resets: i64,
    pub week: f64,
    pub week_resets: i64,
}

fn file() -> std::path::PathBuf {
    crate::store::home().join("limits").join("claude-plan")
}

pub fn read() -> Option<Usage> {
    let text = std::fs::read_to_string(file()).ok()?;
    let n: Vec<&str> = text.split_whitespace().collect();
    Some(Usage { session: n.first()?.parse().ok()?, session_resets: n.get(1)?.parse().ok()?, week: n.get(2)?.parse().ok()?, week_resets: n.get(3)?.parse().ok()? })
}

thread_local! {
    // A run and the spend it records happen on the same thread, one after the other, so the
    // share a run saw waits here instead of going through every caller.
    static SEEN: Cell<f64> = const { Cell::new(0.0) };
}

/// A run starts: nothing seen yet.
pub fn start() {
    SEEN.with(|s| s.set(0.0));
}

/// A run reported the plan. Keeps it, and counts the rise of the session since the last report.
pub fn seen(u: Usage) {
    let rise = match read() {
        Some(p) if p.session_resets == u.session_resets => (u.session - p.session).max(0.0),
        // A new session started since: all of it is new.
        Some(_) => u.session,
        // The first report ever: no telling what came before.
        None => 0.0,
    };
    SEEN.with(|s| s.set(s.get() + rise));
    let _ = std::fs::create_dir_all(file().parent().unwrap());
    let _ = std::fs::write(file(), format!("{} {} {} {}", u.session, u.session_resets, u.week, u.week_resets));
}

/// The session share this thread's last run used, once: the spend ledger takes it.
pub fn take() -> f64 {
    SEEN.with(|s| s.replace(0.0))
}

/// Whether the company shows plan usage instead of dollars.
pub fn shown(c: &Company) -> Result<bool> {
    Ok(c.setting("spend.show")?.as_deref() == Some("plan"))
}

pub fn set_shown(c: &Company, plan: bool) -> Result<()> {
    c.set_setting("spend.show", if plan { "plan" } else { "dollars" })
}

/// "4%", "<1%" or "0%": a share of a session.
pub fn label(pct: f64) -> String {
    if pct > 0.0 && pct < 0.5 { "<1%".into() } else { format!("{pct:.0}%") }
}

/// The plan for the screens: null when no run reported one yet. A window already over reads 0%.
pub fn now() -> Value {
    let Some(u) = read() else { return Value::Null };
    let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
    let pct = |p: f64, resets: i64| if resets > at { p } else { 0.0 };
    json!({
        "session": pct(u.session, u.session_resets), "session_resets": (u.session_resets > at).then_some(u.session_resets),
        "week": pct(u.week, u.week_resets), "week_resets": (u.week_resets > at).then_some(u.week_resets),
    })
}

/// What tickets and people cost, the way the company chose to see it: dollars, or plan.
pub struct Labels {
    plan: bool,
    used: HashMap<i64, f64>,
}

impl Labels {
    pub fn new(c: &Company) -> Result<Labels> {
        let plan = shown(c)?;
        Ok(Labels { plan, used: if plan { c.plan_by_ticket()? } else { HashMap::new() } })
    }

    pub fn ticket(&self, t: &Ticket) -> String {
        self.tickets(&[t])
    }

    pub fn tickets(&self, ts: &[&Ticket]) -> String {
        if self.plan {
            label(ts.iter().filter_map(|t| self.used.get(&t.id)).sum())
        } else {
            crate::work::spend(ts.iter().map(|t| t.cost_usd).sum(), ts.iter().map(|t| t.tokens).sum())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_read_short() {
        assert_eq!(label(0.0), "0%");
        assert_eq!(label(0.3), "<1%");
        assert_eq!(label(3.6), "4%");
    }
}
