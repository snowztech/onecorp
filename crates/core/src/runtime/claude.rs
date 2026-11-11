//! Claude Code, run headless with stream-json output.

use super::{Event, Opts, clip, stream};
use anyhow::Result;
use serde_json::{Value, json};
use std::process::Command;

// Edits are auto-accepted inside the worktree. Bash is limited to read-only git plus the
// company's build and test commands, and OneCorp commits the work when the agent finishes.
// The company tools are always allowed.
// Read-only text tools too: agents pipe test output through tail or head, and one blocked
// part of a pipe blocks the whole command.
const ALLOWED: &str = "Bash(git status:*),Bash(git diff:*),Bash(git log:*),Bash(ls:*),Bash(cat:*),Bash(head:*),Bash(tail:*),Bash(wc:*),Bash(grep:*),Bash(echo:*),Bash(which:*),Bash(pwd),mcp__onecorp";

pub fn run(o: &Opts, prompt: &str, on: impl FnMut(&str, Event) -> Result<()>) -> Result<()> {
    let mut cmd = Command::new("claude");
    // Without acceptEdits, edits need a permission nobody can give in headless mode, so they're refused.
    let mode = if o.read_only { "default" } else { "acceptEdits" };
    cmd.args(["-p", "--output-format", "stream-json", "--verbose", "--permission-mode", mode])
        // Only the company server loads, if any. The user's own MCP servers stay out.
        .arg("--strict-mcp-config")
        .args(["--allowedTools", &allowed(o)])
        .current_dir(o.cwd)
        .envs(o.env.iter().map(|(k, v)| (k, v)));
    if let Some(m) = o.mcp {
        let env: serde_json::Map<String, Value> = m.env.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        let mcp = json!({"mcpServers": {"onecorp": {"command": m.command, "args": m.args, "env": env}}});
        cmd.args(["--mcp-config", &mcp.to_string()]);
    }
    if let Some(m) = o.model {
        cmd.args(["--model", m]);
    }
    if o.run_any && !o.read_only {
        let never: Vec<String> = super::NEVER_ANY.iter().map(|c| format!("Bash({c}:*)")).collect();
        cmd.args(["--disallowedTools", &never.join(",")]);
    }
    // Only for read-only runs: with edits accepted, an added folder could be changed outside a worktree.
    if o.read_only {
        for d in o.dirs {
            cmd.arg("--add-dir").arg(d);
        }
    }
    if let Some(s) = o.resume {
        cmd.args(["--resume", s]);
    }
    // The prompt goes on stdin: --allowedTools takes a list and would swallow a trailing argument.
    stream(cmd, prompt, parse, o.on_spawn, on)
}

fn allowed(o: &Opts) -> String {
    let mut out = ALLOWED.to_string();
    if o.run_any && !o.read_only {
        out.push_str(",Bash");
    }
    for c in o.commands {
        out.push_str(&format!(",Bash({c}:*)"));
        // A script on the list runs as `tools/x.sh` or `./tools/x.sh`, whichever the agent types.
        match c.strip_prefix("./") {
            Some(bare) => out.push_str(&format!(",Bash({bare}:*)")),
            None if c.split_whitespace().next().is_some_and(|w| w.contains('/') && !w.starts_with('/')) => out.push_str(&format!(",Bash(./{c}:*)")),
            None => {}
        }
    }
    // Other folders a ticket run reads, like the wiki's reading copy: reading only. `//` is an absolute path.
    if !o.read_only {
        for d in o.dirs {
            out.push_str(&format!(",Read(/{}/**)", d.display()));
        }
    }
    out
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
            let n = |k: &str| u[k].as_u64().unwrap_or(0);
            let (fresh, cached, output) = (n("input_tokens") + n("cache_creation_input_tokens"), n("cache_read_input_tokens"), n("output_tokens"));
            let tokens = fresh + cached + output;
            let usage = (tokens > 0).then_some(Event::Usage { fresh, cached, output });
            usage.into_iter().chain([Event::Done {
                ok: v["subtype"] == "success" && v["is_error"] != true,
                cost: v["total_cost_usd"].as_f64().unwrap_or(0.0),
                tokens,
                text: s(&v, "result"),
            }])
            .collect()
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
        "mcp__onecorp__post_message" | "mcp__onecorp__propose_wiki" | "mcp__onecorp__remember" => "text",
        "mcp__onecorp__create_ticket" => "title",
        "mcp__onecorp__update_ticket" => "note",
        _ => return String::new(),
    };
    // Whole: the caller strips the worktree path first and keeps what fits the log.
    input[key].as_str().unwrap_or("").trim().to_string()
}

fn result_text(content: &Value) -> String {
    match content {
        Value::String(s) => clip(s, 600),
        Value::Array(items) => clip(items.iter().find_map(|i| i["text"].as_str()).unwrap_or(""), 600),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_tools_follow_the_list_or_run_anything() {
        let (cmds, dirs) = (vec!["tools/check.sh".to_string(), "./gradlew assembleDebug".to_string()], vec![std::path::PathBuf::from("/c/reading/wiki")]);
        let opts = |run_any, read_only| Opts { cwd: std::path::Path::new("/wt"), model: None, mcp: None, resume: None, env: &[], read_only, on_spawn: None, commands: &cmds, dirs: &dirs, run_any };
        let listed = allowed(&opts(false, false));
        for rule in ["Bash(tools/check.sh:*)", "Bash(./tools/check.sh:*)", "Bash(./gradlew assembleDebug:*)", "Bash(gradlew assembleDebug:*)", "Read(//c/reading/wiki/**)"] {
            assert!(listed.split(',').any(|r| r == rule), "{rule} in {listed}");
        }
        assert!(!listed.split(',').any(|r| r == "Bash"));
        assert!(allowed(&opts(true, false)).split(',').any(|r| r == "Bash"), "running anything allows every command");
        assert!(!allowed(&opts(true, true)).split(',').any(|r| r == "Bash"), "never for a read-only run");
    }

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
        assert_eq!(parse(res), vec![Event::ToolResult { error: true, text: "permission denied\nx".into() }]);

        let done = r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.42,"num_turns":7,"result":"Done","usage":{"input_tokens":10,"cache_read_input_tokens":90,"output_tokens":5}}"#;
        assert_eq!(parse(done), vec![Event::Usage { fresh: 10, cached: 90, output: 5 }, Event::Done { ok: true, cost: 0.42, tokens: 105, text: "Done".into() }]);

        assert!(parse("not json").is_empty());
    }
}
