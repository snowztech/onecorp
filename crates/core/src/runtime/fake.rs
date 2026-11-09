//! A stand-in agent: no AI, no network, no cost. It adds one line to WORK.md in its worktree and
//! reports events like a real runtime, so the whole ticket loop can be tested end to end and the
//! app can be tried for free (`onecorp hire Demo --runtime fake`).
//!
//! A test steers it through the prompt: a ticket containing `[fail]` fails, and a conflict
//! prompt makes it resolve the markers by keeping both sides.

use super::{Event, Opts};
use anyhow::Result;
use std::fs;

pub fn run(o: &Opts, prompt: &str, mut on: impl FnMut(&str, Event) -> Result<()>) -> Result<()> {
    let session = o.resume.map(str::to_string).unwrap_or_else(|| format!("fake-{}", std::process::id()));
    on("", Event::Init { session, model: "no AI".into() })?;

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
