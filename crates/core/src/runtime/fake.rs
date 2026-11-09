//! A stand-in agent: no AI, no network, no cost. It adds one line to WORK.md in its worktree and
//! reports events like a real runtime, so the whole ticket loop can be tested end to end and the
//! app can be tried for free (`onecorp hire Demo --runtime fake`).
//!
//! A test steers it through the prompt: a ticket containing `[fail]` fails, and a conflict
//! prompt makes it resolve the markers by keeping both sides. `[fail once]` fails the first run
//! in a worktree and succeeds after, to test retries.

use super::{Event, Opts};
use anyhow::Result;
use std::fs;

pub fn run(o: &Opts, prompt: &str, mut on: impl FnMut(&str, Event) -> Result<()>) -> Result<()> {
    let session = o.resume.map(str::to_string).unwrap_or_else(|| format!("fake-{}", std::process::id()));
    on("", Event::Init { session, model: "no AI".into() })?;

    // Asked to decide as a lead: pass up anything marked [founder], otherwise go with the default.
    if prompt.contains("End your reply with exactly one of these lines") {
        let field = |name: &str| prompt.lines().find_map(|l| l.strip_prefix(name)).map(str::trim).unwrap_or("").to_string();
        let question = field("Question: ");
        let pick = [field("Their default: "), field("Options: ").split(", ").next().unwrap_or("").to_string()].into_iter().find(|x| !x.is_empty()).unwrap_or_else(|| "yes".into());
        let text = if question.contains("[founder]") { "FOUNDER: this is a product call.".to_string() } else { format!("Looks fine to me.\nDECISION: {pick}") };
        on("", Event::Say(text.clone()))?;
        on("", Event::Done { ok: true, cost: 0.0, tokens: 0, text })?;
        return Ok(());
    }

    // Mentioned in the team chat: a short, friendly reply that quotes what it was asked, so tests can check it.
    if let Some(said) = prompt.lines().find_map(|l| l.strip_prefix("They wrote: ")) {
        let text = format!("Got it: \"{}\". I'll keep it in mind on my next ticket.", said.trim());
        on("", Event::Done { ok: true, cost: 0.0, tokens: 0, text })?;
        return Ok(());
    }

    // Writing the handbook: a short one, from the prompt's company name.
    if company_ticket(o).is_some_and(|(c, t)| c.ticket(t).ok().flatten().is_some_and(|t| t.kind == "handbook")) {
        let name = prompt.split("Start with \"# ").nth(1).and_then(|r| r.split(" handbook").next()).unwrap_or("the company");
        let building = if prompt.contains("## What we're building:") { "## What we're building\n\nA notes app for writers, from the README.\n\n" } else { "" };
        let text = format!("# {name} handbook\n\n## Mission\n\nNotes for writers.\n\n{building}## Codebase\n\n- A README, and not much else yet.\n\n## Decisions\n");
        on("", Event::Done { ok: true, cost: 0.0, tokens: 0, text })?;
        return Ok(());
    }

    // On a plan ticket, planning or planning again: create tickets through the real company tools.
    if company_ticket(o).is_some_and(|(c, t)| c.ticket(t).ok().flatten().is_some_and(|t| t.kind == "plan")) {
        let made = plan(o, prompt)?;
        let text = format!("Planned the first version in {} tickets: setup first, then the core and the docs in parallel.", made.len());
        on("", Event::Say(text.clone()))?;
        on("", Event::Done { ok: true, cost: 0.0, tokens: 0, text })?;
        return Ok(());
    }
    // Read-only means read-only, like the real runtimes: never write a file.
    if o.read_only {
        on("", Event::Done { ok: true, cost: 0.0, tokens: 0, text: "Looked around, changed nothing.".into() })?;
        return Ok(());
    }

    let first_try = o.cwd.join(".fake-tried");
    if prompt.contains("[fail once]") && !first_try.exists() {
        fs::write(&first_try, "")?;
        on("", Event::Done { ok: false, cost: 0.0, tokens: 0, text: "Failed on purpose, this once.".into() })?;
        return Ok(());
    }
    if prompt.contains("[fail]") {
        on("", Event::Done { ok: false, cost: 0.0, tokens: 0, text: "Failed on purpose.".into() })?;
        return Ok(());
    }

    let path = o.cwd.join("WORK.md");
    let mut doc = fs::read_to_string(&path).unwrap_or_default();
    let said = if prompt.contains("conflict markers") {
        doc = doc.lines().filter(|l| !l.starts_with("<<<<<<<") && !l.starts_with("=======") && !l.starts_with(">>>>>>>")).map(|l| format!("{l}\n")).collect();
        "Kept both sides and removed the conflict markers.".to_string()
    } else {
        // The line that says what to do: the ticket on a first run, the founder's words on a resume.
        let task = prompt
            .lines()
            .find(|l| l.starts_with("Your ticket is ") || l.starts_with("Their answer: "))
            .map(|l| l.trim_start_matches("Your ticket is ").trim_start_matches("Their answer: ").to_string())
            .or_else(|| prompt.split("asked for changes:").nth(1).and_then(|r| r.lines().find(|l| !l.trim().is_empty())).map(|l| format!("change: {}", l.trim())))
            .unwrap_or_else(|| "work".into());
        doc.push_str(&format!("- {task}\n"));
        format!("Wrote \"{task}\" to WORK.md.")
    };
    fs::write(&path, doc)?;
    on("", Event::Tool { name: "Write".into(), detail: path.to_string_lossy().into_owned() })?;
    on("", Event::Say(said.clone()))?;
    on("", Event::Done { ok: true, cost: 0.0, tokens: 0, text: said })?;
    Ok(())
}

/// Calls `create_ticket` like an agent would, through the company's MCP handler, in-process: the
/// stand-in has no MCP client, and in tests there's no onecorp binary to start one with.
/// The company and ticket this run is for, read from its company tool settings.
fn company_ticket(o: &Opts) -> Option<(crate::store::Company, i64)> {
    let args = &o.mcp?.args;
    let arg = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    Some((crate::store::Company::open(&arg("--company")?).ok()?, arg("--ticket")?.parse().ok()?))
}

fn plan(o: &Opts, prompt: &str) -> Result<Vec<String>> {
    let args = o.mcp.map(|m| m.args.clone()).unwrap_or_default();
    let arg = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned().unwrap_or_default();
    let (c, ticket) = company_ticket(o).ok_or_else(|| anyhow::anyhow!("no company to plan for"))?;
    let ctx = crate::mcp::Ctx { ticket, employee: arg("--employee") };
    let team = c.employees()?;
    let engineer = team.iter().find(|e| e.role.to_lowercase().contains("engineer")).or(team.first()).map(|e| e.id.clone()).unwrap_or_default();
    let writer = team.iter().find(|e| !e.role.to_lowercase().contains("engineer")).map(|e| e.id.clone()).unwrap_or(engineer.clone());
    let brief = c.brief()?;
    let idea = prompt
        .split("in the founder's words:")
        .nth(1)
        .unwrap_or(&brief)
        .lines()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("the product")
        .trim()
        .to_string();

    let mut keys: Vec<String> = vec![];
    let tickets = [
        ("Set up the project", "A clean skeleton that builds, with a README saying how to run it.", &engineer, None),
        ("Build the core", "The main thing the brief asks for, end to end.", &engineer, Some(0)),
        ("Write the docs", "How to use it, for someone who has never seen it.", &writer, Some(0)),
    ];
    for (title, details, who, after) in tickets {
        let mut args = serde_json::json!({"title": format!("{title}: {idea}"), "description": details, "assignee": who});
        if let Some(i) = after {
            args["after"] = serde_json::json!(keys[i]);
        }
        let call = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "create_ticket", "arguments": args}});
        let reply = crate::mcp::handle(&c, &ctx, &call).unwrap_or_default();
        let text = reply["result"]["content"][0]["text"].as_str().unwrap_or("").to_string();
        anyhow::ensure!(reply["result"]["isError"].is_null(), "create_ticket failed: {text}");
        keys.push(text.split_whitespace().next().unwrap_or("").to_string());
    }
    Ok(keys)
}
