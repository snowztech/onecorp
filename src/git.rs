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

fn branch_exists(repo: &Path, branch: &str) -> Result<bool> {
    let st = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")])
        .output()?
        .status;
    Ok(st.success())
}
