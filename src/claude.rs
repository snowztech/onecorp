//! Runs Claude Code headless and turns its stream-json output into events.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Debug, PartialEq)]
pub enum Event {
    Init { session: String, model: String },
    Say(String),
    Tool { name: String, detail: String },
    ToolResult { error: bool, text: String },
    Done { ok: bool, cost: f64, turns: u64, text: String },
}

// Edits are auto-accepted inside the worktree. Bash is denied except for git basics,
// so the agent can commit but can't run arbitrary commands yet.
const ALLOWED: &str = "Bash(git add:*),Bash(git commit:*),Bash(git status:*),Bash(git diff:*),Bash(git log:*)";

pub fn run(cwd: &Path, model: Option<&str>, prompt: &str, mut on: impl FnMut(&str, Event) -> Result<()>) -> Result<()> {
    let mut cmd = Command::new("claude");
    cmd.args(["-p", "--output-format", "stream-json", "--verbose", "--permission-mode", "acceptEdits"])
        .args(["--allowedTools", ALLOWED]);
    if let Some(m) = model {
        cmd.args(["--model", m]);
    }
    let mut child = cmd
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("could not start claude. Is Claude Code installed and on PATH?")?;

    // The prompt goes on stdin: --allowedTools takes a list and would swallow a trailing prompt argument.
    // Dropping the handle closes stdin so claude starts.
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
        bail!("claude exited ({status}) without a result: {}", stderr.trim());
    }
    Ok(())
}

pub fn parse(line: &str) -> Vec<Event> {
    let Ok(v) = serde_json::from_str::<Value>(line) else { return vec![] };
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let blocks = || v.pointer("/message/content").and_then(Value::as_array).cloned().unwrap_or_default();

    match v["type"].as_str() {
        Some("system") if v["subtype"] == "init" => vec![Event::Init { session: s(&v, "session_id"), model: s(&v, "model") }],
        Some("assistant") => blocks()
            .iter()
            .filter_map(|b| match b["type"].as_str() {
                Some("text") => Some(Event::Say(s(b, "text"))),
                Some("tool_use") => Some(Event::Tool { name: s(b, "name"), detail: tool_detail(&s(b, "name"), &b["input"]) }),
                _ => None,
            })
            .collect(),
        Some("user") => blocks()
            .iter()
            .filter(|b| b["type"] == "tool_result")
            .map(|b| Event::ToolResult { error: b["is_error"] == true, text: result_text(&b["content"]) })
            .collect(),
        Some("result") => vec![Event::Done {
            ok: v["subtype"] == "success" && v["is_error"] != true,
            cost: v["total_cost_usd"].as_f64().unwrap_or(0.0),
            turns: v["num_turns"].as_u64().unwrap_or(0),
            text: s(&v, "result"),
        }],
        _ => vec![],
    }
}

fn tool_detail(name: &str, input: &Value) -> String {
    let key = match name {
        "Bash" => "command",
        "Read" | "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => "file_path",
        "Grep" | "Glob" => "pattern",
        "WebFetch" => "url",
        _ => return String::new(),
    };
    // Not truncated: the caller strips the worktree path first, and absolute paths are long.
    let s = input[key].as_str().unwrap_or("");
    s.lines().find(|l| !l.trim().is_empty()).unwrap_or("").to_string()
}

fn result_text(content: &Value) -> String {
    match content {
        Value::String(s) => first_line(s),
        Value::Array(items) => first_line(items.iter().find_map(|i| i["text"].as_str()).unwrap_or("")),
        _ => String::new(),
    }
}

pub fn first_line(s: &str) -> String {
    s.lines().find(|l| !l.trim().is_empty()).unwrap_or("").chars().take(120).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_events_we_care_about() {
        let init = r#"{"type":"system","subtype":"init","session_id":"abc","model":"claude-sonnet"}"#;
        assert_eq!(parse(init), vec![Event::Init { session: "abc".into(), model: "claude-sonnet".into() }]);

        let tool = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Looking.\nmore"},{"type":"tool_use","name":"Edit","input":{"file_path":"src/main.rs"}}]}}"#;
        assert_eq!(
            parse(tool),
            vec![Event::Say("Looking.\nmore".into()), Event::Tool { name: "Edit".into(), detail: "src/main.rs".into() }]
        );

        let res = r#"{"type":"user","message":{"content":[{"type":"tool_result","is_error":true,"content":"permission denied\nx"}]}}"#;
        assert_eq!(parse(res), vec![Event::ToolResult { error: true, text: "permission denied".into() }]);

        let done = r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.42,"num_turns":7,"result":"Done"}"#;
        assert_eq!(parse(done), vec![Event::Done { ok: true, cost: 0.42, turns: 7, text: "Done".into() }]);

        assert!(parse("not json").is_empty());
    }
}
