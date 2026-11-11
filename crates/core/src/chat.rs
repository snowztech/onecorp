//! Team chat: #general for everyone and one channel per department. Employees post with the
//! post_message tool and read recent chat before each ticket. The founder posts from the app or
//! `onecorp say`, and whoever they @mention replies in the channel.

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
    c.post(&channel, "founder", text, None)?;
    if channel == HELP {
        return Ok(vec![c.post_pending(HELP, ONECORP)?]);
    }
    // Replies cost money. A paused company reads your message later, with its next ticket.
    if c.paused()? {
        return Ok(vec![]);
    }
    // A direct message always gets its answer: there's only one person to ask. Someone at their
    // daily cap, or whose agent hit its limit, reads it later, with their next ticket.
    let capped = crate::limits::away(c)?;
    let who = if dm_of(&channel).is_some() { members(c, &channel)? } else { mentions(c, &channel, text)? };
    who.iter().filter(|e| !capped.contains(&e.id)).map(|e| c.post_pending(&channel, &e.id)).collect()
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
    let before: Vec<Message> = c.messages(Some(&slot.channel))?.into_iter().filter(|m| m.id < id && !m.pending).collect();
    let asked = before.iter().rev().find(|m| m.author == "founder").map(|m| m.text.clone()).unwrap_or_default();
    let doing = in_progress(c, &e.id)?;
    let team = team_of(c, &e)?;
    let recent: Vec<&Message> = before.iter().rev().take(20).rev().collect();
    let history: Vec<String> = recent.iter().map(|m| line(c, m)).collect();
    let said: Vec<&str> = recent.iter().map(|m| m.text.as_str()).collect();
    let prompt = format!(
        "You are {name}, {role} at {company}. {bio}\n\n\
         {founder} {place}. The conversation so far, with long messages shortened here (everyone saw them in full):\n\n{history}\n\n\
         They wrote: {asked}\n\n\
         Reply like a colleague would in chat, in your own manner: plain words, no headings, and short. \
         A few lines at most, unless they asked for detail. Everyone in the channel reads it. \
         Read the code if it helps you answer, but don't change any files or run commands that need approval. \
         {checkout}\
         {propose}\n\n{knowledge}{tickets}{now}{mentioned}{elsewhere}{notes}",
        name = e.name,
        role = e.role,
        company = c.name,
        bio = crate::voice::intro(c, &e)?,
        founder = crate::founder::name(),
        place = if dm_of(&slot.channel).is_some() { "sent you a direct message".to_string() } else { format!("mentioned you in #{}, the team chat", slot.channel) },
        history = history.join("\n"),
        knowledge = crate::work::company_knowledge(c)?,
        tickets = own_tickets(c, &e.id)?,
        now = doing.as_ref().map(|(_, text)| text.clone()).unwrap_or_default(),
        mentioned = mentioned_tickets(c, &said.join("\n"))?,
        elsewhere = match seen_elsewhere(c, &e, &slot.channel)? {
            lines if lines.is_empty() => String::new(),
            lines => format!("\n\nMeanwhile in your other channels, newest last:\n{}", lines.join("\n")),
        },
        notes = crate::work::own_notes(c, &e.id)?,
        checkout = checkout_note(c),
        propose = propose_note(&e, &team),
    );
    let repo = c.repo()?;
    let model = crate::work::talk_model(c, &e)?;
    let mut dirs = crate::work::other_repos(c)?;
    dirs.extend(doing.and_then(|(wt, _)| wt));
    let opts = runtime::Opts { cwd: &repo, model: model.as_deref(), mcp: None, resume: None, env: &[], read_only: true, on_spawn: None, commands: &[], dirs: &dirs };
    let (mut said, mut cost, mut tokens) = (String::new(), 0.0, 0);
    let run = runtime::run(&e.runtime, &opts, &prompt, |_, ev| {
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
        said = propose(c, &e, &team, id, &said)?;
    }
    let text = match (run, limited) {
        (_, true) => format!("(My usage limit is reached. I'm back at {}.)", crate::limits::until(&e.runtime).map(crate::limits::label).unwrap_or_default()),
        (Ok(()), _) if !said.trim().is_empty() => runtime::plain(said.trim()),
        (Ok(()), _) => "(no reply)".into(),
        (Err(err), _) => format!("(couldn't reply: {err:#})"),
    };
    let cost = crate::spend::record(c, &e.id, None, &e.runtime, model.as_deref(), cost, tokens)?;
    c.finish_message(id, &text, cost, tokens)
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
    let mine: Vec<String> = c.tickets()?.into_iter().filter(|t| t.assignee.as_deref() == Some(employee)).map(|t| format!("- {} {} ({})", c.key(t.id), t.title, t.status)).collect();
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

/// What someone has seen lately outside `skip`: #general, their team's channel, their DMs, and
/// anything that @mentions them. Newest last.
fn seen_elsewhere(c: &Company, e: &Employee, skip: &str) -> Result<Vec<String>> {
    const RECENT: usize = 15;
    let home = home(c, e)?;
    let tag = format!("@{}", e.id);
    let msgs: Vec<Message> = c
        .messages(None)?
        .into_iter()
        .filter(|m| !m.pending && m.channel != skip && m.channel != HELP && (m.channel == GENERAL || m.channel == home || m.channel == dm(&e.id) || m.text.to_lowercase().contains(&tag)))
        .collect();
    Ok(msgs
        .iter()
        .rev()
        .take(RECENT)
        .rev()
        .map(|m| format!("{} {}", if dm_of(&m.channel).is_some() { "(direct message)".to_string() } else { format!("#{}", m.channel) }, line(c, m)))
        .collect())
}

/// "Name: text", with long messages cut so one rant can't fill the prompt. The cut is marked,
/// or an employee reading its own reply thinks it failed to send and says it again.
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
