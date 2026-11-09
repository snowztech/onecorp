//! Creating a company in one step, so the CLI and the app can't drift apart: the repo if you're
//! starting from nothing, the company, its team, how its work ships, and the first plan.

use crate::forge::Forge;
use crate::store::{Company, Employee};
use crate::templates::{self, Person};
use crate::ship;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

/// `brief` is what you're building. With a brief and a team, planning starts right away.
pub fn new_company(name: &str, repo: &Path, mission: &str, people: &[Person], brief: &str) -> Result<(Company, Vec<Employee>)> {
    // Checked first, so a bad team never leaves a half-made company behind.
    templates::check(people)?;
    let c = Company::create(name, repo, mission)?;
    c.set_ship_mode(ship::default_mode(&c.repo()?))?;
    let hired = templates::hire(&c, people)?;
    crate::store::set_current(&c.name)?;
    if !brief.trim().is_empty() {
        c.write_brief(brief)?;
        if !hired.is_empty() {
            crate::work::kickoff(&c, None)?;
        }
    }
    Ok((c, hired))
}

/// Deletes a company: its tickets, chat, handbook and memory, everything OneCorp keeps for it.
/// Your repo is never touched, and branches stay in it, since they may hold work you want.
/// Running agents are stopped and the company's worktrees are removed through git first, so
/// the repo isn't left pointing at folders that are gone.
pub fn delete_company(name: &str) -> Result<()> {
    let c = Company::open(name)?;
    for t in c.tickets()? {
        if ["doing", "queued"].contains(&t.status.as_str()) {
            let _ = crate::work::stop(&c, &t);
        }
    }
    let workspaces = c.dir.join("wt");
    if let Ok(repo) = c.repo() {
        for t in c.tickets()? {
            let Some(wt) = t.worktree.as_deref().map(Path::new) else { continue };
            // Same guard as git::discard: only folders inside this company's own wt/ are removed.
            let inside = wt.canonicalize().ok().zip(workspaces.canonicalize().ok()).is_some_and(|(w, root)| w.starts_with(&root) && w != root);
            if inside {
                let _ = Command::new("git").arg("-C").arg(&repo).args(["worktree", "remove", "--force"]).arg(wt).output();
            }
        }
        let _ = Command::new("git").arg("-C").arg(&repo).args(["worktree", "prune"]).output();
    }
    let dir = c.dir.clone();
    drop(c);
    // The company's folder sits in OneCorp's home. Checked anyway, since this deletes recursively.
    if dir.parent() != Some(crate::store::home().join("companies").as_path()) {
        bail!("{} isn't a company folder, so it was left alone", dir.display());
    }
    std::fs::remove_dir_all(&dir).with_context(|| format!("couldn't delete {}", dir.display()))?;
    if crate::store::current().ok().as_deref() == Some(name) {
        match crate::store::list()?.first() {
            Some(other) => crate::store::set_current(other)?,
            None => {
                let _ = std::fs::remove_file(crate::store::home().join("current"));
            }
        }
    }
    Ok(())
}

/// Where new repos go by default: ~/dev when it exists, otherwise your home folder.
pub fn default_parent() -> PathBuf {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let dev = home.join("dev");
    if dev.is_dir() { dev } else { home }
}

/// Where to also create the repo, besides your Mac.
pub struct Remote<'a> {
    /// owner/name, like snowztech/notes
    pub full_name: &'a str,
    pub private: bool,
}

/// Starts a repo from nothing: a folder, `git init` on main, a README and a first commit, and
/// optionally the same repo on GitHub. Refuses a folder that already has things in it.
pub fn create_repo(path: &Path, title: &str, mission: &str, remote: Option<Remote>) -> Result<PathBuf> {
    if path.exists() && std::fs::read_dir(path)?.next().is_some() {
        bail!("{} already has files in it. Pick an empty or new folder, or use it as an existing repo", path.display());
    }
    std::fs::create_dir_all(path).with_context(|| format!("couldn't create {}", path.display()))?;
    let readme = if mission.trim().is_empty() { format!("# {title}\n") } else { format!("# {title}\n\n{}\n", mission.trim()) };
    std::fs::write(path.join("README.md"), readme)?;

    // The first commit needs a git identity. Yours from git config, or your founder name if you have none yet.
    let configured = Command::new("git").arg("-C").arg(path).args(["config", "user.email"]).output().is_ok_and(|o| o.status.success());
    let founder = crate::founder::name();
    let who: Vec<String> = if configured { vec![] } else { vec!["-c".into(), format!("user.name={founder}"), "-c".into(), "user.email=founder@onecorp.local".into()] };
    for args in [vec!["init", "-q", "-b", "main"], vec!["add", "-A"], vec!["commit", "-q", "-m", "Start"]] {
        let out = Command::new("git").arg("-C").arg(path).args(&who).args(&args).output()?;
        if !out.status.success() {
            bail!("git {} failed: {}", args[0], String::from_utf8_lossy(&out.stderr).trim());
        }
    }
    if let Some(r) = remote {
        Forge::GitHub.create_repo(path, r.full_name, r.private)?;
    }
    Ok(path.canonicalize()?)
}
