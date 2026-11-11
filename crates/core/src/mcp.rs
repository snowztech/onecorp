//! The company's MCP server. Claude Code starts it over stdio for each ticket, so the agent can
//! talk back to the company: ask the founder something, post to a channel, mark itself blocked.
//!
//! MCP over stdio is newline-delimited JSON-RPC 2.0. We only need four methods, so it's done by hand.

use crate::store::{Company, Question};
use crate::work::Job;
use anyhow::Result;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

pub struct Ctx {
    pub ticket: i64,
    pub employee: String,
}

pub fn serve(c: &Company, ctx: &Ctx) -> Result<()> {
    let stdout = std::io::stdout();
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(c, ctx, &msg),
            Err(e) => Some(json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}})),
        };
        if let Some(reply) = reply {
            let mut out = stdout.lock();
            writeln!(out, "{reply}")?;
            out.flush()?;
        }
    }
    Ok(())
}

/// Lets any OneCorp binary act as the company MCP server when started as
/// `<binary> mcp --company <name> --ticket <id> --employee <id>`, which is how agents launch it.
/// Returns None when the arguments are something else. The desktop app calls this first thing.
pub fn from_args(args: &[String]) -> Option<Result<()>> {
    if args.get(1).map(String::as_str) != Some("mcp") {
        return None;
    }
    let get = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    Some((|| {
        let company = get("--company").ok_or_else(|| anyhow::anyhow!("mcp needs --company"))?;
        let ticket = get("--ticket").and_then(|t| t.parse().ok()).ok_or_else(|| anyhow::anyhow!("mcp needs --ticket"))?;
        let employee = get("--employee").ok_or_else(|| anyhow::anyhow!("mcp needs --employee"))?;
        serve(&Company::open(&company)?, &Ctx { ticket, employee })
    })())
}

/// Returns None for notifications, which get no reply.
pub fn handle(c: &Company, ctx: &Ctx, msg: &Value) -> Option<Value> {
    let id = msg.get("id")?.clone();
    let result = match msg["method"].as_str().unwrap_or("") {
        "initialize" => Ok(json!({
            "protocolVersion": msg["params"]["protocolVersion"].as_str().unwrap_or("2025-06-18"),
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "onecorp", "version": env!("CARGO_PKG_VERSION")},
        })),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools()})),
        "tools/call" => Ok(match call(c, ctx, &msg["params"]["name"], &msg["params"]["arguments"]) {
            Ok(text) => json!({"content": [{"type": "text", "text": text}]}),
            Err(e) => json!({"content": [{"type": "text", "text": format!("{e:#}")}], "isError": true}),
        }),
        m => Err(json!({"code": -32601, "message": format!("unknown method {m}")})),
    };
    Some(match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
    })
}

fn tools() -> Value {
    json!([
        {
            "name": "ask_human",
            "description": "Ask the founder a question. It lands in their inbox. Use blocking=true only when you can't continue \
                            without the answer, then stop working. Otherwise give a default, carry on with it, and say so.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {"type": "string"},
                    "options": {"type": "array", "items": {"type": "string"}, "description": "Possible answers, if it's a choice"},
                    "default": {"type": "string", "description": "What you'll go with if nobody answers"},
                    "blocking": {"type": "boolean", "description": "True if you are stopping until this is answered"}
                },
                "required": ["question"]
            }
        },
        {
            "name": "ask_to_run",
            "description": "Ask the founder to let you run a command you aren't allowed yet, like a code generator or a build step \
                            in a subfolder. Once allowed, the whole team can run it in this repo. Ask for the shortest command \
                            that covers it: 'make sqlc', not 'make sqlc -v'. Then stop and summarize: the ticket picks back up with the answer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "command": {"type": "string", "description": "The command, like 'xcodegen generate'. It also covers what follows it"},
                    "why": {"type": "string", "description": "One sentence on what it's for, like 'the Xcode project is generated, I need it to build'"}
                },
                "required": ["command", "why"]
            }
        },
        {
            "name": "post_message",
            "description": "Post a short message to the team chat. Use it for things teammates should know, not for progress logs. Mention someone with @firstname.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "channel": {"type": "string", "description": "general for the whole company, or a department's channel like engineering. Defaults to your department's"},
                    "text": {"type": "string"}
                },
                "required": ["text"]
            }
        },
        {
            "name": "propose_wiki",
            "description": "Propose adding something to the company wiki that every employee reads: a decision, a convention, \
                            a fact about the codebase. Only things that should outlive this ticket. The founder approves it from their inbox.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "One line, written as a rule or fact, like 'Auth tokens are hashed at rest with sha2'"},
                    "section": {"type": "string", "enum": ["Decisions", "Conventions"], "description": "Defaults to Decisions"}
                },
                "required": ["text"]
            }
        },
        {
            "name": "create_ticket",
            "description": "Create a ticket for work that should happen but isn't yours to do now: a follow-up, a piece of a bigger plan, \
                            a bug you noticed. It goes to the backlog, and the founder starts it. Give it to a teammate when you know who fits.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": {"type": "string", "description": "What needs doing, short, like 'Add rate limiting to the public API'"},
                    "description": {"type": "string", "description": "What done looks like, and anything the person doing it should know"},
                    "assignee": {"type": "string", "description": "A teammate's first name or id, from the team list"},
                    "after": {"type": "string", "description": "A ticket this one should wait for, like SNO-3"},
                    "repo": {"type": "string", "description": "Which of the company's repos the work happens in, by name, when it has more than one. Leave it out for the main repo"}
                },
                "required": ["title"]
            }
        },
        {
            "name": "remember",
            "description": "Save a one-line note for your future self: where things live in the code, how the founder likes things done, \
                            a trap you hit. Only you read these, at the start of every ticket. Rules for the whole team go to propose_wiki instead.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": {"type": "string", "description": "One line, like 'Greeting text lives in main.rs, nowhere else'"}
                },
                "required": ["text"]
            }
        },
        {
            "name": "update_ticket",
            "description": "Mark your ticket blocked, with what you need. It goes to the founder's Inbox to answer, and the ticket resumes with their answer.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "status": {"type": "string", "enum": ["blocked"]},
                    "note": {"type": "string", "description": "What you need from the founder to carry on, in a sentence or two"}
                },
                "required": ["status", "note"]
            }
        }
    ])
}

fn call(c: &Company, ctx: &Ctx, name: &Value, args: &Value) -> Result<String> {
    // What agents write for people to read gets the writing rule enforced. A command is code, and kept exactly.
    let s = |k: &str| {
        let v = args[k].as_str().unwrap_or("").trim();
        if k == "command" { v.to_string() } else { crate::runtime::plain(v) }
    };
    match name.as_str().unwrap_or("") {
        "ask_human" => {
            let question = s("question");
            anyhow::ensure!(!question.is_empty(), "question is required");
            let options = args["options"]
                .as_array()
                .map(|o| o.iter().filter_map(Value::as_str).collect::<Vec<_>>().join(", "))
                .unwrap_or_default();
            let blocking = args["blocking"] == true;
            // Someone with a lead asks them first. The lead decides or passes it up to the founder.
            let lead = c.lead_of(&ctx.employee)?;
            let n = c.ask(&Question {
                ticket_id: Some(ctx.ticket),
                employee: ctx.employee.clone(),
                question,
                options,
                default_answer: s("default"),
                blocking,
                kind: "question".into(),
                stage: if lead.is_some() { "lead" } else { "founder" }.into(),
                ..Default::default()
            })?;
            let to = match &lead {
                Some(l) => {
                    c.enqueue(ctx.ticket, &Job::Consult { question: n })?;
                    format!("{}, your lead", l.name)
                }
                None => "the founder".into(),
            };
            // Out loud in the team channel, so the founder sees the team think, not only the outcome.
            let asked = c.question(n)?.map(|q| q.question).unwrap_or_default();
            let who = match &lead {
                Some(l) => format!("@{}", l.id),
                None => "the founder, in their Inbox".into(),
            };
            crate::chat::note(c, &ctx.employee, &ctx.employee, &format!("{asked}\n(Asking {who}.)"), Some(ctx.ticket))?;
            if blocking {
                c.set_status(ctx.ticket, "blocked")?;
                Ok(format!("Question #{n} went to {to}, and the ticket is marked blocked. Stop here and summarize where you are."))
            } else {
                Ok(format!("Question #{n} went to {to}. Carry on with your default and mention it in your summary."))
            }
        }
        "ask_to_run" => {
            let command = s("command").split_whitespace().collect::<Vec<_>>().join(" ");
            anyhow::ensure!(!command.is_empty(), "command is required");
            if let Some(why) = crate::commands::never(&command) {
                anyhow::bail!("{command} can't be allowed: {why}. Do without it.");
            }
            let t = c.ticket(ctx.ticket)?.ok_or_else(|| anyhow::anyhow!("no ticket"))?;
            let have = crate::commands::for_ticket(c, &t)?;
            if have.iter().any(|h| command == *h || command.starts_with(&format!("{h} "))) {
                return Ok(format!("{command} is already allowed. Run it."));
            }
            // Straight to the founder: what runs on their Mac is their call, not a lead's.
            let n = c.ask(&Question {
                ticket_id: Some(ctx.ticket),
                employee: ctx.employee.clone(),
                question: command,
                options: crate::commands::repo_of(c, &t)?.name,
                blocking: true,
                kind: "command".into(),
                stage: "founder".into(),
                note: Some(s("why")).filter(|w| !w.is_empty()),
                ..Default::default()
            })?;
            c.set_status(ctx.ticket, "blocked")?;
            Ok(format!("Request #{n} is in the founder's Inbox, and the ticket is marked blocked. Stop here and summarize where you are."))
        }
        "post_message" => {
            let text = s("text");
            anyhow::ensure!(!text.is_empty(), "text is required");
            let channel = match s("channel") {
                typed if !typed.trim().is_empty() => crate::chat::resolve(c, &typed)?,
                _ => match c.employee(&ctx.employee)? {
                    Some(e) => crate::chat::home(c, &e)?,
                    None => crate::chat::GENERAL.into(),
                },
            };
            c.post(&channel, &ctx.employee, &text, Some(ctx.ticket))?;
            Ok(format!("Posted in #{channel}."))
        }
        "propose_wiki" => {
            let text = s("text");
            anyhow::ensure!(!text.is_empty(), "text is required");
            let section = if s("section").eq_ignore_ascii_case("conventions") { "Conventions" } else { "Decisions" };
            let n = c.ask(&Question {
                ticket_id: Some(ctx.ticket),
                employee: ctx.employee.clone(),
                question: text,
                options: section.into(),
                kind: "wiki".into(),
                ..Default::default()
            })?;
            Ok(format!("Proposal #{n} is in the founder's inbox. Carry on, it doesn't block your ticket."))
        }
        "create_ticket" => {
            let assignee = match s("assignee") {
                a if a.is_empty() => None,
                a => Some(crate::store::slug(&a)).filter(|id| c.employee(id).ok().flatten().is_some()).ok_or_else(|| anyhow::anyhow!("nobody called {a} works here. Leave assignee out, or use a name from the team list"))?.into(),
            };
            let after = match s("after") {
                a if a.is_empty() => None,
                a => Some(c.parse_key(&a)?),
            };
            // Created while planning, it belongs to that plan, and starts when the founder starts the plan.
            let plan = c.ticket(ctx.ticket)?.filter(|t| t.kind == "plan").map(|t| t.id);
            let new = crate::store::NewTicket { title: s("title"), description: s("description"), assignee, created_by: Some(ctx.employee.clone()), after, plan, repo: Some(s("repo")).filter(|r| !r.is_empty()) };
            let id = c.new_ticket(&new, "backlog")?;
            c.add_event(id, "create", &format!("created by {} while working on {}", ctx.employee, c.key(ctx.ticket)), "")?;
            Ok(match plan {
                Some(_) => format!("{} is in the plan.", c.key(id)),
                None => format!("{} is in the backlog. The founder starts it.", c.key(id)),
            })
        }
        "remember" => {
            let text = s("text");
            anyhow::ensure!(!text.is_empty(), "text is required");
            c.remember(&ctx.employee, &text)?;
            Ok("Saved to your notes. You'll see it at the start of your next ticket.".into())
        }
        "update_ticket" => {
            anyhow::ensure!(s("status") == "blocked", "the only status you can set is blocked");
            let note = s("note");
            anyhow::ensure!(!note.trim().is_empty(), "note is required: say what you need to carry on");
            // A block lands in the founder's Inbox as a question, so there's always something to answer.
            c.ask_to_unblock(ctx.ticket, &ctx.employee, &note)?;
            c.set_status(ctx.ticket, "blocked")?;
            c.add_event(ctx.ticket, "block", &note, &args.to_string())?;
            Ok("Ticket marked blocked, and your note is in the founder's Inbox. Stop here and summarize where you are.".into())
        }
        other => anyhow::bail!("no tool called {other}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Employee;

    #[test]
    fn speaks_enough_mcp_and_writes_to_the_company() {
        let dir = std::env::temp_dir().join(format!("onecorp-mcp-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = Company::open_dir("snowztech", dir.clone()).unwrap();
        let maya = Employee {
            id: "maya".into(),
            name: "Maya".into(),
            role: "Engineer".into(),
            runtime: "claude".into(),
            model: None,
            bio: String::new(),
            department: None,
        };
        c.hire(&maya).unwrap();
        let t = c.add_ticket("magic links", "maya").unwrap();
        let ctx = Ctx { ticket: t, employee: "maya".into() };
        let req = |id: i64, method: &str, params: Value| json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params});

        let init = handle(&c, &ctx, &req(1, "initialize", json!({"protocolVersion": "2025-06-18"}))).unwrap();
        assert_eq!(init["result"]["serverInfo"]["name"], "onecorp");
        assert!(handle(&c, &ctx, &json!({"jsonrpc": "2.0", "method": "notifications/initialized"})).is_none());

        let list = handle(&c, &ctx, &req(2, "tools/list", json!({}))).unwrap();
        assert_eq!(list["result"]["tools"].as_array().unwrap().len(), 7);

        let args = json!({"question": "15 or 60 minutes?", "options": ["15", "60"], "default": "15", "blocking": true});
        let ask = handle(&c, &ctx, &req(3, "tools/call", json!({"name": "ask_human", "arguments": args}))).unwrap();
        assert!(ask["result"]["isError"].is_null());
        let inbox = c.inbox().unwrap();
        assert_eq!((inbox[0].options.as_str(), inbox[0].blocking), ("15, 60", true));
        assert_eq!(c.ticket(t).unwrap().unwrap().status, "blocked");

        // A clean finish must not unblock a ticket the agent marked blocked.
        c.finish_ticket(t, "review", 0.1, 0).unwrap();
        assert_eq!(c.ticket(t).unwrap().unwrap().status, "blocked");

        let post = json!({"name": "post_message", "arguments": {"channel": "#general", "text": "hashing tokens at rest"}});
        handle(&c, &ctx, &req(4, "tools/call", post)).unwrap();
        let general = c.messages(Some("general")).unwrap();
        assert!(general[0].text.starts_with("15 or 60 minutes?"), "the question was said out loud first");
        assert_eq!(general.last().unwrap().text, "hashing tokens at rest");

        let prop = json!({"name": "propose_wiki", "arguments": {"text": "Tokens are hashed at rest", "section": "conventions"}});
        handle(&c, &ctx, &req(7, "tools/call", prop)).unwrap();
        let p = c.inbox().unwrap().into_iter().find(|q| q.kind == "wiki").unwrap();
        assert_eq!((p.question.as_str(), p.options.as_str(), p.blocking), ("Tokens are hashed at rest", "Conventions", false));

        let note = json!({"name": "remember", "arguments": {"text": "  auth lives in\n src/auth  "}});
        handle(&c, &ctx, &req(8, "tools/call", note)).unwrap();
        let mem = c.memory("maya").unwrap();
        assert!(mem.starts_with("- 20") && mem.ends_with(" auth lives in src/auth\n"), "{mem}");

        let make = json!({"name": "create_ticket", "arguments": {"title": "Rate limit the API", "description": "60 a minute", "assignee": "Maya", "after": c.key(t)}});
        let made = handle(&c, &ctx, &req(9, "tools/call", make)).unwrap();
        assert!(made["result"]["isError"].is_null(), "{made}");
        let new = c.tickets().unwrap().into_iter().find(|x| x.title == "Rate limit the API").unwrap();
        assert_eq!((new.status.as_str(), new.assignee.as_deref(), new.after, new.created_by.as_str()), ("backlog", Some("maya"), Some(t), "maya"));
        let nobody = json!({"name": "create_ticket", "arguments": {"title": "x", "assignee": "Zed"}});
        assert_eq!(handle(&c, &ctx, &req(10, "tools/call", nobody)).unwrap()["result"]["isError"], true);

        let bad = handle(&c, &ctx, &req(5, "tools/call", json!({"name": "nope", "arguments": {}}))).unwrap();
        assert_eq!(bad["result"]["isError"], true);
        assert_eq!(handle(&c, &ctx, &req(6, "nope", json!({}))).unwrap()["error"]["code"], -32601);

        // Firing someone with tickets keeps their history, and rehiring brings them back.
        c.fire("maya").unwrap();
        assert!(c.employee("maya").unwrap().is_none());
        assert_eq!(c.ticket(t).unwrap().unwrap().assignee.as_deref(), Some("maya"));
        c.hire(&maya).unwrap();
        assert!(c.employee("maya").unwrap().is_some());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_block_always_reaches_the_inbox() {
        let dir = std::env::temp_dir().join(format!("onecorp-mcp-block-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = Company::open_dir("snowztech", dir.clone()).unwrap();
        let maya = Employee { id: "maya".into(), name: "Maya".into(), role: "Engineer".into(), runtime: "claude".into(), model: None, bio: String::new(), department: None };
        c.hire(&maya).unwrap();
        let req = |args: Value| json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {"name": "update_ticket", "arguments": args}});

        // Blocked with a note: the note is a question in your Inbox, and the ticket waits on it.
        let t = c.add_ticket("rebase on main", "maya").unwrap();
        let ctx = Ctx { ticket: t, employee: "maya".into() };
        handle(&c, &ctx, &req(json!({"status": "blocked", "note": "git rebase origin/main needs your approval"}))).unwrap();
        let q = c.inbox().unwrap().into_iter().find(|q| q.ticket_id == Some(t)).expect("an Inbox item for the block");
        assert_eq!((q.question.as_str(), q.blocking), ("git rebase origin/main needs your approval", true));
        assert_eq!(c.ticket(t).unwrap().unwrap().status, "blocked");

        // A ticket an older version left blocked with only a note gets its question on recovery, once.
        let old = c.add_ticket("stuck since 0.1.3", "maya").unwrap();
        c.set_status(old, "blocked").unwrap();
        c.add_event(old, "block", "run the rebase yourself, or allow it", "").unwrap();
        c.recover_interrupted().unwrap();
        c.recover_interrupted().unwrap();
        let asked: Vec<_> = c.inbox().unwrap().into_iter().filter(|q| q.ticket_id == Some(old)).collect();
        assert_eq!(asked.len(), 1);
        assert_eq!(asked[0].question, "run the rebase yourself, or allow it");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
