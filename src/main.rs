use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use onecorp::agent::{RUNTIMES, first_line};
use onecorp::work::{self, Job, spend};
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
    /// Work queued tickets for every company, several employees at a time
    Serve {
        /// How many employees work at the same time
        #[arg(long, default_value_t = 3)]
        max: usize,
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
            let id = c.add_ticket(&title, &who.id)?;
            work::dispatch(&c, id, Job::Start)?;
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
                    0 => work::resume(&c, &q)?,
                    n => println!("{key} is still waiting on {n} more: onecorp inbox"),
                }
            } else if !q.blocking && status == "review" && !q.default_answer.eq_ignore_ascii_case(text) {
                // They went ahead with their default. A different answer means the work needs a change.
                let went_with = if q.default_answer.is_empty() { "their own call".to_string() } else { format!("\"{}\"", q.default_answer) };
                let who = c.employee(&q.employee)?.map(|e| e.name).unwrap_or_else(|| q.employee.clone());
                println!("{who} went with {went_with}, so {key} goes back to apply your answer.\n");
                work::resume(&c, &q)?;
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
            work::approve(&c, &find_ticket(&c, &ticket)?)?;
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
            work::dispatch(&c, t.id, Job::Resume { why: "with your feedback".into(), prompt, label: "changes from review".into() })?;
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
        Cmd::Serve { max } => onecorp::serve::serve(max)?,
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

fn find_ticket(c: &store::Company, key: &str) -> Result<store::Ticket> {
    c.ticket(c.parse_key(key)?)?.ok_or_else(|| anyhow::anyhow!("no ticket {key} in {}. See: onecorp status", c.name))
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
