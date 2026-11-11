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
    /// Other repos a read-only run may look at, like an api next to the app. Codex's read-only
    /// sandbox reads anywhere already, so only Claude Code needs them.
    pub dirs: &'a [std::path::PathBuf],
}

pub fn run(runtime: &str, o: &Opts, prompt: &str, on: impl FnMut(&str, Event) -> Result<()>) -> Result<()> {
    match runtime {
        "claude" => claude::run(o, prompt, on),
        "codex" => codex::run(o, prompt, on),
        "fake" => fake::run(o, prompt, on),
        r => bail!("{r} isn't supported yet. Available: {}", RUNTIMES.join(", ")),
    }
}

/// The models an agent offers, newest first: (id passed to --model, name, what it's good at).
/// Claude Code takes these ids as they are. Codex keeps its own current list in its cache, so it's
/// read from there and never goes out of date. An empty list means "use the agent's default".
pub fn models(runtime: &str) -> Vec<(String, String, String)> {
    let own = |rows: &[(&str, &str, &str)]| rows.iter().map(|(a, b, c)| (a.to_string(), b.to_string(), c.to_string())).collect();
    match runtime {
        "claude" => own(&[
            ("claude-opus-5-5", "Opus 5.5", "Most capable, for hard problems"),
            ("claude-sonnet-5-5", "Sonnet 5.5", "Fast and capable, for most work"),
            ("claude-haiku-5-5", "Haiku 5.5", "Fastest and cheapest, for simple tasks"),
            ("claude-fable-5-1", "Fable 5.1", "The newest family member"),
        ]),
        "codex" => codex_models(),
        _ => vec![],
    }
}

/// An agent's model for a tier: "strongest" for leads and hard calls, "balanced" for most work,
/// "cheapest" for short, simple jobs. Returns (id, name). Codex's own list says which fits each.
pub fn tier_model(runtime: &str, tier: &str) -> Option<(String, String)> {
    match runtime {
        "claude" => {
            let (id, name) = match tier {
                "strongest" => ("claude-opus-5-5", "Opus 5.5"),
                "cheapest" => ("claude-haiku-5-5", "Haiku 5.5"),
                _ => ("claude-sonnet-5-5", "Sonnet 5.5"),
            };
            Some((id.into(), name.into()))
        }
        "codex" => codex_models()
            .into_iter()
            .find(|(id, _, about)| {
                let about = about.to_lowercase();
                match tier {
                    "strongest" => about.contains("frontier") || about.contains("most demanding"),
                    "cheapest" => about.contains("fast and affordable") || id.contains("luna"),
                    _ => about.contains("workhorse"),
                }
            })
            .map(|(id, name, _)| (id, name)),
        _ => None,
    }
}

/// The model for talk (chat replies and a lead's quick decisions): "cheapest" or "balanced".
/// Talk is short, so a smaller model answers it well for a fraction of the cost.
pub fn talk_model(runtime: &str, tier: &str) -> Option<(String, String)> {
    tier_model(runtime, if tier == "cheapest" { "cheapest" } else { "balanced" })
}

/// What an agent runs when no model is set: your own setting for it, like `"model": "opus"` in
/// Claude Code's settings. A name to show, so "Default" never hides which model does the work.
pub fn default_model(runtime: &str) -> Option<String> {
    let home = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let set = match runtime {
        "claude" => std::env::var("ANTHROPIC_MODEL").ok().filter(|m| !m.is_empty()).or_else(|| {
            let text = std::fs::read_to_string(home.join(".claude/settings.json")).ok()?;
            serde_json::from_str::<serde_json::Value>(&text).ok()?["model"].as_str().map(String::from)
        }),
        "codex" => {
            let dir = std::env::var("CODEX_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| home.join(".codex"));
            let text = std::fs::read_to_string(dir.join("config.toml")).ok()?;
            text.lines().find_map(|l| {
                let (k, v) = l.split_once('=')?;
                (k.trim() == "model").then(|| v.trim().trim_matches('"').to_string())
            })
        }
        _ => None,
    }?;
    // A short name like "opus" or an exact id: show the model's own name when we know it.
    let set = set.trim().to_string();
    Some(models(runtime).into_iter().find(|(id, _, _)| *id == set || id.contains(&format!("-{set}-")) || id.ends_with(&format!("-{set}"))).map(|(_, name, _)| name).unwrap_or(set))
}

fn codex_models() -> Vec<(String, String, String)> {
    let home = std::env::var("CODEX_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".codex"));
    let Ok(text) = std::fs::read_to_string(home.join("models_cache.json")) else { return vec![] };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { return vec![] };
    v["models"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|m| m["visibility"] == "list")
        .filter_map(|m| Some((m["slug"].as_str()?.to_string(), m["display_name"].as_str()?.to_string(), m["description"].as_str().unwrap_or("").to_string())))
        .take(6)
        .collect()
}

/// One sentence for the prompt about what the agent can run, since each runtime's sandbox differs.
pub fn abilities(runtime: &str, commands: &[String]) -> String {
    match runtime {
        "codex" => "You can run commands, like builds and tests, in a sandbox without network access. Run the tests before you finish.".into(),
        "fake" => "You are a stand-in with no AI.".into(),
        // Said plainly, so the agent doesn't report a read-only worktree when only commands are limited.
        _ if commands.is_empty() => "You can read and edit files, and run read-only git commands. No build or test commands are set up for this repo, so don't try to run any, and don't create or edit .claude/settings.json: OneCorp decides what you can run. If you need a build or test command, ask for it with ask_to_run.".into(),
        _ => format!(
            "You can run read-only git commands and these: {}. Run the build and tests before you finish, and fix what breaks. \
             OneCorp decides what you can run, from the list the founder keeps in Settings, so don't create or edit .claude/settings.json. \
             If you need another command, ask for it with ask_to_run.",
            commands.join(", ")
        ),
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
