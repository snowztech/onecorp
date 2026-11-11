//! A stand-in agent: no AI, no network, no cost. It adds one line to WORK.md in its worktree and
//! reports events like a real runtime, so the whole ticket loop can be tested end to end and the
//! app can be tried for free (`onecorp hire Demo --runtime fake`).
//!
//! A test steers it through the prompt: a ticket containing `[fail]` fails, and a conflict
//! prompt makes it resolve the markers by keeping both sides. `[fail once]` fails the first run
//! in a worktree and succeeds after, to test retries. `[limit]` answers like a plan that hit its
//! usage limit.

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

    // Ask OneCorp: a bug report draft when asked about a [bug], a plain answer otherwise.
    if prompt.starts_with("You are Ask OneCorp") {
        let text = if prompt.contains("[bug]") { "That's a bug.\nISSUE: bug | Drafts vanish\n**What happened**\nThe draft vanished." } else { "Leads can propose tickets for their team." };
        on("", Event::Done { ok: true, cost: 0.0, tokens: 0, text: text.into() })?;
        return Ok(());
    }

    // Mentioned in the team chat: a short, friendly reply that quotes what it was asked, so tests can check it.
    if let Some(said) = prompt.lines().find_map(|l| l.strip_prefix("They wrote: ")) {
        let mut text = format!("Got it: \"{}\". I'll keep it in mind on my next ticket.", said.trim());
        // Asked to [split] work: one ticket for Maya, one for Rui, and one with no title, which is dropped.
        if said.contains("[split]") {
            text.push_str("\nTICKET: Maya | Cache the checkout page | Under 200ms\n- TICKET: Rui | Write the release note | \nTICKET: Maya |  | nothing");
        }
        on("", Event::Done { ok: true, cost: 0.0, tokens: 0, text })?;
        return Ok(());
    }

    // Writing the wiki: a short one, from the prompt's company name.
    if company_ticket(o).is_some_and(|(c, t)| c.ticket(t).ok().flatten().is_some_and(|t| t.kind == "wiki")) {
        // One short page for each page the ticket asks for, like a resumed session still knows them.
        let asked = company_ticket(o).and_then(|(c, t)| c.ticket(t).ok().flatten()).map(|t| t.description).unwrap_or_default();
        let pages: Vec<String> = crate::wiki::DRAFTS
            .iter()
            .map(|(p, _)| p.to_string())
            .filter(|p| asked.trim().is_empty() || asked.lines().any(|l| l.trim() == p))
            .map(|p| format!("# {p}\n\n{}\n", if p == crate::wiki::OVERVIEW { "## What we're building\n\nA notes app for writers, from the README.\n\n## Where we are\n\nJust a README so far." } else { "- From the README, and not much else yet." }))
            .collect();
        on("", Event::Done { ok: true, cost: 0.0, tokens: 0, text: pages.join("\n") })?;
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

    // Markers come from this run's own ticket, so another ticket's title in the prompt (the team
    // list says who's on what) can't steer it.
    let own = company_ticket(o).and_then(|(c, t)| c.ticket(t).ok().flatten()).map(|t| t.title).unwrap_or_else(|| prompt.to_string());
    let first_try = o.cwd.join(".fake-tried");
    if own.contains("[fail once]") && !first_try.exists() {
        fs::write(&first_try, "")?;
        on("", Event::Done { ok: false, cost: 0.0, tokens: 0, text: "Failed on purpose, this once.".into() })?;
        return Ok(());
    }
    // Like a plan that ran out: Claude's own wording, with a reset an hour from now.
    if own.contains("[limit]") {
        let reset = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() + 3600).unwrap_or(0);
        on("", Event::Done { ok: false, cost: 0.0, tokens: 0, text: format!("Claude AI usage limit reached|{reset}") })?;
        return Ok(());
    }
    // An audit: changes no files, proposes one follow-up, and the reply is the report.
    if own.contains("[report]")
        && let Some((c, ticket)) = company_ticket(o)
    {
        let employee = o.mcp.and_then(|m| m.args.iter().position(|a| a == "--employee").and_then(|i| m.args.get(i + 1)).cloned()).unwrap_or_default();
        let ctx = crate::mcp::Ctx { ticket, employee: employee.clone() };
        let call = serde_json::json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "create_ticket", "arguments": {"title": "Add meta descriptions", "assignee": employee}}});
        crate::mcp::handle(&c, &ctx, &call);
        let text = "# SEO audit\n\nPages have no meta descriptions. Proposed a ticket to add them.".to_string();
        on("", Event::Done { ok: true, cost: 0.0, tokens: 0, text })?;
        return Ok(());
    }
    if own.contains("[fail]") {
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
    // `[land on main]` misbehaves: it commits and moves main itself, around the founder's approval.
    if own.contains("[land on main]") {
        let git = |args: &[&str]| std::process::Command::new("git").arg("-C").arg(o.cwd).args(args).envs(o.env.iter().map(|(k, v)| (k, v))).status();
        git(&["add", "-A"])?;
        git(&["commit", "-q", "-m", "straight to main"])?;
        git(&["update-ref", "refs/heads/main", "HEAD"])?;
    }
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
