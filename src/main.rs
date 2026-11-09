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
        /// What the company is for. Goes at the top of the handbook
        #[arg(long)]
        mission: Option<String>,
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
    /// The handbook every employee reads before each ticket
    Handbook {
        #[command(subcommand)]
        action: Option<HandbookCmd>,
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
    /// Show what a ticket changed
    Diff {
        ticket: String,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Merge a reviewed ticket into the branch it started from
    Approve {
        ticket: String,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Send a ticket back with feedback. The same session picks it up
    Changes {
        ticket: String,
        /// What to change. Quote it if it has spaces
        #[arg(allow_hyphen_values = true)]
        feedback: String,
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

#[derive(Subcommand)]
enum HandbookCmd {
    /// Add a line, like "No ORMs" or "Tokens are hashed at rest"
    Add {
        text: String,
        /// Which section it goes under
        #[arg(long, default_value = "Decisions")]
        section: String,
    },
    /// Open it in $EDITOR
    Edit,
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
        Cmd::New { name, repo, mission } => {
            let c = store::Company::create(&name, &repo, mission.as_deref().unwrap_or(""))?;
            store::set_current(&name)?;
            println!("Created {} around {}. It's now your current company.", c.name, c.repo()?.display());
            println!("Its handbook is at {}. Everyone reads it before each ticket.", c.handbook_path().display());
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
            let t = find_ticket(&c, &ticket)?;
            let id = t.id;
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
        Cmd::Handbook { action, company } => {
            let c = open(company)?;
            match action {
                None => {
                    let doc = c.handbook()?;
                    if doc.trim().is_empty() {
                        println!("No handbook yet. Start one: onecorp handbook add \"<a decision everyone should know>\"");
                    } else {
                        print!("{doc}");
                    }
                }
                Some(HandbookCmd::Add { text, section }) => {
                    if text.trim().is_empty() {
                        bail!("what should everyone know? For example: onecorp handbook add \"No new dependencies without asking\"");
                    }
                    c.add_to_handbook(section.trim(), &text)?;
                    println!("Added to {}. Everyone sees it from their next ticket.", section.trim());
                }
                Some(HandbookCmd::Edit) => {
                    let path = c.handbook_path();
                    if !path.exists() {
                        c.add_to_handbook("Decisions", "")?;
                    }
                    // $EDITOR can carry flags, like "code -w".
                    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
                    let mut parts = editor.split_whitespace();
                    let program = parts.next().unwrap_or("vi");
                    std::process::Command::new(program).args(parts).arg(&path).status()?;
                }
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
                if q.kind == "handbook" {
                    println!("#{:<3} {ticket:<7} {}  [handbook]\n     Add to {}: {}", q.id, q.employee, q.options, q.question);
                    println!("     onecorp answer {} yes   (or no, or your own wording)\n", q.id);
                    continue;
                }
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
            if q.kind == "handbook" {
                match text.to_lowercase().as_str() {
                    "no" | "n" => println!("Left out of the handbook."),
                    yes => {
                        let line = if matches!(yes, "yes" | "y") { q.question.as_str() } else { text };
                        c.add_to_handbook(&q.options, line)?;
                        println!("Added to {}: {line}", q.options);
                    }
                }
                return Ok(());
            }
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
        Cmd::Diff { ticket, company } => {
            let c = open(company)?;
            let t = find_ticket(&c, &ticket)?;
            let Some(branch) = &t.branch else { bail!("{} hasn't started, so there's nothing to show", c.key(t.id)) };
            if t.status == "done" {
                println!("{} is merged. See it in your repo's history.", c.key(t.id));
                return Ok(());
            }
            let base = t.base.clone().unwrap_or_else(|| "HEAD".into());
            // Inherit the terminal so git's colors and pager work as usual.
            std::process::Command::new("git").arg("-C").arg(c.repo()?).args(["diff", &format!("{base}...{branch}")]).status()?;
        }
        Cmd::Approve { ticket, company } => {
            let c = open(company)?;
            approve(&c, &find_ticket(&c, &ticket)?)?;
        }
        Cmd::Changes { ticket, feedback, company } => {
            let c = open(company)?;
            let t = find_ticket(&c, &ticket)?;
            let key = c.key(t.id);
            if t.status != "review" {
                bail!("{key} is {}. You can only ask for changes on a ticket in review", t.status);
            }
            if feedback.trim().is_empty() {
                bail!("what should change? For example: onecorp changes {key} \"use a match instead of if/else\"");
            }
            let prompt = format!(
                "The founder reviewed your work and asked for changes:\n\n{}\n\n\
                 Make them. Same rules as before: don't commit, and end with two or three sentences on what changed.",
                feedback.trim()
            );
            c.add_event(t.id, "review", &format!("changes requested: {}", first_line(&feedback)), "")?;
            pick_up(&c, &t, "with your feedback", &prompt, "changes from review")?;
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

/// Cuts at a word boundary so commit subjects don't end mid-word.
fn short(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let cut: String = s.chars().take(max - 3).collect();
    format!("{}...", cut.rsplit_once(' ').map(|(a, _)| a).unwrap_or(&cut))
}

/// Dollars when the runtime reports them, tokens otherwise (Codex on a subscription has no bill per run).
fn spend(cost: f64, tokens: i64) -> String {
    if cost > 0.0 || tokens == 0 { format!("${cost:.2}") } else { format!("{}k tok", (tokens + 500) / 1000) }
}

fn work(c: &store::Company, e: &Employee, title: &str) -> Result<()> {
    let id = c.add_ticket(title, &e.id)?;
    let key = c.key(id);
    let wt = c.dir.join("wt").join(key.to_lowercase());
    let repo = c.repo()?;
    let started = git::current_branch(&repo).and_then(|base| {
        let branch = git::add_worktree(&repo, &wt, &format!("onecorp/{}/{}", c.name, key.to_lowercase()))?;
        Ok((branch, base))
    });
    let (branch, base) = match started {
        Ok(b) => b,
        Err(err) => {
            c.finish_ticket(id, "failed", 0.0, 0)?;
            return Err(err);
        }
    };
    c.start_ticket(id, &branch, &base, &wt)?;
    println!("{key}  {title}\n      {} is on it ({}), in {}\n", e.name, e.runtime, wt.display());

    let prompt = format!(
        "You are {name}, {role} at {company}. {bio}\n\n\
         Your ticket is {key}: {title}\n\n\
         You are in a git worktree on branch {branch}. {abilities} Don't commit: when you finish, \
         OneCorp commits your changes under your name. \
         End with two or three sentences: what you changed and anything the reviewer should check.\n\n\
         You have company tools. If a decision is the founder's to make, ask with ask_human: give a default and carry on, \
         or set blocking=true and stop if you truly can't continue. Use post_message for things teammates should know. \
         If something outside your control stops you, use update_ticket to mark the ticket blocked. \
         If you learn or decide something every teammate should know from now on, propose it with propose_handbook.\n\n\
         {knowledge}",
        name = e.name,
        role = e.role,
        company = c.name,
        bio = e.bio,
        abilities = agent::abilities(&e.runtime),
        knowledge = company_knowledge(c)?,
    );
    drive(c, e, id, &wt, None, &prompt, title)
}

/// What every employee should know before starting: the handbook, and who does what.
fn company_knowledge(c: &store::Company) -> Result<String> {
    // Keeps a runaway handbook from eating the whole context window.
    const MAX: usize = 12_000;
    let mut doc = c.handbook()?.trim().to_string();
    if doc.len() > MAX {
        let cut = (0..=MAX).rev().find(|&i| doc.is_char_boundary(i)).unwrap_or(0);
        doc.truncate(cut);
        doc.push_str("\n\n(The handbook goes on. Ask the founder if you need more.)");
    }
    let team: Vec<String> = c.employees()?.iter().map(|e| format!("- {}, {} (works on {})", e.name, e.role, e.runtime)).collect();
    let mut out = String::new();
    if !doc.is_empty() {
        out.push_str(&format!("The company handbook. Everyone reads it, so follow it:\n\n{doc}\n\n"));
    }
    out.push_str(&format!("Who works here:\n{}", team.join("\n")));
    Ok(out)
}

fn find_ticket(c: &store::Company, key: &str) -> Result<store::Ticket> {
    c.ticket(c.parse_key(key)?)?.ok_or_else(|| anyhow::anyhow!("no ticket {key} in {}. See: onecorp status", c.name))
}

/// Picks a ticket's session back up with the founder's answer.
fn resume(c: &store::Company, q: &store::Question) -> Result<()> {
    let Some(t) = q.ticket_id.map(|id| c.ticket(id)).transpose()?.flatten() else { return Ok(()) };
    let prompt = format!(
        "The founder answered your question #{n}: \"{question}\"\n\nTheir answer: {answer}\n\n\
         Pick the ticket back up from where you left off and apply this answer. Same rules as before: \
         don't commit, and end with two or three sentences on what changed.",
        n = q.id,
        question = q.question,
        answer = q.answer.as_deref().unwrap_or(""),
    );
    pick_up(c, &t, "with your answer", &prompt, &format!("apply answer to #{}", q.id))
}

/// Sends a ticket back to its employee: same session, same worktree, new instructions.
fn pick_up(c: &store::Company, t: &store::Ticket, why: &str, prompt: &str, label: &str) -> Result<()> {
    let key = c.key(t.id);
    let (Some(wt), Some(session)) = (&t.worktree, &t.session_id) else {
        bail!("{key} has no session to pick back up");
    };
    let who = t.assignee.as_deref().unwrap_or("");
    let Some(e) = c.employee(who)? else {
        bail!("{who} no longer works at {}, so {key} stays {}", c.name, t.status);
    };
    c.set_status(t.id, "doing")?;
    println!("{key}  {}\n      {} picks it back up {why}\n", t.title, e.name);
    drive(c, &e, t.id, std::path::Path::new(wt), Some(session), prompt, label)
}

/// Merges a reviewed ticket into its base branch. A conflict goes back to the employee to fix.
fn approve(c: &store::Company, t: &store::Ticket) -> Result<()> {
    let key = c.key(t.id);
    match t.status.as_str() {
        "review" => {}
        "done" => bail!("{key} is already merged"),
        s => bail!("{key} is {s}. Only tickets in review can be approved"),
    }
    let (Some(wt), Some(branch)) = (&t.worktree, &t.branch) else { bail!("{key} has no branch to merge") };
    let (repo, wt) = (c.repo()?, std::path::Path::new(wt));
    let base = match &t.base {
        Some(b) => b.clone(),
        None => git::current_branch(&repo)?,
    };
    let who = t.assignee.as_deref().and_then(|id| c.employee(id).ok().flatten());
    let identity = who.as_ref().map(|e| git::identity(&e.name, &format!("{}@{}.onecorp", e.id, c.name))).unwrap_or_default();

    if let git::Merge::Conflicts(files) = git::merge_base_into(wt, &base, &identity)? {
        let name = who.as_ref().map(|e| e.name.as_str()).unwrap_or("its owner");
        println!("{base} moved since {key} started, and these files conflict: {}", files.join(", "));
        println!("Sending it back to {name} to fix.\n");
        c.add_event(t.id, "merge", &format!("conflicts with {base}: {}", files.join(", ")), "")?;
        let prompt = format!(
            "The founder wants to merge your work, but {base} moved since you started and git found conflicts in: {files}.\n\n\
             The merge is in progress in your worktree. Open each file, resolve the conflict markers so both sides' intent is kept, \
             and remove every marker. Don't run git merge, git add or git commit: OneCorp finishes the merge when you're done. \
             End with two or three sentences on how you resolved each conflict.",
            files = files.join(", "),
        );
        pick_up(c, t, "to fix the conflicts", &prompt, "resolve conflicts")?;
        println!("Once you've checked it, approve again: onecorp approve {key}");
        return Ok(());
    }

    git::fast_forward(&repo, &base, branch)?;
    c.set_status(t.id, "done")?;
    c.add_event(t.id, "merge", &format!("merged into {base}"), "")?;
    // The work is safe on base now, so a failed cleanup is only worth a warning.
    if let Err(e) = git::clean_up(&repo, wt, branch) {
        eprintln!("warning: merged, but couldn't remove the worktree: {e:#}");
    }
    println!("Merged {key} into {base}. New tickets start from it.");
    Ok(())
}

/// Runs one agent session on a ticket, logging every event, commits what it left, then reports where the ticket landed.
/// `label` names this run in the commit message, like the ticket title or "changes from review".
fn drive(c: &store::Company, e: &Employee, id: i64, wt: &std::path::Path, resume: Option<&str>, prompt: &str, label: &str) -> Result<()> {
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
        let msg = short(&format!("{key}: {label}"), 72);
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
        let item = if waiting == 1 { "item" } else { "items" };
        println!("{waiting} {item} in your inbox: onecorp inbox");
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

    #[test]
    fn short_cuts_at_a_word() {
        assert_eq!(super::short("SNO-1: fix it", 72), "SNO-1: fix it");
        assert_eq!(super::short("SNO-1: change the description line", 24), "SNO-1: change the...");
    }
}
