//! Codex, run with `codex exec --json`.
//!
//! Shapes, from codex-cli 0.166: `thread.started` carries the session id, each `item.completed`
//! is a message, command, file change or MCP call, and `turn.completed` carries token usage.

use super::{Event, Opts, first_line, stream};
use anyhow::Result;
use serde_json::Value;
use std::process::Command;

pub fn run(o: &Opts, prompt: &str, on: impl FnMut(&str, Event) -> Result<()>) -> Result<()> {
    // -c values are parsed as TOML. JSON strings and string arrays are valid TOML too.
    let q = |s: &str| serde_json::to_string(s).unwrap();
    let env = o.mcp.env.iter().map(|(k, v)| format!("{k}={}", q(v))).collect::<Vec<_>>().join(",");

    let mut cmd = Command::new("codex");
    cmd.args(["exec", "--json", "--skip-git-repo-check"])
        // The user's config (and their own MCP servers) stays out. Login still works.
        .arg("--ignore-user-config")
        // Writes stay inside the worktree. .git is read-only there, which is why OneCorp commits for the agent.
        .args(["--sandbox", if o.read_only { "read-only" } else { "workspace-write" }, "--cd"])
        .arg(o.cwd)
        .args(["-c", &format!("mcp_servers.onecorp.command={}", q(&o.mcp.command))])
        .args(["-c", &format!("mcp_servers.onecorp.args={}", serde_json::to_string(&o.mcp.args)?)])
        .args(["-c", &format!("mcp_servers.onecorp.env={{{env}}}")])
        // Without this, every company tool call is refused in non-interactive mode.
        .args(["-c", "mcp_servers.onecorp.default_tools_approval_mode=\"approve\""])
        .current_dir(o.cwd)
        .envs(o.env.iter().map(|(k, v)| (k, v)));
    if let Some(m) = o.model {
        cmd.args(["--model", m]);
    }
    if let Some(s) = o.resume {
        cmd.args(["resume", s]);
    }
    // No prompt argument: codex reads it from stdin.
    stream(cmd, prompt, parse, o.on_spawn, on)
}

pub fn parse(line: &str) -> Vec<Event> {
    let Ok(v) = serde_json::from_str::<Value>(line) else { return vec![] };
    let s = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    match v["type"].as_str() {
        Some("thread.started") => vec![Event::Init { session: s(&v, "thread_id"), model: String::new() }],
        Some("item.completed") => item(&v["item"]),
        Some("turn.completed") => {
            let u = &v["usage"];
            let tokens = u["input_tokens"].as_u64().unwrap_or(0) + u["output_tokens"].as_u64().unwrap_or(0);
            // Codex doesn't send a final summary here. The caller uses the last message instead.
            vec![Event::Done { ok: true, cost: 0.0, tokens, text: String::new() }]
        }
        Some("turn.failed") => {
            vec![Event::Done { ok: false, cost: 0.0, tokens: 0, text: first_line(&s(&v["error"], "message")) }]
        }
        Some("error") => vec![Event::ToolResult { error: true, text: first_line(&s(&v, "message")) }],
        _ => vec![],
    }
}

fn item(i: &Value) -> Vec<Event> {
    let s = |k: &str| i.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    match i["type"].as_str() {
        Some("agent_message") => vec![Event::Say(s("text"))],
        Some("command_execution") => {
            let mut out = vec![Event::Tool { name: "Bash".into(), detail: unwrap_shell(&s("command")) }];
            // Just the code: the first output line is often normal output, like `grep` finding nothing.
            if let Some(code) = i["exit_code"].as_i64().filter(|c| *c != 0) {
                out.push(Event::ToolResult { error: true, text: format!("exit {code}") });
            }
            out
        }
        Some("file_change") => {
            let paths: Vec<_> = i["changes"].as_array().into_iter().flatten().filter_map(|c| c["path"].as_str()).collect();
            vec![Event::Tool { name: "Edit".into(), detail: paths.join(", ") }]
        }
        Some("mcp_tool_call") => {
            let tool = s("tool");
            let a = &i["arguments"];
            let detail = ["question", "text", "note", "title"].iter().find_map(|k| a[k].as_str()).unwrap_or("").to_string();
            // Named like Claude's MCP tools so logs read the same whatever the runtime.
            let mut out = vec![Event::Tool { name: format!("mcp__{}__{tool}", s("server")), detail }];
            if let Some(msg) = i["error"]["message"].as_str() {
                out.push(Event::ToolResult { error: true, text: first_line(msg) });
            }
            out
        }
        _ => vec![],
    }
}

/// Codex wraps commands as `/bin/zsh -lc "..."`. Show only the inner command.
fn unwrap_shell(cmd: &str) -> String {
    let inner = cmd.split_once(" -lc ").map(|(_, rest)| rest).unwrap_or(cmd).trim();
    let inner = inner.strip_prefix(['"', '\'']).and_then(|r| r.strip_suffix(['"', '\''])).unwrap_or(inner);
    inner.lines().next().unwrap_or("").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_codex_exec_json() {
        let start = r#"{"type":"thread.started","thread_id":"t-1"}"#;
        assert_eq!(parse(start), vec![Event::Init { session: "t-1".into(), model: String::new() }]);

        let cmd = r#"{"type":"item.completed","item":{"type":"command_execution","command":"/bin/zsh -lc 'cat main.rs'","aggregated_output":"","exit_code":0}}"#;
        assert_eq!(parse(cmd), vec![Event::Tool { name: "Bash".into(), detail: "cat main.rs".into() }]);

        let failed = r#"{"type":"item.completed","item":{"type":"command_execution","command":"/bin/zsh -lc \"cargo test\"","aggregated_output":"error: nope\n","exit_code":101}}"#;
        assert_eq!(
            parse(failed),
            vec![Event::Tool { name: "Bash".into(), detail: "cargo test".into() }, Event::ToolResult { error: true, text: "exit 101".into() }]
        );

        let mcp = r#"{"type":"item.completed","item":{"type":"mcp_tool_call","server":"onecorp","tool":"post_message","arguments":{"text":"hi"},"error":null}}"#;
        assert_eq!(parse(mcp), vec![Event::Tool { name: "mcp__onecorp__post_message".into(), detail: "hi".into() }]);

        let edit = r#"{"type":"item.completed","item":{"type":"file_change","changes":[{"path":"a.rs","kind":"update"},{"path":"b.rs","kind":"add"}]}}"#;
        assert_eq!(parse(edit), vec![Event::Tool { name: "Edit".into(), detail: "a.rs, b.rs".into() }]);

        let done = r#"{"type":"turn.completed","usage":{"input_tokens":1000,"cached_input_tokens":900,"output_tokens":50}}"#;
        assert_eq!(parse(done), vec![Event::Done { ok: true, cost: 0.0, tokens: 1050, text: String::new() }]);

        assert!(parse(r#"{"type":"item.started","item":{"type":"command_execution"}}"#).is_empty());
    }
}
