//! Claude Code, run headless with stream-json output.

use super::{Event, Opts, clip, stream};
use anyhow::Result;
use serde_json::{Value, json};
use std::process::Command;

// Edits are auto-accepted inside the worktree. With "Only allowed", Bash is limited to read-only
// git plus the repo's commands. With "Anything", it's open except for NEVER_ANY.
// The company tools are always allowed.
// Read-only text tools too: agents pipe test output through tail or head, and one blocked
// part of a pipe blocks the whole command.
const ALLOWED: &str = "Bash(git status:*),Bash(git diff:*),Bash(git log:*),Bash(ls:*),Bash(cat:*),Bash(head:*),Bash(tail:*),Bash(wc:*),Bash(grep:*),Bash(echo:*),Bash(which:*),Bash(pwd),mcp__onecorp";

pub fn run(o: &Opts, prompt: &str, on: impl FnMut(&str, Event) -> Result<()>) -> Result<()> {
    let mut cmd = Command::new("claude");
    // Without acceptEdits, edits need a permission nobody can give in headless mode, so they're refused.
    let mode = if o.read_only { "default" } else { "acceptEdits" };
    cmd.args(["-p", "--output-format", "stream-json", "--verbose", "--include-partial-messages", "--permission-mode", mode])
        // Only the company server loads, if any. The user's own MCP servers stay out.
        .arg("--strict-mcp-config")
        // So do their plugins, skills, hooks and personal CLAUDE.md: they're read again on every
        // turn, and they'd change how an employee behaves. The repo's own settings still apply.
        .args(["--setting-sources", "project"])
        .args(["--allowedTools", &allowed(o)])
        .current_dir(o.cwd)
        // ask_teammate waits for a teammate's chat reply, which can take a few minutes.
        .env("MCP_TOOL_TIMEOUT", "600000")
        .envs(o.env.iter().map(|(k, v)| (k, v)));
    if let Some(m) = o.mcp {
        let env: serde_json::Map<String, Value> = m.env.iter().map(|(k, v)| (k.clone(), json!(v))).collect();
        let mcp = json!({"mcpServers": {"onecorp": {"command": m.command, "args": m.args, "env": env}}});
        cmd.args(["--mcp-config", &mcp.to_string()]);
    }
    cmd.args(["--settings", &settings(o, user_settings()).to_string()]);
    if let Some(m) = o.model {
        cmd.args(["--model", m]);
    }
    if let Some(e) = o.effort {
        cmd.args(["--effort", e]);
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
        // A run that only reads and talks, like a chat reply, is mostly its fixed prompt. The
        // tools it can't use and the skills list are most of that. The company tools still load.
        cmd.args(["--tools", "Read,Grep,Glob,Bash", "--disable-slash-commands"]);
    }
    if let Some(s) = o.resume {
        cmd.args(["--resume", s]);
    }
    if let Some(system) = o.system {
        cmd.args(["--append-system-prompt", system]);
        // A ticket run's folder is its own worktree, which Claude Code names in its system prompt.
        // Moved to the first message, the system prompt is the same for every ticket, so the
        // shared base before it comes from cache. A chat reply always runs in the same folder.
        if !o.read_only {
            cmd.arg("--exclude-dynamic-system-prompt-sections");
        }
    }
    if let Some(max) = o.max_cost {
        cmd.args(["--max-budget-usd", &format!("{max:.2}")]);
    }
    // The prompt goes on stdin: --allowedTools takes a list and would swallow a trailing argument.
    stream(cmd, prompt, parse, o.on_spawn, on)
}

/// What a run still needs from the user's own Claude Code settings once they're left out: how
/// they sign in, and the model "Default" stands for.
fn user_settings() -> serde_json::Map<String, Value> {
    let Some(home) = std::env::var("HOME").ok() else { return Default::default() };
    let text = std::fs::read_to_string(std::path::Path::new(&home).join(".claude/settings.json")).unwrap_or_default();
    kept(&serde_json::from_str(&text).unwrap_or_default())
}

fn kept(all: &Value) -> serde_json::Map<String, Value> {
    const KEEP: [&str; 5] = ["env", "apiKeyHelper", "awsAuthRefresh", "awsCredentialExport", "model"];
    KEEP.iter().filter_map(|k| Some((k.to_string(), all.get(k)?.clone()))).collect()
}

/// The most of a successful command's output a ticket run reads inline, in characters. Past it,
/// Claude Code saves the output to a file and the agent gets the end of it and the path. A long
/// test log otherwise sits in the conversation and is read again on every later step. Failed
/// commands keep their whole output: that's where the error is.
const COMMAND_OUTPUT: u64 = 8_000;

/// The settings a run gets: the user's sign-in and model, and for ticket runs, shorter command output.
fn settings(o: &Opts, mut keep: serde_json::Map<String, Value>) -> Value {
    if !o.read_only {
        keep.insert("bashOutputMaxChars".into(), json!(COMMAND_OUTPUT));
    }
    Value::Object(keep)
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
        Some("rate_limit_event") => {
            let w = &v["rate_limit_info"]["unifiedWindows"];
            let pct = |k: &str| w[k]["utilization"].as_f64().map(|u| u * 100.0);
            let at = |k: &str| w[k]["resetsAt"].as_i64();
            match (pct("five_hour"), at("five_hour"), pct("seven_day"), at("seven_day")) {
                (Some(session), Some(session_resets), Some(week), Some(week_resets)) => vec![Event::Plan(crate::plan::Usage { session, session_resets, week, week_resets })],
                _ => vec![],
            }
        }
        Some("stream_event") if v["event"]["delta"]["type"] == "text_delta" => vec![Event::Delta(s(&v["event"]["delta"], "text"))],
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
            let mut tokens = fresh + cached + output;
            let usage = (tokens > 0).then_some(Event::Usage { fresh, cached, output });
            // A run stopped at its budget reports no usage of its own, only what each model used.
            if tokens == 0 {
                let models = v["modelUsage"].as_object().into_iter().flatten();
                tokens = models.flat_map(|(_, m)| ["inputTokens", "cacheReadInputTokens", "cacheCreationInputTokens", "outputTokens"].map(|k| m[k].as_u64().unwrap_or(0))).sum();
            }
            let stopped = v["subtype"] == "error_max_budget_usd";
            usage.into_iter().chain([Event::Done {
                ok: v["subtype"] == "success" && v["is_error"] != true,
                cost: v["total_cost_usd"].as_f64().unwrap_or(0.0),
                tokens,
                text: if stopped { super::BUDGET_STOP.to_string() } else { s(&v, "result") },
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
        let opts = |run_any, read_only| Opts { cwd: std::path::Path::new("/wt"), model: None, mcp: None, resume: None, env: &[], read_only, on_spawn: None, commands: &cmds, dirs: &dirs, run_any, max_cost: None, system: None, effort: None };
        let listed = allowed(&opts(false, false));
        for rule in ["Bash(tools/check.sh:*)", "Bash(./tools/check.sh:*)", "Bash(./gradlew assembleDebug:*)", "Bash(gradlew assembleDebug:*)", "Read(//c/reading/wiki/**)"] {
            assert!(listed.split(',').any(|r| r == rule), "{rule} in {listed}");
        }
        assert!(!listed.split(',').any(|r| r == "Bash"));
        assert!(allowed(&opts(true, false)).split(',').any(|r| r == "Bash"), "running anything allows every command");
        assert!(!allowed(&opts(true, true)).split(',').any(|r| r == "Bash"), "never for a read-only run");
    }

    #[test]
    fn keeps_sign_in_and_model_from_the_users_settings() {
        let all = json!({"model": "opus", "env": {"CLAUDE_CODE_USE_BEDROCK": "1"}, "hooks": {"SessionStart": []}, "enabledPlugins": {"x": true}});
        assert_eq!(Value::Object(kept(&all)), json!({"env": {"CLAUDE_CODE_USE_BEDROCK": "1"}, "model": "opus"}));
        assert!(kept(&json!({"hooks": {}})).is_empty(), "nothing to pass when only the rest is set");
    }

    #[test]
    fn a_run_stopped_at_its_budget_says_so_and_still_counts() {
        let line = r#"{"type":"result","subtype":"error_max_budget_usd","is_error":true,"total_cost_usd":0.52,"usage":{"input_tokens":0,"output_tokens":0},"modelUsage":{"claude-haiku-5-5":{"inputTokens":4,"outputTokens":248,"cacheReadInputTokens":33466,"cacheCreationInputTokens":1051}},"errors":["Reached maximum budget ($0.5)"]}"#;
        assert_eq!(parse(line), vec![Event::Done { ok: false, cost: 0.52, tokens: 34769, text: crate::runtime::BUDGET_STOP.into() }]);
    }

    #[test]
    fn text_comes_as_it_is_written() {
        let delta = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hel"}}}"#;
        assert_eq!(parse(delta), vec![Event::Delta("Hel".into())]);
        let other = r#"{"type":"stream_event","event":{"type":"content_block_delta","index":1,"delta":{"type":"input_json_delta","partial_json":"{"}}}"#;
        assert!(parse(other).is_empty(), "only text");
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

    #[test]
    fn reads_the_plan_from_rate_limit_events() {
        let line = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1795188000,"rateLimitType":"five_hour","unifiedWindows":{"five_hour":{"utilization":0.13,"resetsAt":1795188000,"status":"allowed"},"seven_day":{"utilization":0.1,"resetsAt":1795737600,"status":"allowed"}}}}"#;
        let Event::Plan(u) = &parse(line)[0] else { panic!("a plan event") };
        assert_eq!((u.session.round(), u.session_resets, u.week.round(), u.week_resets), (13.0, 1795188000, 10.0, 1795737600));
        let api_key = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#;
        assert!(parse(api_key).is_empty(), "no plan on an API key");
    }
}
