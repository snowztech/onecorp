use anyhow::{Result, bail};
use clap::{Parser, Subcommand};
use onecorp::runtime::RUNTIMES;
use onecorp::work::{self, spend};
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
        /// The company's git repo. With --init, a new folder to start one in
        #[arg(long)]
        repo: PathBuf,
        /// Start a new repo there: git init, a README and a first commit
        #[arg(long)]
        init: bool,
        /// With --init, also create it on GitHub under this account or org, like snowztech
        #[arg(long, requires = "init")]
        github: Option<String>,
        /// With --github, make it public. Private by default
        #[arg(long, requires = "github")]
        public: bool,
        /// Start with a team: solo, small, demo (no AI, free) or empty
        #[arg(long)]
        team: Option<String>,
        /// Runtime for the starting team, like claude or codex
        #[arg(long, default_value = "claude")]
        runtime: String,
        /// What you're building. With a team, planning starts right away
        #[arg(long)]
        brief: Option<String>,
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
        /// What done looks like, links, constraints
        #[arg(short, long)]
        description: Option<String>,
        /// Save it to the backlog instead of starting now
        #[arg(long)]
        later: bool,
        /// Wait for another ticket to be merged first, like SNO-3
        #[arg(long)]
        after: Option<String>,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// What you're building, in your words. Everyone reads it, and planning starts from it
    Brief {
        /// Replace it with this text
        #[arg(long)]
        set: Option<String>,
        /// Open it in $EDITOR
        #[arg(long)]
        edit: bool,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Plan a project into tickets you review and start. With no name, plans the first version from the brief
    Plan {
        /// The project, like "Stripe billing"
        project: Option<String>,
        /// What the project should do
        #[arg(long)]
        goal: Option<String>,
        /// Who plans it. By default a product person, then a lead
        #[arg(long)]
        by: Option<String>,
        /// Write the brief and plan it in one go
        #[arg(long)]
        brief: Option<String>,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Throw a plan away, with its tickets
    Dismiss {
        plan: String,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Start a ticket from the backlog
    Start {
        ticket: String,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Give a ticket to someone
    Assign {
        ticket: String,
        employee: String,
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
    /// Stop a ticket that's running, or cancel one waiting in the queue
    Stop {
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
        /// Channel, like engineering or #eng. All channels when left out
        channel: Option<String>,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// Post in team chat. Anyone you @mention replies
    Say {
        /// Channel, like general or engineering
        channel: String,
        /// What to say, like "@maya how does login work?"
        #[arg(allow_hyphen_values = true, num_args = 1..)]
        text: Vec<String>,
        #[arg(short, long)]
        company: Option<String>,
    },
    /// You, the founder: your name and photo, shown in the app and known by your team
    Founder {
        #[arg(long)]
        name: Option<String>,
        /// A photo file: png, jpg, webp, gif or heic
        #[arg(long)]
        photo: Option<PathBuf>,
        /// Go back to the generated face
        #[arg(long, conflicts_with = "photo")]
        no_photo: bool,
        /// Forget your name and photo. Your companies stay
        #[arg(long, conflicts_with_all = ["name", "photo", "no_photo"])]
        forget: bool,
    },
    /// Delete a company and everything OneCorp kept for it. Your repo and its branches stay
    Delete {
        name: String,
        /// Don't ask to type the name first
        #[arg(long)]
        yes: bool,
    },
    /// How approved work lands: local merges, or pull requests on github or gitlab
    Ship {
        /// local, github or gitlab. Leave out to see the current one
        mode: Option<String>,
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
        Cmd::New { name, repo, init, github, public, mission, team, runtime, brief } => {
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
            let people = template.map(|t| t.people(&runtime)).unwrap_or_default();
            onecorp::templates::check(&people)?;
            let mission_text = mission.as_deref().unwrap_or("");
            let repo = if init {
                let full = github.as_ref().map(|owner| format!("{owner}/{}", onecorp::templates::suggest_name(&name)));
                let remote = full.as_deref().map(|f| onecorp::setup::Remote { full_name: f, private: !public });
                let made = onecorp::setup::create_repo(&repo, &name, mission_text, remote)?;
                match &full {
                    Some(f) => println!("Started a new repo at {}, and on GitHub as {f}.", made.display()),
                    None => println!("Started a new repo at {}.", made.display()),
                }
                made
            } else {
                repo
            };
            let (c, hired) = onecorp::setup::new_company(&name, &repo, mission_text, &people, brief.as_deref().unwrap_or(""))?;
            println!("Created {} around {}. It's now your current company.", c.name, c.repo()?.display());
            for e in &hired {
                println!("  hired {} as {} ({})", e.name, e.role, e.runtime);
            }
            match onecorp::ship::forge(&c)? {
                Some(f) => println!("Finished work opens {}s on {}. Change it with: onecorp ship local", f.pr_word(), f.name()),
                None => println!("Approved work merges into your local repo. To open pull requests instead: onecorp ship github"),
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
        Cmd::Run { task, to, description, later, after, company } => {
            let title = task.join(" ");
            if title.trim().is_empty() {
                bail!("say what needs doing, for example: onecorp run \"add a --version flag\"");
            }
            let c = open(company)?;
            // A ticket saved for later can wait for its owner too. One to start now needs one.
            let who = match (&to, later) {
                (None, true) => None,
                _ => Some(assignee(&c, to)?.id),
            };
            let after = after.map(|a| find_ticket(&c, &a).map(|t| t.id)).transpose()?;
            let new = store::NewTicket { title, description: description.unwrap_or_default(), assignee: who, after, ..Default::default() };
            let id = c.new_ticket(&new, "backlog")?;
            let key = c.key(id);
            if later {
                println!("{key} is in the backlog. Start it with: onecorp start {key}");
            } else {
                work::start(&c, &find_ticket(&c, &key)?)?;
            }
        }
        Cmd::Brief { set, edit, company } => {
            let c = open(company)?;
            if let Some(text) = set {
                c.write_brief(&text)?;
                println!("Brief saved. Everyone reads it from their next ticket. Plan it: onecorp plan");
            } else if edit {
                if !c.brief_path().exists() {
                    c.write_brief("What you're building, who it's for, and what the first version needs.")?;
                }
                open_in_editor(&c.brief_path())?;
            } else {
                match c.brief()? {
                    b if b.is_empty() => println!("No brief yet. Write one: onecorp brief --set \"A notes app for writers...\""),
                    b => println!("{b}"),
                }
            }
        }
        Cmd::Plan { project, goal, by, brief, company } => {
            let c = open(company)?;
            if let Some(text) = brief {
                c.write_brief(&text)?;
            }
            let id = match project {
                Some(name) => work::plan_project(&c, &name, goal.as_deref().unwrap_or(""), by.as_deref())?,
                None => work::kickoff(&c, by.as_deref())?,
            };
            let key = c.key(id);
            if c.ticket(id)?.is_some_and(|t| t.status == "review") {
                println!("\nThe plan is ready. See it: onecorp logs {key}, and its tickets: onecorp status");
                println!("Start it: onecorp approve {key}. Redo it: onecorp changes {key} \"...\". Drop it: onecorp dismiss {key}");
            }
        }
        Cmd::Dismiss { plan, company } => {
            let c = open(company)?;
            work::dismiss_plan(&c, &find_ticket(&c, &plan)?)?;
        }
        Cmd::Start { ticket, company } => {
            let c = open(company)?;
            work::start(&c, &find_ticket(&c, &ticket)?)?;
        }
        Cmd::Assign { ticket, employee, company } => {
            let c = open(company)?;
            let t = find_ticket(&c, &ticket)?;
            let e = c.employee(&slug(&employee))?.ok_or_else(|| anyhow::anyhow!("nobody called {employee} works at {}. See: onecorp team", c.name))?;
            if !["backlog", "todo", "waiting", "failed"].contains(&t.status.as_str()) {
                bail!("{} is {}, so it stays with whoever has it", c.key(t.id), t.status);
            }
            c.assign(t.id, &e.id)?;
            println!("{} is now {}'s.", c.key(t.id), e.name);
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
                let title = match (t.kind.as_str(), t.plan) {
                    ("plan", _) => format!("[plan] {} ({} tickets)", t.title, c.plan_tickets(t.id)?.len()),
                    (_, Some(p)) => format!("{}  (from {})", t.title, c.key(p)),
                    _ => t.title.clone(),
                };
                println!("{:<8} {:<7} {:<10} {:>8}  {title}", c.key(t.id), t.status, who, spend(t.cost_usd, t.tokens));
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
            onecorp::ship::approve(&c, &find_ticket(&c, &ticket)?)?;
        }
        Cmd::Stop { ticket, company } => {
            let c = open(company)?;
            work::stop(&c, &find_ticket(&c, &ticket)?)?;
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
            let channel = channel.map(|ch| onecorp::chat::resolve(&c, &ch)).transpose()?;
            let msgs = c.messages(channel.as_deref())?;
            if msgs.is_empty() {
                println!("Nothing posted yet.");
            }
            for m in msgs.iter().filter(|m| !m.pending) {
                let ticket = m.ticket.map(|t| format!("  ({})", c.key(t))).unwrap_or_default();
                println!("{} #{:<12} {}: {}{ticket}", m.at.get(11..16).unwrap_or(&m.at), m.channel, m.author, m.text);
            }
        }
        Cmd::Say { channel, text, company } => {
            let c = open(company)?;
            for id in onecorp::chat::say(&c, &channel, &text.join(" "))? {
                onecorp::chat::reply(&c, id)?;
                if let Some(m) = c.message(id)? {
                    println!("{}: {}", m.author, m.text);
                }
            }
        }
        Cmd::Serve { max } => onecorp::serve::serve(max)?,
        Cmd::Founder { forget: true, .. } => {
            onecorp::founder::remove()?;
            println!("Forgot your profile. The app asks who you are next time.");
        }
        Cmd::Delete { name, yes } => {
            let c = store::Company::open(&name)?;
            let tickets = c.tickets()?.len();
            drop(c);
            if !yes {
                println!("This deletes {name}: its team, {tickets} tickets, chat, handbook and memory. Your repo and its branches stay.");
                if ask(&format!("Type {name} to delete it"), "")? != name {
                    bail!("not deleted");
                }
            }
            onecorp::setup::delete_company(&name)?;
            println!("Deleted {name}.");
        }
        Cmd::Founder { name, photo, no_photo, .. } => {
            use onecorp::founder::Photo;
            let current = onecorp::founder::load();
            if name.is_some() || photo.is_some() || no_photo {
                let name = name.or_else(|| current.as_ref().map(|f| f.name.clone()));
                let Some(name) = name else { bail!("say what to call you first: onecorp founder --name Lucas") };
                let choice = match (&photo, no_photo) {
                    (Some(p), _) => Photo::Use(p),
                    (None, true) => Photo::Remove,
                    (None, false) => Photo::Keep,
                };
                let f = onecorp::founder::save(&name, choice)?;
                println!("You're {}, the founder.{}", f.name, if f.photo.is_some() { " Photo saved." } else { "" });
            } else {
                match current {
                    Some(f) => println!("{}, the founder{}", f.name, f.photo.map(|p| format!(". Photo: {}", p.display())).unwrap_or_default()),
                    None => println!("No founder profile yet. Set one: onecorp founder --name Lucas --photo ~/me.jpg"),
                }
            }
        }
        Cmd::Ship { mode, company } => {
            let c = open(company)?;
            if let Some(m) = mode {
                onecorp::ship::set_mode(&c, &m.to_lowercase())?;
            }
            match onecorp::ship::forge(&c)? {
                Some(f) => println!("{}: finished work opens a {} on {}, and approving merges it.", c.name, f.pr_word(), f.name()),
                None => println!("{}: approving merges into your local repo. Push it yourself when you want.", c.name),
            }
        }
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

