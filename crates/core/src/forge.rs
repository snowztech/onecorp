//! Where pull requests live: GitHub through `gh`, GitLab through `glab`. The only module that talks
//! to them. Both use the login you already have, so OneCorp needs no account of its own.
//!
//! Tests point `ONECORP_GH` or `ONECORP_GLAB` at a stand-in script, so no network is needed.

use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Forge {
    GitHub,
    GitLab,
}

/// A pull request (merge request on GitLab).
#[derive(Debug, PartialEq)]
pub struct Pr {
    pub number: i64,
    pub url: String,
}

#[derive(Debug, PartialEq)]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

impl Forge {
    pub fn parse(mode: &str) -> Option<Forge> {
        match mode {
            "github" => Some(Forge::GitHub),
            "gitlab" => Some(Forge::GitLab),
            _ => None,
        }
    }

    pub fn mode(self) -> &'static str {
        match self {
            Forge::GitHub => "github",
            Forge::GitLab => "gitlab",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Forge::GitHub => "GitHub",
            Forge::GitLab => "GitLab",
        }
    }

    /// The word each forge uses, for messages.
    pub fn pr_word(self) -> &'static str {
        match self {
            Forge::GitHub => "pull request",
            Forge::GitLab => "merge request",
        }
    }

    /// Which forge a repo's `origin` points at, if any.
    pub fn detect(repo: &Path) -> Option<Forge> {
        let url = crate::git::remote_url(repo, "origin")?;
        if url.contains("github.com") {
            Some(Forge::GitHub)
        } else if url.contains("gitlab") {
            Some(Forge::GitLab)
        } else {
            None
        }
    }

    fn cli(self) -> String {
        let (var, default) = match self {
            Forge::GitHub => ("ONECORP_GH", "gh"),
            Forge::GitLab => ("ONECORP_GLAB", "glab"),
        };
        std::env::var(var).unwrap_or_else(|_| default.into())
    }

    fn run(self, repo: &Path, args: &[&str], stdin: Option<&str>) -> Result<String> {
        let cli = self.cli();
        let mut child = Command::new(&cli)
            .args(args)
            .current_dir(repo)
            .env("PATH", crate::runtime::path())
            .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("couldn't run {cli}. Install it and sign in with `{cli} auth login`"))?;
        if let Some(text) = stdin {
            child.stdin.take().unwrap().write_all(text.as_bytes())?;
        }
        let out = child.wait_with_output()?;
        if !out.status.success() {
            bail!("{cli} {} failed: {}", args.first().unwrap_or(&""), String::from_utf8_lossy(&out.stderr).trim());
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    /// Whether the CLI is installed and signed in, with what it said when it isn't.
    pub fn signed_in(self, repo: &Path) -> std::result::Result<String, String> {
        match self.run(repo, &["auth", "status"], None) {
            Ok(out) => Ok(out.lines().find(|l| l.contains("Logged in")).unwrap_or("signed in").trim().to_string()),
            Err(e) => Err(format!("{e:#}")),
        }
    }

    /// Opens a pull request from `branch` into `base`. The branch must already be pushed.
    pub fn open(self, repo: &Path, branch: &str, base: &str, title: &str, body: &str) -> Result<Pr> {
        let out = match self {
            Forge::GitHub => self.run(repo, &["pr", "create", "--head", branch, "--base", base, "--title", title, "--body-file", "-"], Some(body))?,
            Forge::GitLab => self.run(
                repo,
                &["mr", "create", "--source-branch", branch, "--target-branch", base, "--title", title, "--description", body, "--yes"],
                None,
            )?,
        };
        // Both print the new URL. The number is its last part: .../pull/12 or .../merge_requests/12.
        let url = out.lines().rev().find(|l| l.starts_with("http")).map(str::trim).unwrap_or("").to_string();
        let number = url.rsplit('/').next().and_then(|n| n.parse().ok()).with_context(|| format!("couldn't find the {} number in: {out}", self.pr_word()))?;
        Ok(Pr { number, url })
    }

    pub fn state(self, repo: &Path, number: i64) -> Result<PrState> {
        let n = number.to_string();
        let out = match self {
            Forge::GitHub => self.run(repo, &["pr", "view", &n, "--json", "state"], None)?,
            Forge::GitLab => self.run(repo, &["mr", "view", &n, "--output", "json"], None)?,
        };
        let v: Value = serde_json::from_str(&out).with_context(|| format!("unexpected answer from {}: {out}", self.cli()))?;
        Ok(match v["state"].as_str().unwrap_or("").to_ascii_lowercase().as_str() {
            "merged" => PrState::Merged,
            "closed" => PrState::Closed,
            _ => PrState::Open,
        })
    }

    /// Where you can create repos: your own account, then the organizations you belong to.
    pub fn owners(self) -> Result<Vec<String>> {
        let here = std::env::temp_dir();
        match self {
            Forge::GitHub => {
                let me = self.run(&here, &["api", "user", "--jq", ".login"], None)?;
                let orgs = self.run(&here, &["api", "user/orgs", "--jq", ".[].login"], None).unwrap_or_default();
                Ok(std::iter::once(me).chain(orgs.lines().map(String::from)).filter(|o| !o.is_empty()).collect())
            }
            Forge::GitLab => bail!("creating repos on GitLab isn't supported yet"),
        }
    }

    /// Creates `full_name` (owner/name) from the local repo at `dir`, sets it as origin and pushes.
    pub fn create_repo(self, dir: &Path, full_name: &str, private: bool) -> Result<String> {
        if self != Forge::GitHub {
            bail!("creating repos on {} isn't supported yet. Create it there and add it as origin", self.name());
        }
        let visibility = if private { "--private" } else { "--public" };
        let out = self.run(dir, &["repo", "create", full_name, visibility, "--source", ".", "--remote", "origin", "--push"], None)?;
        Ok(out.lines().find(|l| l.starts_with("http")).unwrap_or(full_name).trim().to_string())
    }

    /// Merges with a merge commit, not a squash, so each employee's commits keep their name.
    pub fn merge(self, repo: &Path, number: i64) -> Result<()> {
        let n = number.to_string();
        match self {
            Forge::GitHub => self.run(repo, &["pr", "merge", &n, "--merge"], None)?,
            Forge::GitLab => self.run(repo, &["mr", "merge", &n, "--yes"], None)?,
        };
        Ok(())
    }
}
