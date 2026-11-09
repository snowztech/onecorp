//! Claude Code, run headless with stream-json output.

use super::{Event, Opts, first_line, stream};
use anyhow::Result;
use serde_json::{Value, json};
use std::process::Command;

// Edits are auto-accepted inside the worktree. Bash is limited to read-only git, and OneCorp
// commits the work when the agent finishes. The company tools are always allowed.
const ALLOWED: &str = "Bash(git status:*),Bash(git diff:*),Bash(git log:*),mcp__onecorp";

pub fn run(o: &Opts, prompt: &str, on: impl FnMut(&str, Event) -> Result<()>) -> Result<()> {
    let env: serde_json::Map<String, Value> = o.mcp.env.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
    let mcp = json!({"mcpServers": {"onecorp": {"command": o.mcp.command, "args": o.mcp.args, "env": env}}});

    let mut cmd = Command::new("claude");
    // Without acceptEdits, edits need a permission nobody can give in headless mode, so they're refused.
    let mode = if o.read_only { "default" } else { "acceptEdits" };
    cmd.args(["-p", "--output-format", "stream-json", "--verbose", "--permission-mode", mode])
        // Only the company server loads. The user's own MCP servers stay out.
        .args(["--mcp-config", &mcp.to_string(), "--strict-mcp-config"])
        .args(["--allowedTools", ALLOWED])
        .current_dir(o.cwd)
        .envs(o.env.iter().map(|(k, v)| (k, v)));
    if let Some(m) = o.model {
        cmd.args(["--model", m]);
    }
    if let Some(s) = o.resume {
        cmd.args(["--resume", s]);
    }
    // The prompt goes on stdin: --allowedTools takes a list and would swallow a trailing argument.
    stream(cmd, prompt, parse, o.on_spawn, on)
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
        Some("result") => {
            let u = &v["usage"];
            let tokens = ["input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens", "output_tokens"]
                .iter()
                .map(|k| u[k].as_u64().unwrap_or(0))
                .sum();
            vec![Event::Done {
                ok: v["subtype"] == "success" && v["is_error"] != true,
                cost: v["total_cost_usd"].as_f64().unwrap_or(0.0),
                tokens,
                text: s(&v, "result"),
            }]
        }
        _ => vec![],
    }
}

fn tool_detail(name: &str, input: &Value) -> String {
    let key = match name {
        "Bash" => "command",
        "Read" | "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => "file_path",
        "Grep" | "Glob" => "pattern",
        "WebFetch" => "url",
        "mcp__onecorp__ask_human" => "question",
        "mcp__onecorp__post_message" | "mcp__onecorp__propose_handbook" | "mcp__onecorp__remember" => "text",
        "mcp__onecorp__update_ticket" => "note",
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

        let done = r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.42,"num_turns":7,"result":"Done","usage":{"input_tokens":10,"cache_read_input_tokens":90,"output_tokens":5}}"#;
        assert_eq!(parse(done), vec![Event::Done { ok: true, cost: 0.42, tokens: 105, text: "Done".into() }]);

        assert!(parse("not json").is_empty());
    }
}
