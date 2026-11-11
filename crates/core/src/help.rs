//! Ask OneCorp: the help built into the app. It answers from OneCorp's own docs, built into the
//! binary so they match the version you run, and from your company as it is now. When something
//! looks like a bug or an idea worth sharing, it drafts a GitHub issue for you to review and post.

use crate::chat::{HELP, ONECORP};
use crate::runtime::{self, Event};
use crate::store::Company;
use anyhow::Result;
use serde_json::json;

/// Read in full with every question.
const DOCS: &[(&str, &str)] = &[
    ("README.md", include_str!("../../../README.md")),
    ("docs/COLLABORATION.md", include_str!("../../../docs/COLLABORATION.md")),
];
/// Long ones, written next to the conversation and read when a question needs them.
const MORE: &[(&str, &str)] = &[("ARCHITECTURE.md", include_str!("../../../docs/ARCHITECTURE.md")), ("CHANGELOG.md", include_str!("../../../CHANGELOG.md"))];

/// The changelog's latest releases, enough for "is this new?" without the whole history.
fn recent_changes() -> String {
    let log = MORE[1].1;
    let starts: Vec<usize> = log.match_indices("\n## [").map(|(i, _)| i).collect();
    log[starts.first().copied().unwrap_or(0)..starts.get(3).copied().unwrap_or(log.len())].trim().to_string()
}

/// Writes the long docs where the help run can read them: its working folder, read-only.
fn write_more(c: &Company) -> Result<std::path::PathBuf> {
    let dir = c.dir.join("reading").join("onecorp");
    std::fs::create_dir_all(&dir)?;
    for (name, text) in MORE {
        if std::fs::read_to_string(dir.join(name)).ok().as_deref() != Some(*text) {
            std::fs::write(dir.join(name), text)?;
        }
    }
    Ok(dir)
}

const ISSUE: &str = "ISSUE:";

/// Answers the founder's latest message in the help channel. It runs on the team's agent with the
/// chat model, read-only, from the company's folder rather than any repo.
pub fn reply(c: &Company, id: i64) -> Result<()> {
    let Some(e) = c.employees()?.into_iter().next() else {
        return c.finish_message(id, "Hire someone first. I run on your team's agent, so I need one to answer.", 0.0, 0);
    };
    let before: Vec<_> = c.messages(Some(HELP))?.into_iter().filter(|m| m.id < id && !m.pending).collect();
    let recent: Vec<String> = before.iter().rev().take(12).rev().map(|m| crate::chat::line(c, m)).collect();
    let said: Vec<&str> = before.iter().rev().take(12).map(|m| m.text.as_str()).collect();
    let mut docs: Vec<String> = DOCS.iter().map(|(name, text)| format!("=== {name} ===\n{text}")).collect();
    docs.push(format!("=== Latest releases, from CHANGELOG.md ===\n{}", recent_changes()));
    let more = write_more(c)?;
    let prompt = format!(
        "You are Ask OneCorp, the help built into OneCorp {version}, the Mac app where {founder} runs a company of AI employees. \
         You're not an employee and don't work on their product. You explain how OneCorp works, what it can and can't do, \
         and whether something they saw is a bug.\n\n\
         Answer from OneCorp's docs below and from their company as it is now. Talk like a helpful colleague in chat: plain words, \
         short, no headings. Say plainly when something isn't possible yet, and point to the way that comes closest. \
         Don't guess about behavior the docs don't cover: say you're not sure.\n\n\
         When they found a bug, or want something OneCorp doesn't do, offer to report it. Once they want that, or it's clearly a bug, \
         end your reply with a line `{ISSUE} bug | title` or `{ISSUE} idea | title`, then the issue body in Markdown. \
         For a bug: **What happened**, **What you expected**, **Steps to get there**. For an idea: **What you're trying to do**, \
         **What gets in the way today**, **What would help**. The issue is public: leave out names, repo paths, code and anything \
         about their product. They review it on GitHub before it's posted.\n\n\
         The conversation so far:\n{history}\n\n\
         Their company now:\n{team}\n{tickets}{mentioned}\n\n\
         OneCorp's docs are below. For how the code works, which helps tell a bug from intended behavior, read \
         {more}/ARCHITECTURE.md. The full history is {more}/CHANGELOG.md.\n\n{docs}",
        version = env!("CARGO_PKG_VERSION"),
        founder = crate::founder::name(),
        history = recent.join("\n"),
        team = crate::work::who_does_what(c)?.join("\n"),
        tickets = open_tickets(c)?,
        mentioned = crate::chat::mentioned_tickets(c, &said.join("\n"))?,
        docs = docs.join("\n\n"),
        more = more.display(),
    );
    let model = crate::work::talk_model(c, &e)?;
    let opts = runtime::Opts { cwd: &c.dir, model: model.as_deref(), mcp: None, resume: None, env: &[], read_only: true, on_spawn: None, commands: &[], dirs: &[] };
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
    let cost = crate::spend::record(c, ONECORP, None, &e.runtime, model.as_deref(), cost, tokens)?;
    let text = match run {
        Err(err) => format!("(couldn't answer: {err:#})"),
        Ok(()) => {
            let (answer, issue) = split_issue(&said);
            if let Some((kind, title, body)) = issue {
                let body = format!("{body}\n\n---\nOneCorp {} on macOS, {}", env!("CARGO_PKG_VERSION"), agent_name(&e.runtime));
                c.set_issue(id, &json!({"kind": kind, "title": title, "body": body}).to_string())?;
            }
            match answer.trim() {
                "" if said.trim().is_empty() => "(no answer)".into(),
                "" => "Here's a draft for GitHub.".into(),
                a => runtime::plain(a),
            }
        }
    };
    c.finish_message(id, &text, cost, tokens)
}

/// Splits a reply at its `ISSUE: kind | title` line: the answer before it, and the issue after.
fn split_issue(said: &str) -> (String, Option<(String, String, String)>) {
    let lines: Vec<&str> = said.lines().collect();
    let Some(at) = lines.iter().position(|l| l.trim().trim_matches('`').starts_with(ISSUE)) else { return (said.to_string(), None) };
    let head = lines[at].trim().trim_matches('`').trim_start_matches(ISSUE);
    let (kind, title) = head.split_once('|').map(|(k, t)| (k.trim().to_lowercase(), t.trim())).unwrap_or(("bug".into(), head.trim()));
    let kind = if kind == "idea" { "idea" } else { "bug" };
    let body = lines[at + 1..].join("\n").trim().to_string();
    let answer = lines[..at].join("\n");
    if title.is_empty() {
        return (answer, None);
    }
    (answer, Some((kind.into(), title.into(), body)))
}

/// Tickets that aren't merged, with what went wrong for the ones that stopped, so "why did X
/// stop?" has an answer.
fn open_tickets(c: &Company) -> Result<String> {
    let mut out = vec![];
    for t in c.tickets()?.into_iter().rev().filter(|t| t.status != "done").take(30) {
        let mut line = format!("- {} {} ({}, {})", c.key(t.id), t.title, t.status, t.assignee.as_deref().unwrap_or("nobody"));
        if ["failed", "blocked"].contains(&t.status.as_str())
            && let Some((_, _, why)) = c.events(t.id)?.into_iter().rev().find(|(_, kind, _)| kind == "error" || kind == "stop" || kind == "block")
        {
            line.push_str(&format!(": {}", runtime::clip(&why, 300)));
        }
        out.push(line);
    }
    Ok(if out.is_empty() { "No open tickets.".into() } else { format!("Open tickets, newest first:\n{}", out.join("\n")) })
}

fn agent_name(runtime: &str) -> &str {
    match runtime {
        "codex" => "Codex",
        "claude" => "Claude Code",
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_issue_line_splits_the_reply() {
        let (answer, issue) = split_issue("Looks like a bug.\n`ISSUE: bug | Chat loses my draft`\n**What happened**\nIt vanished.");
        assert_eq!(answer, "Looks like a bug.");
        assert_eq!(issue, Some(("bug".into(), "Chat loses my draft".into(), "**What happened**\nIt vanished.".into())));
        assert_eq!(split_issue("Just an answer.").1, None);
        assert_eq!(split_issue("ISSUE: idea | Dark icons\nbody").1.unwrap().0, "idea");
    }
}
