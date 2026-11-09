use crate::work::Job;
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS spend (
  id INTEGER PRIMARY KEY,
  at TEXT NOT NULL DEFAULT (datetime('now')),
  employee TEXT NOT NULL,
  ticket_id INTEGER,
  runtime TEXT NOT NULL,
  model TEXT,
  cost_usd REAL NOT NULL,
  tokens INTEGER NOT NULL,
  estimated INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS employees (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  role TEXT NOT NULL,
  runtime TEXT NOT NULL,
  model TEXT,
  bio TEXT NOT NULL DEFAULT '',
  hired_at TEXT NOT NULL DEFAULT (datetime('now')),
  fired_at TEXT,
  department INTEGER REFERENCES departments(id)
);
CREATE TABLE IF NOT EXISTS departments (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL UNIQUE COLLATE NOCASE,
  lead TEXT REFERENCES employees(id),
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS tickets (
  -- AUTOINCREMENT: a removed ticket's key is never handed to a new one.
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  title TEXT NOT NULL,
  description TEXT NOT NULL DEFAULT '',
  created_by TEXT NOT NULL DEFAULT 'founder',
  after INTEGER REFERENCES tickets(id),
  kind TEXT NOT NULL DEFAULT 'work',
  plan INTEGER REFERENCES tickets(id),
  assignee TEXT REFERENCES employees(id),
  status TEXT NOT NULL DEFAULT 'todo',
  branch TEXT,
  base TEXT,
  worktree TEXT,
  session_id TEXT,
  cost_usd REAL NOT NULL DEFAULT 0,
  tokens INTEGER NOT NULL DEFAULT 0,
  agent_pid INTEGER,
  pr_number INTEGER,
  pr_url TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS events (
  id INTEGER PRIMARY KEY,
  ticket_id INTEGER NOT NULL REFERENCES tickets(id),
  at TEXT NOT NULL DEFAULT (datetime('now')),
  kind TEXT NOT NULL,
  summary TEXT NOT NULL,
  raw TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS inbox (
  id INTEGER PRIMARY KEY,
  ticket_id INTEGER REFERENCES tickets(id),
  employee TEXT NOT NULL,
  question TEXT NOT NULL,
  options TEXT NOT NULL DEFAULT '',
  default_answer TEXT NOT NULL DEFAULT '',
  blocking INTEGER NOT NULL DEFAULT 0,
  kind TEXT NOT NULL DEFAULT 'question',
  stage TEXT NOT NULL DEFAULT 'founder',
  via TEXT,
  note TEXT,
  answered_by TEXT,
  answer TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  answered_at TEXT
);
CREATE TABLE IF NOT EXISTS jobs (
  id INTEGER PRIMARY KEY,
  ticket_id INTEGER NOT NULL REFERENCES tickets(id),
  kind TEXT NOT NULL,
  why TEXT NOT NULL DEFAULT '',
  prompt TEXT NOT NULL DEFAULT '',
  label TEXT NOT NULL DEFAULT '',
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  started_at TEXT,
  finished_at TEXT,
  error TEXT,
  pid INTEGER
);
CREATE TABLE IF NOT EXISTS messages (
  id INTEGER PRIMARY KEY,
  channel TEXT NOT NULL,
  author TEXT NOT NULL,
  text TEXT NOT NULL,
  ticket_id INTEGER REFERENCES tickets(id),
  at TEXT NOT NULL DEFAULT (datetime('now'))
);
";

/// Ids are the lowercased first name: "Maya Okafor" becomes maya.
/// Accents fold to plain letters so "Tomás" becomes tomas, which is what people will type.
pub fn slug(name: &str) -> String {
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

/// Everything lives under ~/.onecorp, or $ONECORP_HOME when set (handy for tests).
/// A ledger row for reports: (local day, employee, runtime, plan, dollars, tokens, estimated).
pub type SpendRow = (String, String, String, Option<i64>, f64, i64, bool);

/// One run in the spend ledger: who, for which ticket, on what, and what it cost.
pub struct Spent<'a> {
    pub employee: &'a str,
    pub ticket: Option<i64>,
    pub runtime: &'a str,
    pub model: Option<&'a str>,
    pub cost: f64,
    pub tokens: u64,
    /// Priced from tokens, because the agent didn't report a cost.
    pub estimated: bool,
}

/// The wiki section that says what the company is building.
pub const BUILDING: &str = "What we're building";

pub fn home() -> PathBuf {
    match std::env::var("ONECORP_HOME") {
        Ok(h) => PathBuf::from(h),
        Err(_) => PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".onecorp"),
    }
}

pub fn current() -> Result<String> {
    let name = fs::read_to_string(home().join("current"))
        .context("no company selected. Run: onecorp new <name> --repo <path>")?;
    Ok(name.trim().to_string())
}

pub fn set_current(name: &str) -> Result<()> {
    fs::create_dir_all(home())?;
    fs::write(home().join("current"), name)?;
    Ok(())
}

pub fn list() -> Result<Vec<String>> {
    let Ok(dir) = fs::read_dir(home().join("companies")) else { return Ok(vec![]) };
    let mut names: Vec<String> = dir.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort();
    Ok(names)
}

#[derive(Clone, Debug)]
pub struct Employee {
    pub id: String,
    pub name: String,
    pub role: String,
    pub runtime: String,
    pub model: Option<String>,
    pub bio: String,
    pub department: Option<i64>,
}

pub struct Department {
    pub id: i64,
    pub name: String,
    pub lead: Option<String>,
}

/// (time, ticket, employee, kind, summary, ticket title)
pub type FeedEvent = (String, i64, String, String, String, String);

/// A chat message. `author` is an employee id, or "founder".
#[derive(Clone, Debug)]
pub struct Message {
    pub id: i64,
    pub at: String,
    pub channel: String,
    pub author: String,
    pub text: String,
    pub ticket: Option<i64>,
    /// A reply someone is still writing.
    pub pending: bool,
}

/// A job serve has claimed and is about to run.
pub struct Claimed {
    pub id: i64,
    pub ticket: i64,
    pub job: Job,
}

#[derive(Default)]
pub struct Question {
    pub id: i64,
    pub ticket_id: Option<i64>,
    pub employee: String,
    pub question: String,
    /// Comma separated, empty when the question is open-ended.
    pub options: String,
    pub default_answer: String,
    pub blocking: bool,
    pub answer: Option<String>,
    /// "question", or "wiki" for a proposed wiki addition (then `options` holds the section).
    pub kind: String,
    /// "lead" while the asker's department lead looks at it, "founder" once it's yours.
    pub stage: String,
    /// The lead who passed it up to you, and why.
    pub via: Option<String>,
    pub note: Option<String>,
    /// Who answered: "founder", or the id of the lead who decided.
    pub answered_by: Option<String>,
}

pub struct Ticket {
    pub id: i64,
    pub title: String,
    pub assignee: Option<String>,
    pub status: String,
    pub branch: Option<String>,
    pub cost_usd: f64,
    pub tokens: i64,
    pub worktree: Option<String>,
    pub session_id: Option<String>,
    /// The branch the ticket started from, and where it lands when approved.
    pub base: Option<String>,
    /// Its pull request, once it has one.
    pub pr_number: Option<i64>,
    pub pr_url: Option<String>,
    /// The details beyond the title: what done looks like, links, constraints.
    pub description: String,
    /// "founder", or the id of the employee who created it.
    pub created_by: String,
    /// Another ticket this one waits for: it starts once that one is merged.
    pub after: Option<i64>,
    /// "work" for code, "plan" for a planning ticket whose result is other tickets.
    pub kind: String,
    /// The plan ticket this one came from, if any.
    pub plan: Option<i64>,
}

/// A ticket to create. Everything but the title is optional.
#[derive(Default)]
pub struct NewTicket {
    pub title: String,
    pub description: String,
    pub assignee: Option<String>,
    pub created_by: Option<String>,
    pub after: Option<i64>,
    pub plan: Option<i64>,
}

pub struct Company {
    pub name: String,
    pub dir: PathBuf,
    db: Connection,
}

impl Company {
    pub fn create(name: &str, repo: &Path, mission: &str) -> Result<Company> {
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            bail!("company name can only use letters, digits, - and _");
        }
        let dir = home().join("companies").join(name);
        if dir.exists() {
            bail!("company {name} already exists");
        }
        let repo = repo.canonicalize().with_context(|| format!("repo not found: {}", repo.display()))?;
        if !repo.join(".git").exists() {
            bail!("{} is not a git repo", repo.display());
        }
        fs::create_dir_all(&dir)?;
        let c = Self::open_dir(name, dir)?;
        c.db.execute("INSERT INTO meta (key, value) VALUES ('repo', ?1)", [repo.to_string_lossy()])?;
        let mission = if mission.trim().is_empty() { "What the company is for, in a sentence or two." } else { mission.trim() };
        fs::write(c.wiki_path(), format!("# {name} wiki\n\n## Mission\n\n{mission}\n\n## Decisions\n\n## Conventions\n"))?;
        Ok(c)
    }

    pub fn open(name: &str) -> Result<Company> {
        let dir = home().join("companies").join(name);
        if !dir.exists() {
            bail!("no company named {name}. Create it with: onecorp new {name} --repo <path>");
        }
        Self::open_dir(name, dir)
    }

    pub(crate) fn open_dir(name: &str, dir: PathBuf) -> Result<Company> {
        let db = Connection::open(dir.join("company.db"))?;
        // The CLI and the agent's MCP server write at the same time. WAL plus a busy timeout
        // lets them take turns instead of failing with "database is locked".
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode = WAL;")?;
        db.execute_batch(SCHEMA)?;
        migrate(&db)?;
        Ok(Company { name: name.to_string(), dir, db })
    }

    /// How approved work lands: "local", "github" or "gitlab". See ship.rs.
    pub fn ship_mode(&self) -> Result<String> {
        Ok(self.db.query_row("SELECT value FROM meta WHERE key = 'ship'", [], |r| r.get(0)).optional()?.unwrap_or_else(|| "local".into()))
    }

    /// The company's look: a badge color, and a logo image if you added one.
    pub fn look(&self) -> Result<(String, Option<PathBuf>)> {
        let color = self.db.query_row("SELECT value FROM meta WHERE key = 'color'", [], |r| r.get(0)).optional()?.unwrap_or_else(|| "#2b55c9".to_string());
        let logo = fs::read_dir(&self.dir).ok().and_then(|d| d.flatten().map(|e| e.path()).find(|p| p.file_stem().is_some_and(|s| s == "logo")));
        Ok((color, logo))
    }

    /// Sets the badge color, and with `logo` a new image (copied in, so the original can move) or
    /// None to go back to the letter badge. `keep_logo` leaves the logo as it is.
    pub fn set_look(&self, color: &str, logo: Option<&Path>, keep_logo: bool) -> Result<()> {
        if !(color.len() == 7 && color.starts_with('#') && color[1..].chars().all(|c| c.is_ascii_hexdigit())) {
            bail!("a color looks like #2b55c9");
        }
        if !keep_logo {
            let new = match logo {
                Some(src) => {
                    let ext = src.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                    if !["png", "jpg", "jpeg", "webp", "svg", "gif"].contains(&ext.as_str()) {
                        bail!("use an image for the logo: png, jpg, webp, svg or gif");
                    }
                    if fs::metadata(src).with_context(|| format!("couldn't read {}", src.display()))?.len() > 4 * 1024 * 1024 {
                        bail!("that logo is over 4 MB. Pick a smaller one");
                    }
                    let tmp = self.dir.join(format!("logo-new.{ext}"));
                    fs::copy(src, &tmp)?;
                    Some((tmp, self.dir.join(format!("logo.{ext}"))))
                }
                None => None,
            };
            // The old logo goes only once the new one is safely copied.
            if let (_, Some(old)) = self.look()? {
                fs::remove_file(old)?;
            }
            if let Some((tmp, dest)) = new {
                fs::rename(tmp, dest)?;
            }
        }
        self.db.execute("INSERT INTO meta (key, value) VALUES ('color', ?1) ON CONFLICT (key) DO UPDATE SET value = ?1", [color])?;
        Ok(())
    }

    /// The local date `n` days ago, from the same clock as the spend ledger.
    pub fn spent_day(&self, n: i64) -> Result<String> {
        Ok(self.db.query_row("SELECT date('now', 'localtime', '-' || ?1 || ' days')", [n], |r| r.get(0))?)
    }

    /// Today's date in local time, from the same clock as the spend ledger.
    pub fn today(&self) -> Result<String> {
        Ok(self.db.query_row("SELECT date('now', 'localtime')", [], |r| r.get(0))?)
    }

    /// A company setting, like its budget. None when it was never set.
    pub fn setting(&self, key: &str) -> Result<Option<String>> {
        Ok(self.db.query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0)).optional()?)
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.db.execute("INSERT INTO meta (key, value) VALUES (?1, ?2) ON CONFLICT (key) DO UPDATE SET value = ?2", [key, value])?;
        Ok(())
    }

    /// One run's cost, in the spend ledger.
    pub fn add_spend(&self, s: &Spent) -> Result<()> {
        self.db.execute(
            "INSERT INTO spend (employee, ticket_id, runtime, model, cost_usd, tokens, estimated) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![s.employee, s.ticket, s.runtime, s.model, s.cost, s.tokens as i64, s.estimated],
        )?;
        Ok(())
    }

    /// Spend since `days` ago, local time, today being 0: (dollars, tokens), for one person or everyone.
    pub fn spent_since(&self, days: i64, employee: Option<&str>) -> Result<(f64, i64)> {
        Ok(self.db.query_row(
            "SELECT coalesce(sum(cost_usd), 0), coalesce(sum(tokens), 0) FROM spend
             WHERE date(at, 'localtime') >= date('now', 'localtime', '-' || ?1 || ' days') AND (?2 IS NULL OR employee = ?2)",
            params![days, employee],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }

    pub fn spend_rows(&self) -> Result<Vec<SpendRow>> {
        let mut st = self.db.prepare(
            "SELECT date(s.at, 'localtime'), s.employee, s.runtime, t.plan, s.cost_usd, s.tokens, s.estimated
             FROM spend s LEFT JOIN tickets t ON t.id = s.ticket_id ORDER BY s.id",
        )?;
        let rows = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn daily_cap(&self, employee: &str) -> Result<f64> {
        Ok(self.db.query_row("SELECT daily_cap FROM employees WHERE id = ?1", [employee], |r| r.get(0)).optional()?.unwrap_or(0.0))
    }

    pub fn set_daily_cap(&self, employee: &str, dollars: f64) -> Result<()> {
        if dollars.is_nan() || dollars < 0.0 {
            bail!("a cap is a positive amount, or 0 for none");
        }
        self.db.execute("UPDATE employees SET daily_cap = ?2 WHERE id = ?1", params![employee, dollars])?;
        Ok(())
    }

    /// A paused company starts nothing new: no tickets, no lead consults, no chat replies. Work
    /// already running finishes. Your off switch for spending.
    pub fn paused(&self) -> Result<bool> {
        Ok(self.db.query_row("SELECT value FROM meta WHERE key = 'paused'", [], |r| r.get::<_, String>(0)).optional()?.is_some_and(|v| v == "1"))
    }

    pub fn set_paused(&self, paused: bool) -> Result<()> {
        self.db.execute(
            "INSERT INTO meta (key, value) VALUES ('paused', ?1) ON CONFLICT (key) DO UPDATE SET value = ?1",
            [if paused { "1" } else { "0" }],
        )?;
        Ok(())
    }

    pub fn set_ship_mode(&self, mode: &str) -> Result<()> {
        self.db.execute("INSERT INTO meta (key, value) VALUES ('ship', ?1) ON CONFLICT (key) DO UPDATE SET value = ?1", [mode])?;
        Ok(())
    }

    pub fn set_pr(&self, ticket: i64, number: i64, url: &str) -> Result<()> {
        self.db.execute("UPDATE tickets SET pr_number = ?2, pr_url = ?3 WHERE id = ?1", params![ticket, number, url])?;
        Ok(())
    }

    pub fn repo(&self) -> Result<PathBuf> {
        let repo: String = self.db.query_row("SELECT value FROM meta WHERE key = 'repo'", [], |r| r.get(0))?;
        Ok(PathBuf::from(repo))
    }

    /// Ticket keys look like SNO-12: the first three letters of the company name.
    pub fn key(&self, id: i64) -> String {
        let prefix: String = self.name.chars().filter(char::is_ascii_alphanumeric).take(3).collect();
        format!("{}-{id}", prefix.to_ascii_uppercase())
    }

    /// Accepts "SNO-12" or "12".
    pub fn parse_key(&self, key: &str) -> Result<i64> {
        let num = key.rsplit('-').next().unwrap_or(key);
        num.parse().with_context(|| format!("not a ticket: {key}"))
    }

    /// Hiring someone who was fired before brings their record back with the new details.
    pub fn hire(&self, e: &Employee) -> Result<()> {
        if self.employee(&e.id)?.is_some() {
            bail!("{} already works at {}", e.id, self.name);
        }
        self.db.execute(
            "INSERT INTO employees (id, name, role, runtime, model, bio, department) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT (id) DO UPDATE SET name = ?2, role = ?3, runtime = ?4, model = ?5, bio = ?6, department = ?7,
               fired_at = NULL, hired_at = datetime('now')",
            params![e.id, e.name, e.role, e.runtime, e.model, e.bio, e.department],
        )?;
        // The first person in a department leads it.
        if let Some(d) = e.department {
            self.db.execute("UPDATE departments SET lead = ?2 WHERE id = ?1 AND lead IS NULL", params![d, e.id])?;
        }
        Ok(())
    }

    pub fn add_department(&self, name: &str) -> Result<Department> {
        let name = name.trim();
        if name.is_empty() {
            bail!("a department needs a name, like Engineering");
        }
        if let Some(d) = self.department(name)? {
            bail!("there's already a department called {}", d.name);
        }
        self.db.execute("INSERT INTO departments (name) VALUES (?1)", [name])?;
        Ok(Department { id: self.db.last_insert_rowid(), name: name.to_string(), lead: None })
    }

    /// Finds a department by name, ignoring case.
    pub fn department(&self, name: &str) -> Result<Option<Department>> {
        Ok(self
            .db
            .query_row("SELECT id, name, lead FROM departments WHERE name = ?1 COLLATE NOCASE", [name.trim()], |r| {
                Ok(Department { id: r.get(0)?, name: r.get(1)?, lead: r.get(2)? })
            })
            .optional()?)
    }

    pub fn departments(&self) -> Result<Vec<Department>> {
        let mut st = self.db.prepare("SELECT id, name, lead FROM departments ORDER BY id")?;
        let rows = st.query_map([], |r| Ok(Department { id: r.get(0)?, name: r.get(1)?, lead: r.get(2)? }))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Moves someone into a department. They lead it if it had no lead yet.
    pub fn set_department(&self, employee: &str, department: i64) -> Result<()> {
        self.db.execute("UPDATE employees SET department = ?2 WHERE id = ?1", params![employee, department])?;
        self.db.execute("UPDATE departments SET lead = ?2 WHERE id = ?1 AND lead IS NULL", params![department, employee])?;
        Ok(())
    }

    pub fn set_lead(&self, department: i64, employee: &str) -> Result<()> {
        self.db.execute("UPDATE departments SET lead = ?2 WHERE id = ?1", params![department, employee])?;
        Ok(())
    }

    /// Fired employees keep their row, so old tickets still say who did them.
    pub fn fire(&self, id: &str) -> Result<()> {
        let busy: i64 =
            self.db.query_row("SELECT count(*) FROM tickets WHERE assignee = ?1 AND status = 'doing'", [id], |r| r.get(0))?;
        if busy > 0 {
            bail!("{id} is working on a ticket right now. Wait for it to finish first");
        }
        let n = self.db.execute("UPDATE employees SET fired_at = datetime('now') WHERE id = ?1 AND fired_at IS NULL", [id])?;
        if n == 0 {
            bail!("nobody called {id} works at {}", self.name);
        }
        // A department they led passes to whoever joined it next, or waits for a new lead.
        self.db.execute(
            "UPDATE departments SET lead = (SELECT e.id FROM employees e WHERE e.department = departments.id AND e.fired_at IS NULL ORDER BY e.hired_at LIMIT 1)
             WHERE lead = ?1",
            [id],
        )?;
        Ok(())
    }

    /// How someone looks: the seed their drawn face comes from (their name unless you picked
    /// another), and their photo if you added one.
    pub fn avatar(&self, id: &str) -> Result<(Option<String>, Option<PathBuf>)> {
        let seed = self.db.query_row("SELECT face FROM employees WHERE id = ?1", [id], |r| r.get(0)).optional()?.flatten();
        let photo = fs::read_dir(self.dir.join("avatars")).ok().and_then(|d| d.flatten().map(|e| e.path()).find(|p| p.file_stem().is_some_and(|s| s == id)));
        Ok((seed, photo))
    }

    pub fn set_face(&self, id: &str, seed: Option<&str>) -> Result<()> {
        self.db.execute("UPDATE employees SET face = ?2 WHERE id = ?1", params![id, seed.filter(|s| !s.trim().is_empty())])?;
        Ok(())
    }

    /// A photo for someone, copied in, or None to go back to their drawn face.
    pub fn set_photo(&self, id: &str, photo: Option<&Path>) -> Result<()> {
        let dir = self.dir.join("avatars");
        let new = match photo {
            Some(src) => {
                let ext = src.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
                if !["png", "jpg", "jpeg", "webp", "gif"].contains(&ext.as_str()) {
                    bail!("use a photo: png, jpg, webp or gif");
                }
                if fs::metadata(src).with_context(|| format!("couldn't read {}", src.display()))?.len() > 4 * 1024 * 1024 {
                    bail!("that photo is over 4 MB. Pick a smaller one");
                }
                fs::create_dir_all(&dir)?;
                let tmp = dir.join(format!("{id}-new.{ext}"));
                fs::copy(src, &tmp)?;
                Some((tmp, dir.join(format!("{id}.{ext}"))))
            }
            None => None,
        };
        // The old photo goes only once the new one is safely copied.
        if let (_, Some(old)) = self.avatar(id)? {
            fs::remove_file(old)?;
        }
        if let Some((tmp, dest)) = new {
            fs::rename(tmp, dest)?;
        }
        Ok(())
    }

    /// Changes how someone works: their name as shown, role, runtime, model and bio. Their id stays,
    /// so their tickets, memory and git identity follow them.
    pub fn update_employee(&self, e: &Employee) -> Result<()> {
        if e.name.trim().is_empty() {
            bail!("everyone needs a name");
        }
        let n = self.db.execute(
            "UPDATE employees SET name = ?2, role = ?3, runtime = ?4, model = ?5, bio = ?6 WHERE id = ?1 AND fired_at IS NULL",
            params![e.id, e.name.trim(), e.role.trim(), e.runtime, e.model, e.bio],
        )?;
        if n == 0 {
            bail!("nobody called {} works at {}", e.id, self.name);
        }
        Ok(())
    }

    pub fn employee(&self, id: &str) -> Result<Option<Employee>> {
        Ok(self
            .db
            .query_row(
                "SELECT id, name, role, runtime, model, bio, department FROM employees WHERE id = ?1 AND fired_at IS NULL",
                [id],
                row_to_employee,
            )
            .optional()?)
    }

    pub fn employees(&self) -> Result<Vec<Employee>> {
        let mut st = self
            .db
            .prepare("SELECT id, name, role, runtime, model, bio, department FROM employees WHERE fired_at IS NULL ORDER BY hired_at")?;
        let rows = st.query_map([], row_to_employee)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// A ticket to start now, with just a title and who does it.
    pub fn add_ticket(&self, title: &str, assignee: &str) -> Result<i64> {
        self.new_ticket(&NewTicket { title: title.into(), assignee: Some(assignee.into()), ..Default::default() }, "todo")
    }

    /// Creates a ticket in `status`: "backlog" to save it for later, "todo" to start it.
    pub fn new_ticket(&self, t: &NewTicket, status: &str) -> Result<i64> {
        let title = t.title.trim();
        if title.is_empty() {
            bail!("a ticket needs a title: say what needs doing");
        }
        if let Some(who) = &t.assignee
            && self.employee(who)?.is_none()
        {
            bail!("nobody called {who} works at {}", self.name);
        }
        if let Some(after) = t.after
            && self.ticket(after)?.is_none()
        {
            bail!("there's no ticket {} to wait for", self.key(after));
        }
        self.db.execute(
            "INSERT INTO tickets (title, description, assignee, created_by, after, plan, status) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![title, t.description.trim(), t.assignee, t.created_by.as_deref().unwrap_or("founder"), t.after, t.plan, status],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    /// A planning ticket: its work is reading the brief and the repo and creating tickets.
    pub fn set_kind(&self, id: i64, kind: &str) -> Result<()> {
        self.db.execute("UPDATE tickets SET kind = ?2 WHERE id = ?1", params![id, kind])?;
        Ok(())
    }

    /// A project: a plan ticket whose title names it and whose description says what it should do.
    pub fn new_plan(&self, title: &str, goal: &str, planner: &str) -> Result<i64> {
        self.db.execute("INSERT INTO tickets (title, description, assignee, kind, status) VALUES (?1, ?2, ?3, 'plan', 'todo')", params![title, goal, planner])?;
        Ok(self.db.last_insert_rowid())
    }

    /// The tickets a plan created, oldest first.
    pub fn plan_tickets(&self, plan: i64) -> Result<Vec<Ticket>> {
        Ok(self.tickets()?.into_iter().filter(|t| t.plan == Some(plan)).collect())
    }

    /// Deletes a ticket that never started, like one you removed from a plan. Anything that was
    /// waiting for it stops waiting.
    pub fn delete_ticket(&self, id: i64) -> Result<()> {
        let Some(t) = self.ticket(id)? else { bail!("no ticket {}", self.key(id)) };
        if t.status != "backlog" || t.branch.is_some() {
            bail!("{} has already started, so it can't be removed. Stop it instead", self.key(id));
        }
        self.forget_ticket(id)
    }

    /// Removes a ticket and everything recorded about it. Tickets that waited for it, or came
    /// from it as a plan, stay. Callers check it's safe to remove first.
    pub(crate) fn forget_ticket(&self, id: i64) -> Result<()> {
        self.db.execute("UPDATE tickets SET after = NULL WHERE after = ?1", [id])?;
        self.db.execute("UPDATE tickets SET plan = NULL WHERE plan = ?1", [id])?;
        self.db.execute("UPDATE messages SET ticket_id = NULL WHERE ticket_id = ?1", [id])?;
        for table in ["events", "jobs", "inbox"] {
            self.db.execute(&format!("DELETE FROM {table} WHERE ticket_id = ?1"), [id])?;
        }
        self.db.execute("DELETE FROM tickets WHERE id = ?1", [id])?;
        Ok(())
    }

    /// A planning ticket runs on the repo itself, read-only, so it has no branch or worktree of its own.
    pub fn start_plan_ticket(&self, id: i64) -> Result<()> {
        self.db.execute("UPDATE tickets SET status = 'doing', worktree = ?2 WHERE id = ?1", params![id, self.repo()?.to_string_lossy()])?;
        Ok(())
    }

    /// Puts a section in the wiki, replacing one with the same name. Used for notes OneCorp
    /// writes itself, like what it learned from the repo.
    pub fn set_wiki_section(&self, name: &str, body: &str) -> Result<()> {
        let doc = self.wiki()?;
        let mut out = String::new();
        let (mut skipping, mut heading) = (false, name.to_string());
        for line in doc.lines() {
            if let Some(h) = line.strip_prefix("## ") {
                skipping = h.trim().eq_ignore_ascii_case(name);
                if skipping {
                    heading = h.trim().to_string();
                    continue;
                }
            }
            if !skipping {
                out.push_str(line);
                out.push('\n');
            }
        }
        let out = format!("{}\n\n## {heading}\n\n{}\n", out.trim_end(), body.trim());
        self.write_wiki(&out)
    }

    /// What you're building: the wiki's "What we're building" section.
    pub fn brief(&self) -> Result<String> {
        let doc = self.wiki()?;
        let mut lines = doc.lines().skip_while(|l| !l.strip_prefix("## ").is_some_and(|h| h.trim().eq_ignore_ascii_case(BUILDING)));
        lines.next();
        Ok(lines.take_while(|l| !l.starts_with("## ")).collect::<Vec<_>>().join("\n").trim().to_string())
    }

    /// Writes what you're building into the wiki, right after the mission.
    pub fn write_brief(&self, text: &str) -> Result<()> {
        let doc = self.wiki()?;
        let mut out: Vec<String> = vec![];
        let (mut skipping, mut placed) = (false, false);
        for line in doc.lines() {
            if let Some(h) = line.strip_prefix("## ") {
                skipping = h.trim().eq_ignore_ascii_case(BUILDING);
                if skipping {
                    continue;
                }
                // Into the mission's place in line: just before the section that follows it.
                if !placed && out.iter().any(|l| l.strip_prefix("## ").is_some_and(|h| h.trim().eq_ignore_ascii_case("Mission"))) {
                    out.push(format!("## {BUILDING}\n\n{}\n", text.trim()));
                    placed = true;
                }
            }
            if !skipping {
                out.push(line.to_string());
            }
        }
        let mut doc = out.join("\n");
        if !placed {
            doc = format!("{}\n\n## {BUILDING}\n\n{}\n", doc.trim_end(), text.trim());
        }
        self.write_wiki(&format!("{}\n", doc.trim_end()))?;
        Ok(())
    }

    pub fn assign(&self, ticket: i64, employee: &str) -> Result<()> {
        if self.employee(employee)?.is_none() {
            bail!("nobody called {employee} works at {}", self.name);
        }
        self.db.execute("UPDATE tickets SET assignee = ?2 WHERE id = ?1", params![ticket, employee])?;
        Ok(())
    }

    pub fn set_after(&self, ticket: i64, after: Option<i64>) -> Result<()> {
        self.db.execute("UPDATE tickets SET after = ?2 WHERE id = ?1", params![ticket, after])?;
        Ok(())
    }

    /// Tickets waiting for `ticket` to be merged before they start.
    pub fn waiting_for(&self, ticket: i64) -> Result<Vec<i64>> {
        let mut st = self.db.prepare("SELECT id FROM tickets WHERE after = ?1 AND status = 'waiting' ORDER BY id")?;
        let rows = st.query_map([ticket], |r| r.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn start_ticket(&self, id: i64, branch: &str, base: &str, worktree: &Path) -> Result<()> {
        self.db.execute(
            "UPDATE tickets SET status = 'doing', branch = ?2, base = ?3, worktree = ?4 WHERE id = ?1",
            params![id, branch, base, worktree.to_string_lossy()],
        )?;
        Ok(())
    }

    pub fn set_session(&self, id: i64, session: &str) -> Result<()> {
        self.db.execute("UPDATE tickets SET session_id = ?2 WHERE id = ?1", params![id, session])?;
        Ok(())
    }

    /// A ticket the agent marked blocked stays blocked when the run ends cleanly.
    pub fn finish_ticket(&self, id: i64, status: &str, cost: f64, tokens: u64) -> Result<()> {
        self.db.execute(
            "UPDATE tickets SET cost_usd = cost_usd + ?3, tokens = tokens + ?4,
               status = CASE WHEN status = 'blocked' AND ?2 = 'review' THEN 'blocked' ELSE ?2 END
             WHERE id = ?1",
            params![id, status, cost, tokens as i64],
        )?;
        Ok(())
    }

    /// The process id of the agent working on a ticket right now, if any.
    pub fn agent_pid(&self, ticket: i64) -> Result<Option<u32>> {
        Ok(self.db.query_row("SELECT agent_pid FROM tickets WHERE id = ?1", [ticket], |r| r.get(0)).optional()?.flatten())
    }

    pub fn set_agent_pid(&self, ticket: i64, pid: Option<u32>) -> Result<()> {
        self.db.execute("UPDATE tickets SET agent_pid = ?2 WHERE id = ?1", params![ticket, pid])?;
        Ok(())
    }

    /// Drops a ticket's jobs that haven't started yet. Returns how many.
    pub fn cancel_queued(&self, ticket: i64) -> Result<usize> {
        Ok(self.db.execute(
            "UPDATE jobs SET started_at = datetime('now'), finished_at = datetime('now'), error = 'cancelled' WHERE ticket_id = ?1 AND started_at IS NULL",
            [ticket],
        )?)
    }

    /// Back to a fresh ticket: no branch, worktree or session.
    pub fn reset_ticket(&self, id: i64) -> Result<()> {
        self.db.execute(
            "UPDATE tickets SET status = 'todo', branch = NULL, base = NULL, worktree = NULL, session_id = NULL, pr_number = NULL, pr_url = NULL WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }

    pub fn set_status(&self, id: i64, status: &str) -> Result<()> {
        self.db.execute("UPDATE tickets SET status = ?2 WHERE id = ?1", params![id, status])?;
        Ok(())
    }

    pub fn enqueue(&self, ticket: i64, job: &Job) -> Result<()> {
        let (kind, why, prompt, label) = match job {
            Job::Start => ("start", "", "", String::new()),
            Job::Resume { why, prompt, label } => ("resume", why.as_str(), prompt.as_str(), label.clone()),
            // The question id rides in `label`.
            Job::Consult { question } => ("consult", "", "", question.to_string()),
        };
        self.db.execute(
            "INSERT INTO jobs (ticket_id, kind, why, prompt, label) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![ticket, kind, why, prompt, label],
        )?;
        Ok(())
    }

    /// Takes the oldest queued job whose ticket and employee are both free. Each employee works
    /// one ticket at a time, like a person, which also keeps their runtime's rate limits sane.
    pub fn claim_job(&self) -> Result<Option<Claimed>> {
        self.claim(None)
    }

    /// Like `claim_job`, for one kind of job only.
    pub fn claim_job_of(&self, kind: &str) -> Result<Option<Claimed>> {
        self.claim(Some(kind))
    }

    fn claim(&self, kind: Option<&str>) -> Result<Option<Claimed>> {
        // People at their daily cap take no new work today. Matched against a ",a,b," list.
        let capped = format!(",{},", crate::spend::capped(self)?.join(","));
        let next = self
            .db
            .query_row(
                "SELECT j.id, j.ticket_id, j.kind, j.why, j.prompt, j.label FROM jobs j JOIN tickets t ON t.id = j.ticket_id
                 WHERE j.started_at IS NULL AND (?1 IS NULL OR j.kind = ?1) AND NOT EXISTS (
                   SELECT 1 FROM jobs r JOIN tickets rt ON rt.id = r.ticket_id
                   WHERE r.started_at IS NOT NULL AND r.finished_at IS NULL
                     AND (r.ticket_id = j.ticket_id OR rt.assignee = t.assignee))
                   AND instr(?2, ',' || coalesce(t.assignee, '') || ',') = 0
                 ORDER BY j.id LIMIT 1",
                params![kind, capped],
                |r| {
                    let job = match r.get::<_, String>(2)?.as_str() {
                        "start" => Job::Start,
                        "consult" => Job::Consult { question: r.get::<_, String>(5)?.parse().unwrap_or(0) },
                        _ => Job::Resume { why: r.get(3)?, prompt: r.get(4)?, label: r.get(5)? },
                    };
                    Ok(Claimed { id: r.get(0)?, ticket: r.get(1)?, job })
                },
            )
            .optional()?;
        if let Some(j) = &next {
            self.db.execute("UPDATE jobs SET started_at = datetime('now'), pid = ?2 WHERE id = ?1", params![j.id, std::process::id()])?;
        }
        Ok(next)
    }

    /// Records a job that this process runs right away, so it can be recovered if the process dies.
    pub fn run_now(&self, ticket: i64, job: &Job) -> Result<i64> {
        self.enqueue(ticket, job)?;
        let id = self.db.last_insert_rowid();
        self.db.execute("UPDATE jobs SET started_at = datetime('now'), pid = ?2 WHERE id = ?1", params![id, std::process::id()])?;
        Ok(id)
    }

    pub fn finish_job(&self, id: i64, error: Option<&str>) -> Result<()> {
        self.db.execute("UPDATE jobs SET finished_at = datetime('now'), error = ?2 WHERE id = ?1", params![id, error])?;
        Ok(())
    }

    /// Jobs whose process is gone (Ctrl-C, a crash, the app closed mid-ticket) would otherwise
    /// stay "doing" forever. Marks them and their tickets failed and returns the ticket keys.
    /// Safe to call anytime: a job whose process is still alive is left alone.
    pub fn recover_interrupted(&self) -> Result<Vec<String>> {
        // Chat replies run on a thread with no job behind them. A reply still "typing" after
        // 30 minutes lost its process, most likely when the app quit.
        self.db.execute(
            "UPDATE messages SET pending = 0, text = '(stopped before replying)' WHERE pending = 1 AND at < datetime('now', '-30 minutes')",
            [],
        )?;
        let mut st = self.db.prepare("SELECT id, ticket_id, pid FROM jobs WHERE started_at IS NOT NULL AND finished_at IS NULL")?;
        let running: Vec<(i64, i64, Option<u32>)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?.collect::<Result<_, _>>()?;
        let mut keys = vec![];
        for (job, ticket, pid) in running {
            if pid.is_some_and(alive) {
                continue;
            }
            self.finish_job(job, Some("interrupted"))?;
            self.set_status(ticket, "failed")?;
            self.add_event(ticket, "error", "stopped before it finished", "")?;
            keys.push(self.key(ticket));
        }
        Ok(keys)
    }

    pub fn memory_path(&self, employee: &str) -> PathBuf {
        self.dir.join("memory").join(format!("{employee}.md"))
    }

    /// An employee's own notes from past tickets, or empty.
    pub fn memory(&self, employee: &str) -> Result<String> {
        match fs::read_to_string(self.memory_path(employee)) {
            Ok(s) => Ok(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(e.into()),
        }
    }

    /// Appends a dated one-line note to an employee's memory.
    pub fn remember(&self, employee: &str, note: &str) -> Result<()> {
        let note = note.split_whitespace().collect::<Vec<_>>().join(" ");
        if note.is_empty() {
            return Ok(());
        }
        let today: String = self.db.query_row("SELECT date('now')", [], |r| r.get(0))?;
        let path = self.memory_path(employee);
        fs::create_dir_all(path.parent().unwrap())?;
        let mut f = fs::OpenOptions::new().create(true).append(true).open(path)?;
        std::io::Write::write_all(&mut f, format!("- {today} {note}\n").as_bytes())?;
        Ok(())
    }

    pub fn wiki_path(&self) -> PathBuf {
        self.dir.join("wiki.md")
    }

    /// The wiki's text, or empty if there isn't one yet.
    pub fn wiki(&self) -> Result<String> {
        match fs::read_to_string(self.wiki_path()) {
            Ok(s) => Ok(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(e.into()),
        }
    }

    /// Replaces the whole wiki, for editing it as text.
    pub fn write_wiki(&self, text: &str) -> Result<()> {
        let text = text.trim_end();
        fs::write(self.wiki_path(), format!("{text}\n"))?;
        Ok(())
    }

    pub fn add_to_wiki(&self, section: &str, text: &str) -> Result<()> {
        let doc = self.wiki()?;
        let doc = if doc.is_empty() { format!("# {} wiki\n", self.name) } else { doc };
        fs::write(self.wiki_path(), add_to_section(&doc, section, text))?;
        Ok(())
    }

    pub fn ask(&self, q: &Question) -> Result<i64> {
        self.db.execute(
            "INSERT INTO inbox (ticket_id, employee, question, options, default_answer, blocking, kind, stage)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![q.ticket_id, q.employee, q.question, q.options, q.default_answer, q.blocking, q.kind, if q.stage.is_empty() { "founder" } else { &q.stage }],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    /// Open questions waiting on you, oldest first. Ones a lead is still looking at aren't here.
    pub fn inbox(&self) -> Result<Vec<Question>> {
        self.questions("answer IS NULL AND stage = 'founder'")
    }

    /// Open questions a department lead is looking at before they'd reach you.
    pub fn with_leads(&self) -> Result<Vec<Question>> {
        self.questions("answer IS NULL AND stage = 'lead'")
    }

    pub fn question(&self, id: i64) -> Result<Option<Question>> {
        Ok(self.questions(&format!("id = {id}"))?.pop())
    }

    fn questions(&self, filter: &str) -> Result<Vec<Question>> {
        let mut st = self.db.prepare(&format!("SELECT id, ticket_id, employee, question, options, default_answer, blocking, answer, kind, stage, via, note, answered_by FROM inbox WHERE {filter} ORDER BY id"))?;
        let rows = st.query_map([], row_to_question)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Hands a question to a lead first, or up to you with the lead's reason.
    pub fn route(&self, id: i64, stage: &str, via: Option<&str>, note: Option<&str>) -> Result<()> {
        self.db.execute("UPDATE inbox SET stage = ?2, via = ?3, note = ?4 WHERE id = ?1", params![id, stage, via, note])?;
        Ok(())
    }

    /// The person whose questions go to someone before they reach you: their department's lead,
    /// unless they lead it themselves.
    pub fn lead_of(&self, employee: &str) -> Result<Option<Employee>> {
        let lead: Option<String> = self
            .db
            .query_row(
                "SELECT d.lead FROM employees e JOIN departments d ON d.id = e.department WHERE e.id = ?1 AND d.lead IS NOT NULL AND d.lead != e.id",
                [employee],
                |r| r.get(0),
            )
            .optional()?;
        match lead {
            Some(id) => self.employee(&id),
            None => Ok(None),
        }
    }

    /// Adds the cost of a run that isn't the ticket's own work, like a lead answering a question.
    pub fn add_cost(&self, ticket: i64, cost: f64, tokens: u64) -> Result<()> {
        self.db.execute("UPDATE tickets SET cost_usd = cost_usd + ?2, tokens = tokens + ?3 WHERE id = ?1", params![ticket, cost, tokens as i64])?;
        Ok(())
    }

    /// Saves your answer and returns the answered question.
    pub fn answer(&self, id: i64, answer: &str) -> Result<Question> {
        self.answer_as(id, answer, "founder")
    }

    /// Saves an answer from `by`, the founder or a lead.
    pub fn answer_as(&self, id: i64, answer: &str, by: &str) -> Result<Question> {
        let n = self.db.execute(
            "UPDATE inbox SET answer = ?2, answered_at = datetime('now'), answered_by = ?3 WHERE id = ?1 AND answer IS NULL",
            params![id, answer, by],
        )?;
        if n == 0 {
            bail!("no open question #{id}. See: onecorp inbox");
        }
        Ok(self.db.query_row(
            "SELECT id, ticket_id, employee, question, options, default_answer, blocking, answer, kind, stage, via, note, answered_by FROM inbox WHERE id = ?1",
            [id],
            row_to_question,
        )?)
    }

    pub fn open_blocking(&self, ticket: i64) -> Result<i64> {
        Ok(self.db.query_row(
            "SELECT count(*) FROM inbox WHERE ticket_id = ?1 AND blocking = 1 AND answer IS NULL",
            [ticket],
            |r| r.get(0),
        )?)
    }

    pub fn post(&self, channel: &str, author: &str, text: &str, ticket: Option<i64>) -> Result<i64> {
        self.db.execute(
            "INSERT INTO messages (channel, author, text, ticket_id) VALUES (?1, ?2, ?3, ?4)",
            params![channel, author, text, ticket],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    /// A reply that shows as "typing" until `finish_message` fills it in.
    pub fn post_pending(&self, channel: &str, author: &str) -> Result<i64> {
        self.db.execute("INSERT INTO messages (channel, author, text, pending) VALUES (?1, ?2, '', 1)", params![channel, author])?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn finish_message(&self, id: i64, text: &str, cost: f64, tokens: u64) -> Result<()> {
        self.db.execute(
            "UPDATE messages SET text = ?2, pending = 0, cost_usd = ?3, tokens = ?4, at = datetime('now') WHERE id = ?1",
            params![id, text, cost, tokens as i64],
        )?;
        Ok(())
    }

    /// Removes a message. Its cost stays in the spend ledger, since the run already happened.
    pub fn delete_message(&self, id: i64) -> Result<()> {
        self.db.execute("DELETE FROM messages WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Messages in a channel, or every channel when None. Oldest first.
    pub fn messages(&self, channel: Option<&str>) -> Result<Vec<Message>> {
        let mut st = self.db.prepare(
            "SELECT id, at, channel, author, text, ticket_id, pending FROM messages
             WHERE ?1 IS NULL OR channel = ?1 ORDER BY id",
        )?;
        let rows = st.query_map([channel], row_to_message)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn message(&self, id: i64) -> Result<Option<Message>> {
        Ok(self
            .db
            .query_row("SELECT id, at, channel, author, text, ticket_id, pending FROM messages WHERE id = ?1", [id], row_to_message)
            .optional()?)
    }

    /// Every channel anything was ever posted in.
    pub fn channel_names(&self) -> Result<Vec<String>> {
        let mut st = self.db.prepare("SELECT DISTINCT channel FROM messages ORDER BY channel")?;
        let rows = st.query_map([], |r| r.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// What chat replies cost, which belongs to no ticket.
    pub fn chat_spend(&self) -> Result<(f64, i64)> {
        Ok(self.db.query_row("SELECT coalesce(sum(cost_usd), 0), coalesce(sum(tokens), 0) FROM messages", [], |r| Ok((r.get(0)?, r.get(1)?)))?)
    }

    pub fn add_event(&self, ticket: i64, kind: &str, summary: &str, raw: &str) -> Result<()> {
        self.db.execute(
            "INSERT INTO events (ticket_id, kind, summary, raw) VALUES (?1, ?2, ?3, ?4)",
            params![ticket, kind, summary, raw],
        )?;
        Ok(())
    }

    pub fn ticket(&self, id: i64) -> Result<Option<Ticket>> {
        Ok(self
            .db
            .query_row("SELECT id, title, assignee, status, branch, cost_usd, worktree, session_id, tokens, base, pr_number, pr_url, description, created_by, after, kind, plan FROM tickets WHERE id = ?1", [id], row_to_ticket)
            .optional()?)
    }

    pub fn tickets(&self) -> Result<Vec<Ticket>> {
        let mut st = self.db.prepare("SELECT id, title, assignee, status, branch, cost_usd, worktree, session_id, tokens, base, pr_number, pr_url, description, created_by, after, kind, plan FROM tickets ORDER BY id")?;
        let rows = st.query_map([], row_to_ticket)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// The company's latest events across all tickets, newest first: (time, ticket, employee, kind, summary).
    /// The moments worth seeing in the Live feed, newest first. A list of what to show, not what to
    /// hide: anything new stays in the ticket's own log until it's chosen for the feed.
    pub fn recent_events(&self, limit: i64) -> Result<Vec<FeedEvent>> {
        let mut st = self.db.prepare(
            "SELECT e.at, e.ticket_id, coalesce(t.assignee, ''), e.kind, e.summary, t.title
             FROM events e JOIN tickets t ON t.id = e.ticket_id
             WHERE e.kind IN ('start', 'done', 'merge', 'pr', 'block', 'lead', 'create', 'review', 'stop', 'retry', 'wait', 'wiki') ORDER BY e.id DESC LIMIT ?1",
        )?;
        let rows = st.query_map([limit], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// (time, kind, summary) for one ticket, oldest first.
    pub fn events(&self, ticket: i64) -> Result<Vec<(String, String, String)>> {
        let mut st = self.db.prepare("SELECT at, kind, summary FROM events WHERE ticket_id = ?1 ORDER BY id")?;
        let rows = st.query_map([ticket], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
}

fn row_to_ticket(r: &rusqlite::Row) -> rusqlite::Result<Ticket> {
    Ok(Ticket {
        id: r.get(0)?,
        title: r.get(1)?,
        assignee: r.get(2)?,
        status: r.get(3)?,
        branch: r.get(4)?,
        cost_usd: r.get(5)?,
        worktree: r.get(6)?,
        session_id: r.get(7)?,
        tokens: r.get(8)?,
        base: r.get(9)?,
        pr_number: r.get(10)?,
        pr_url: r.get(11)?,
        description: r.get(12)?,
        created_by: r.get(13)?,
        after: r.get(14)?,
        kind: r.get(15)?,
        plan: r.get(16)?,
    })
}

fn row_to_question(r: &rusqlite::Row) -> rusqlite::Result<Question> {
    Ok(Question {
        id: r.get(0)?,
        ticket_id: r.get(1)?,
        employee: r.get(2)?,
        question: r.get(3)?,
        options: r.get(4)?,
        default_answer: r.get(5)?,
        blocking: r.get(6)?,
        answer: r.get(7)?,
        kind: r.get(8)?,
        stage: r.get(9)?,
        via: r.get(10)?,
        note: r.get(11)?,
        answered_by: r.get(12)?,
    })
}

/// Whether a process is still running. `kill -0` sends nothing, it only checks.
pub(crate) fn alive(pid: u32) -> bool {
    pid == std::process::id()
        || std::process::Command::new("kill").args(["-0", &pid.to_string()]).stderr(std::process::Stdio::null()).status().is_ok_and(|s| s.success())
}

/// Columns added after the first release. CREATE TABLE IF NOT EXISTS skips tables that already
/// exist, so older company files get these added here instead.
fn migrate(db: &Connection) -> Result<()> {
    const ADDED: &[(&str, &str, &str)] = &[
        ("employees", "fired_at", "TEXT"),
        ("employees", "department", "INTEGER REFERENCES departments(id)"),
        ("employees", "face", "TEXT"),
        ("employees", "daily_cap", "REAL NOT NULL DEFAULT 0"),
        ("tickets", "tokens", "INTEGER NOT NULL DEFAULT 0"),
        ("tickets", "base", "TEXT"),
        ("inbox", "kind", "TEXT NOT NULL DEFAULT 'question'"),
        ("jobs", "pid", "INTEGER"),
        ("tickets", "agent_pid", "INTEGER"),
        ("tickets", "pr_number", "INTEGER"),
        ("tickets", "pr_url", "TEXT"),
        ("tickets", "description", "TEXT NOT NULL DEFAULT ''"),
        ("tickets", "created_by", "TEXT NOT NULL DEFAULT 'founder'"),
        ("tickets", "after", "INTEGER REFERENCES tickets(id)"),
        ("tickets", "kind", "TEXT NOT NULL DEFAULT 'work'"),
        ("tickets", "plan", "INTEGER REFERENCES tickets(id)"),
        ("inbox", "stage", "TEXT NOT NULL DEFAULT 'founder'"),
        ("inbox", "via", "TEXT"),
        ("inbox", "note", "TEXT"),
        ("inbox", "answered_by", "TEXT"),
        ("messages", "pending", "INTEGER NOT NULL DEFAULT 0"),
        ("messages", "cost_usd", "REAL NOT NULL DEFAULT 0"),
        ("messages", "tokens", "INTEGER NOT NULL DEFAULT 0"),
    ];
    for (table, column, decl) in ADDED {
        let has: bool = db.query_row(
            &format!("SELECT count(*) > 0 FROM pragma_table_info('{table}') WHERE name = ?1"),
            [column],
            |r| r.get(0),
        )?;
        if !has {
            db.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"))?;
        }
    }
    Ok(())
}

fn row_to_message(r: &rusqlite::Row) -> rusqlite::Result<Message> {
    Ok(Message { id: r.get(0)?, at: r.get(1)?, channel: r.get(2)?, author: r.get(3)?, text: r.get(4)?, ticket: r.get(5)?, pending: r.get(6)? })
}

fn row_to_employee(r: &rusqlite::Row) -> rusqlite::Result<Employee> {
    Ok(Employee { id: r.get(0)?, name: r.get(1)?, role: r.get(2)?, runtime: r.get(3)?, model: r.get(4)?, bio: r.get(5)?, department: r.get(6)? })
}

/// Adds `text` as a bullet at the end of the `## section`, creating the section if it's missing.
pub fn add_to_section(doc: &str, section: &str, text: &str) -> String {
    let bullet = format!("- {}", text.trim());
    let heading = format!("## {section}");
    let lines: Vec<&str> = doc.lines().collect();
    let Some(start) = lines.iter().position(|l| l.trim().eq_ignore_ascii_case(&heading)) else {
        return format!("{}\n\n{heading}\n\n{bullet}\n", doc.trim_end());
    };
    let end = lines[start + 1..].iter().position(|l| l.starts_with("## ")).map_or(lines.len(), |i| start + 1 + i);
    // Insert after the section's last non-blank line, so the blank line before the next heading stays.
    let at = (start + 1..end).rev().find(|&i| !lines[i].trim().is_empty()).map_or(start + 1, |i| i + 1);
    let mut out: Vec<String> = lines[..at].iter().map(|l| l.to_string()).collect();
    if at == start + 1 {
        out.push(String::new());
    }
    out.push(bullet);
    out.extend(lines[at..].iter().map(|l| l.to_string()));
    out.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_folds_accents_and_keeps_the_first_name() {
        assert_eq!(slug("Tomás Reis"), "tomas");
        assert_eq!(slug("  Inês "), "ines");
        assert_eq!(slug("JOÃO"), "joao");
        assert_eq!(slug("Maya-Okafor"), "mayaokafor");
        assert_eq!(slug("李"), "");
    }

    #[test]
    fn claims_one_ticket_per_employee_at_a_time() {
        let dir = std::env::temp_dir().join(format!("onecorp-jobs-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = Company::open_dir("snowztech", dir.clone()).unwrap();
        for id in ["maya", "tomas"] {
            let e = Employee { id: id.into(), name: id.into(), role: "Engineer".into(), runtime: "claude".into(), model: None, bio: String::new(), department: None };
            c.hire(&e).unwrap();
        }
        let m1 = c.add_ticket("one", "maya").unwrap();
        let m2 = c.add_ticket("two", "maya").unwrap();
        let t1 = c.add_ticket("three", "tomas").unwrap();
        for t in [m1, m2, t1] {
            c.enqueue(t, &Job::Start).unwrap();
        }

        let a = c.claim_job().unwrap().unwrap();
        let b = c.claim_job().unwrap().unwrap();
        assert_eq!((a.ticket, b.ticket), (m1, t1), "maya's second ticket waits, tomas works in parallel");
        assert!(c.claim_job().unwrap().is_none());

        c.finish_job(a.id, None).unwrap();
        assert_eq!(c.claim_job().unwrap().unwrap().ticket, m2);

        // Departments: the first hire leads, names are unique ignoring case.
        let eng = c.add_department("Engineering").unwrap();
        assert!(c.add_department("engineering").is_err());
        c.set_department("tomas", eng.id).unwrap();
        c.set_department("maya", eng.id).unwrap();
        assert_eq!(c.department("ENGINEERING").unwrap().unwrap().lead.as_deref(), Some("tomas"));

        // While their process is alive, running jobs are left alone.
        assert!(c.recover_interrupted().unwrap().is_empty());
        // Serve died with t1 and m2 running: both come back failed instead of stuck.
        c.db.execute("UPDATE jobs SET pid = 999999 WHERE finished_at IS NULL", []).unwrap();
        assert_eq!(c.recover_interrupted().unwrap(), vec![c.key(m2), c.key(t1)]);
        assert_eq!(c.ticket(t1).unwrap().unwrap().status, "failed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adds_bullets_to_the_right_section() {
        let doc = "# acme wiki\n\n## Mission\n\nNotes.\n\n## Decisions\n\n## Conventions\n";
        let one = add_to_section(doc, "Decisions", "No ORMs");
        assert_eq!(one, "# acme wiki\n\n## Mission\n\nNotes.\n\n## Decisions\n\n- No ORMs\n\n## Conventions\n");
        let two = add_to_section(&one, "decisions", "Hash tokens at rest");
        assert!(two.contains("- No ORMs\n- Hash tokens at rest\n\n## Conventions"));
        let three = add_to_section(&two, "Conventions", "Small PRs");
        assert!(three.ends_with("## Conventions\n\n- Small PRs\n"));
        let four = add_to_section("# x\n", "Glossary", "TVL means total value locked");
        assert_eq!(four, "# x\n\n## Glossary\n\n- TVL means total value locked\n");
    }
}
