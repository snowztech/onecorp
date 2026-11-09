use anyhow::{Result, bail};
use std::path::Path;
use std::process::Command;

pub fn add_worktree(repo: &Path, path: &Path, branch: &str) -> Result<()> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "add", "-b", branch])
        .arg(path)
        .output()?;
    if !out.status.success() {
        bail!("git worktree add failed: {}", String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(())
}
