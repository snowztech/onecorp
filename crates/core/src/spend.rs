//! What your team spends. Every run lands in a ledger in dollars: what the agent reports, or an
//! estimate from its tokens when it doesn't say (Codex reports tokens only). Budgets are checked
//! after every run: a heads-up at 80% of the day's budget, and a pause at 100%, lifted the next
//! day. People can have their own daily cap too.

use crate::store::{Company, home};
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

/// The agents you use on a subscription, like Claude Max or ChatGPT Pro, instead of paying per
/// use. Yours, not a company's, so it's kept once for all of them.
pub fn plans() -> serde_json::Map<String, Value> {
    std::fs::read_to_string(home().join("plans.json")).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

pub fn set_plan(runtime: &str, subscription: bool) -> Result<()> {
    let mut all = plans();
    if subscription {
        all.insert(runtime.into(), json!(true));
    } else {
        all.remove(runtime);
    }
    std::fs::create_dir_all(home())?;
    std::fs::write(home().join("plans.json"), serde_json::to_string_pretty(&all)?)?;
    Ok(())
}

/// Each agent you use on a subscription: the API value of what all your companies ran on it this
/// month, and how often it hit its usage limit this week.
pub fn plan_use() -> Result<Vec<Value>> {
    let plans = plans();
    let mut out = vec![];
    for runtime in plans.keys() {
        let (mut month, mut limits) = (0.0, 0);
        for name in crate::store::list()? {
            let Ok(c) = Company::open(&name) else { continue };
            let start = format!("{}-01", &c.spent_day(0)?[..7]);
            month += c.spend_rows()?.iter().filter(|r| r.2 == *runtime && r.0 >= start).map(|r| r.4).sum::<f64>();
            let label = if runtime == "codex" { "Codex" } else { "Claude Code" };
            limits += c.limit_waits(7)?.iter().filter(|s| s.starts_with(label)).count();
        }
        out.push(json!({"runtime": runtime, "month": month, "limits": limits}));
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
