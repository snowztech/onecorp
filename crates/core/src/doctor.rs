//! `onecorp doctor`: checks that everything OneCorp depends on is in place, and says how to fix
//! what isn't. Kept in the library so the desktop app can show the same checks.

use crate::{git, runtime, serve, store};
use std::process::Command;

#[derive(Debug, PartialEq)]
pub enum Level {
    Ok,
    /// Works, but something is worth knowing or will bite later.
    Warn,
    /// Something won't work until it's fixed.
    Fail,
}

pub struct Check {
    pub level: Level,
    pub what: String,
    pub detail: String,
    pub fix: Option<String>,
}

fn check(level: Level, what: &str, detail: impl Into<String>, fix: Option<&str>) -> Check {
    Check { level, what: what.into(), detail: detail.into(), fix: fix.map(String::from) }
}

/// The first line of `<program> --version`, found with the same PATH agents run with.
fn version(program: &str) -> Option<String> {
    let out = Command::new(program).arg("--version").env("PATH", runtime::path()).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").trim().to_string())
}

pub fn run() -> Vec<Check> {
    let mut checks = vec![];

    checks.push(match version("git") {
        Some(v) => check(Level::Ok, "git", v, None),
        None => check(Level::Fail, "git", "not found", Some("Install git: xcode-select --install")),
    });

    let installs = [("claude", "Claude Code", "npm install -g @anthropic-ai/claude-code, then run claude once to sign in"), ("codex", "Codex", "npm install -g @openai/codex, then run codex once to sign in")];
    let mut found = vec!["fake"];
    for (program, name, how) in installs {
        checks.push(match version(program) {
            Some(v) => {
                found.push(program);
                check(Level::Ok, name, v, None)
            }
            None => check(Level::Warn, name, "not installed. Employees can't run on it", Some(how)),
        });
    }

    let home = store::home();
    let writable = std::fs::create_dir_all(&home).is_ok() && std::fs::metadata(&home).is_ok_and(|m| !m.permissions().readonly());
    checks.push(if writable {
        check(Level::Ok, "home", home.display().to_string(), None)
    } else {
        check(Level::Fail, "home", format!("{} isn't writable", home.display()), Some("Fix its permissions, or set ONECORP_HOME to another folder"))
    });

    let companies = store::list().unwrap_or_default();
    if companies.is_empty() {
        checks.push(check(Level::Warn, "companies", "none yet", Some("onecorp new <name> --repo <path to a git repo>")));
    }
    for name in companies {
        checks.extend(company(&name, &found));
    }

    checks.push(if serve::running() {
        check(Level::Ok, "serve", "running, so work is queued and runs in parallel", None)
    } else {
        check(Level::Ok, "serve", "not running. Work runs in your terminal, one ticket at a time", Some("onecorp serve (or open the app) to run it in the background"))
    });
    checks
}

fn company(name: &str, runtimes: &[&str]) -> Vec<Check> {
    let what = format!("company {name}");
    let c = match store::Company::open(name) {
        Ok(c) => c,
        Err(e) => return vec![check(Level::Fail, &what, format!("can't open it: {e:#}"), None)],
    };
    let repo = match c.repo() {
        Ok(r) if r.join(".git").exists() => r,
        Ok(r) => return vec![check(Level::Fail, &what, format!("its repo {} is gone or isn't a git repo anymore", r.display()), Some("Move it back, or create the company again around the new path"))],
        Err(e) => return vec![check(Level::Fail, &what, format!("no repo on record: {e:#}"), None)],
    };
    let mut out = vec![];
    match git::current_branch(&repo) {
        Ok(b) => out.push(check(Level::Ok, &what, format!("{} on {b}", repo.display()), None)),
        Err(_) => out.push(check(Level::Fail, &what, format!("{} is on a detached HEAD, so tickets have no branch to start from", repo.display()), Some("git checkout main (or your usual branch) in that repo"))),
    }
    let team = c.employees().unwrap_or_default();
    if team.is_empty() {
        out.push(check(Level::Warn, &what, "nobody works here yet", Some(&format!("onecorp hire --company {name}"))));
    }
    for e in team.iter().filter(|e| !runtimes.contains(&e.runtime.as_str())) {
        out.push(check(Level::Fail, &what, format!("{} works on {}, which isn't installed", e.name, e.runtime), Some(&format!("Install {} (see above), or rehire {} on another runtime", e.runtime, e.id))));
    }
    out
}
