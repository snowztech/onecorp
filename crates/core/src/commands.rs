//! The build and test commands employees may run in their worktree, so work comes back tested.
//! Claude Code only gets read-only git plus this list. Codex runs commands in its own sandbox.
//!
//! Worked out from the repo (Cargo.toml means cargo, a pnpm lockfile means pnpm...) until you
//! set your own list in Settings.

use crate::store::Company;
use anyhow::Result;
use std::path::Path;

/// What the repo looks like it builds and tests with.
pub fn detect(repo: &Path) -> Vec<String> {
    let has = |f: &str| repo.join(f).exists();
    let mut out: Vec<String> = vec![];
    if has("Cargo.toml") {
        out.extend(["cargo build", "cargo test", "cargo check", "cargo clippy", "cargo fmt"].map(String::from));
    }
    if has("package.json") {
        let pm = if has("pnpm-lock.yaml") {
            "pnpm"
        } else if has("bun.lockb") || has("bun.lock") {
            "bun"
        } else if has("yarn.lock") {
            "yarn"
        } else {
            "npm"
        };
        // A fresh worktree has no node_modules, so installing comes first.
        out.extend(["install", "test", "run"].map(|s| format!("{pm} {s}")));
    }
    if has("go.mod") {
        out.extend(["go build", "go test", "go vet"].map(String::from));
    }
    if has("pyproject.toml") || has("requirements.txt") || has("setup.py") {
        out.extend(["pytest", "python -m pytest"].map(String::from));
    }
    if has("Makefile") {
        out.push("make".into());
    }
    out
}

/// The list in use: yours if you set one, otherwise what the repo looks like.
pub fn list(c: &Company) -> Result<Vec<String>> {
    match c.setting("commands")? {
        Some(s) => Ok(parse(&s)),
        None => Ok(detect(&c.repo()?)),
    }
}

/// Saves your own list. One command per line or comma. An empty list means none at all.
pub fn set(c: &Company, text: &str) -> Result<()> {
    c.set_setting("commands", &parse(text).join("\n"))
}

/// Back to what the repo looks like.
pub fn reset(c: &Company) -> Result<()> {
    c.delete_setting("commands")
}

fn parse(text: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for cmd in text.split(['\n', ',']).map(|s| s.split_whitespace().collect::<Vec<_>>().join(" ")) {
        if !cmd.is_empty() && !out.contains(&cmd) {
            out.push(cmd);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_repo_and_your_list() {
        let dir = std::env::temp_dir().join(format!("onecorp-commands-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert!(detect(&dir).is_empty());
        std::fs::write(dir.join("package.json"), "{}").unwrap();
        std::fs::write(dir.join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(detect(&dir), ["pnpm install", "pnpm test", "pnpm run"]);
        assert_eq!(parse("cargo test,  npm  test\n\ncargo test"), ["cargo test", "npm test"]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
