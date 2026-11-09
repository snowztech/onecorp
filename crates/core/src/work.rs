//! Running tickets: starting them, sending them back to their employee, approving them.
//! The CLI and `onecorp serve` both go through `dispatch`, which runs a job now or queues it for serve.

use crate::runtime::{self, Event, first_line};
use crate::store::{self, Company, Employee, Question, Ticket};
use crate::{git, serve};
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

fn out(s: &str) {
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
        Job::Start => start(c, &t),
        Job::Resume { why, prompt, label } => pick_up(c, &t, why, prompt, label),
        Job::Consult { question } => consult(c, *question),
    }
}

/// Runs queued lead consultations here, for when serve isn't running.
pub fn drain_consults(c: &Company) -> Result<()> {
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
         Decide it yourself when the handbook, the code in this worktree or sound judgment settle it. Pass it to the founder \
         when it's about product direction, money, anything public, security, or something the handbook says is theirs. \
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
    let opts = runtime::Opts { cwd: &cwd, model: lead.model.as_deref(), mcp: &mcp, resume: None, env: &[], read_only: true, on_spawn: None };
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

fn start(c: &Company, t: &Ticket) -> Result<()> {
    let (id, key, title) = (t.id, c.key(t.id), t.title.as_str());
    let e = employee_of(c, t)?;
    let wt = c.dir.join("wt").join(key.to_lowercase());
    let repo = c.repo()?;
    let started = git::current_branch(&repo).and_then(|base| {
        let branch = git::add_worktree(&repo, &wt, &format!("onecorp/{}/{}", c.name, key.to_lowercase()))?;
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
         Your ticket is {key}: {title}\n\n\
         You are in a git worktree on branch {branch}. {abilities} Don't commit: when you finish, \
         OneCorp commits your changes under your name. \
         End with two or three sentences: what you changed and anything the reviewer should check.\n\n\
         You have company tools. If a decision is the founder's to make, ask with ask_human: give a default and carry on, \
         or set blocking=true and stop if you truly can't continue. Use post_message for things teammates should know. \
         If something outside your control stops you, use update_ticket to mark the ticket blocked. \
         If you learn or decide something every teammate should know from now on, propose it with propose_handbook. \
         If you learn something that would help you on future tickets, like where code lives or how the founder likes things, \
         save it with remember: one line per note.\n\n\
         {knowledge}{notes}",
        name = e.name,
        role = e.role,
        company = c.name,
        bio = e.bio,
        abilities = runtime::abilities(&e.runtime),
        knowledge = company_knowledge(c)?,
        notes = own_notes(c, &e.id)?,
    );
    drive(c, &e, id, &wt, None, &prompt, title)
}

/// What every employee should know before starting: the handbook, and who does what.
fn company_knowledge(c: &Company) -> Result<String> {
    // Keeps a runaway handbook from eating the whole context window.
    const MAX: usize = 12_000;
    let mut doc = c.handbook()?.trim().to_string();
    if doc.len() > MAX {
        let cut = (0..=MAX).rev().find(|&i| doc.is_char_boundary(i)).unwrap_or(0);
        doc.truncate(cut);
        doc.push_str("\n\n(The handbook goes on. Ask the founder if you need more.)");
    }
    let team: Vec<String> = c.employees()?.iter().map(|e| format!("- {}, {} (works on {})", e.name, e.role, e.runtime)).collect();
    let mut out = String::new();
    if !doc.is_empty() {
        out.push_str(&format!("The company handbook. Everyone reads it, so follow it:\n\n{doc}\n\n"));
    }
    out.push_str(&format!("Who works here:\n{}", team.join("\n")));
    Ok(out)
}

/// The employee's own notes, newest kept when they get long.
fn own_notes(c: &Company, employee: &str) -> Result<String> {
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
        _ => "The founder".into(),
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

/// Settles an inbox item. A handbook proposal is accepted with "yes", dropped with "no", or replaced
/// by your own wording. A question's answer resumes the ticket when it unblocks it, or when the
/// employee already went with a different default.
pub fn answer(c: &Company, id: i64, text: &str) -> Result<()> {
    let text = text.trim();
    if text.is_empty() {
        bail!("what's the answer? For example: onecorp answer {id} \"go with 15 minutes\"");
    }
    let q = c.answer(id, text)?;
    if q.kind == "handbook" {
        match text.to_lowercase().as_str() {
            "no" | "n" => out("Left out of the handbook."),
            yes => {
                let line = if matches!(yes, "yes" | "y") { q.question.as_str() } else { text };
                c.add_to_handbook(&q.options, line)?;
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
    let prompt = format!(
        "The founder reviewed your work and asked for changes:\n\n{feedback}\n\n\
         Make them. Same rules as before: don't commit, and end with two or three sentences on what changed."
    );
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
        git::discard(&c.repo()?, Path::new(wt), branch);
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

/// Merges a reviewed ticket into its base branch. A conflict goes back to the employee to fix.
pub fn approve(c: &Company, t: &Ticket) -> Result<()> {
    let key = c.key(t.id);
    match t.status.as_str() {
        "review" => {}
        "done" => bail!("{key} is already merged"),
        s => bail!("{key} is {s}. Only tickets in review can be approved"),
    }
    let (Some(wt), Some(branch)) = (&t.worktree, &t.branch) else { bail!("{key} has no branch to merge") };
    let (repo, wt) = (c.repo()?, Path::new(wt));
    let base = match &t.base {
        Some(b) => b.clone(),
        None => git::current_branch(&repo)?,
    };
    let who = t.assignee.as_deref().and_then(|id| c.employee(id).ok().flatten());
    let identity = who.as_ref().map(|e| git::identity(&e.name, &format!("{}@{}.onecorp", e.id, c.name))).unwrap_or_default();

    if let git::Merge::Conflicts(files) = git::merge_base_into(wt, &base, &identity)? {
        let name = who.as_ref().map(|e| e.name.as_str()).unwrap_or("its owner");
        out(&format!("{base} moved since {key} started, and these files conflict: {}", files.join(", ")));
        out(&format!("Sending it back to {name} to fix.\n"));
        c.add_event(t.id, "merge", &format!("conflicts with {base}: {}", files.join(", ")), "")?;
        let prompt = format!(
            "The founder wants to merge your work, but {base} moved since you started and git found conflicts in: {files}.\n\n\
             The merge is in progress in your worktree. Open each file, resolve the conflict markers so both sides' intent is kept, \
             and remove every marker. Don't run git merge, git add or git commit: OneCorp finishes the merge when you're done. \
             End with two or three sentences on how you resolved each conflict.",
            files = files.join(", "),
        );
        let job = Job::Resume { why: "to fix the conflicts".into(), prompt, label: "resolve conflicts".into() };
        dispatch(c, t.id, job)?;
        out(&format!("Once it's back in review and you've checked it, approve again: onecorp approve {key}"));
        return Ok(());
    }

    git::fast_forward(&repo, &base, branch)?;
    c.set_status(t.id, "done")?;
    c.add_event(t.id, "merge", &format!("merged into {base}"), "")?;
    // The work is safe on base now, so a failed cleanup is only worth a warning.
    if let Err(e) = git::clean_up(&repo, wt, branch) {
        out(&format!("warning: merged, but couldn't remove the worktree: {e:#}"));
    }
    out(&format!("Merged {key} into {base}. New tickets start from it."));
    Ok(())
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
    let opts = runtime::Opts { cwd: wt, model: e.model.as_deref(), mcp: &mcp, resume, env: &identity, read_only: false, on_spawn: Some(&remember_pid) };

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
    if ok {
        let msg = short(&format!("{key}: {label}"), 72);
        if let Some(hash) = git::commit_all(wt, &msg, &identity)? {
            c.add_event(id, "commit", &format!("{hash} {msg}"), "")?;
            out(&format!("  commit {hash} as {}", e.name));
        }
    }
    c.finish_ticket(id, if ok { "review" } else { "failed" }, cost, tokens)?;

    let t = c.ticket(id)?.ok_or_else(|| anyhow!("{key} disappeared"))?;
    let what = match t.status.as_str() {
        "review" => "ready for review".to_string(),
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
