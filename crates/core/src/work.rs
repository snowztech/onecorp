//! Running tickets: starting them, sending them back to their employee, approving them.
//! The CLI and `onecorp serve` both go through `dispatch`, which runs a job now or queues it for serve.

use crate::runtime::{self, Event, first_line};
use crate::store::{self, Company, Employee, Question, Ticket};
use crate::{git, serve, ship};
use anyhow::{Result, anyhow, bail};
use std::cell::RefCell;
use std::path::Path;

thread_local! {
    /// Prefix for every line this thread prints. Empty in the CLI. Serve sets it per job,
    /// like "SNO-3 maya  ", so parallel tickets stay readable.
    static TAG: RefCell<String> = const { RefCell::new(String::new()) };
}

pub fn set_tag(tag: &str) {
    TAG.with(|t| *t.borrow_mut() = tag.to_string());
}

pub(crate) fn out(s: &str) {
    TAG.with(|t| {
        let t = t.borrow();
        for line in s.split('\n') {
            println!("{t}{line}");
        }
    });
}

/// Work waiting for an employee.
pub enum Job {
    /// Create the worktree and do the ticket from scratch.
    Start,
    /// Continue the ticket's session with new instructions.
    Resume { why: String, prompt: String, label: String },
    /// Ask the asker's department lead to answer an inbox question, or pass it up to the founder.
    Consult { question: i64 },
}

/// Queues the job when `onecorp serve` is running, otherwise runs it right here.
pub fn dispatch(c: &Company, ticket: i64, job: Job) -> Result<()> {
    let capped = c.ticket(ticket)?.and_then(|t| t.assignee).is_some_and(|a| crate::spend::capped(c).is_ok_and(|v| v.contains(&a)));
    if c.paused()? || capped {
        let key = c.key(ticket);
        c.enqueue(ticket, &job)?;
        c.set_status(ticket, "queued")?;
        let why = if capped { "its assignee reached their daily cap".to_string() } else { format!("{} is paused", c.name) };
        out(&format!("{key} is queued: {why}. It starts when that changes, or with: onecorp resume"));
        return Ok(());
    }
    if !serve::running() {
        // Recorded as a job too, so a Ctrl-C mid-run gets noticed instead of leaving the ticket "doing".
        let id = c.run_now(ticket, &job)?;
        let result = execute(c, ticket, &job);
        c.finish_job(id, result.as_ref().err().map(|e| format!("{e:#}")).as_deref())?;
        result?;
        // Questions the agent asked its lead were queued during the run. With nobody to pick them up, do it now.
        return drain_consults(c);
    }
    let key = c.key(ticket);
    c.enqueue(ticket, &job)?;
    c.set_status(ticket, "queued")?;
    out(&format!("{key} is queued. onecorp serve picks it up. Follow along: onecorp logs {key}"));
    Ok(())
}

pub fn execute(c: &Company, ticket: i64, job: &Job) -> Result<()> {
    let t = c.ticket(ticket)?.ok_or_else(|| anyhow!("no ticket {}", c.key(ticket)))?;
    match job {
        Job::Start => run_ticket(c, &t),
        Job::Resume { why, prompt, label } => pick_up(c, &t, why, prompt, label),
        Job::Consult { question } => consult(c, *question),
    }
}

/// Runs everything queued, here, for when serve isn't running. Used after resuming a paused company.
pub fn run_queued(c: &Company) -> Result<()> {
    while let Some(job) = c.claim_job()? {
        let result = execute(c, job.ticket, &job.job);
        c.finish_job(job.id, result.as_ref().err().map(|e| format!("{e:#}")).as_deref())?;
        result?;
    }
    Ok(())
}

/// Runs queued lead consultations here, for when serve isn't running.
pub fn drain_consults(c: &Company) -> Result<()> {
    if c.paused()? {
        return Ok(());
    }
    while let Some(job) = c.claim_job_of("consult")? {
        let result = execute(c, job.ticket, &job.job);
        c.finish_job(job.id, result.as_ref().err().map(|e| format!("{e:#}")).as_deref())?;
        result?;
    }
    Ok(())
}

/// A lead's verdict on a question from someone in their department.
#[derive(Debug, PartialEq)]
pub enum Verdict {
    Decide(String),
    Escalate(String),
}

/// Reads the last DECISION: or FOUNDER: line. Anything unclear goes up to the founder, which is the safe side.
pub fn verdict(text: &str) -> Verdict {
    for line in text.lines().rev() {
        let l = line.trim().trim_start_matches(['*', '_', '`']).trim();
        let upper = l.to_ascii_uppercase();
        if let Some(rest) = upper.strip_prefix("DECISION:").map(|_| &l["DECISION:".len()..]) {
            let answer = rest.trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, '*' | '_' | '`'));
            if !answer.is_empty() {
                return Verdict::Decide(answer.to_string());
            }
        }
        if upper.starts_with("FOUNDER:") {
            let why = l["FOUNDER:".len()..].trim_matches(|ch: char| ch.is_whitespace() || matches!(ch, '*' | '_' | '`'));
            return Verdict::Escalate(why.to_string());
        }
    }
    Verdict::Escalate(first_line(text))
}

/// The lead looks at a question from their report, read-only in the asker's worktree, and either
/// answers it, which resumes the asker like a founder answer would, or passes it up to the founder.
fn consult(c: &Company, id: i64) -> Result<()> {
    let Some(q) = c.question(id)? else { return Ok(()) };
    if q.answer.is_some() || q.stage != "lead" {
        return Ok(()); // The founder already stepped in.
    }
    let Some(lead) = c.lead_of(&q.employee)? else {
        c.route(id, "founder", None, None)?;
        return Ok(());
    };
    let Some(t) = q.ticket_id.map(|tid| c.ticket(tid)).transpose()?.flatten() else { return Ok(()) };
    let key = c.key(t.id);
    let asker = c.employee(&q.employee)?.map(|e| e.name).unwrap_or_else(|| q.employee.clone());
    let dept = c.departments()?.into_iter().find(|d| d.lead.as_deref() == Some(lead.id.as_str())).map(|d| d.name).unwrap_or_else(|| "your department".into());
    let mut details = format!("Question: {}", q.question);
    if !q.options.is_empty() {
        details.push_str(&format!("\nOptions: {}", q.options));
    }
    if !q.default_answer.is_empty() {
        details.push_str(&format!("\nTheir default: {}", q.default_answer));
    }
    let prompt = format!(
        "You are {name}, {role} at {company}, and you lead {dept}. {bio}\n\n\
         {asker}, who reports to you, asked a question while working on {key}: {title}\n\n{details}\n\n\
         Decide it yourself when the wiki, the code in this worktree or sound judgment settle it. Pass it to the founder \
         when it's about product direction, money, anything public, security, or something the wiki says is theirs. \
         Look at the code if it helps, but don't change any files.\n\n\
         End your reply with exactly one of these lines:\nDECISION: the answer, in a few words\nFOUNDER: why the founder should decide\n\n{knowledge}",
        name = lead.name,
        role = lead.role,
        company = c.name,
        bio = lead.bio,
        title = t.title,
        knowledge = company_knowledge(c)?,
    );
    out(&format!("{key}  {} asked {}: {}", first_line(&asker), lead.name, q.question));

    let cwd = t.worktree.as_deref().map(std::path::PathBuf::from).filter(|p| p.exists()).unwrap_or(c.repo()?);
    let mcp = company_tools(c, t.id, &lead.id)?;
    let opts = runtime::Opts { cwd: &cwd, model: lead.model.as_deref(), mcp: Some(&mcp), resume: None, env: &[], read_only: true, on_spawn: None };
    let (mut said, mut cost, mut tokens) = (String::new(), 0.0, 0);
    runtime::run(&lead.runtime, &opts, &prompt, |_, ev| {
        match ev {
            Event::Say(text) => said = text,
            Event::Done { cost: c2, tokens: t2, text, .. } => {
                (cost, tokens) = (c2, t2);
                if !text.is_empty() {
                    said = text;
                }
            }
            _ => {}
        }
        Ok(())
    })?;
    let cost = crate::spend::record(c, &lead.id, Some(t.id), &lead.runtime, lead.model.as_deref(), cost, tokens)?;
    c.add_cost(t.id, cost, tokens)?;

    match verdict(&said) {
        Verdict::Decide(answer) => {
            c.add_event(t.id, "lead", &format!("{} answered #{id}: {answer}", lead.name), "")?;
            out(&format!("{key}  {} answered: {answer}", lead.name));
            let q = c.answer_as(id, &answer, &lead.id)?;
            follow_up(c, &q, &answer)
        }
        Verdict::Escalate(why) => {
            c.route(id, "founder", Some(&lead.id), Some(&why))?;
            c.add_event(t.id, "lead", &format!("{} passed #{id} to you: {why}", lead.name), "")?;
            out(&format!("{key}  {} passed it to you: {}. See: onecorp inbox", lead.name, why.trim_end_matches('.')));
            Ok(())
        }
    }
}

/// The company MCP server for one ticket and employee: this same binary, started by the agent.
fn company_tools(c: &Company, ticket: i64, employee: &str) -> Result<runtime::Mcp> {
    Ok(runtime::Mcp {
        command: std::env::current_exe()?.to_string_lossy().into_owned(),
        args: ["mcp", "--company", &c.name, "--ticket", &ticket.to_string(), "--employee", employee].map(String::from).to_vec(),
        env: vec![("ONECORP_HOME".into(), store::home().to_string_lossy().into_owned())],
    })
}

/// Dollars when the runtime reports them, tokens otherwise (Codex on a subscription has no bill per run).
pub fn spend(cost: f64, tokens: i64) -> String {
    if cost > 0.0 || tokens == 0 { format!("${cost:.2}") } else { format!("{}k tok", (tokens + 500) / 1000) }
}

/// Cuts at a word boundary so commit subjects don't end mid-word.
pub fn short(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max - 3).collect();
    format!("{}...", cut.rsplit_once(' ').map(|(a, _)| a).unwrap_or(&cut))
}

fn employee_of(c: &Company, t: &Ticket) -> Result<Employee> {
    let who = t.assignee.as_deref().unwrap_or("");
    c.employee(who)?.ok_or_else(|| anyhow!("{who} no longer works at {}, so {} stays {}", c.name, c.key(t.id), t.status))
}

fn run_ticket(c: &Company, t: &Ticket) -> Result<()> {
    if t.kind == "plan" {
        return run_plan(c, t);
    }
    if t.kind == "wiki" {
        return run_wiki(c, t);
    }
    let (id, key, title) = (t.id, c.key(t.id), t.title.as_str());
    let e = employee_of(c, t)?;
    let wt = c.dir.join("wt").join(key.to_lowercase());
    let repo = c.repo()?;
    let started = git::current_branch(&repo).and_then(|base| {
        // With pull requests, start from what's on the forge, not whatever is checked out locally.
        let from = ship::start_point(c, &repo, &base)?;
        let branch = git::add_worktree(&repo, &wt, &format!("onecorp/{}/{}", c.name, key.to_lowercase()), &from)?;
        Ok((branch, base))
    });
    let (branch, base) = match started {
        Ok(b) => b,
        Err(err) => {
            c.finish_ticket(id, "failed", 0.0, 0)?;
            return Err(err);
        }
    };
    c.start_ticket(id, &branch, &base, &wt)?;
    out(&format!("{key}  {title}\n      {} is on it ({}), in {}\n", e.name, e.runtime, wt.display()));

    let prompt = format!(
        "You are {name}, {role} at {company}. {bio}\n\n\
         Your ticket is {key}: {title}\n\n{details}\
         You are in a git worktree on branch {branch}. {abilities} Don't commit: when you finish, \
         OneCorp commits your changes under your name. \
         End with two or three sentences: what you changed and anything the reviewer should check.\n\n\
         You have company tools. If a decision is the founder's to make, ask with ask_human: give a default and carry on, \
         or set blocking=true and stop if you truly can't continue. Use post_message for things teammates should know. \
         If something outside your control stops you, use update_ticket to mark the ticket blocked. \
         If you learn or decide something every teammate should know from now on, propose it with propose_wiki. \
         If you learn something that would help you on future tickets, like where code lives or how the founder likes things, \
         save it with remember: one line per note.\n\n\
         {knowledge}{notes}{chat}",
        name = e.name,
        role = e.role,
        company = c.name,
        bio = e.bio,
        abilities = runtime::abilities(&e.runtime),
        details = ticket_details(c, t)?,
        knowledge = company_knowledge(c)?,
        notes = own_notes(c, &e.id)?,
        chat = crate::chat::catch_up(c, &e)?,
    );
    drive(c, &e, id, &wt, None, &prompt, title)
}

/// The ticket's details and who asked for it, as a paragraph for the prompt. Empty when there are none.
fn ticket_details(c: &Company, t: &Ticket) -> Result<String> {
    let mut out = String::new();
    if !t.description.is_empty() {
        out.push_str(&format!("Details:\n{}\n\n", t.description));
    }
    if t.created_by != "founder"
        && let Some(e) = c.employee(&t.created_by)?
    {
        out.push_str(&format!("{}, {}, created this ticket.\n\n", e.name, e.role));
    }
    Ok(out)
}

/// Starts a saved ticket. It needs someone to do it, and if it waits for another ticket that
/// isn't merged yet, it waits, then starts by itself once that one lands.
pub fn start(c: &Company, t: &Ticket) -> Result<()> {
    let key = c.key(t.id);
    if !["backlog", "todo", "waiting"].contains(&t.status.as_str()) {
        bail!("{key} is {}. Only tickets that haven't started can be started", t.status);
    }
    let Some(who) = &t.assignee else { bail!("{key} has nobody on it yet. Give it to someone: onecorp assign {key} <name>") };
    if c.employee(who)?.is_none() {
        bail!("{who} no longer works at {}. Give {key} to someone else: onecorp assign {key} <name>", c.name);
    }
    if let Some(dep) = t.after.map(|a| c.ticket(a)).transpose()?.flatten().filter(|d| d.status != "done") {
        c.set_status(t.id, "waiting")?;
        c.add_event(t.id, "wait", &format!("waits for {} to be merged", c.key(dep.id)), "")?;
        out(&format!("{key} starts once {} is merged.", c.key(dep.id)));
        return Ok(());
    }
    c.set_status(t.id, "todo")?;
    dispatch(c, t.id, Job::Start)
}

/// Starts the tickets that were waiting for `done` to be merged.
pub fn release_waiting(c: &Company, done: i64) -> Result<()> {
    for id in c.waiting_for(done)? {
        if let Some(t) = c.ticket(id)? {
            out(&format!("{} was waiting for {}, so it starts now.", c.key(id), c.key(done)));
            start(c, &t)?;
        }
    }
    Ok(())
}

/// Who plans: the one you pick, or someone whose role sounds like product, or a department lead,
/// or whoever's first on the team.
pub fn pick_planner(c: &Company, by: Option<&str>) -> Result<Employee> {
    let team = c.employees()?;
    if let Some(by) = by.filter(|b| !b.trim().is_empty()) {
        let id = store::slug(by);
        return team.into_iter().find(|e| e.id == id).ok_or_else(|| anyhow!("nobody called {by} works at {}", c.name));
    }
    let leads: Vec<String> = c.departments()?.into_iter().filter_map(|d| d.lead).collect();
    let product = |e: &&Employee| ["product", "pm", "manager", "founder's office"].iter().any(|w| e.role.to_lowercase().contains(w)) && !e.role.to_lowercase().contains("designer");
    let pick = team.iter().find(product).or_else(|| team.iter().find(|e| leads.contains(&e.id))).or(team.first());
    pick.map(|e| c.employee(&e.id)).transpose()?.flatten().ok_or_else(|| anyhow!("nobody works at {} yet. Hire someone to plan with", c.name))
}

/// Kicks off the brief: the planner reads it and the repo, and proposes the first tickets as a plan.
/// Returns the plan ticket.
pub fn kickoff(c: &Company, by: Option<&str>) -> Result<i64> {
    if c.brief()?.is_empty() {
        bail!("write the brief first: what you're building and who it's for. onecorp brief --set \"...\"");
    }
    plan_project(c, "First version", "", by)
}

/// Deletes a ticket that isn't running: one that's merged, stopped, or never started. Merged
/// work stays in your repo. A stopped ticket's worktree and branch go with it.
pub fn delete(c: &Company, t: &Ticket) -> Result<()> {
    let key = c.key(t.id);
    match t.status.as_str() {
        "doing" | "queued" | "blocked" => bail!("{key} is still running. Stop it first, then delete it"),
        "review" => bail!("{key} is waiting for your review. Merge it or dismiss it first"),
        _ => {}
    }
    if t.status == "failed"
        && t.kind != "plan"
        && let (Some(wt), Some(branch)) = (t.worktree.as_deref(), t.branch.as_deref())
    {
        git::discard(&c.repo()?, Path::new(wt), branch, &c.dir.join("wt"));
    }
    c.forget_ticket(t.id)?;
    out(&format!("Deleted {key}."));
    Ok(())
}

/// Has someone read the repo and write the company wiki, so the team starts out knowing the
/// codebase. Your engineering lead by default. Runs read-only, like a plan.
pub fn write_wiki(c: &Company, by: Option<&str>) -> Result<i64> {
    let writer = match by {
        Some(id) => c.employee(id)?.ok_or_else(|| anyhow!("nobody called {id} works here"))?,
        None => {
            let depts = c.departments()?;
            let lead = depts.iter().find(|d| d.name.to_lowercase().contains("eng")).or(depts.first()).and_then(|d| d.lead.clone());
            match lead.and_then(|id| c.employee(&id).ok().flatten()) {
                Some(e) => e,
                None => c.employees()?.into_iter().next().ok_or_else(|| anyhow!("hire someone first"))?,
            }
        }
    };
    let id = c.new_ticket(&crate::store::NewTicket { title: "Write the wiki".into(), assignee: Some(writer.id.clone()), ..Default::default() }, "todo")?;
    c.set_kind(id, "wiki")?;
    out(&format!("{} writes the wiki: {}.", c.key(id), writer.name));
    dispatch(c, id, Job::Start)?;
    Ok(id)
}

fn run_wiki(c: &Company, t: &Ticket) -> Result<()> {
    let (id, key) = (t.id, c.key(t.id));
    let e = employee_of(c, t)?;
    c.start_plan_ticket(id)?;
    let mission = wiki_section(&c.wiki()?, "Mission");
    let prompt = format!(
        "You are {name}, {role} at {company}. {bio}\n\n\
         Your ticket is {key}: write the company wiki. Every teammate reads it before every ticket, so it's how a \
         new hire learns this codebase. Read the repo as much as you need, but don't change any files.\n\n\
         Reply with the wiki in Markdown and nothing else. Start with \"# {company} wiki\", then these sections:\n\
         ## Mission: {mission}\n\
         {building}\
         ## Codebase: what this is, the stack, and where things live, folder by folder.\n\
         ## Build and test: the exact commands, and what to run before calling work done.\n\
         ## Conventions: what you can see the code already does (naming, structure, style, testing). Only what's really there.\n\
         ## Decisions: leave it empty.\n\
         Short bullets, plain words, nothing invented. If something isn't clear from the repo, leave it out.\n\n{knowledge}",
        name = e.name,
        role = e.role,
        company = c.name,
        bio = e.bio,
        building = match c.brief()?.trim() {
            "" => "## What we're building: the product, who it's for and what it does today, in a short paragraph. The team plans from it.\n".to_string(),
            b => format!("## What we're building: keep exactly what the founder wrote: {b}\n"),
        },
        mission = if mission.is_empty() { "one or two sentences on what the company is for, from the README.".to_string() } else { format!("keep exactly this: {mission}") },
        knowledge = company_knowledge(c)?,
    );
    drive(c, &e, id, &c.repo()?, None, &prompt, &t.title)?;
    // The reply is the wiki. Only a reply that looks like one replaces what's there.
    let said = c.events(id)?.into_iter().rev().find(|(_, kind, _)| kind == "wiki-text").map(|(_, _, s)| s).unwrap_or_default();
    let doc = said.trim().trim_start_matches("```markdown").trim_start_matches("```md").trim_start_matches("```").trim_end_matches("```").trim();
    let t = c.ticket(id)?.ok_or_else(|| anyhow!("{key} disappeared"))?;
    if t.status == "review" && doc.starts_with('#') && doc.contains("\n## ") {
        // What you wrote about what we're building always wins over the draft.
        let yours = c.brief()?;
        c.write_wiki(&format!("{doc}\n"))?;
        if !yours.is_empty() {
            c.write_brief(&yours)?;
        }
        c.set_status(id, "done")?;
        c.add_event(id, "wiki", "wrote the wiki", "")?;
        out(&format!("{key}  the wiki is written. Read it: onecorp wiki"));
    } else if t.status == "review" {
        c.set_status(id, "failed")?;
        c.add_event(id, "error", "the reply wasn't a wiki page, so nothing was changed", "")?;
    }
    Ok(())
}

/// The text of one `## name` section of a Markdown document.
fn wiki_section(doc: &str, name: &str) -> String {
    let mut lines = doc.lines().skip_while(|l| !l.strip_prefix("## ").is_some_and(|h| h.trim().eq_ignore_ascii_case(name)));
    lines.next();
    lines.take_while(|l| !l.starts_with("## ")).collect::<Vec<_>>().join("\n").trim().to_string()
}

/// Plans a project: a feature or a goal, like "Stripe billing". The planner breaks it into tickets
/// you review. With no goal it plans the first version from the company brief.
pub fn plan_project(c: &Company, name: &str, goal: &str, by: Option<&str>) -> Result<i64> {
    let name = name.trim();
    if name.is_empty() {
        bail!("name the project, like Stripe billing");
    }
    let planner = pick_planner(c, by)?;
    let id = c.new_plan(name, goal.trim(), &planner.id)?;
    out(&format!("{} plans {name} with {}, {}.", c.key(id), planner.name, planner.role));
    dispatch(c, id, Job::Start)?;
    Ok(id)
}

fn run_plan(c: &Company, t: &Ticket) -> Result<()> {
    let (id, key) = (t.id, c.key(t.id));
    let e = employee_of(c, t)?;
    c.start_plan_ticket(id)?;
    out(&format!("{key}  {}\n      {} is on it ({}), reading the repo\n", t.title, e.name, e.runtime));
    let prompt = format!(
        "You are {name}, {role} at {company}. {bio}\n\n\
         Your ticket is {key}: {what} You're in the repo read-only: look around as much as you need, but don't change files.\n\n\
         Break it into {size} tickets with create_ticket, in the order they should happen. Each ticket:\n\
         - is small enough to finish in a day, and has a clear title\n\
         - says in its description what done looks like, so whoever picks it up needs nothing else\n\
         - goes to the teammate who fits it best, by role, from the team list\n\
         - waits for an earlier ticket with after only when it really can't start without it, so work can run in parallel\n\n\
         Create them one by one. Each call tells you the new ticket's key, for use in after. If something the plan depends on \
         is the founder's call, ask with ask_human. End with a short summary: the milestones, what runs in parallel, and the risks.\n\n\
         {knowledge}",
        name = e.name,
        role = e.role,
        company = c.name,
        bio = e.bio,
        what = if t.description.trim().is_empty() {
            "plan the first version of what the founder wants to build, using the brief below and the code in this repo.".to_string()
        } else {
            format!("plan the project \"{}\". What the founder wants from it:\n\n{}\n\nThe brief below is the product it's part of.", t.title, t.description.trim())
        },
        size = if t.description.trim().is_empty() { "5 to 12" } else { "2 to 10" },
        knowledge = company_knowledge(c)?,
    );
    drive(c, &e, id, &c.repo()?, None, &prompt, &t.title)
}

/// Starts a reviewed plan: each of its tickets starts, and ones waiting for others start in order
/// as those land. Tickets with nobody on them stay in the backlog.
pub fn start_plan(c: &Company, plan: &Ticket) -> Result<()> {
    let key = c.key(plan.id);
    if plan.status != "review" {
        bail!("{key} is {}. A plan starts once it's ready for review", plan.status);
    }
    let tickets = c.plan_tickets(plan.id)?;
    let (mut started, mut skipped) = (0, vec![]);
    for t in tickets.iter().filter(|t| t.status == "backlog") {
        if t.assignee.is_none() {
            skipped.push(c.key(t.id));
            continue;
        }
        start(c, t)?;
        started += 1;
    }
    c.set_status(plan.id, "done")?;
    c.add_event(plan.id, "merge", &format!("the founder started {started} tickets"), "")?;
    out(&format!("Started {key}: {started} tickets."));
    if !skipped.is_empty() {
        out(&format!("{} have nobody on them yet, so they wait in the backlog: onecorp assign <ticket> <name>", skipped.join(", ")));
    }
    Ok(())
}

/// Removes the plan's tickets that haven't started. Returns how many.
fn drop_plan_tickets(c: &Company, plan: &Ticket) -> Result<usize> {
    let mut n = 0;
    for t in c.plan_tickets(plan.id)?.iter().filter(|t| t.status == "backlog") {
        c.delete_ticket(t.id)?;
        n += 1;
    }
    Ok(n)
}

/// Throws a plan away: its tickets go, and the plan ticket stops. Retry it to plan from scratch.
pub fn dismiss_plan(c: &Company, plan: &Ticket) -> Result<()> {
    let key = c.key(plan.id);
    if plan.kind != "plan" {
        bail!("{key} isn't a plan");
    }
    let n = drop_plan_tickets(c, plan)?;
    c.set_status(plan.id, "failed")?;
    c.add_event(plan.id, "stop", &format!("dismissed by the founder, {n} tickets removed"), "")?;
    out(&format!("Dismissed {key} and its {n} tickets. Plan again anytime: onecorp retry {key}"));
    Ok(())
}

/// What every employee should know before starting: the wiki, and who does what.
pub(crate) fn company_knowledge(c: &Company) -> Result<String> {
    // Keeps a runaway wiki from eating the whole context window.
    const MAX: usize = 12_000;
    let mut doc = c.wiki()?.trim().to_string();
    if doc.len() > MAX {
        let cut = (0..=MAX).rev().find(|&i| doc.is_char_boundary(i)).unwrap_or(0);
        doc.truncate(cut);
        doc.push_str("\n\n(The wiki goes on. Ask the founder if you need more.)");
    }
    let team: Vec<String> = c.employees()?.iter().map(|e| format!("- {}, {} (works on {})", e.name, e.role, e.runtime)).collect();
    let mut out = String::new();
    if let Some(f) = crate::founder::load() {
        out.push_str(&format!("You work for {}, the founder of {}. Questions with ask_human go to them.\n\n", f.name, c.name));
    }
    if !doc.is_empty() {
        out.push_str(&format!("The company wiki. Everyone reads it, so follow it:\n\n{doc}\n\n"));
    }
    out.push_str(&format!("Who works here:\n{}", team.join("\n")));
    Ok(out)
}

/// The employee's own notes, newest kept when they get long.
pub(crate) fn own_notes(c: &Company, employee: &str) -> Result<String> {
    const MAX: usize = 4_000;
    let notes = c.memory(employee)?;
    let mut lines: Vec<&str> = notes.lines().filter(|l| !l.trim().is_empty()).collect();
    while lines.iter().map(|l| l.len() + 1).sum::<usize>() > MAX && lines.len() > 1 {
        lines.remove(0);
    }
    if lines.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("\n\nYour own notes from past tickets. Only you see these:\n{}", lines.join("\n")))
}

/// Sends the founder's answer back to the ticket's session.
pub fn resume(c: &Company, q: &Question) -> Result<()> {
    let Some(ticket) = q.ticket_id else { return Ok(()) };
    // Who answered, so the asker knows whether it came from their lead or the founder.
    let by = match q.answered_by.as_deref() {
        Some(id) if id != "founder" => c.employee(id)?.map(|e| format!("{}, your lead,", e.name)).unwrap_or_else(|| "Your lead".into()),
        _ => crate::founder::load().map(|f| format!("{}, the founder,", f.name)).unwrap_or_else(|| "The founder".into()),
    };
    let prompt = format!(
        "{by} answered your question #{n}: \"{question}\"\n\nTheir answer: {answer}\n\n\
         Pick the ticket back up from where you left off and apply this answer. Same rules as before: \
         don't commit, and end with two or three sentences on what changed.",
        n = q.id,
        question = q.question,
        answer = q.answer.as_deref().unwrap_or(""),
    );
    let label = format!("apply answer to #{}", q.id);
    dispatch(c, ticket, Job::Resume { why: "with your answer".into(), prompt, label })
}

/// Settles an inbox item. A wiki proposal is accepted with "yes", dropped with "no", or replaced
/// by your own wording. A question's answer resumes the ticket when it unblocks it, or when the
/// employee already went with a different default.
pub fn answer(c: &Company, id: i64, text: &str) -> Result<()> {
    let text = text.trim();
    if text.is_empty() {
        bail!("what's the answer? For example: onecorp answer {id} \"go with 15 minutes\"");
    }
    let q = c.answer(id, text)?;
    if q.kind == "wiki" {
        match text.to_lowercase().as_str() {
            "no" | "n" => out("Left out of the wiki."),
            yes => {
                let line = if matches!(yes, "yes" | "y") { q.question.as_str() } else { text };
                c.add_to_wiki(&q.options, line)?;
                out(&format!("Added to {}: {line}", q.options));
            }
        }
        return Ok(());
    }
    out(&format!("Answered #{id}."));
    follow_up(c, &q, text)
}

/// What an answer sets in motion, whoever gave it: it resumes the ticket when it unblocks it,
/// or when the employee already went ahead with a different default.
fn follow_up(c: &Company, q: &Question, text: &str) -> Result<()> {
    let Some(tid) = q.ticket_id else { return Ok(()) };
    let (key, status) = (c.key(tid), c.ticket(tid)?.map(|t| t.status).unwrap_or_default());

    if q.blocking && status == "blocked" {
        match c.open_blocking(tid)? {
            0 => resume(c, q)?,
            n => out(&format!("{key} is still waiting on {n} more: onecorp inbox")),
        }
    } else if !q.blocking && status == "review" && !q.default_answer.eq_ignore_ascii_case(text) {
        // They went ahead with their default. A different answer means the work needs a change.
        let went_with = if q.default_answer.is_empty() { "their own call".to_string() } else { format!("\"{}\"", q.default_answer) };
        let who = c.employee(&q.employee)?.map(|e| e.name).unwrap_or_else(|| q.employee.clone());
        out(&format!("{who} went with {went_with}, so {key} goes back to apply the answer.\n"));
        resume(c, q)?;
    }
    Ok(())
}

/// Sends a ticket in review back with the founder's feedback. The feedback also goes into the
/// employee's memory, since how the founder reviews is the most useful thing a coworker can learn.
pub fn request_changes(c: &Company, t: &Ticket, feedback: &str) -> Result<()> {
    let key = c.key(t.id);
    let feedback = feedback.trim();
    if t.status != "review" {
        bail!("{key} is {}. You can only ask for changes on a ticket in review", t.status);
    }
    if feedback.is_empty() {
        bail!("what should change? For example: onecorp changes {key} \"use a match instead of if/else\"");
    }
    let prompt = if t.kind == "plan" {
        let dropped = drop_plan_tickets(c, t)?;
        format!(
            "The founder read your plan and wants it redone:\n\n{feedback}\n\n\
             Your {dropped} tickets were removed. Create the new plan from scratch with create_ticket, same rules as before, \
             and end with a short summary of it."
        )
    } else {
        format!(
            "The founder reviewed your work and asked for changes:\n\n{feedback}\n\n\
             Make them. Same rules as before: don't commit, and end with two or three sentences on what changed."
        )
    };
    c.add_event(t.id, "review", &format!("changes requested: {}", first_line(feedback)), "")?;
    if let Some(who) = &t.assignee {
        c.remember(who, &format!("On {key} the founder asked for changes: {feedback}"))?;
    }
    dispatch(c, t.id, Job::Resume { why: "with your feedback".into(), prompt, label: "changes from review".into() })
}

/// Stops a ticket: ends its agent if it's working, or cancels it if it's waiting in the queue.
/// Either way it ends up stopped (failed), and `retry` picks it back up, in the same session when there is one.
pub fn stop(c: &Company, t: &Ticket) -> Result<()> {
    let key = c.key(t.id);
    match t.status.as_str() {
        "doing" => {
            c.add_event(t.id, "stop", "stopped by the founder", "")?;
            // No pid means nothing to end: it's between steps, or on the stand-in runtime.
            if let Some(pid) = c.agent_pid(t.id)? {
                // Ask nicely first, so the agent can wrap up its session file.
                let kill = |sig: &str| std::process::Command::new("kill").args([sig, &pid.to_string()]).status();
                let _ = kill("-TERM");
                for _ in 0..30 {
                    if !store::alive(pid) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                if store::alive(pid) {
                    let _ = kill("-KILL");
                }
                c.set_agent_pid(t.id, None)?;
            }
            c.set_status(t.id, "failed")?;
            out(&format!("Stopped {key}. Pick it back up anytime: onecorp retry {key}"));
        }
        "queued" => {
            c.cancel_queued(t.id)?;
            c.add_event(t.id, "stop", "cancelled before it started", "")?;
            c.set_status(t.id, "failed")?;
            out(&format!("Cancelled {key} before it started. Run it later: onecorp retry {key}"));
        }
        "waiting" => {
            c.set_status(t.id, "backlog")?;
            c.add_event(t.id, "stop", "back to the backlog, it won't start by itself", "")?;
            out(&format!("{key} is back in the backlog. Start it again: onecorp start {key}"));
        }
        s => bail!("{key} is {s}, so there's nothing to stop"),
    }
    Ok(())
}

/// Tries a failed ticket again. With a session and a worktree to go back to, the same session picks
/// up where it stopped, so finished work isn't redone. Without them, it starts over cleanly.
pub fn retry(c: &Company, t: &Ticket) -> Result<()> {
    let key = c.key(t.id);
    if t.status != "failed" {
        bail!("{key} is {}. Only tickets that stopped or failed can be retried", t.status);
    }
    c.add_event(t.id, "retry", "the founder asked to try again", "")?;
    // A plan starts over from the brief. It has no worktree of its own to resume or throw away:
    // it runs on your repo, read-only.
    if t.kind == "plan" {
        drop_plan_tickets(c, t)?;
        c.reset_ticket(t.id)?;
        return dispatch(c, t.id, Job::Start);
    }
    let resumable = t.session_id.is_some() && t.worktree.as_deref().is_some_and(|w| Path::new(w).exists());
    if resumable {
        let prompt = format!(
            "Your last run on {key} stopped before it finished. Check what's already in your worktree, then carry on \
             and finish the ticket: {}\n\nSame rules as before: don't commit, and end with two or three sentences on what changed.",
            t.title
        );
        return dispatch(c, t.id, Job::Resume { why: "where it stopped".into(), prompt, label: "finish after retry".into() });
    }
    if let (Some(wt), Some(branch)) = (&t.worktree, &t.branch) {
        git::discard(&c.repo()?, Path::new(wt), branch, &c.dir.join("wt"));
    }
    c.reset_ticket(t.id)?;
    dispatch(c, t.id, Job::Start)
}

/// Sends a ticket back to its employee: same session, same worktree, new instructions.
fn pick_up(c: &Company, t: &Ticket, why: &str, prompt: &str, label: &str) -> Result<()> {
    let key = c.key(t.id);
    let (Some(wt), Some(session)) = (&t.worktree, &t.session_id) else {
        bail!("{key} has no session to pick back up");
    };
    let e = employee_of(c, t)?;
    c.set_status(t.id, "doing")?;
    out(&format!("{key}  {}\n      {} picks it back up {why}\n", t.title, e.name));
    drive(c, &e, t.id, Path::new(wt), Some(session), prompt, label)
}

/// Runs one agent session on a ticket, logging every event, commits what it left, then reports where the ticket landed.
/// `label` names this run in the commit message, like the ticket title or "changes from review".
fn drive(c: &Company, e: &Employee, id: i64, wt: &Path, resume: Option<&str>, prompt: &str, label: &str) -> Result<()> {
    let key = c.key(id);
    // The agent starts this same binary as the company's MCP server, scoped to this ticket.
    let mcp = company_tools(c, id, &e.id)?;
    let identity = git::identity(&e.name, &format!("{}@{}.onecorp", e.id, c.name));
    let remember_pid = |pid: u32| {
        let _ = c.set_agent_pid(id, Some(pid));
    };
    // A plan only reads the repo and creates tickets. It must never change your checkout.
    let planning = c.ticket(id)?.is_some_and(|t| t.kind == "plan" || t.kind == "wiki");
    let opts = runtime::Opts { cwd: wt, model: e.model.as_deref(), mcp: Some(&mcp), resume, env: &identity, read_only: planning, on_spawn: Some(&remember_pid) };

    let wt_prefix = format!("{}/", wt.display());
    let mut outcome = None;
    let mut last_say = String::new();
    let run = runtime::run(&e.runtime, &opts, prompt, |raw, ev| {
        let (kind, summary) = match &ev {
            Event::Init { session, model } => {
                c.set_session(id, session)?;
                let verb = if resume.is_some() { "resumed" } else { "started" };
                // Codex doesn't say which model it picked, so fall back to what we asked for.
                let model = if model.is_empty() { e.model.as_deref().unwrap_or("default model") } else { model };
                ("start", format!("{} ({model}), {verb}", e.runtime))
            }
            Event::Say(text) => {
                last_say = text.clone();
                ("say", first_line(text))
            }
            Event::Tool { name, detail } => {
                let name = name.strip_prefix("mcp__onecorp__").unwrap_or(name);
                ("tool", first_line(&format!("{name} {}", detail.replace(&wt_prefix, ""))).trim_end().to_string())
            }
            Event::ToolResult { error: true, text } => ("error", text.clone()),
            Event::ToolResult { .. } => return Ok(()),
            Event::Done { ok, cost, tokens, text } => {
                outcome = Some((*ok, *cost, *tokens, text.clone()));
                let verb = if *ok { "finished" } else { "failed" };
                ("done", format!("{verb}, {}", spend(*cost, *tokens as i64)))
            }
        };
        c.add_event(id, kind, &summary, raw)?;
        out(&format!("  {kind:<6} {summary}"));
        Ok(())
    })
    .inspect_err(|_| {
        let _ = c.set_agent_pid(id, None);
        let _ = c.finish_ticket(id, "failed", 0.0, 0);
    });
    if let Err(err) = run {
        // A stop you asked for ends the agent on purpose. That's not an error worth shouting about.
        // The agent may print a few more lines after the stop, so look back to this run's start.
        let stopped = c.events(id)?.iter().rev().take_while(|(_, kind, _)| kind != "start").any(|(_, kind, _)| kind == "stop");
        if stopped {
            out(&format!("{key} stopped. Pick it back up anytime: onecorp retry {key}"));
            return Ok(());
        }
        return Err(err);
    }
    c.set_agent_pid(id, None)?;

    let (ok, cost, tokens, text) = outcome.unwrap_or_default();
    let text = if text.is_empty() { last_say } else { text };
    // A writing ticket's whole reply is its work, kept in full for whoever reads it next.
    if c.ticket(id)?.is_some_and(|t| t.kind == "wiki") {
        c.add_event(id, "wiki-text", &text, "")?;
    }
    if ok && !planning {
        let msg = short(&format!("{key}: {label}"), 72);
        if let Some(hash) = git::commit_all(wt, &msg, &identity)? {
            c.add_event(id, "commit", &format!("{hash} {msg}"), "")?;
            out(&format!("  commit {hash} as {}", e.name));
        }
    }
    let cost = crate::spend::record(c, &e.id, Some(id), &e.runtime, e.model.as_deref(), cost, tokens)?;
    c.finish_ticket(id, if ok { "review" } else { "failed" }, cost, tokens)?;

    let mut t = c.ticket(id)?.ok_or_else(|| anyhow!("{key} disappeared"))?;
    if t.status == "review" && !planning {
        ship::publish_or_note(c, &t, &text);
        t = c.ticket(id)?.ok_or_else(|| anyhow!("{key} disappeared"))?;
    }
    let what = match t.status.as_str() {
        "review" => match &t.pr_url {
            Some(url) => format!("ready for review: {url}"),
            None => "ready for review".to_string(),
        },
        "blocked" => "blocked. See what they need: onecorp inbox".to_string(),
        s => s.to_string(),
    };
    out(&format!("\n{text}\n\n{key} is {what}"));
    out(&format!(
        "Branch {}, this run {}, ticket total {}.",
        t.branch.unwrap_or_default(),
        spend(cost, tokens as i64),
        spend(t.cost_usd, t.tokens)
    ));
    let waiting = c.inbox()?.len();
    if waiting > 0 && t.status != "blocked" {
        let item = if waiting == 1 { "item" } else { "items" };
        out(&format!("{waiting} {item} in your inbox: onecorp inbox"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Verdict, verdict};

    #[test]
    fn reads_a_leads_verdict() {
        assert_eq!(verdict("Checked the auth code.\nDECISION: 15 minutes"), Verdict::Decide("15 minutes".into()));
        assert_eq!(verdict("**DECISION:** use sha2"), Verdict::Decide("use sha2".into()));
        assert_eq!(verdict("decision: Keep it"), Verdict::Decide("Keep it".into()));
        assert_eq!(verdict("FOUNDER: pricing is your call"), Verdict::Escalate("pricing is your call".into()));
        assert_eq!(verdict("**FOUNDER:** it touches billing"), Verdict::Escalate("it touches billing".into()));
        assert_eq!(verdict("Not sure what to do here."), Verdict::Escalate("Not sure what to do here.".into()));
    }

    #[test]
    fn short_cuts_at_a_word() {
        assert_eq!(super::short("SNO-1: fix it", 72), "SNO-1: fix it");
        assert_eq!(super::short("SNO-1: change the description line", 24), "SNO-1: change the...");
    }
}
