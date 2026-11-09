use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use onecorp::agent::{self, Event, RUNTIMES, first_line};
use onecorp::git;
use onecorp::store::{self, Employee};
use std::io::{BufRead, IsTerminal, Write};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "onecorp", about = "Run a company of AI employees")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create a company around a local git repo
    New {
        name: String,
        #[arg(long)]
        repo: PathBuf,
    },
    /// Switch the current company
    Use { name: String },
    /// List companies
    Companies,
    /// Hire someone. Run with no arguments to be asked step by step
    Hire {
        /// Their name, like "Maya" or "Maya Okafor"
        name: Option<String>,
        /// Start from a preset: engineer, reviewer, designer, writer
        #[arg(long = "as")]
        preset: Option<String>,
        /// Job title, if you want something other than the preset's
        #[arg(long)]
        role: Option<String>,
        /// How they work, in a sentence or two
        #[arg(long)]
        bio: Option<String>,
        /// Which agent does the work
        #[arg(long, default_value = "claude")]
        runtime: String,
        /// Model for the runtime, like sonnet or opus. Defaults to the runtime's own default
        #[arg(long)]
        model: Option<String>,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Let someone go. Their past tickets stay in the history
    Fire {
        name: String,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Show who works here
    Team {
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Open a ticket and have someone work on it now
    Run {
        /// What needs doing, in plain words
        task: Vec<String>,
        /// Who does it. Optional when there's only one employee
        #[arg(long)]
        to: Option<String>,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Show tickets and what they cost
    Status {
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Show everything that happened on a ticket
    Logs {
        ticket: String,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Questions your team is waiting on
    Inbox {
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Answer a question from the inbox
    Answer {
        /// Question number, like 3 or #3
        question: String,
        /// Quote it if it has spaces. It may start with a dash, like --name
        #[arg(allow_hyphen_values = true)]
        answer: String,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Read team chat
    Chat {
        /// Channel, like eng or #eng. All channels when left out
        channel: Option<String>,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// The company MCP server. Started by the agent, not by you
    #[command(hide = true)]
    Mcp {
        #[arg(long)]
        company: String,
        #[arg(long)]
        ticket: i64,
        #[arg(long)]
        employee: String,
    },
}

struct Preset {
    key: &'static str,
    role: &'static str,
    bio: &'static str,
}

const PRESETS: &[Preset] = &[
    Preset {
        key: "engineer",
        role: "Engineer",
        bio: "Reads the code around a change before touching it. Makes the smallest diff that does the job and says what to check.",
    },
    Preset {
        key: "reviewer",
        role: "Reviewer",
        bio: "Reads diffs carefully. Flags risks, missing tests and new dependencies that don't earn their place.",
    },
    Preset {
        key: "designer",
        role: "Product designer",
        bio: "Designs in code. Keeps screens simple and copy short, and follows the styles already in the repo.",
    },
    Preset {
        key: "writer",
        role: "Writer",
        bio: "Writes docs, changelogs and posts. Short sentences, plain words, no hype.",
    },
];


fn main() {
    if let Err(e) = run(Cli::parse().cmd) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}

fn open(company: Option<String>) -> Result<store::Company> {
    store::Company::open(&match company {
        Some(c) => c,
        None => store::current()?,
    })
}

fn run(cmd: Cmd) -> Result<()> {
    match cmd {
        Cmd::New { name, repo } => {
            let c = store::Company::create(&name, &repo)?;
            store::set_current(&name)?;
            println!("Created {} around {}. It's now your current company.", c.name, c.repo()?.display());
            println!("Next, hire someone: onecorp hire");
        }
        Cmd::Use { name } => {
            store::Company::open(&name)?;
            store::set_current(&name)?;
            println!("Now working in {name}.");
        }
        Cmd::Companies => {
            let current = store::current().unwrap_or_default();
            for name in store::list()? {
                println!("{} {name}", if name == current { "*" } else { " " });
            }
        }
        Cmd::Hire { name, preset, role, bio, runtime, model, company } => {
            let c = open(company)?;
            let e = hire_form(name, preset, role, bio, runtime, model)?;
            c.hire(&e)?;
            println!("Hired {} as {} at {}.", e.name, e.role, c.name);
            println!("Give them a ticket: onecorp run \"<task>\" --to {}", e.id);
        }
        Cmd::Fire { name, company } => {
            let c = open(company)?;
            c.fire(&slug(&name))?;
            println!("{name} no longer works at {}.", c.name);
        }
        Cmd::Team { company } => {
            let c = open(company)?;
            let team = c.employees()?;
            if team.is_empty() {
                println!("Nobody works at {} yet. Hire someone: onecorp hire", c.name);
            }
            let tickets = c.tickets()?;
            for e in &team {
                let theirs: Vec<_> = tickets.iter().filter(|t| t.assignee.as_deref() == Some(&e.id)).collect();
                let cost = theirs.iter().fold(0.0, |s, t| s + t.cost_usd);
                let tokens: i64 = theirs.iter().map(|t| t.tokens).sum();
                let rt = match &e.model {
                    Some(m) => format!("{} ({m})", e.runtime),
                    None => e.runtime.clone(),
                };
                let n = theirs.len();
                let plural = if n == 1 { "ticket" } else { "tickets" };
                println!("{:<10} {:<20} {:<18} {:<16} {n} {plural}, {}", e.id, e.name, e.role, rt, spend(cost, tokens));
            }
        }
        Cmd::Run { task, to, company } => {
            let title = task.join(" ");
            if title.trim().is_empty() {
                bail!("say what needs doing, for example: onecorp run \"add a --version flag\"");
            }
            let c = open(company)?;
            let who = assignee(&c, to)?;
            work(&c, &who, &title)?;
        }
        Cmd::Status { company } => {
            let c = open(company)?;
            let tickets = c.tickets()?;
            if tickets.is_empty() {
                println!("No tickets yet. Start one with: onecorp run \"<task>\"");
                return Ok(());
            }
            let cost = tickets.iter().fold(0.0, |s, t| s + t.cost_usd);
            let tokens: i64 = tickets.iter().map(|t| t.tokens).sum();
            for t in &tickets {
                let who = t.assignee.as_deref().unwrap_or("-");
                println!("{:<8} {:<7} {:<10} {:>8}  {}", c.key(t.id), t.status, who, spend(t.cost_usd, t.tokens), t.title);
            }
            println!("{:<27} ${cost:.2} and {}k tokens", "total", (tokens + 500) / 1000);
        }
        Cmd::Logs { ticket, company } => {
            let c = open(company)?;
            let id = c.parse_key(&ticket)?;
            let Some(t) = c.ticket(id)? else { bail!("no ticket {ticket} in {}", c.name) };
            println!("{}  {}  [{}]  ${:.2}", c.key(t.id), t.title, t.status, t.cost_usd);
            if let Some(who) = &t.assignee {
                println!("assignee {who}");
            }
            if let Some(b) = &t.branch {
                println!("branch   {b}");
            }
            for (at, kind, summary) in c.events(id)? {
                println!("{at}  {kind:<6} {summary}");
            }
        }
        Cmd::Inbox { company } => {
            let c = open(company)?;
            let qs = c.inbox()?;
            if qs.is_empty() {
                println!("Inbox zero. Nobody is waiting on you.");
            }
            for q in qs {
                let ticket = q.ticket_id.map(|t| c.key(t)).unwrap_or_default();
                let tag = if q.blocking { "  [blocking]" } else { "" };
                println!("#{:<3} {ticket:<7} {}{tag}\n     {}", q.id, q.employee, q.question);
                let mut extra = vec![];
                if !q.options.is_empty() {
                    extra.push(format!("options: {}", q.options));
                }
                if !q.default_answer.is_empty() {
                    extra.push(format!("default: {}", q.default_answer));
                }
                if !extra.is_empty() {
                    println!("     {}", extra.join("   "));
                }
                println!();
            }
        }
        Cmd::Answer { question, answer, company } => {
            let c = open(company)?;
            let id: i64 = question.trim_start_matches('#').parse().map_err(|_| anyhow::anyhow!("not a question number: {question}"))?;
            let text = answer;
            if text.trim().is_empty() {
                bail!("what's the answer? For example: onecorp answer {id} \"go with 15 minutes\"");
            }
            let text = text.trim();
            let q = c.answer(id, text)?;
            println!("Answered #{id}.");
            let Some(tid) = q.ticket_id else { return Ok(()) };
            let (key, status) = (c.key(tid), c.ticket(tid)?.map(|t| t.status).unwrap_or_default());

            if q.blocking && status == "blocked" {
                match c.open_blocking(tid)? {
                    0 => resume(&c, &q)?,
                    n => println!("{key} is still waiting on {n} more: onecorp inbox"),
                }
            } else if !q.blocking && status == "review" && !q.default_answer.eq_ignore_ascii_case(text) {
                // They went ahead with their default. A different answer means the work needs a change.
                let went_with = if q.default_answer.is_empty() { "their own call".to_string() } else { format!("\"{}\"", q.default_answer) };
                let who = c.employee(&q.employee)?.map(|e| e.name).unwrap_or_else(|| q.employee.clone());
                println!("{who} went with {went_with}, so {key} goes back to apply your answer.\n");
                resume(&c, &q)?;
            }
        }
        Cmd::Chat { channel, company } => {
            let c = open(company)?;
            let channel = channel.map(|ch| ch.trim_start_matches('#').to_string());
            let msgs = c.messages(channel.as_deref())?;
            if msgs.is_empty() {
                println!("Nothing posted yet.");
            }
            for (at, ch, author, text, ticket) in msgs {
                let ticket = ticket.map(|t| format!("  ({})", c.key(t))).unwrap_or_default();
                println!("{} #{ch:<8} {author}: {text}{ticket}", at.get(11..16).unwrap_or(&at));
            }
        }
        Cmd::Mcp { company, ticket, employee } => {
            let c = store::Company::open(&company)?;
            onecorp::mcp::serve(&c, &onecorp::mcp::Ctx { ticket, employee })?;
        }
    }
    Ok(())
}

/// Ids are the lowercased first name: "Maya Okafor" becomes maya.
/// Accents fold to plain letters so "Tomás" becomes tomas, which is what people will type.
fn slug(name: &str) -> String {
    let fold = |c: char| match c {
        'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'ö' | 'õ' | 'ø' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        c => c,
    };
    let first = name.split_whitespace().next().unwrap_or("").to_lowercase();
    first.chars().map(fold).filter(char::is_ascii_alphanumeric).collect()
}


/// Fills in whatever the flags left out: from a preset, from questions when we're in a terminal, or from defaults.
fn hire_form(
    name: Option<String>,
    preset: Option<String>,
    role: Option<String>,
    bio: Option<String>,
    runtime: String,
    model: Option<String>,
) -> Result<Employee> {
    let interactive = name.is_none() && std::io::stdin().is_terminal();
    let name = match name {
        Some(n) => n,
        None if interactive => loop {
            let n = ask("Name", "")?;
            if !n.is_empty() {
                break n;
            }
        },
        None => bail!("who are you hiring? For example: onecorp hire Maya --as engineer"),
    };
    if slug(&name).is_empty() {
        bail!("a name needs at least one letter or digit");
    }

    let keys: Vec<_> = PRESETS.iter().map(|p| p.key).collect();
    let preset = match preset {
        Some(p) => Some(p),
        None if interactive => Some(ask(&format!("Job ({})", keys.join(", ")), "engineer")?),
        None => None,
    };
    let preset = match preset {
        Some(p) => Some(
            PRESETS
                .iter()
                .find(|x| x.key == p.to_lowercase())
                .ok_or_else(|| anyhow::anyhow!("no preset called {p}. Pick one of: {}", keys.join(", ")))?,
        ),
        None => None,
    };

    let role = role.or(preset.map(|p| p.role.to_string())).unwrap_or_else(|| "Engineer".into());
    let default_bio = preset.map(|p| p.bio).unwrap_or(PRESETS[0].bio);
    let bio = match bio {
        Some(b) => b,
        None if interactive => ask("How do they work?", default_bio)?,
        None => default_bio.to_string(),
    };
    if !RUNTIMES.contains(&runtime.as_str()) {
        bail!("{runtime} isn't supported yet. Available: {}", RUNTIMES.join(", "));
    }
    let model = match model {
        Some(m) => Some(m),
        None if interactive => Some(ask("Model (blank for the runtime's default)", "")?).filter(|m| !m.is_empty()),
        None => None,
    };

    Ok(Employee { id: slug(&name), name, role, runtime, model, bio })
}

fn ask(label: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        print!("{label}: ");
    } else {
        print!("{label} [{default}]: ");
    }
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().lock().read_line(&mut line)?;
    let line = line.trim();
    Ok(if line.is_empty() { default.to_string() } else { line.to_string() })
}

fn assignee(c: &store::Company, to: Option<String>) -> Result<Employee> {
    if let Some(to) = to {
        return c.employee(&slug(&to))?.ok_or_else(|| anyhow::anyhow!("nobody called {to} works at {}. See: onecorp team", c.name));
    }
    let mut team = c.employees()?;
    match team.len() {
        0 => bail!("nobody works at {} yet. Hire someone first: onecorp hire", c.name),
        1 => Ok(team.remove(0)),
        _ => {
            let ids: Vec<_> = team.iter().map(|e| e.id.as_str()).collect();
            bail!("who should do it? Add --to with one of: {}", ids.join(", "))
        }
    }
}

/// Dollars when the runtime reports them, tokens otherwise (Codex on a subscription has no bill per run).
fn spend(cost: f64, tokens: i64) -> String {
    if cost > 0.0 || tokens == 0 { format!("${cost:.2}") } else { format!("{}k tok", (tokens + 500) / 1000) }
}

fn work(c: &store::Company, e: &Employee, title: &str) -> Result<()> {
    let id = c.add_ticket(title, &e.id)?;
    let key = c.key(id);
    let wt = c.dir.join("wt").join(key.to_lowercase());
    let branch = match git::add_worktree(&c.repo()?, &wt, &format!("onecorp/{}/{}", c.name, key.to_lowercase())) {
        Ok(b) => b,
        Err(err) => {
            c.finish_ticket(id, "failed", 0.0, 0)?;
            return Err(err);
        }
    };
    c.start_ticket(id, &branch, &wt)?;
    println!("{key}  {title}\n      {} is on it ({}), in {}\n", e.name, e.runtime, wt.display());

    let prompt = format!(
        "You are {name}, {role} at {company}. {bio}\n\n\
         Your ticket is {key}: {title}\n\n\
         You are in a git worktree on branch {branch}. {abilities} Don't commit: when you finish, \
         OneCorp commits your changes under your name. \
         End with two or three sentences: what you changed and anything the reviewer should check.\n\n\
         You have company tools. If a decision is the founder's to make, ask with ask_human: give a default and carry on, \
         or set blocking=true and stop if you truly can't continue. Use post_message for things teammates should know. \
         If something outside your control stops you, use update_ticket to mark the ticket blocked.",
        name = e.name,
        role = e.role,
        company = c.name,
        bio = e.bio,
        abilities = agent::abilities(&e.runtime),
    );
    drive(c, e, id, &wt, None, &prompt)
}

/// Picks a ticket's session back up, in the same worktree, with the founder's answer.
fn resume(c: &store::Company, q: &store::Question) -> Result<()> {
    let Some(t) = q.ticket_id.map(|id| c.ticket(id)).transpose()?.flatten() else { return Ok(()) };
    let key = c.key(t.id);
    let (Some(wt), Some(session)) = (&t.worktree, &t.session_id) else {
        bail!("{key} has no session to pick back up");
    };
    let Some(e) = c.employee(&q.employee)? else {
        bail!("{} no longer works at {}, so {key} stays {}", q.employee, c.name, t.status);
    };
    c.set_status(t.id, "doing")?;
    println!("{key}  {}\n      {} picks it back up with your answer\n", t.title, e.name);
    let prompt = format!(
        "The founder answered your question #{n}: \"{question}\"\n\nTheir answer: {answer}\n\n\
         Pick the ticket back up from where you left off and apply this answer. Same rules as before: \
         don't commit, and end with two or three sentences on what changed.",
        n = q.id,
        question = q.question,
        answer = q.answer.as_deref().unwrap_or(""),
    );
    drive(c, &e, t.id, std::path::Path::new(wt), Some(session), &prompt)
}

/// Runs one agent session on a ticket, logging every event, commits what it left, then reports where the ticket landed.
fn drive(c: &store::Company, e: &Employee, id: i64, wt: &std::path::Path, resume: Option<&str>, prompt: &str) -> Result<()> {
    let key = c.key(id);
    // The agent starts this same binary as the company's MCP server, scoped to this ticket.
    let mcp = agent::Mcp {
        command: std::env::current_exe()?.to_string_lossy().into_owned(),
        args: ["mcp", "--company", &c.name, "--ticket", &id.to_string(), "--employee", &e.id].map(String::from).to_vec(),
        env: vec![("ONECORP_HOME".into(), store::home().to_string_lossy().into_owned())],
    };
    let identity = git::identity(&e.name, &format!("{}@{}.onecorp", e.id, c.name));
    let opts = agent::Opts { cwd: wt, model: e.model.as_deref(), mcp: &mcp, resume, env: &identity };

    let wt_prefix = format!("{}/", wt.display());
    let mut outcome = None;
    let mut last_say = String::new();
    agent::run(&e.runtime, &opts, prompt, |raw, ev| {
        let (kind, summary) = match &ev {
            Event::Init { session, model } => {
                c.set_session(id, session)?;
                let verb = if resume.is_some() { "resumed" } else { "started" };
                // Codex doesn't say which model it picked, so fall back to what we asked for.
                let model = if model.is_empty() { e.model.as_deref().unwrap_or("default model") } else { model };
                ("start", format!("{} ({model}), {verb}", e.runtime))
            }
            Event::Say(text) => {
                last_say = text.clone();
                ("say", first_line(text))
            }
            Event::Tool { name, detail } => {
                let name = name.strip_prefix("mcp__onecorp__").unwrap_or(name);
                ("tool", first_line(&format!("{name} {}", detail.replace(&wt_prefix, ""))).trim_end().to_string())
            }
            Event::ToolResult { error: true, text } => ("error", text.clone()),
            Event::ToolResult { .. } => return Ok(()),
            Event::Done { ok, cost, tokens, text } => {
                outcome = Some((*ok, *cost, *tokens, text.clone()));
                let verb = if *ok { "finished" } else { "failed" };
                ("done", format!("{verb}, {}", spend(*cost, *tokens as i64)))
            }
        };
        c.add_event(id, kind, &summary, raw)?;
        println!("  {kind:<6} {summary}");
        Ok(())
    })
    .inspect_err(|_| {
        let _ = c.finish_ticket(id, "failed", 0.0, 0);
    })?;

    let (ok, cost, tokens, text) = outcome.unwrap_or_default();
    let text = if text.is_empty() { last_say } else { text };
    if ok {
        let t = c.ticket(id)?.ok_or_else(|| anyhow::anyhow!("{key} disappeared"))?;
        let msg: String = format!("{key}: {}", t.title).chars().take(72).collect();
        if let Some(hash) = git::commit_all(wt, &msg, &identity)? {
            c.add_event(id, "commit", &format!("{hash} {msg}"), "")?;
            println!("  commit {hash} as {}", e.name);
        }
    }
    c.finish_ticket(id, if ok { "review" } else { "failed" }, cost, tokens)?;

    let t = c.ticket(id)?.ok_or_else(|| anyhow::anyhow!("{key} disappeared"))?;
    let what = match t.status.as_str() {
        "review" => "ready for review".to_string(),
        "blocked" => "blocked. See what they need: onecorp inbox".to_string(),
        s => s.to_string(),
    };
    println!("\n{text}\n\n{key} is {what}");
    println!(
        "Branch {}, this run {}, ticket total {}.",
        t.branch.unwrap_or_default(),
        spend(cost, tokens as i64),
        spend(t.cost_usd, t.tokens)
    );
    let waiting = c.inbox()?.len();
    if waiting > 0 && t.status != "blocked" {
        let q = if waiting == 1 { "question" } else { "questions" };
        println!("{waiting} {q} in your inbox: onecorp inbox");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn slug_folds_accents_and_keeps_the_first_name() {
        assert_eq!(super::slug("Tomás Reis"), "tomas");
        assert_eq!(super::slug("  Inês "), "ines");
        assert_eq!(super::slug("JOÃO"), "joao");
        assert_eq!(super::slug("Maya-Okafor"), "mayaokafor");
        assert_eq!(super::slug("李"), "");
    }
}
