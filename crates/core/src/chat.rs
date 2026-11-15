//! Team chat: #general for everyone and one channel per department. Employees post with the
//! post_message tool and read recent chat before each ticket. The founder posts from the app or
//! `onecorp say`, and whoever they @mention replies in the channel. Without a mention, whoever
//! replied last carries on.

use crate::runtime::{self, Event};
use crate::store::{Company, Employee, Message};
use crate::templates::suggest_name;
use anyhow::{Result, bail, ensure};

pub const GENERAL: &str = "general";
/// Ask OneCorp: you and OneCorp's built-in help. Not a team channel, so no employee reads it.
pub const HELP: &str = "help";
/// Who Ask OneCorp's replies are by. Never an employee id: those come from names, which are words.
pub const ONECORP: &str = "onecorp";

/// The direct message channel between you and one employee. Department channels only use
/// letters, digits and dashes, so a DM can never clash with one.
pub fn dm(employee: &str) -> String {
    format!("dm:{employee}")
}
fn dm_of(channel: &str) -> Option<&str> {
    channel.strip_prefix("dm:")
}

pub struct Channel {
    pub name: String,
    pub title: String,
    pub department: Option<i64>,
}

/// #general, then one per department, then any older channel that still has messages.
pub fn channels(c: &Company) -> Result<Vec<Channel>> {
    let mut out = vec![Channel { name: GENERAL.into(), title: "General".into(), department: None }];
    for d in c.departments()? {
        out.push(Channel { name: suggest_name(&d.name), title: d.name, department: Some(d.id) });
    }
    for name in c.channel_names()? {
        if dm_of(&name).is_none() && name != HELP && !out.iter().any(|ch| ch.name == name) {
            out.push(Channel { title: name.clone(), name, department: None });
        }
    }
    Ok(out)
}

/// A channel from what someone typed: "#Engineering", "engineering" or just "eng". Empty means #general.
pub fn resolve(c: &Company, typed: &str) -> Result<String> {
    if typed.trim() == HELP {
        return Ok(HELP.into());
    }
    if let Some(id) = dm_of(typed.trim()) {
        c.employee(id)?.ok_or_else(|| anyhow::anyhow!("nobody called {id} works here"))?;
        return Ok(dm(id));
    }
    let want = suggest_name(typed.trim().trim_start_matches('#'));
    if want.is_empty() {
        return Ok(GENERAL.into());
    }
    let all = channels(c)?;
    if all.iter().any(|ch| ch.name == want) {
        return Ok(want);
    }
    let starts: Vec<&Channel> = all.iter().filter(|ch| ch.name.starts_with(&want)).collect();
    if let [one] = starts[..] {
        return Ok(one.name.clone());
    }
    let names: Vec<String> = all.iter().map(|ch| format!("#{}", ch.name)).collect();
    bail!("there's no #{want}. Channels: {}", names.join(", "))
}

/// Your department's channel, where your posts go unless you say otherwise.
pub fn home(c: &Company, e: &Employee) -> Result<String> {
    Ok(e.department.and_then(|id| c.departments().ok()?.into_iter().find(|d| d.id == id)).map(|d| suggest_name(&d.name)).unwrap_or_else(|| GENERAL.into()))
}

/// Who's in a channel: everyone for #general, the department's people for its channel.
pub fn members(c: &Company, channel: &str) -> Result<Vec<Employee>> {
    let team = c.employees()?;
    if let Some(id) = dm_of(channel) {
        return Ok(team.into_iter().filter(|e| e.id == id).collect());
    }
    let dept = channels(c)?.into_iter().find(|ch| ch.name == channel).and_then(|ch| ch.department);
    Ok(match dept {
        Some(id) => team.into_iter().filter(|e| e.department == Some(id)).collect(),
        None => team,
    })
}

/// Who an @mention points at: @maya, @Maya, or @tomas for Tomás, and @all (or @everyone,
/// @channel) for everyone in the channel. Each person once.
pub fn mentions(c: &Company, channel: &str, text: &str) -> Result<Vec<Employee>> {
    let team = c.employees()?;
    let mut out: Vec<Employee> = vec![];
    for word in text.split_whitespace().filter_map(|w| w.strip_prefix('@')) {
        let id = crate::store::slug(word);
        if ["all", "everyone", "channel"].contains(&id.as_str()) {
            for e in members(c, channel)? {
                if !out.iter().any(|o| o.id == e.id) {
                    out.push(e);
                }
            }
            continue;
        }
        if let Some(e) = team.iter().find(|e| e.id == id)
            && !out.iter().any(|o| o.id == e.id)
        {
            out.push(e.clone());
        }
    }
    Ok(out)
}

/// The founder posts. Everyone they @mention gets a reply slot, returned so the caller can run
/// `reply` on each, right away or on a thread.
pub fn say(c: &Company, channel: &str, text: &str) -> Result<Vec<i64>> {
    let text = text.trim();
    ensure!(!text.is_empty(), "say something first");
    let channel = resolve(c, channel)?;
    let last = c.recent_messages(&channel, None, 1)?.pop();
    let asked = c.post(&channel, "founder", text, None)?;
    if channel == HELP {
        return Ok(vec![c.post_pending(HELP, ONECORP, Some(asked))?]);
    }
    // Replies cost money. A paused company reads your message later, with its next ticket.
    if c.paused()? {
        return Ok(vec![]);
    }
    // A direct message always gets its answer: there's only one person to ask. Someone at their
    // daily cap, or whose agent hit its limit, reads it later, with their next ticket.
    let capped = crate::limits::away(c)?;
    let who = if dm_of(&channel).is_some() { members(c, &channel)? } else { mentions(c, &channel, text)? };
    // No @mention carries on the conversation: whoever replied last answers. A work update or a
    // note is tied to a ticket, not a reply to you, so it doesn't count.
    let who = match last.filter(|m| who.is_empty() && m.ticket.is_none() && !m.pending) {
        Some(m) => c.employee(&m.author)?.into_iter().collect(),
        None => who,
    };
    who.iter().filter(|e| !capped.contains(&e.id)).map(|e| c.post_pending(&channel, &e.id, Some(asked))).collect()
}

/// The most questions one ticket asks teammates. Each answer is a chat run, so this keeps a
/// confused run from spending its way around the team.
const ASKS_PER_TICKET: usize = 5;

/// An employee asks a teammate during a ticket and waits for the answer, like asking across the
/// desk. The question and the answer are posted in chat, in a channel both are in, and kept on the
/// ticket. The answer comes from a chat reply, which has no company tools, so it can't ask anyone
/// in turn. Returns what the asker reads: the answer, or why there isn't one.
pub fn ask_teammate(c: &Company, asker: &str, ticket: i64, who: &str, question: &str) -> Result<String> {
    let question = question.trim();
    ensure!(!question.is_empty(), "say what you want to know");
    let Some(me) = c.employee(asker)? else { anyhow::bail!("{asker} doesn't work here") };
    let Some(them) = c.employee(&crate::store::slug(who.trim_start_matches('@')))? else {
        let names: Vec<String> = c.employees()?.into_iter().filter(|e| e.id != me.id).map(|e| format!("{} ({})", e.name, e.role)).collect();
        anyhow::bail!("nobody called {who} works here. Your teammates: {}", names.join(", "));
    };
    ensure!(them.id != me.id, "that's you. Ask a teammate");
    let first = them.name.split_whitespace().next().unwrap_or(&them.name).to_string();
    if c.paused()? {
        return Ok(format!("{first} can't answer: {} is paused. Carry on with your best judgment and say what you assumed in your summary.", c.name));
    }
    if crate::limits::away(c)?.contains(&them.id) {
        return Ok(format!("{first} can't answer right now (their daily cap or their agent's usage limit). Carry on with your best judgment and say what you assumed in your summary."));
    }
    let asked = c.events(ticket)?.iter().filter(|(_, kind, _)| kind == "teammate").count();
    if asked >= ASKS_PER_TICKET {
        return Ok(format!("You've asked teammates {ASKS_PER_TICKET} times on this ticket already. Decide with what you have, or ask the founder with ask_human."));
    }
    // A channel you both read: your team's when they're in it, otherwise #general.
    let mine = home(c, &me)?;
    let channel = if members(c, &mine)?.iter().any(|e| e.id == them.id) { mine } else { GENERAL.to_string() };
    let asked = c.post(&channel, &me.id, &format!("@{} {question}", them.id), Some(ticket))?;
    let slot = c.post_pending(&channel, &them.id, Some(asked))?;
    reply(c, slot)?;
    let answer = c.message(slot)?.map(|m| m.text).unwrap_or_default();
    c.add_event(ticket, "teammate", &format!("asked {first}: {} {first}: {}", crate::runtime::clip(question, 300), crate::runtime::clip(&answer, 600)), "")?;
    Ok(format!("{first} answered in #{channel}:\n\n{answer}"))
}

/// Says in the asker's team channel how a question is moving: asked, decided by the lead, passed
/// to the founder, answered. Only a record of what happened, so it never starts a reply or a loop.
pub fn note(c: &Company, asker: &str, author: &str, text: &str, ticket: Option<i64>) -> Result<()> {
    let channel = match c.employee(asker)? {
        Some(e) => home(c, &e)?,
        None => GENERAL.into(),
    };
    c.post(&channel, author, text, ticket)?;
    Ok(())
}

/// A mentioned employee answers in the channel. Read-only in the repo and with no company tools,
/// so a chat reply can look at the code but never changes it. When there's work to do, the reply
/// proposes tickets, for themselves or, from a lead, for their team. They wait for the founder to
/// start them.
pub fn reply(c: &Company, id: i64) -> Result<()> {
    let Some(slot) = c.message(id)? else { return Ok(()) };
    if slot.author == ONECORP {
        return crate::help::reply(c, id);
    }
    let Some(e) = c.employee(&slot.author)? else {
        return c.finish_message(id, "(they've left the company)", 0.0, 0);
    };
    const HISTORY: usize = 12;
    let before: Vec<Message> = c.recent_messages(&slot.channel, Some(id), HISTORY)?.into_iter().filter(|m| !m.pending).collect();
    // What this answers: the message it was asked for. Replies from before that was kept answer
    // whoever wrote last, founder or teammate.
    let question = match slot.reply_to.map(|q| c.message(q)).transpose()?.flatten() {
        Some(q) => Some(q),
        None => before.iter().rev().find(|m| m.author != e.id).cloned(),
    };
    let question = question.as_ref();
    let asked = question.map(|m| m.text.clone()).unwrap_or_default();
    let asker = match question.map(|m| m.author.as_str()) {
        Some(id) if id != "founder" => c.employee(id)?.map(|a| format!("{}, {} here,", a.name, a.role)),
        _ => None,
    };
    let doing = in_progress(c, &e.id)?;
    let team = team_of(c, &e)?;
    let recent: Vec<&Message> = before.iter().collect();
    // The last few in full, so nobody thinks their own message was cut off. Older ones shortened.
    const IN_FULL: usize = 4;
    let history: Vec<String> = recent
        .iter()
        .enumerate()
        .map(|(i, m)| if i + IN_FULL >= recent.len() { line_full(c, m) } else { line(c, m) })
        .collect();
    let earlier = c.setting(&summary_key(&slot.channel))?.map(|s| format!("What this conversation covered before that, in your words from last time: {s}\n\n")).unwrap_or_default();
    let said: Vec<&str> = recent.iter().map(|m| m.text.as_str()).collect();
    // What stays the same between this person's replies is their system part, which Claude Code
    // caches: an exchange of a few messages then pays mostly for the conversation, not the company.
    let system = format!(
        "You are {name}, {role} at {company}. {bio}\n\n\
         Reply like a colleague would in chat, in your own manner: plain words, no headings, and short. \
         A few lines at most, unless they asked for detail. Everyone in the channel reads it. \
         Read the code if it helps you answer, but don't change any files or run commands that need approval. \
         {checkout}\
         {propose}\n\n{comment}\n\n{summary}\n\n{rules}{notes}",
        summary = SUMMARY_NOTE,
        name = e.name,
        role = e.role,
        company = c.name,
        bio = crate::voice::intro(c, &e)?,
        checkout = checkout_note(c),
        propose = propose_note(&e, &team),
        comment = COMMENT_NOTE,
        rules = crate::work::company_rules(c, &[])?,
        notes = crate::work::own_notes(c, &e.id)?,
    );
    let prompt = format!(
        "{founder} {place}. {earlier}The latest messages, older ones shortened here (everyone saw them in full):\n\n{history}\n\n\
         They wrote: {asked}\n\n{team}{done}{tickets}{now}{mentioned}{elsewhere}",
        founder = asker.unwrap_or_else(crate::founder::name),
        place = if dm_of(&slot.channel).is_some() { "sent you a direct message".to_string() } else { format!("mentioned you in #{}, the team chat", slot.channel) },
        history = history.join("\n"),
        team = crate::work::team_now(c)?,
        // Changes with every merge, so it's here and not in the cached system part.
        done = recent_work(c)?,
        tickets = own_tickets(c, &e.id)?,
        now = doing.as_ref().map(|(_, text)| text.clone()).unwrap_or_default(),
        mentioned = mentioned_tickets(c, &said.join("\n"))?,
        elsewhere = match seen_elsewhere(c, &e, &slot.channel)? {
            lines if lines.is_empty() => String::new(),
            lines => format!("\n\nMeanwhile in your other channels, newest last:\n{}", lines.join("\n")),
        },
    );
    let repo = c.repo()?;
    let model = crate::work::talk_model(c, &e)?;
    let mut dirs = crate::work::reading_dirs(c)?;
    dirs.extend(doing.and_then(|(wt, _)| wt));
    // And their work waiting for review, which isn't on the checkout either.
    for t in c.tickets()?.into_iter().filter(|t| t.assignee.as_deref() == Some(e.id.as_str()) && t.status == "review") {
        dirs.extend(t.worktree.map(std::path::PathBuf::from).filter(|p| p.is_dir()));
    }
    let opts = runtime::Opts { cwd: &repo, model: model.as_deref(), mcp: None, resume: None, env: &one_off_cache(), read_only: true, on_spawn: None, commands: &[], dirs: &dirs, run_any: false, max_cost: None, system: Some(&system) };
    let (mut said, mut cost, mut tokens) = (String::new(), 0.0, 0);
    // The reply shows as it's written, a few times a second, not every word.
    let (mut live, mut shown_at) = (String::new(), std::time::Instant::now());
    let run = runtime::run(&e.runtime, &opts, &prompt, |_, ev| {
        match ev {
            Event::Delta(text) => {
                live.push_str(&text);
                if shown_at.elapsed() > std::time::Duration::from_millis(400) {
                    c.write_pending(id, &shown_so_far(&live))?;
                    shown_at = std::time::Instant::now();
                }
            }
            // Text before a tool call was "let me look": what shows is the answer that follows.
            Event::Tool { .. } => live.clear(),
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
    });
    let failed = match &run {
        Ok(()) => (said.trim().is_empty()).then(String::new),
        Err(err) => Some(format!("{err:#}")),
    };
    let limited = match &failed {
        Some(why) => crate::limits::hit(&e.runtime, &format!("{why} {said}"))?,
        None => false,
    };
    if run.is_ok() && !limited {
        let (kept, summary) = take_summary(&said);
        said = kept;
        if let Some(s) = summary {
            c.set_setting(&summary_key(&slot.channel), &s)?;
        }
        said = comment(c, &e, &said)?;
        said = propose(c, &e, &team, id, &said)?;
    }
    let text = match (run, limited) {
        (_, true) => format!("(My usage limit is reached. I'm back at {}.)", crate::limits::until(&e.runtime).map(crate::limits::label).unwrap_or_default()),
        (Ok(()), _) if !said.trim().is_empty() => runtime::plain(said.trim()),
        (Ok(()), _) => "(no reply)".into(),
        (Err(err), _) => format!("(couldn't reply: {err:#})"),
    };
    let cost = crate::spend::record(c, "chat", &e.id, None, &e.runtime, model.as_deref(), cost, tokens)?;
    c.finish_message(id, &text, cost, tokens)
}

/// For a run that answers once and is done, like a chat reply: its conversation is cached for
/// 5 minutes instead of Claude Code's hour. Writing to the hour-long cache costs twice the input
/// price, and nothing comes back to read a one-off answer later. The system part keeps the hour,
/// so the next reply still reads it from cache. Ticket runs keep the hour: a step there can take
/// longer than 5 minutes.
pub(crate) fn one_off_cache() -> [(String, String); 1] {
    [("CLAUDE_CODE_MESSAGES_PROMPT_CACHE_TTL".into(), "5m".into())]
}

/// The people a lead can give work to: everyone in the departments they lead.
fn team_of(c: &Company, e: &Employee) -> Result<Vec<Employee>> {
    let mut team = vec![];
    for other in c.employees()? {
        if c.lead_of(&other.id)?.is_some_and(|l| l.id == e.id) {
            team.push(other);
        }
    }
    Ok(team)
}

const TICKET: &str = "TICKET:";
const COMMENT: &str = "COMMENT:";
const SUMMARY: &str = "SUMMARY:";

/// How a reply keeps the conversation going past the messages it's shown: it ends with a summary
/// line that nobody sees, saved for the next reply in this channel. A few sentences cost a lot
/// less than resending a long conversation, and the cost stays the same however long it runs.
const SUMMARY_NOTE: &str = "End every reply with one last line, SUMMARY: then two or three sentences on what this \
    conversation has covered and agreed so far, keeping whatever the earlier summary said that still matters. \
    Nobody sees that line. It's how you remember this conversation next time.";

fn summary_key(channel: &str) -> String {
    format!("chat.summary.{channel}")
}

/// The reply without its summary line, and the summary.
fn take_summary(said: &str) -> (String, Option<String>) {
    let mut summary = None;
    let kept: Vec<&str> = said
        .lines()
        .filter(|l| match l.trim_start().trim_start_matches(['-', '*', ' ']).strip_prefix(SUMMARY) {
            Some(rest) => {
                summary = Some(rest.trim().to_string()).filter(|s| !s.is_empty());
                false
            }
            None => true,
        })
        .collect();
    (kept.join("\n").trim_end().to_string(), summary)
}

/// What a reply being written shows so far: up to its first line for OneCorp, like a proposed
/// ticket or its summary, which are taken out when it's done.
fn shown_so_far(live: &str) -> String {
    let end = live.lines().position(|l| {
        let l = l.trim_start().trim_start_matches(['-', '*', ' ']);
        [TICKET, COMMENT, SUMMARY].iter().any(|m| l.starts_with(m) || (m.starts_with(l) && !l.is_empty() && l.chars().all(|c| c.is_ascii_uppercase())))
    });
    let text = match end {
        Some(n) => live.lines().take(n).collect::<Vec<_>>().join("\n"),
        None => live.to_string(),
    };
    runtime::plain(text.trim_end())
}

const COMMENT_NOTE: &str = "To add a comment to an existing ticket, so it stays with the ticket and whoever works on it reads it, \
    put it after your reply: a line COMMENT: <ticket key>, then the comment on the lines below. It ends at the next COMMENT: or TICKET: line. \
    That and proposing tickets are all you can do from chat: you can't change files, or start, move or close tickets. \
    Never say you did something you can't do here. Say what the founder can do instead.";

fn marker(line: &str) -> Option<(&'static str, &str)> {
    let l = line.trim_start().trim_start_matches(['-', '*', ' ']);
    [TICKET, COMMENT].into_iter().find_map(|m| l.strip_prefix(m).map(|rest| (m, rest)))
}

/// Saves the reply's comment blocks on their tickets and returns the reply without them. A comment
/// on a ticket that doesn't exist is said in the reply, so nobody believes it was posted.
fn comment(c: &Company, e: &Employee, said: &str) -> Result<String> {
    let (mut keep, mut failed) = (vec![], vec![]);
    let mut open: Option<(String, Vec<&str>)> = None;
    let mut save = |block: Option<(String, Vec<&str>)>| -> Result<()> {
        let Some((key, body)) = block else { return Ok(()) };
        let text = runtime::plain(body.join("\n").trim());
        match c.parse_key(&key).ok().filter(|id| c.ticket(*id).ok().flatten().is_some()) {
            Some(id) if !text.is_empty() => c.add_event(id, "comment", &format!("{}: {text}", e.name), ""),
            Some(_) => Ok(()),
            None => {
                failed.push(key);
                Ok(())
            }
        }
    };
    for line in said.lines() {
        match marker(line) {
            Some((COMMENT, rest)) => {
                let (key, first) = rest.split_once('|').unwrap_or((rest, ""));
                save(open.take())?;
                open = Some((key.trim().to_string(), vec![first.trim()].into_iter().filter(|f| !f.is_empty()).collect()));
            }
            Some(_) => {
                save(open.take())?;
                keep.push(line);
            }
            None => match &mut open {
                Some((_, body)) => body.push(line),
                None => keep.push(line),
            },
        }
    }
    save(open.take())?;
    let mut out = keep.join("\n");
    if !failed.is_empty() {
        out.push_str(&format!("\n\n(Couldn't comment on {}: no such ticket.)", failed.join(", ")));
    }
    Ok(out)
}

fn propose_note(e: &Employee, team: &[Employee]) -> String {
    let who = if team.is_empty() {
        "yourself".to_string()
    } else {
        let names: Vec<String> = team.iter().map(|t| format!("{} ({})", t.name, t.role)).collect();
        format!("yourself, or someone on your team: {}. You lead them, so split the work the way you would", names.join(", "))
    };
    format!(
        "If they ask you to look into, fix or build something that needs real work, don't just suggest it: propose the tickets.          After your reply, add one line per ticket: {TICKET} who | title | what to do and how you'll know it's done.          Who is {who}. Keep it to the few tickets that matter. The founder sees them under your message and starts them with one click.          For a question or a chat, propose nothing. Your name here is {}.",
        e.name
    )
}

/// Turns the reply's ticket lines into tickets in the backlog, linked to the message, and returns
/// the reply without them. Work for someone outside the lead's team goes to whoever proposed it.
fn propose(c: &Company, e: &Employee, team: &[Employee], message: i64, said: &str) -> Result<String> {
    const MOST: usize = 8;
    let (lines, rest): (Vec<&str>, Vec<&str>) = said.lines().partition(|l| l.trim_start().trim_start_matches(['-', '*', ' ']).starts_with(TICKET));
    for line in lines.iter().take(MOST) {
        let body = line.trim_start().trim_start_matches(['-', '*', ' ']).trim_start_matches(TICKET);
        let mut parts = body.splitn(3, '|').map(str::trim);
        let (who, title, details) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next().unwrap_or(""));
        if title.is_empty() {
            continue;
        }
        let named = |x: &Employee| [x.id.as_str(), x.name.as_str(), x.name.split_whitespace().next().unwrap_or("")].iter().any(|n| crate::store::slug(n) == crate::store::slug(who));
        let assignee = team.iter().find(|t| named(t)).map_or(e.id.clone(), |t| t.id.clone());
        let new = crate::store::NewTicket {
            title: title.into(),
            description: details.into(),
            assignee: Some(assignee),
            created_by: Some(e.id.clone()),
            ..Default::default()
        };
        let id = c.new_ticket(&new, "backlog")?;
        c.link_message(id, message)?;
        c.add_event(id, "create", &format!("proposed by {} in chat", e.name), "")?;
    }
    Ok(rest.join("\n"))
}

/// The tickets someone has been given, newest last, so a chat reply knows its own work.
pub fn own_tickets(c: &Company, employee: &str) -> Result<String> {
    const RECENT: usize = 10;
    // Work in review isn't on the checkout a reply reads, so where it is comes with it.
    let line = |t: &crate::store::Ticket| match t.worktree.as_deref().filter(|w| t.status == "review" && std::path::Path::new(w).is_dir()) {
        Some(wt) => format!("- {} {} (review, your changes are in {wt})", c.key(t.id), t.title),
        None => format!("- {} {} ({})", c.key(t.id), t.title, t.status),
    };
    let mine: Vec<String> = c.tickets()?.iter().filter(|t| t.assignee.as_deref() == Some(employee)).map(line).collect();
    if mine.is_empty() {
        return Ok(String::new());
    }
    let recent = &mine[mine.len().saturating_sub(RECENT)..];
    Ok(format!("\n\nYour tickets, newest last. You did these yourself:\n{}", recent.join("\n")))
}

/// The ticket someone is on right now, with their latest steps and where their changes are, so a
/// chat reply about it is true. The checkout a reply reads isn't their worktree.
fn in_progress(c: &Company, employee: &str) -> Result<Option<(Option<std::path::PathBuf>, String)>> {
    const STEPS: usize = 12;
    let Some(t) = c.tickets()?.into_iter().find(|t| t.assignee.as_deref() == Some(employee) && ["doing", "blocked"].contains(&t.status.as_str())) else {
        return Ok(None);
    };
    let wt = t.worktree.as_deref().map(std::path::PathBuf::from).filter(|p| p.is_dir());
    let events = c.events(t.id)?;
    let steps: Vec<&(String, String, String)> = events.iter().filter(|(_, k, _)| ["say", "tool", "error"].contains(&k.as_str())).collect();
    let steps: Vec<String> = steps[steps.len().saturating_sub(STEPS)..]
        .iter()
        .map(|(at, k, s)| format!("{} {k} {}", at.get(11..16).unwrap_or(at), crate::runtime::clip(&s.replace('\n', " "), 200)))
        .collect();
    let place = match &wt {
        Some(p) => format!(" Your changes so far are in your worktree, {}, not in the checkout you're reading: look there.", p.display()),
        None => String::new(),
    };
    let text = format!(
        "\n\nYou're on {} {} right now ({}).{place} Your latest steps, newest last:\n{}",
        c.key(t.id),
        t.title,
        t.status,
        if steps.is_empty() { "(nothing yet)".to_string() } else { steps.join("\n") }
    );
    Ok(Some((wt, text)))
}

/// Tickets named in the conversation, like SNO-12, in full: what they ask for, who has them and
/// where they stand. Tickets live in the company, not the repo, so this is the only way a reply
/// can read one. The latest three mentioned, newest first.
pub fn mentioned_tickets(c: &Company, text: &str) -> Result<String> {
    const MAX: usize = 1_500;
    let prefix = c.key(0).rsplit_once('-').map(|(p, _)| format!("{p}-")).unwrap_or_default();
    let mut ids: Vec<i64> = vec![];
    for word in text.split(|ch: char| !ch.is_ascii_alphanumeric() && ch != '-').rev() {
        let Some(num) = word.to_ascii_uppercase().strip_prefix(&prefix).and_then(|n| n.parse::<i64>().ok()) else { continue };
        if !ids.contains(&num) {
            ids.push(num);
        }
    }
    let cut = |s: &str| if s.chars().count() > MAX { format!("{}…", s.chars().take(MAX).collect::<String>()) } else { s.to_string() };
    let mut out = vec![];
    for id in ids {
        let Some(t) = c.ticket(id)? else { continue };
        let who = match t.assignee.as_deref() {
            Some(a) => c.employee(a)?.map(|e| e.name).unwrap_or_else(|| a.to_string()),
            None => "nobody yet".into(),
        };
        let mut block = format!("{} {}\nStatus: {}. Assigned to {who}.", c.key(id), t.title, t.status);
        if !t.description.trim().is_empty() {
            block.push_str(&format!("\nWhat it asks: {}", cut(t.description.trim())));
        }
        if let Some(last) = crate::view::last_said(c, id).filter(|s| !s.trim().is_empty()) {
            block.push_str(&format!("\nLast update: {}", cut(last.trim())));
        }
        out.push(block);
        if out.len() == 3 {
            break;
        }
    }
    if out.is_empty() {
        return Ok(String::new());
    }
    Ok(format!("\n\nTickets mentioned in this conversation, as they stand now:\n\n{}", out.join("\n\n")))
}

/// The repo a chat reply reads is the founder's own checkout, which may be on a branch of theirs.
/// Said plainly, so a reply doesn't mistake the founder's work in progress for the team's.
fn checkout_note(c: &Company) -> String {
    let Ok(repo) = c.repo() else { return String::new() };
    let (Ok(here), Ok(base)) = (crate::git::current_branch(&repo), crate::git::default_branch(&repo)) else { return String::new() };
    if here == base {
        return String::new();
    }
    format!("The checkout you're reading is the founder's own and is on {here}, their branch. The team's work starts from {base}, so read {base} for where things stand. ")
}

/// Recent chat for a ticket prompt: #general and the employee's own channel, plus anything that
/// mentions them elsewhere. Empty when there's nothing.
pub fn catch_up(c: &Company, e: &Employee) -> Result<String> {
    let lines = seen_elsewhere(c, e, "")?;
    if lines.is_empty() {
        return Ok(String::new());
    }
    Ok(format!(
        "\n\nRecent team chat, newest last. Your posts go to #{} unless you name another channel:\n{}",
        home(c, e)?,
        lines.join("\n")
    ))
}

/// What the whole team finished lately, the same for every chat reply and Ask OneCorp: newest
/// first, one line each with who, when and the first sentence of their summary. Without it a
/// reply only knows its own tickets, so work merged since the last reply goes unseen. Kept short
/// on purpose: about 20 lines, a few hundred tokens. Older work is there by naming the ticket.
pub fn recent_work(c: &Company) -> Result<String> {
    const SHOWN: usize = 20;
    const GIST: usize = 160;
    let done = c.recently_done(SHOWN)?;
    if done.is_empty() {
        return Ok(String::new());
    }
    let names: std::collections::HashMap<String, String> = c.employees()?.into_iter().map(|e| (e.id, e.name)).collect();
    let lines: Vec<String> = done
        .iter()
        .map(|f| {
            let (id, title, at, said) = (&f.id, &f.title, &f.at, &f.said);
            let who = f.assignee.as_deref().map(|w| names.get(w).and_then(|n| n.split_whitespace().next()).unwrap_or(w).to_string()).unwrap_or_else(|| "nobody".into());
            let flat = said.replace('\n', " ");
            // The first sentence says what changed. The rest is detail for the reviewer.
            let gist = flat.split_inclusive(". ").next().unwrap_or("").trim();
            let gist = if gist.is_empty() { String::new() } else { format!(": {}", runtime::clip(gist, GIST)) };
            format!("- {} {} ({who}, {}){gist}", c.key(*id), title, at.get(..10).unwrap_or(at))
        })
        .collect();
    let total = c.done_count()?;
    let more = if total > SHOWN as i64 { format!(" The {} before these: name one, like {}, to see it.", total - SHOWN as i64, c.key(1)) } else { String::new() };
    Ok(format!("\n\nWhat the team finished lately, newest first. All of it is merged.{more}\n{}", lines.join("\n")))
}

/// What someone has seen lately outside `skip`: #general, their team's channel, their DMs, and
/// anything that @mentions them. Newest last.
fn seen_elsewhere(c: &Company, e: &Employee, skip: &str) -> Result<Vec<String>> {
    const RECENT: usize = 15;
    let (home, dm) = (home(c, e)?, dm(&e.id));
    Ok(c.messages_seen(&[GENERAL, &home, &dm], &format!("@{}", e.id), skip, RECENT)?
        .iter()
        .map(|m| format!("{} {}", if dm_of(&m.channel).is_some() { "(direct message)".to_string() } else { format!("#{}", m.channel) }, line(c, m)))
        .collect())
}

/// "Name: text", with long messages cut so one rant can't fill the prompt. The cut is marked,
/// or an employee reading its own reply thinks it failed to send and says it again.
/// Like `line`, with the whole message.
fn line_full(c: &Company, m: &Message) -> String {
    let who = line(c, &Message { text: String::new(), ..m.clone() });
    format!("{who}{}", m.text.replace('\n', " "))
}

pub(crate) fn line(c: &Company, m: &Message) -> String {
    let who = if m.author == "founder" {
        format!("{} (founder)", crate::founder::name())
    } else {
        c.employee(&m.author).ok().flatten().map(|e| e.name).unwrap_or_else(|| m.author.clone())
    };
    const MAX: usize = 400;
    let mut text: String = m.text.chars().take(MAX).collect();
    if m.text.chars().count() > MAX {
        text.push('…');
    }
    format!("{who}: {}", text.replace('\n', " "))
}

#[cfg(test)]
mod tests {
    use super::{shown_so_far, take_summary};

    #[test]
    fn the_summary_line_is_kept_apart() {
        let (said, summary) = take_summary("Yes, Friday works.\nSUMMARY: Release is Friday, Sam owns the notes.");
        assert_eq!((said.as_str(), summary.as_deref()), ("Yes, Friday works.", Some("Release is Friday, Sam owns the notes.")));
        assert_eq!(take_summary("No summary here").1, None);
        assert_eq!(take_summary("ok\n- SUMMARY:  ").1, None, "an empty one is no summary");
    }

    #[test]
    fn a_reply_being_written_hides_what_isnt_for_people() {
        assert_eq!(shown_so_far("Sure, on it.\nTICKET: Maya | Fix add"), "Sure, on it.");
        assert_eq!(shown_so_far("Sure.\nSUMM"), "Sure.", "even half written");
        assert_eq!(shown_so_far("All good.\nSUMMARY: x"), "All good.");
        assert_eq!(shown_so_far("Plain text"), "Plain text");
    }
}
