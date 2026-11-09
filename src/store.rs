use crate::work::Job;
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
  base TEXT,
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
  kind TEXT NOT NULL DEFAULT 'question',
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
  error TEXT
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

/// A job serve has claimed and is about to run.
pub struct Claimed {
    pub id: i64,
    pub ticket: i64,
    pub job: Job,
}

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
    /// "question", or "handbook" for a proposed handbook addition (then `options` holds the section).
    pub kind: String,
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
        fs::write(c.handbook_path(), format!("# {name} handbook\n\n## Mission\n\n{mission}\n\n## Decisions\n\n## Conventions\n"))?;
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

    pub fn set_status(&self, id: i64, status: &str) -> Result<()> {
        self.db.execute("UPDATE tickets SET status = ?2 WHERE id = ?1", params![id, status])?;
        Ok(())
    }

    pub fn enqueue(&self, ticket: i64, job: &Job) -> Result<()> {
        let (kind, why, prompt, label) = match job {
            Job::Start => ("start", "", "", ""),
            Job::Resume { why, prompt, label } => ("resume", why.as_str(), prompt.as_str(), label.as_str()),
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
        let next = self
            .db
            .query_row(
                "SELECT j.id, j.ticket_id, j.kind, j.why, j.prompt, j.label FROM jobs j JOIN tickets t ON t.id = j.ticket_id
                 WHERE j.started_at IS NULL AND NOT EXISTS (
                   SELECT 1 FROM jobs r JOIN tickets rt ON rt.id = r.ticket_id
                   WHERE r.started_at IS NOT NULL AND r.finished_at IS NULL
                     AND (r.ticket_id = j.ticket_id OR rt.assignee = t.assignee))
                 ORDER BY j.id LIMIT 1",
                [],
                |r| {
                    let job = match r.get::<_, String>(2)?.as_str() {
                        "start" => Job::Start,
                        _ => Job::Resume { why: r.get(3)?, prompt: r.get(4)?, label: r.get(5)? },
                    };
                    Ok(Claimed { id: r.get(0)?, ticket: r.get(1)?, job })
                },
            )
            .optional()?;
        if let Some(j) = &next {
            self.db.execute("UPDATE jobs SET started_at = datetime('now') WHERE id = ?1", [j.id])?;
        }
        Ok(next)
    }

    pub fn finish_job(&self, id: i64, error: Option<&str>) -> Result<()> {
        self.db.execute("UPDATE jobs SET finished_at = datetime('now'), error = ?2 WHERE id = ?1", params![id, error])?;
        Ok(())
    }

    /// Jobs that were running when serve stopped can't be trusted to resume cleanly.
    /// Marks them and their tickets failed, and returns the ticket keys.
    pub fn recover_interrupted(&self) -> Result<Vec<String>> {
        let mut st = self.db.prepare("SELECT id, ticket_id FROM jobs WHERE started_at IS NOT NULL AND finished_at IS NULL")?;
        let stuck: Vec<(i64, i64)> = st.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?.collect::<Result<_, _>>()?;
        for (job, ticket) in &stuck {
            self.finish_job(*job, Some("interrupted"))?;
            self.set_status(*ticket, "failed")?;
            self.add_event(*ticket, "error", "interrupted when onecorp serve stopped", "")?;
        }
        Ok(stuck.iter().map(|(_, t)| self.key(*t)).collect())
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

    pub fn handbook_path(&self) -> PathBuf {
        self.dir.join("handbook.md")
    }

    /// The handbook's text, or empty if there isn't one yet.
    pub fn handbook(&self) -> Result<String> {
        match fs::read_to_string(self.handbook_path()) {
            Ok(s) => Ok(s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn add_to_handbook(&self, section: &str, text: &str) -> Result<()> {
        let doc = self.handbook()?;
        let doc = if doc.is_empty() { format!("# {} handbook\n", self.name) } else { doc };
        fs::write(self.handbook_path(), add_to_section(&doc, section, text))?;
        Ok(())
    }

    pub fn ask(&self, q: &Question) -> Result<i64> {
        self.db.execute(
            "INSERT INTO inbox (ticket_id, employee, question, options, default_answer, blocking, kind)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![q.ticket_id, q.employee, q.question, q.options, q.default_answer, q.blocking, q.kind],
        )?;
        Ok(self.db.last_insert_rowid())
    }

    /// Open questions, oldest first.
    pub fn inbox(&self) -> Result<Vec<Question>> {
        let mut st = self.db.prepare(
            "SELECT id, ticket_id, employee, question, options, default_answer, blocking, answer, kind
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
            "SELECT id, ticket_id, employee, question, options, default_answer, blocking, answer, kind FROM inbox WHERE id = ?1",
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
            .query_row("SELECT id, title, assignee, status, branch, cost_usd, worktree, session_id, tokens, base FROM tickets WHERE id = ?1", [id], row_to_ticket)
            .optional()?)
    }

    pub fn tickets(&self) -> Result<Vec<Ticket>> {
        let mut st = self.db.prepare("SELECT id, title, assignee, status, branch, cost_usd, worktree, session_id, tokens, base FROM tickets ORDER BY id")?;
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
        base: r.get(9)?,
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
    })
}

fn row_to_employee(r: &rusqlite::Row) -> rusqlite::Result<Employee> {
    Ok(Employee { id: r.get(0)?, name: r.get(1)?, role: r.get(2)?, runtime: r.get(3)?, model: r.get(4)?, bio: r.get(5)? })
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
    fn claims_one_ticket_per_employee_at_a_time() {
        let dir = std::env::temp_dir().join(format!("onecorp-jobs-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = Company::open_dir("snowztech", dir.clone()).unwrap();
        for id in ["maya", "tomas"] {
            let e = Employee { id: id.into(), name: id.into(), role: "Engineer".into(), runtime: "claude".into(), model: None, bio: String::new() };
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

        // Serve died with t1 and m2 running: both come back failed instead of stuck.
        assert_eq!(c.recover_interrupted().unwrap(), vec![c.key(m2), c.key(t1)]);
        assert_eq!(c.ticket(t1).unwrap().unwrap().status, "failed");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn adds_bullets_to_the_right_section() {
        let doc = "# acme handbook\n\n## Mission\n\nNotes.\n\n## Decisions\n\n## Conventions\n";
        let one = add_to_section(doc, "Decisions", "No ORMs");
        assert_eq!(one, "# acme handbook\n\n## Mission\n\nNotes.\n\n## Decisions\n\n- No ORMs\n\n## Conventions\n");
        let two = add_to_section(&one, "decisions", "Hash tokens at rest");
        assert!(two.contains("- No ORMs\n- Hash tokens at rest\n\n## Conventions"));
        let three = add_to_section(&two, "Conventions", "Small PRs");
        assert!(three.ends_with("## Conventions\n\n- Small PRs\n"));
        let four = add_to_section("# x\n", "Glossary", "TVL means total value locked");
        assert_eq!(four, "# x\n\n## Glossary\n\n- TVL means total value locked\n");
    }
}
