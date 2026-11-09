//! Running tickets: starting them, sending them back to their employee, approving them.
//! The CLI and `onecorp serve` both go through `dispatch`, which runs a job now or queues it for serve.

use crate::agent::{self, Event, first_line};
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
}

/// Queues the job when `onecorp serve` is running, otherwise runs it right here.
pub fn dispatch(c: &Company, ticket: i64, job: Job) -> Result<()> {
    if !serve::running() {
        return execute(c, ticket, &job);
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
    }
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
        abilities = agent::abilities(&e.runtime),
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
    let prompt = format!(
        "The founder answered your question #{n}: \"{question}\"\n\nTheir answer: {answer}\n\n\
         Pick the ticket back up from where you left off and apply this answer. Same rules as before: \
         don't commit, and end with two or three sentences on what changed.",
        n = q.id,
        question = q.question,
        answer = q.answer.as_deref().unwrap_or(""),
    );
    let label = format!("apply answer to #{}", q.id);
    dispatch(c, ticket, Job::Resume { why: "with your answer".into(), prompt, label })
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
    let mcp = agent::Mcp {
        command: std::env::current_exe()?.to_string_lossy().into_owned(),
        args: ["mcp", "--company", &c.name, "--ticket", &id.to_string(), "--employee", &e.id].map(String::from).to_vec(),
        env: vec![("ONECORP_HOME".into(), store::home().to_string_lossy().into_owned())],
    };
    let identity = git::identity(&e.name, &format!("{}@{}.onecorp", e.id, c.name));
    let opts = agent::Opts { cwd: wt, model: e.model.as_deref(), mcp: &mcp, resume, env: &identity };

    let wt_prefix = format!("{}/", wt.display());
    let mut outcome = None;
    let mut last_say = String::new();
    agent::run(&e.runtime, &opts, prompt, |raw, ev| {
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
        let _ = c.finish_ticket(id, "failed", 0.0, 0);
    })?;

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
    #[test]
    fn short_cuts_at_a_word() {
        assert_eq!(super::short("SNO-1: fix it", 72), "SNO-1: fix it");
        assert_eq!(super::short("SNO-1: change the description line", 24), "SNO-1: change the...");
    }
}
