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
use std::sync::OnceLock;
use std::time::Duration;

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
    /// The company tools. None for a chat reply, which only talks.
    pub mcp: Option<&'a Mcp>,
    /// Continue this session instead of starting a new one. Must run in the same cwd.
    pub resume: Option<&'a str>,
    /// Extra env for the agent process, like the employee's git identity.
    pub env: &'a [(String, String)],
    /// Look, don't touch: no file edits. Used when a lead answers a question.
    pub read_only: bool,
    /// Called with the agent's process id once it starts, so the run can be stopped.
    pub on_spawn: Option<&'a dyn Fn(u32)>,
    /// Build and test commands Claude Code may run, like "cargo test". Codex has its own sandbox.
    pub commands: &'a [String],
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
pub fn abilities(runtime: &str, commands: &[String]) -> String {
    match runtime {
        "codex" => "You can run commands, like builds and tests, in a sandbox without network access. Run the tests before you finish.".into(),
        "fake" => "You are a stand-in with no AI.".into(),
        _ if commands.is_empty() => "You can only run read-only git commands, so don't try to build or test.".into(),
        _ => format!("You can run read-only git commands and these: {}. Run the build and tests before you finish, and fix what breaks.", commands.join(", ")),
    }
}

/// The PATH agent CLIs run with. Apps opened from the Dock or Finder get a bare PATH, without
/// ~/.local/bin or Homebrew, so `claude` and `codex` wouldn't be found. This adds the login
/// shell's PATH and the usual install folders. Worked out once.
pub fn path() -> &'static str {
    static PATH: OnceLock<String> = OnceLock::new();
    PATH.get_or_init(|| {
        let home = std::env::var("HOME").unwrap_or_default();
        let mut dirs: Vec<String> = std::env::var("PATH").unwrap_or_default().split(':').filter(|d| !d.is_empty()).map(String::from).collect();
        let usual = [".local/bin", ".cargo/bin", ".bun/bin", ".npm-global/bin", ".volta/bin"].map(|d| format!("{home}/{d}"));
        let extra = login_shell_path().unwrap_or_default();
        for d in extra.split(':').map(String::from).chain(usual).chain(["/opt/homebrew/bin".into(), "/usr/local/bin".into()]) {
            if !d.is_empty() && !dirs.contains(&d) && Path::new(&d).is_dir() {
                dirs.push(d);
            }
        }
        dirs.join(":")
    })
}

/// Asks the user's shell for its PATH, giving up after a few seconds. Markers around the value
/// skip anything the shell's startup files print.
fn login_shell_path() -> Option<String> {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/zsh".into());
    let child = Command::new(shell)
        .args(["-ilc", "printf '<<PATH>>%s<<PATH>>' \"$PATH\""])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let (tx, rx) = std::sync::mpsc::channel();
    let id = child.id();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });
    let Ok(Ok(out)) = rx.recv_timeout(Duration::from_secs(3)) else {
        let _ = Command::new("kill").arg(id.to_string()).status();
        return None;
    };
    let text = String::from_utf8_lossy(&out.stdout);
    text.split("<<PATH>>").nth(1).map(str::to_string)
}

/// Starts `cmd`, writes the prompt to its stdin, and feeds each parsed line to `on`.
/// Fails if the process ends without a Done event.
pub fn stream(
    mut cmd: Command,
    prompt: &str,
    parse: fn(&str) -> Vec<Event>,
    on_spawn: Option<&dyn Fn(u32)>,
    mut on: impl FnMut(&str, Event) -> Result<()>,
) -> Result<()> {
    let program = cmd.get_program().to_string_lossy().into_owned();
    // Setting PATH on the command also makes it the PATH used to find the program.
    let mut child = cmd
        .env("PATH", path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("could not start {program}. Is it installed and on PATH?"))?;
    if let Some(f) = on_spawn {
        f(child.id());
    }

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
