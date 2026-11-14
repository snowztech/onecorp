//! What your team spends. Every run lands in a ledger in dollars: what the agent reports, or an
//! estimate from its tokens when it doesn't say (Codex reports tokens only). Budgets are checked
//! after every run: a heads-up at 80% of the day's budget, and a pause at 100%, lifted the next
//! day. People can have their own daily cap too. A ticket run on Claude Code also gets what's left
//! as its own ceiling (`left`), so one long run stops near the budget instead of far past it.

use crate::store::Company;
use anyhow::{Result, bail};
use serde_json::{Value, json};

/// Estimated dollars per million tokens, by agent, for agents that report only tokens. Blended
/// over input and output, so it's a rough guide: change it in Settings to match your plan.
pub const DEFAULT_PRICES: &[(&str, f64)] = &[("codex", 5.0), ("claude", 9.0)];

pub fn price(c: &Company, runtime: &str) -> Result<f64> {
    if let Some(v) = c.setting(&format!("price.{runtime}"))? {
        return Ok(v.parse().unwrap_or(0.0));
    }
    Ok(DEFAULT_PRICES.iter().find(|(r, _)| *r == runtime).map(|(_, p)| *p).unwrap_or(0.0))
}

pub fn set_price(c: &Company, runtime: &str, per_million: f64) -> Result<()> {
    if per_million.is_nan() || per_million < 0.0 {
        bail!("a price is a positive number of dollars per million tokens");
    }
    c.set_setting(&format!("price.{runtime}"), &per_million.to_string())
}

/// The agents `c`'s team uses whose plan hit its usage limit in the last week. A plan is yours,
/// not a company's, so a limit hit in another company counts too, and says where.
pub fn limit_hits(c: &Company) -> Result<Vec<Value>> {
    let used: Vec<String> = c.employees()?.into_iter().map(|e| e.runtime).collect();
    let mut out = vec![];
    for (runtime, label) in [("claude", "Claude Code"), ("codex", "Codex")].into_iter().filter(|(r, _)| used.iter().any(|u| u == r)) {
        let (mut n, mut elsewhere) = (0, vec![]);
        for name in crate::store::list()? {
            let Ok(other) = Company::open(&name) else { continue };
            let here = other.limit_waits(7)?.iter().filter(|s| s.starts_with(label)).count();
            n += here;
            if here > 0 && other.name != c.name {
                elsewhere.push(other.name.clone());
            }
        }
        if n > 0 {
            out.push(json!({"runtime": runtime, "times": n, "elsewhere": elsewhere}));
        }
    }
    Ok(out)
}

/// One run: what it cost and who ran it. Returns the dollars recorded, reported or estimated.
#[allow(clippy::too_many_arguments)]
pub fn record(c: &Company, kind: &str, employee: &str, ticket: Option<i64>, runtime: &str, model: Option<&str>, cost: f64, tokens: u64) -> Result<f64> {
    let (dollars, estimated) = if cost > 0.0 {
        (cost, false)
    } else if tokens > 0 && runtime != "fake" {
        (tokens as f64 / 1_000_000.0 * price(c, runtime)?, true)
    } else {
        (0.0, false)
    };
    if dollars > 0.0 || tokens > 0 {
        c.add_spend(&crate::store::Spent { employee, ticket, runtime, model, cost: dollars, tokens, estimated, kind })?;
    }
    check(c)?;
    Ok(dollars)
}

/// The company's daily budget in dollars. 0 means none.
pub fn budget(c: &Company) -> Result<f64> {
    Ok(c.setting("budget.daily")?.and_then(|v| v.parse().ok()).unwrap_or(0.0))
}

pub fn set_budget(c: &Company, dollars: f64) -> Result<()> {
    if dollars.is_nan() || dollars < 0.0 {
        bail!("a budget is a positive amount, or 0 for none");
    }
    c.set_setting("budget.daily", &dollars.to_string())?;
    // A higher budget lifts today's budget pause right away.
    if paused_by_budget(c)? && (dollars == 0.0 || spent_today(c)? < dollars) {
        resume(c)?;
    }
    Ok(())
}

pub fn spent_today(c: &Company) -> Result<f64> {
    Ok(c.spent_since(0, None)?.0)
}

/// What one run may still spend today: what's left of the company's budget and of the person's
/// own cap, whichever is less. None when neither is set, or the budget was lifted for today.
pub fn left(c: &Company, employee: &str) -> Result<Option<f64>> {
    let (budget, cap, day) = (budget(c)?, c.daily_cap(employee)?, c.today()?);
    let overridden = c.setting("paused.reason")?.is_some_and(|r| r == format!("override:{day}"));
    let company = (budget > 0.0 && !overridden).then_some(budget - spent_today(c)?);
    let own = (cap > 0.0).then_some(cap - c.spent_since(0, Some(employee))?.0);
    // A cent at least: a run only starts while there's something left.
    Ok([company, own].into_iter().flatten().reduce(f64::min).map(|d| d.max(0.01)))
}

/// Why the company is paused, when it's the budget: "budget:<day>".
pub fn paused_by_budget(c: &Company) -> Result<bool> {
    Ok(c.paused()? && c.setting("paused.reason")?.is_some_and(|r| r.starts_with("budget:")))
}

/// After a run: warn at 80%, pause at 100%, unless you chose to go on for today.
pub fn check(c: &Company) -> Result<()> {
    let budget = budget(c)?;
    if budget <= 0.0 {
        return Ok(());
    }
    let (spent, day) = (spent_today(c)?, c.today()?);
    let overridden = c.setting("paused.reason")?.is_some_and(|r| r == format!("override:{day}"));
    if spent >= budget && !c.paused()? && !overridden {
        c.set_paused(true)?;
        c.set_setting("paused.reason", &format!("budget:{day}"))?;
    } else if spent >= budget * 0.8 && c.setting("budget.warned")?.as_deref() != Some(&day) {
        c.set_setting("budget.warned", &day)?;
    }
    Ok(())
}

/// A new day lifts yesterday's budget pause. Called before work is picked up.
pub fn new_day(c: &Company) -> Result<()> {
    if let Some(r) = c.setting("paused.reason")?
        && let Some(day) = r.strip_prefix("budget:")
        && day != c.today()?
        && c.paused()?
    {
        resume(c)?;
    }
    Ok(())
}

/// Go on for today, past the budget. Tomorrow the budget applies again.
pub fn resume_for_today(c: &Company) -> Result<()> {
    c.set_paused(false)?;
    c.set_setting("paused.reason", &format!("override:{}", c.today()?))
}

fn resume(c: &Company) -> Result<()> {
    c.set_paused(false)?;
    c.set_setting("paused.reason", "")
}

/// Whether today's 80% heads-up is on.
pub fn warned_today(c: &Company) -> Result<bool> {
    Ok(c.setting("budget.warned")?.as_deref() == Some(c.today()?.as_str()))
}

/// People who reached their own daily cap.
pub fn capped(c: &Company) -> Result<Vec<String>> {
    let mut out = vec![];
    for e in c.employees()? {
        let cap = c.daily_cap(&e.id)?;
        if cap > 0.0 && c.spent_since(0, Some(&e.id))?.0 >= cap {
            out.push(e.id);
        }
    }
    Ok(out)
}
