//! Runtimes are the agent CLIs that do an employee's work.
//!
//! What every runtime has in common: the events we log, how to start one, and the loop that
//! streams its JSON output. Each runtime (claude.rs, codex.rs) builds its command and parses its own lines.

mod claude;
mod codex;
mod fake;

use anyhow::{Context, Result, bail};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// What you can hire on. `fake` is a stand-in with no AI, for trying things out and for tests.
pub const RUNTIMES: &[&str] = &["claude", "codex", "fake"];

#[derive(Debug, PartialEq)]
pub enum Event {
    Init { session: String, model: String },
    Say(String),
    Tool { name: String, detail: String },
    ToolResult { error: bool, text: String },
    /// `cost` is 0 when the runtime doesn't report dollars (Codex on a subscription).
    Done { ok: bool, cost: f64, tokens: u64, text: String },
}

/// The company MCP server, described once and written in each runtime's config format.
pub struct Mcp {
    pub command: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
}

pub struct Opts<'a> {
    pub cwd: &'a Path,
    pub model: Option<&'a str>,
    pub mcp: &'a Mcp,
    /// Continue this session instead of starting a new one. Must run in the same cwd.
    pub resume: Option<&'a str>,
    /// Extra env for the agent process, like the employee's git identity.
    pub env: &'a [(String, String)],
}

pub fn run(runtime: &str, o: &Opts, prompt: &str, on: impl FnMut(&str, Event) -> Result<()>) -> Result<()> {
    match runtime {
        "claude" => claude::run(o, prompt, on),
        "codex" => codex::run(o, prompt, on),
        "fake" => fake::run(o, prompt, on),
        r => bail!("{r} isn't supported yet. Available: {}", RUNTIMES.join(", ")),
    }
}

/// One sentence for the prompt about what the agent can run, since each runtime's sandbox differs.
pub fn abilities(runtime: &str) -> &'static str {
    match runtime {
        "codex" => "You can run commands, like builds and tests, in a sandbox without network access.",
        "fake" => "You are a stand-in with no AI.",
        _ => "You can only run read-only git commands, so don't try to build or test.",
    }
}

/// Starts `cmd`, writes the prompt to its stdin, and feeds each parsed line to `on`.
/// Fails if the process ends without a Done event.
pub fn stream(mut cmd: Command, prompt: &str, parse: fn(&str) -> Vec<Event>, mut on: impl FnMut(&str, Event) -> Result<()>) -> Result<()> {
    let program = cmd.get_program().to_string_lossy().into_owned();
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("could not start {program}. Is it installed and on PATH?"))?;

    // Dropping the handle closes stdin, which tells the agent the prompt is complete.
    child.stdin.take().unwrap().write_all(prompt.as_bytes())?;

    // Drain stderr on a thread so a chatty stderr can't block stdout.
    let mut stderr = child.stderr.take().unwrap();
    let err_thread = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = stderr.read_to_string(&mut s);
        s
    });

    let mut finished = false;
    for line in BufReader::new(child.stdout.take().unwrap()).lines() {
        let line = line?;
        for ev in parse(&line) {
            finished |= matches!(ev, Event::Done { .. });
            on(&line, ev)?;
        }
    }

    let status = child.wait()?;
    let stderr = err_thread.join().unwrap_or_default();
    if !finished {
        bail!("{program} exited ({status}) without a result: {}", stderr.trim());
    }
    Ok(())
}

pub fn first_line(s: &str) -> String {
    s.lines().find(|l| !l.trim().is_empty()).unwrap_or("").chars().take(120).collect()
}
