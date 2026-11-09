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
  hired_at TEXT NOT NULL DEFAULT (datetime('now')),
  fired_at TEXT
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
  tokens INTEGER NOT NULL DEFAULT 0,
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
  answer TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  answered_at TEXT
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

/// (time, channel, author, text, ticket)
pub type Message = (String, String, String, String, Option<i64>);

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

    pub(crate) fn open_dir(name: &str, dir: PathBuf) -> Result<Company> {
        let db = Connection::open(dir.join("company.db"))?;
        // The CLI and the agent's MCP server write at the same time. WAL plus a busy timeout
        // lets them take turns instead of failing with "database is locked".
        db.busy_timeout(std::time::Duration::from_secs(5))?;
        db.execute_batch("PRAGMA journal_mode = WAL;")?;
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

    /// Hiring someone who was fired before brings their record back with the new details.
    pub fn hire(&self, e: &Employee) -> Result<()> {
        if self.employee(&e.id)?.is_some() {
            bail!("{} already works at {}", e.id, self.name);
        }
        self.db.execute(
            "INSERT INTO employees (id, name, role, runtime, model, bio) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT (id) DO UPDATE SET name = ?2, role = ?3, runtime = ?4, model = ?5, bio = ?6,
               fired_at = NULL, hired_at = datetime('now')",
            params![e.id, e.name, e.role, e.runtime, e.model, e.bio],
        )?;
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
        Ok(())
    }

    pub fn employee(&self, id: &str) -> Result<Option<Employee>> {
        Ok(self
            .db
            .query_row(
                "SELECT id, name, role, runtime, model, bio FROM employees WHERE id = ?1 AND fired_at IS NULL",
                [id],
                row_to_employee,
            )
            .optional()?)
    }

    pub fn employees(&self) -> Result<Vec<Employee>> {
        let mut st = self
            .db
            .prepare("SELECT id, name, role, runtime, model, bio FROM employees WHERE fired_at IS NULL ORDER BY hired_at")?;
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

    pub fn set_status(&self, id: i64, status: &str) -> Result<()> {
        self.db.execute("UPDATE tickets SET status = ?2 WHERE id = ?1", params![id, status])?;
        Ok(())
    }

    pub fn ask(&self, q: &Question) -> Result<i64> {
        self.db.execute(
            "INSERT INTO inbox (ticket_id, employee, question, options, default_answer, blocking)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![q.ticket_id, q.employee, q.question, q.options, q.default_answer, q.blocking],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    /// Open questions, oldest first.
    pub fn inbox(&self) -> Result<Vec<Question>> {
        let mut st = self.db.prepare(
            "SELECT id, ticket_id, employee, question, options, default_answer, blocking, answer
             FROM inbox WHERE answer IS NULL ORDER BY id",
        )?;
        let rows = st.query_map([], row_to_question)?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Saves the answer and returns the answered question.
    pub fn answer(&self, id: i64, answer: &str) -> Result<Question> {
        let n = self.db.execute(
            "UPDATE inbox SET answer = ?2, answered_at = datetime('now') WHERE id = ?1 AND answer IS NULL",
            params![id, answer],
        )?;
        if n == 0 {
            bail!("no open question #{id}. See: onecorp inbox");
        }
        Ok(self.db.query_row(
            "SELECT id, ticket_id, employee, question, options, default_answer, blocking, answer FROM inbox WHERE id = ?1",
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

    pub fn post(&self, channel: &str, author: &str, text: &str, ticket: Option<i64>) -> Result<()> {
        self.db.execute(
            "INSERT INTO messages (channel, author, text, ticket_id) VALUES (?1, ?2, ?3, ?4)",
            params![channel, author, text, ticket],
        )?;
        Ok(())
    }

    /// Messages in a channel, or every channel when None. Oldest first.
    pub fn messages(&self, channel: Option<&str>) -> Result<Vec<Message>> {
        let mut st = self.db.prepare(
            "SELECT at, channel, author, text, ticket_id FROM messages
             WHERE ?1 IS NULL OR channel = ?1 ORDER BY id",
        )?;
        let rows = st.query_map([channel], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
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
            .query_row("SELECT id, title, assignee, status, branch, cost_usd, worktree, session_id, tokens FROM tickets WHERE id = ?1", [id], row_to_ticket)
            .optional()?)
    }

    pub fn tickets(&self) -> Result<Vec<Ticket>> {
        let mut st = self.db.prepare("SELECT id, title, assignee, status, branch, cost_usd, worktree, session_id, tokens FROM tickets ORDER BY id")?;
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
        worktree: r.get(6)?,
        session_id: r.get(7)?,
        tokens: r.get(8)?,
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
    })
}

fn row_to_employee(r: &rusqlite::Row) -> rusqlite::Result<Employee> {
    Ok(Employee { id: r.get(0)?, name: r.get(1)?, role: r.get(2)?, runtime: r.get(3)?, model: r.get(4)?, bio: r.get(5)? })
}
