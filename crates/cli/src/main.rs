use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use onecorp::runtime::RUNTIMES;
use onecorp::work::{self, Job, spend};
use onecorp::store::{self, Employee, slug};
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
        /// Start with a team: solo, small, demo (no AI, free) or empty
        #[arg(long)]
        team: Option<String>,
        /// Runtime for the starting team, like claude or codex
        #[arg(long, default_value = "claude")]
        runtime: String,
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
        /// Department to join, like Engineering. Created if it doesn't exist yet
        #[arg(long)]
        dept: Option<String>,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Departments: list them, add one, set a lead, move someone
    Dept {
        #[command(subcommand)]
        action: Option<DeptCmd>,
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
    /// What an employee has learned. They read it before each ticket
    Memory {
        employee: String,
        /// Open it in $EDITOR
        #[arg(long)]
        edit: bool,
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
    /// Try a ticket that stopped or failed again
    Retry {
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
    /// Check that git, the agent CLIs and your companies are all set up, and how to fix what isn't
    Doctor,
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
enum DeptCmd {
    /// Add a department, like Engineering or Growth
    Add { name: String },
    /// Make someone the lead. Their team's questions reach them before you
    Lead { dept: String, employee: String },
    /// Move someone into a department
    Move { employee: String, dept: String },
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
        Cmd::New { name, repo, mission, team, runtime } => {
            let template = match team.as_deref() {
                Some(key) => Some(onecorp::templates::template(key).ok_or_else(|| {
                    let keys: Vec<_> = onecorp::templates::TEMPLATES.iter().map(|t| t.key).collect();
                    anyhow::anyhow!("no starting team called {key}. Pick one of: {}", keys.join(", "))
                })?),
                None => None,
            };
            if !RUNTIMES.contains(&runtime.as_str()) {
                bail!("{runtime} isn't supported yet. Available: {}", RUNTIMES.join(", "));
            }
            let c = store::Company::create(&name, &repo, mission.as_deref().unwrap_or(""))?;
            store::set_current(&name)?;
            println!("Created {} around {}. It's now your current company.", c.name, c.repo()?.display());
            let mut hired = vec![];
            if let Some(t) = template {
                hired = onecorp::templates::apply(&c, t, &runtime)?;
                for e in &hired {
                    println!("  hired {} as {} ({})", e.name, e.role, e.runtime);
                }
            }
            println!("Its handbook is at {}. Everyone reads it before each ticket.", c.handbook_path().display());
            match hired.first() {
                Some(e) => println!("Next, give someone a ticket: onecorp run \"<task>\" --to {}", e.id),
                None => println!("Next, hire someone: onecorp hire"),
            }
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
        Cmd::Hire { name, preset, role, bio, runtime, model, dept, company } => {
            let c = open(company)?;
            let mut e = hire_form(name, preset, role, bio, runtime, model)?;
            if let Some(d) = dept {
                e.department = Some(match c.department(&d)? {
                    Some(found) => found.id,
                    None => {
                        let made = c.add_department(&d)?;
                        println!("Created the {} department.", made.name);
                        made.id
                    }
                });
            }
            c.hire(&e)?;
            println!("Hired {} as {} at {}.", e.name, e.role, c.name);
            println!("Give them a ticket: onecorp run \"<task>\" --to {}", e.id);
        }
        Cmd::Dept { action, company } => {
            let c = open(company)?;
            let find = |name: &str| -> Result<store::Department> {
                c.department(name)?.ok_or_else(|| anyhow::anyhow!("no department called {name}. Add it: onecorp dept add \"{name}\""))
            };
            let person = |name: &str| -> Result<Employee> {
                c.employee(&slug(name))?.ok_or_else(|| anyhow::anyhow!("nobody called {name} works at {}. See: onecorp team", c.name))
            };
            match action {
                None => {
                    let team = c.employees()?;
                    let depts = c.departments()?;
                    if depts.is_empty() {
                        println!("No departments yet. Add one: onecorp dept add Engineering");
                    }
                    for d in &depts {
                        let members: Vec<&str> = team.iter().filter(|e| e.department == Some(d.id)).map(|e| e.name.as_str()).collect();
                        let lead = d.lead.as_deref().and_then(|l| team.iter().find(|e| e.id == l)).map(|e| e.name.as_str()).unwrap_or("nobody yet");
                        println!("{:<16} lead: {lead:<20} {}", d.name, members.join(", "));
                    }
                    let loose: Vec<&str> = team.iter().filter(|e| e.department.is_none()).map(|e| e.name.as_str()).collect();
                    if !loose.is_empty() {
                        println!("{:<16} {}", "(no department)", loose.join(", "));
                    }
                }
                Some(DeptCmd::Add { name }) => {
                    let d = c.add_department(&name)?;
                    println!("Added {}. Hire into it: onecorp hire --dept \"{}\"", d.name, d.name);
                }
                Some(DeptCmd::Lead { dept, employee }) => {
                    let (d, e) = (find(&dept)?, person(&employee)?);
                    if e.department != Some(d.id) {
                        c.set_department(&e.id, d.id)?;
                    }
                    c.set_lead(d.id, &e.id)?;
                    println!("{} now leads {}.", e.name, d.name);
                }
                Some(DeptCmd::Move { employee, dept }) => {
                    let (d, e) = (find(&dept)?, person(&employee)?);
                    c.set_department(&e.id, d.id)?;
                    println!("{} is now in {}.", e.name, d.name);
                }
            }
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
            for key in c.recover_interrupted()? {
                println!("{key} stopped before it finished, so it's marked failed. Retry it: onecorp retry {key}");
            }
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
        Cmd::Memory { employee, edit, company } => {
            let c = open(company)?;
            let id = slug(&employee);
            let Some(e) = c.employee(&id)? else { bail!("nobody called {employee} works at {}. See: onecorp team", c.name) };
            if edit {
                let path = c.memory_path(&id);
                std::fs::create_dir_all(path.parent().unwrap())?;
                open_in_editor(&path)?;
                return Ok(());
            }
            let notes = c.memory(&id)?;
            if notes.trim().is_empty() {
                println!("{} hasn't noted anything yet. Notes build up as they work and as you review them.", e.name);
            } else {
                print!("{notes}");
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
                    open_in_editor(&path)?;
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
                if let Some(via) = &q.via {
                    let lead = c.employee(via)?.map(|e| e.name).unwrap_or_else(|| via.clone());
                    println!("     {lead} passed it to you: {}", q.note.as_deref().unwrap_or(""));
                }
                println!();
            }
            let leads = c.with_leads()?;
            if !leads.is_empty() {
                let n = leads.len();
                println!("{n} more {} with department leads. You can answer them too: onecorp answer <number>", if n == 1 { "question is" } else { "questions are" });
                for q in leads {
                    println!("  #{:<3} {}: {}", q.id, q.employee, q.question);
                }
            }
        }
        Cmd::Answer { question, answer, company } => {
            let c = open(company)?;
            let id: i64 = question.trim_start_matches('#').parse().map_err(|_| anyhow::anyhow!("not a question number: {question}"))?;
            work::answer(&c, id, &answer)?;
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
        Cmd::Retry { ticket, company } => {
            let c = open(company)?;
            work::retry(&c, &find_ticket(&c, &ticket)?)?;
        }
        Cmd::Changes { ticket, feedback, company } => {
            let c = open(company)?;
            work::request_changes(&c, &find_ticket(&c, &ticket)?, &feedback)?;
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
        Cmd::Doctor => {
            use onecorp::doctor::Level;
            let checks = onecorp::doctor::run();
            for ch in &checks {
                let mark = match ch.level {
                    Level::Ok => "✓",
                    Level::Warn => "!",
                    Level::Fail => "✗",
                };
                println!("{mark} {:<20} {}", ch.what, ch.detail);
                if let (Some(fix), false) = (&ch.fix, ch.level == Level::Ok) {
                    println!("  {:<20} fix: {fix}", "");
                }
            }
            let broken = checks.iter().filter(|c| c.level == Level::Fail).count();
            println!();
            if broken == 0 {
                println!("All set.");
            } else {
                bail!("{broken} {} to fix", if broken == 1 { "thing" } else { "things" });
            }
        }
        Cmd::Mcp { company, ticket, employee } => {
            let c = store::Company::open(&company)?;
            onecorp::mcp::serve(&c, &onecorp::mcp::Ctx { ticket, employee })?;
        }
    }
    Ok(())
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

    let keys: Vec<_> = onecorp::templates::PRESETS.iter().map(|p| p.key).collect();
    let preset = match preset {
        Some(p) => Some(p),
        None if interactive => Some(ask(&format!("Job ({})", keys.join(", ")), "engineer")?),
        None => None,
    };
    let preset = match preset {
        Some(p) => Some(
            onecorp::templates::PRESETS
                .iter()
                .find(|x| x.key == p.to_lowercase())
                .ok_or_else(|| anyhow::anyhow!("no preset called {p}. Pick one of: {}", keys.join(", ")))?,
        ),
        None => None,
    };

    let role = role.or(preset.map(|p| p.role.to_string())).unwrap_or_else(|| "Engineer".into());
    let default_bio = preset.map(|p| p.bio).unwrap_or(onecorp::templates::PRESETS[0].bio);
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

    Ok(Employee { id: slug(&name), name, role, runtime, model, bio, department: None })
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

fn open_in_editor(path: &std::path::Path) -> Result<()> {
    // $EDITOR can carry flags, like "code -w".
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".into());
    let mut parts = editor.split_whitespace();
    let program = parts.next().unwrap_or("vi");
    std::process::Command::new(program).args(parts).arg(path).status()?;
    Ok(())
}

fn find_ticket(c: &store::Company, key: &str) -> Result<store::Ticket> {
    c.ticket(c.parse_key(key)?)?.ok_or_else(|| anyhow::anyhow!("no ticket {key} in {}. See: onecorp status", c.name))
}

