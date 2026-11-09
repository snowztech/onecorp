use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use onecorp::claude::{self, Event, first_line};
use onecorp::{git, store};
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
    /// Open a ticket and have an engineer work on it now
    Run {
        /// What needs doing, in plain words
        task: Vec<String>,
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
        Cmd::Run { task, company } => {
            let title = task.join(" ");
            if title.trim().is_empty() {
                bail!("say what needs doing, for example: onecorp run \"add a --version flag\"");
            }
            work(&open(company)?, &title)?;
        }
        Cmd::Status { company } => {
            let c = open(company)?;
            let tickets = c.tickets()?;
            if tickets.is_empty() {
                println!("No tickets yet. Start one with: onecorp run \"<task>\"");
            }
            let mut total = 0.0;
            for t in &tickets {
                total += t.cost_usd;
                println!("{:<8} {:<7} ${:>6.2}  {}", c.key(t.id), t.status, t.cost_usd, t.title);
            }
            if !tickets.is_empty() {
                println!("{:<16} ${total:>6.2}", "total");
            }
        }
        Cmd::Logs { ticket, company } => {
            let c = open(company)?;
            let id = c.parse_key(&ticket)?;
            let Some(t) = c.ticket(id)? else { bail!("no ticket {ticket} in {}", c.name) };
            println!("{}  {}  [{}]  ${:.2}", c.key(t.id), t.title, t.status, t.cost_usd);
            if let Some(b) = t.branch {
                println!("branch {b}");
            }
            for (at, kind, summary) in c.events(id)? {
                println!("{at}  {kind:<6} {summary}");
            }
        }
    }
    Ok(())
}

fn work(c: &store::Company, title: &str) -> Result<()> {
    let id = c.add_ticket(title)?;
    let key = c.key(id);
    let branch = format!("onecorp/{}", key.to_lowercase());
    let wt = c.dir.join("wt").join(key.to_lowercase());
    git::add_worktree(&c.repo()?, &wt, &branch)?;
    c.start_ticket(id, &branch, &wt)?;
    println!("{key}  {title}\n      worktree {}\n", wt.display());

    let prompt = format!(
        "You are an engineer at {company}. Your ticket is {key}: {title}\n\n\
         You are in a git worktree on branch {branch}. Make the change with the smallest diff that does the job, \
         then commit it with a clear one-line message. You can't run commands other than git, so don't try to build or test. \
         End with two or three sentences: what you changed and anything the reviewer should check.",
        company = c.name
    );

    let wt_prefix = format!("{}/", wt.display());
    let mut outcome = None;
    claude::run(&wt, &prompt, |raw, ev| {
        let (kind, summary) = match &ev {
            Event::Init { session, model } => {
                c.set_session(id, session)?;
                ("start", format!("Claude Code ({model})"))
            }
            Event::Say(text) => ("say", first_line(text)),
            Event::Tool { name, detail } => ("tool", format!("{name} {}", detail.replace(&wt_prefix, "")).trim_end().to_string()),
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
