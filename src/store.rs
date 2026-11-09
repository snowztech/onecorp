use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OptionalExtension, params};
use std::fs;
use std::path::{Path, PathBuf};

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS employees (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  role TEXT NOT NULL,
  runtime TEXT NOT NULL,
  model TEXT,
  bio TEXT NOT NULL DEFAULT '',
  hired_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS tickets (
  id INTEGER PRIMARY KEY,
  title TEXT NOT NULL,
  assignee TEXT REFERENCES employees(id),
  status TEXT NOT NULL DEFAULT 'todo',
  branch TEXT,
  worktree TEXT,
  session_id TEXT,
  cost_usd REAL NOT NULL DEFAULT 0,
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
";

/// Everything lives under ~/.onecorp, or $ONECORP_HOME when set (handy for tests).
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

pub struct Employee {
    pub id: String,
    pub name: String,
    pub role: String,
    pub runtime: String,
    pub model: Option<String>,
    pub bio: String,
}

pub struct Ticket {
    pub id: i64,
    pub title: String,
    pub assignee: Option<String>,
    pub status: String,
    pub branch: Option<String>,
    pub cost_usd: f64,
}

pub struct Company {
    pub name: String,
    pub dir: PathBuf,
    db: Connection,
}

impl Company {
    pub fn create(name: &str, repo: &Path) -> Result<Company> {
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
        Ok(c)
    }

    pub fn open(name: &str) -> Result<Company> {
        let dir = home().join("companies").join(name);
        if !dir.exists() {
            bail!("no company named {name}. Create it with: onecorp new {name} --repo <path>");
        }
        Self::open_dir(name, dir)
    }

    fn open_dir(name: &str, dir: PathBuf) -> Result<Company> {
        let db = Connection::open(dir.join("company.db"))?;
        db.execute_batch(SCHEMA)?;
        Ok(Company { name: name.to_string(), dir, db })
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

    pub fn hire(&self, e: &Employee) -> Result<()> {
        if self.employee(&e.id)?.is_some() {
            bail!("{} already works at {}", e.id, self.name);
        }
        self.db.execute(
            "INSERT INTO employees (id, name, role, runtime, model, bio) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![e.id, e.name, e.role, e.runtime, e.model, e.bio],
        )?;
        Ok(())
    }

    pub fn fire(&self, id: &str) -> Result<()> {
        let busy: i64 =
            self.db.query_row("SELECT count(*) FROM tickets WHERE assignee = ?1 AND status = 'doing'", [id], |r| r.get(0))?;
        if busy > 0 {
            bail!("{id} is working on a ticket right now. Wait for it to finish first");
        }
        // Their past tickets stay, so the history of who did what survives.
        if self.db.execute("DELETE FROM employees WHERE id = ?1", [id])? == 0 {
            bail!("nobody called {id} works at {}", self.name);
        }
        Ok(())
    }

    pub fn employee(&self, id: &str) -> Result<Option<Employee>> {
        Ok(self
            .db
            .query_row("SELECT id, name, role, runtime, model, bio FROM employees WHERE id = ?1", [id], row_to_employee)
            .optional()?)
    }

    pub fn employees(&self) -> Result<Vec<Employee>> {
        let mut st = self.db.prepare("SELECT id, name, role, runtime, model, bio FROM employees ORDER BY hired_at")?;
        let rows = st.query_map([], row_to_employee)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn add_ticket(&self, title: &str, assignee: &str) -> Result<i64> {
        self.db.execute("INSERT INTO tickets (title, assignee) VALUES (?1, ?2)", [title, assignee])?;
        Ok(self.db.last_insert_rowid())
    }

    pub fn start_ticket(&self, id: i64, branch: &str, worktree: &Path) -> Result<()> {
        self.db.execute(
            "UPDATE tickets SET status = 'doing', branch = ?2, worktree = ?3 WHERE id = ?1",
            params![id, branch, worktree.to_string_lossy()],
        )?;
        Ok(())
    }

    pub fn set_session(&self, id: i64, session: &str) -> Result<()> {
        self.db.execute("UPDATE tickets SET session_id = ?2 WHERE id = ?1", params![id, session])?;
        Ok(())
    }

    pub fn finish_ticket(&self, id: i64, status: &str, cost: f64) -> Result<()> {
        self.db.execute(
            "UPDATE tickets SET status = ?2, cost_usd = cost_usd + ?3 WHERE id = ?1",
            params![id, status, cost],
        )?;
        Ok(())
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
            .query_row("SELECT id, title, assignee, status, branch, cost_usd FROM tickets WHERE id = ?1", [id], row_to_ticket)
            .optional()?)
    }

    pub fn tickets(&self) -> Result<Vec<Ticket>> {
        let mut st = self.db.prepare("SELECT id, title, assignee, status, branch, cost_usd FROM tickets ORDER BY id")?;
        let rows = st.query_map([], row_to_ticket)?;
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
    })
}

fn row_to_employee(r: &rusqlite::Row) -> rusqlite::Result<Employee> {
    Ok(Employee { id: r.get(0)?, name: r.get(1)?, role: r.get(2)?, runtime: r.get(3)?, model: r.get(4)?, bio: r.get(5)? })
}
