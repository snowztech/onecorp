//! Team chat: #general for everyone and one channel per department. Employees post with the
//! post_message tool and read recent chat before each ticket. The founder posts from the app or
//! `onecorp say`, and whoever they @mention replies in the channel.

use crate::runtime::{self, Event};
use crate::store::{Company, Employee, Message};
use crate::templates::suggest_name;
use anyhow::{Result, bail, ensure};

pub const GENERAL: &str = "general";

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
        if dm_of(&name).is_none() && !out.iter().any(|ch| ch.name == name) {
            out.push(Channel { title: name.clone(), name, department: None });
        }
    }
    Ok(out)
}

/// A channel from what someone typed: "#Engineering", "engineering" or just "eng". Empty means #general.
pub fn resolve(c: &Company, typed: &str) -> Result<String> {
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

/// A mentioned employee answers in the channel. Read-only in the repo and with no company tools,
/// so a chat reply can look at the code but never changes it or opens tickets.
pub fn reply(c: &Company, id: i64) -> Result<()> {
    let Some(slot) = c.message(id)? else { return Ok(()) };
    let Some(e) = c.employee(&slot.author)? else {
        return c.finish_message(id, "(they've left the company)", 0.0, 0);
    };
    let before: Vec<Message> = c.messages(Some(&slot.channel))?.into_iter().filter(|m| m.id < id && !m.pending).collect();
    let asked = before.iter().rev().find(|m| m.author == "founder").map(|m| m.text.clone()).unwrap_or_default();
    let history: Vec<String> = before.iter().rev().take(20).rev().map(|m| line(c, m)).collect();
    let prompt = format!(
        "You are {name}, {role} at {company}. {bio}\n\n\
         {founder} {place}. The conversation so far:\n\n{history}\n\n\
         They wrote: {asked}\n\n\
         Reply like a colleague would in chat, in your own manner: plain words, no headings. \
         Read the code in this repo if it helps you answer, but don't change any files. \
         If it needs real work, say which ticket you'd suggest and who should take it.\n\n{knowledge}{notes}",
        name = e.name,
        role = e.role,
        company = c.name,
        bio = crate::voice::intro(c, &e)?,
        founder = crate::founder::name(),
        place = if dm_of(&slot.channel).is_some() { "sent you a direct message".to_string() } else { format!("mentioned you in #{}, the team chat", slot.channel) },
        history = history.join("\n"),
        knowledge = crate::work::company_knowledge(c)?,
        notes = crate::work::own_notes(c, &e.id)?,
    );
    let repo = c.repo()?;
    let opts = runtime::Opts { cwd: &repo, model: e.model.as_deref(), mcp: None, resume: None, env: &[], read_only: true, on_spawn: None };
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
    let text = match (run, limited) {
        (_, true) => format!("(My usage limit is reached. I'm back at {}.)", crate::limits::until(&e.runtime).map(crate::limits::label).unwrap_or_default()),
        (Ok(()), _) if !said.trim().is_empty() => said.trim().to_string(),
        (Ok(()), _) => "(no reply)".into(),
        (Err(err), _) => format!("(couldn't reply: {err:#})"),
    };
    let cost = crate::spend::record(c, &e.id, None, &e.runtime, e.model.as_deref(), cost, tokens)?;
    c.finish_message(id, &text, cost, tokens)
}

/// Recent chat for a ticket prompt: #general and the employee's own channel, plus anything that
/// mentions them elsewhere. Empty when there's nothing.
pub fn catch_up(c: &Company, e: &Employee) -> Result<String> {
    const RECENT: usize = 15;
    let home = home(c, e)?;
    let tag = format!("@{}", e.id);
    let msgs: Vec<Message> = c
        .messages(None)?
        .into_iter()
        .filter(|m| !m.pending && (m.channel == GENERAL || m.channel == home || m.channel == dm(&e.id) || m.text.to_lowercase().contains(&tag)))
        .collect();
    if msgs.is_empty() {
        return Ok(String::new());
    }
    let lines: Vec<String> = msgs
        .iter()
        .rev()
        .take(RECENT)
        .rev()
        .map(|m| format!("{} {}", if dm_of(&m.channel).is_some() { "(direct message)".to_string() } else { format!("#{}", m.channel) }, line(c, m)))
        .collect();
    Ok(format!(
        "\n\nRecent team chat, newest last. Your posts go to #{home} unless you name another channel:\n{}",
        lines.join("\n")
    ))
}

/// "Name: text", with long messages cut so one rant can't fill the prompt.
fn line(c: &Company, m: &Message) -> String {
    let who = if m.author == "founder" {
        format!("{} (founder)", crate::founder::name())
    } else {
        c.employee(&m.author).ok().flatten().map(|e| e.name).unwrap_or_else(|| m.author.clone())
    };
    let text: String = m.text.chars().take(400).collect();
    format!("{who}: {}", text.replace('\n', " "))
}
