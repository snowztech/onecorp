use anyhow::{Result, bail};
use std::path::Path;
use std::process::Command;

/// Creates a worktree on a fresh branch and returns the branch name. If `branch` is already taken
/// (an old company with the same name, say), it tries branch-2, branch-3 and so on.
pub fn add_worktree(repo: &Path, path: &Path, branch: &str) -> Result<String> {
    let mut name = branch.to_string();
    for n in 2.. {
        if !branch_exists(repo, &name)? {
            break;
        }
        name = format!("{branch}-{n}");
    }
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "add", "-b", &name])
        .arg(path)
        .output()?;
    if !out.status.success() {
        bail!("git worktree add failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(name)
}

/// The env that makes git commits show up under an employee's name.
pub fn identity(name: &str, email: &str) -> Vec<(String, String)> {
    ["GIT_AUTHOR_NAME", "GIT_COMMITTER_NAME"]
        .iter()
        .map(|k| (k.to_string(), name.to_string()))
        .chain(["GIT_AUTHOR_EMAIL", "GIT_COMMITTER_EMAIL"].iter().map(|k| (k.to_string(), email.to_string())))
        .collect()
}

/// Commits everything left in the worktree. Returns the short hash, or None if there was nothing to commit.
pub fn commit_all(wt: &Path, message: &str, identity: &[(String, String)]) -> Result<Option<String>> {
    let git = |args: &[&str]| -> Result<std::process::Output> {
        let out = Command::new("git").arg("-C").arg(wt).args(args).envs(identity.iter().map(|(k, v)| (k, v))).output()?;
        if !out.status.success() {
            bail!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim());
        }
        Ok(out)
    };
    if git(&["status", "--porcelain"])?.stdout.is_empty() {
        return Ok(None);
    }
    git(&["add", "-A"])?;
    git(&["commit", "-q", "-m", message])?;
    let hash = git(&["rev-parse", "--short", "HEAD"])?.stdout;
    Ok(Some(String::from_utf8_lossy(&hash).trim().to_string()))
}

fn branch_exists(repo: &Path, branch: &str) -> Result<bool> {
    let st = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")])
        .output()?
        .status;
    Ok(st.success())
}
