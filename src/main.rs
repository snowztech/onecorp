use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use onecorp::claude::{self, Event, first_line};
use onecorp::store::{self, Employee};
use onecorp::git;
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

const RUNTIMES: &[&str] = &["claude"];

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
                let rt = match &e.model {
                    Some(m) => format!("{} ({m})", e.runtime),
                    None => e.runtime.clone(),
                };
                let n = theirs.len();
                let plural = if n == 1 { "ticket" } else { "tickets" };
                println!("{:<10} {:<20} {:<18} {:<16} {n} {plural}, ${cost:.2}", e.id, e.name, e.role, rt);
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
            let total = tickets.iter().fold(0.0, |s, t| s + t.cost_usd);
            for t in &tickets {
                let who = t.assignee.as_deref().unwrap_or("-");
                println!("{:<8} {:<7} {:<10} ${:>6.2}  {}", c.key(t.id), t.status, who, t.cost_usd, t.title);
            }
            println!("{:<27} ${total:>6.2}", "total");
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
    }
    Ok(())
}

/// Ids are the lowercased first name: "Maya Okafor" becomes maya.
fn slug(name: &str) -> String {
    name.split_whitespace().next().unwrap_or("").chars().filter(char::is_ascii_alphanumeric).collect::<String>().to_lowercase()
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

fn work(c: &store::Company, e: &Employee, title: &str) -> Result<()> {
    let id = c.add_ticket(title, &e.id)?;
    let key = c.key(id);
    let wt = c.dir.join("wt").join(key.to_lowercase());
    let branch = match git::add_worktree(&c.repo()?, &wt, &format!("onecorp/{}/{}", c.name, key.to_lowercase())) {
        Ok(b) => b,
        Err(err) => {
            c.finish_ticket(id, "failed", 0.0)?;
            return Err(err);
        }
    };
    c.start_ticket(id, &branch, &wt)?;
    println!("{key}  {title}\n      {} is on it, in {}\n", e.name, wt.display());

    let prompt = format!(
        "You are {name}, {role} at {company}. {bio}\n\n\
         Your ticket is {key}: {title}\n\n\
         You are in a git worktree on branch {branch}. Do the work, then commit it with a clear one-line message. \
         You can't run commands other than git, so don't try to build or test. \
         End with two or three sentences: what you changed and anything the reviewer should check.",
        name = e.name,
        role = e.role,
        company = c.name,
        bio = e.bio,
    );

    let wt_prefix = format!("{}/", wt.display());
    let mut outcome = None;
    claude::run(&wt, e.model.as_deref(), &prompt, |raw, ev| {
        let (kind, summary) = match &ev {
            Event::Init { session, model } => {
                c.set_session(id, session)?;
                ("start", format!("Claude Code ({model})"))
            }
            Event::Say(text) => ("say", first_line(text)),
            Event::Tool { name, detail } => ("tool", first_line(&format!("{name} {}", detail.replace(&wt_prefix, ""))).trim_end().to_string()),
            Event::ToolResult { error: true, text } => ("error", text.clone()),
            Event::ToolResult { .. } => return Ok(()),
            Event::Done { ok, cost, turns, text } => {
                outcome = Some((*ok, *cost, text.clone()));
                ("done", format!("{} in {turns} turns, ${cost:.2}", if *ok { "finished" } else { "failed" }))
            }
        };
        c.add_event(id, kind, &summary, raw)?;
        println!("  {kind:<6} {summary}");
        Ok(())
    })
    .inspect_err(|_| {
        let _ = c.finish_ticket(id, "failed", 0.0);
    })?;

    let (ok, cost, text) = outcome.unwrap_or_default();
    c.finish_ticket(id, if ok { "review" } else { "failed" }, cost)?;
    println!("\n{text}\n\n{key} is {}. Branch {branch}, cost ${cost:.2}.", if ok { "ready for review" } else { "failed" });
    Ok(())
}
