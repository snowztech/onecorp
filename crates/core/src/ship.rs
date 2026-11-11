//! How approved work lands in the codebase. Each company has a mode:
//!
//! - `local`: approving merges the ticket's branch into your local base branch.
//! - `github` / `gitlab`: a finished ticket is pushed and opened as a pull request, and approving
//!   merges that pull request. A PR merged or closed on the forge itself is picked up by `sync`.
//!
//! Conflicts work the same in every mode: the ticket goes back to its employee to fix.

use crate::forge::{Forge, PrState};
use crate::store::{Company, Ticket};
use crate::work::{Job, dispatch, out, short};
use crate::git;
use anyhow::{Result, anyhow, bail};
use std::path::Path;

pub fn forge(c: &Company) -> Result<Option<Forge>> {
    Ok(Forge::parse(&c.ship_mode()?))
}

/// The forge for one repo: the company's, when that repo's origin is on it. A second repo with
/// no remote on the forge merges locally instead of failing.
pub fn forge_for(c: &Company, repo: &Path) -> Result<Option<Forge>> {
    // The main repo is what the ship mode was set and checked for, whatever its remote looks like.
    if c.repo().is_ok_and(|main| main == repo) {
        return forge(c);
    }
    Ok(forge(c)?.filter(|f| Forge::detect(repo) == Some(*f)))
}

/// The mode a new company starts with: pull requests when `origin` is on a forge and you're signed
/// in there, local merges otherwise.
pub fn default_mode(repo: &Path) -> &'static str {
    match Forge::detect(repo) {
        Some(f) if f.signed_in(repo).is_ok() => f.mode(),
        _ => "local",
    }
}

/// Switches modes, refusing ones that couldn't work yet and saying how to fix that.
pub fn set_mode(c: &Company, mode: &str) -> Result<()> {
    let repo = c.repo()?;
    if mode != "local" {
        let Some(f) = Forge::parse(mode) else { bail!("no mode called {mode}. Pick local, github or gitlab") };
        if git::remote_url(&repo, "origin").is_none() {
            bail!("{} has no origin remote. Add one, like: git remote add origin <url>", repo.display());
        }
        if let Err(why) = f.signed_in(&repo) {
            bail!("you're not signed in to {}: {why}", f.name());
        }
    }
    c.set_ship_mode(mode)
}

/// Where a new ticket's branch starts: the latest base on the forge, or the local base.
pub fn start_point(c: &Company, repo: &Path, base: &str) -> Result<String> {
    if forge_for(c, repo)?.is_none() {
        return Ok(base.to_string());
    }
    git::fetch(repo, base)?;
    Ok(format!("origin/{base}"))
}

/// Pushes a finished ticket and opens its pull request, or updates the one it has.
pub fn publish(c: &Company, t: &Ticket, summary: &str) -> Result<()> {
    let repo = c.repo_of(t)?;
    let Some(f) = forge_for(c, &repo)? else { return Ok(()) };
    let key = c.key(t.id);
    let (Some(wt), Some(branch)) = (&t.worktree, &t.branch) else { bail!("{key} has no branch to push") };
    let base = t.base.clone().unwrap_or_else(|| "main".into());
    git::push(Path::new(wt), branch)?;
    if let Some(n) = t.pr_number {
        c.add_event(t.id, "pr", &format!("updated #{n}"), "")?;
        return Ok(());
    }
    let who = t.assignee.as_deref().and_then(|id| c.employee(id).ok().flatten()).map(|e| e.name).unwrap_or_default();
    let body = format!("{}\n\n---\nDone by {who} on OneCorp, ticket {key}.", summary.trim());
    let pr = f.open(&repo, branch, &base, &short(&format!("{key}: {}", t.title), 72), &body)?;
    c.set_pr(t.id, pr.number, &pr.url)?;
    c.add_event(t.id, "pr", &format!("opened #{} {}", pr.number, pr.url), "")?;
    out(&format!("  pr     #{} {}", pr.number, pr.url));
    Ok(())
}

/// Publishing failing (signed out, no network) shouldn't lose the work: the ticket stays in
/// review with the reason, and approving tries again.
pub fn publish_or_note(c: &Company, t: &Ticket, summary: &str) {
    if let Err(e) = publish(c, t, summary) {
        let why = format!("couldn't open the pull request: {e:#}");
        let _ = c.add_event(t.id, "error", &why, "");
        out(&format!("  error  {why}. Approving retries it."));
    }
}

/// Lands a reviewed ticket. Its branch first takes in the latest base, and a conflict goes back
/// to the employee. Then it merges locally, or merges the pull request.
pub fn approve(c: &Company, t: &Ticket) -> Result<()> {
    // Approving a plan means starting it. It has nothing to merge.
    if t.kind == "plan" {
        return crate::work::start_plan(c, t);
    }
    if t.kind == "wiki" {
        crate::work::finish_wiki(c, t)?;
        return crate::work::release_waiting(c, t.id);
    }
    let key = c.key(t.id);
    match t.status.as_str() {
        "review" => {}
        "done" => bail!("{key} is already merged"),
        s => bail!("{key} is {s}. Only tickets in review can be approved"),
    }
    let (Some(wt_s), Some(branch)) = (&t.worktree, &t.branch) else { bail!("{key} has no branch to merge") };
    let (repo, wt) = (c.repo_of(t)?, Path::new(wt_s));
    let base = match &t.base {
        Some(b) => b.clone(),
        None => git::default_branch(&repo)?,
    };
    let forge = forge_for(c, &repo)?;
    let who = t.assignee.as_deref().and_then(|id| c.employee(id).ok().flatten());
    let identity = match &who {
        Some(e) => crate::work::signature(c, e, &repo)?.env,
        None => vec![],
    };

    let upstream = if forge.is_some() {
        git::fetch(&repo, &base)?;
        format!("origin/{base}")
    } else {
        base.clone()
    };
    if let git::Merge::Conflicts(files) = git::merge_base_into(wt, &upstream, &identity)? {
        let name = who.as_ref().map(|e| e.name.as_str()).unwrap_or("its owner");
        out(&format!("{base} moved since {key} started, and these files conflict: {}", files.join(", ")));
        out(&format!("Sending it back to {name} to fix.\n"));
        c.add_event(t.id, "merge", &format!("conflicts with {base}: {}", files.join(", ")), "")?;
        let prompt = format!(
            "The founder wants to merge your work, but {base} moved since you started and git found conflicts in: {files}.\n\n\
             The merge is in progress in your worktree. Open each file, resolve the conflict markers so both sides' intent is kept, \
             and remove every marker. Don't run git merge, git add or git commit: OneCorp finishes the merge when you're done. \
             End with two or three sentences on how you resolved each conflict.",
            files = files.join(", "),
        );
        dispatch(c, t.id, Job::Resume { why: "to fix the conflicts".into(), prompt, label: "resolve conflicts".into() })?;
        out(&format!("Once it's back in review and you've checked it, approve again: onecorp approve {key}"));
        return Ok(());
    }

    match forge {
        None => {
            git::fast_forward(&repo, &base, branch)?;
            landed(c, t, None, &format!("merged into {base}"))?;
            out(&format!("Merged {key} into {base}. New tickets start from it."));
        }
        Some(f) => {
            // Push whatever the update from base added, and open the PR if that failed earlier.
            publish(c, t, "")?;
            let t = c.ticket(t.id)?.ok_or_else(|| anyhow!("{key} disappeared"))?;
            let Some(n) = t.pr_number else { bail!("{key} has no {} to merge", f.pr_word()) };
            f.merge(&repo, n)?;
            landed(c, &t, Some(f), &format!("merged #{n} on {}", f.name()))?;
            out(&format!("Merged {key} (#{n}) on {}. New tickets start from it.", f.name()));
        }
    }
    // Only now, so the merge is reported before what it sets off.
    crate::work::release_waiting(c, t.id)
}

/// After the work is on base: bring your local base up to date, clean up, and only then mark the
/// ticket done. If OneCorp stops halfway, the ticket is still in review and the next sync finishes
/// the job, since every step here is safe to repeat.
fn landed(c: &Company, t: &Ticket, forge: Option<Forge>, what: &str) -> Result<()> {
    let repo = c.repo_of(t)?;
    if let (Some(wt), Some(branch)) = (&t.worktree, &t.branch) {
        if forge.is_some() {
            let base = t.base.clone().unwrap_or_else(|| "main".into());
            git::delete_remote_branch(&repo, branch);
            // Your local base follows the forge when it can. If you have local work on it, it waits for you.
            let updated = git::fetch(&repo, &base).and_then(|_| git::fast_forward(&repo, &base, &format!("origin/{base}")));
            if let Err(e) = updated {
                out(&format!("Your local {base} wasn't updated ({e:#}). Pull when you're ready."));
            }
        }
        // The work is safe on base now, so a failed cleanup is only worth a warning.
        if Path::new(wt).exists()
            && let Err(e) = git::clean_up(&repo, Path::new(wt), branch)
        {
            out(&format!("warning: merged, but couldn't remove the worktree: {e:#}"));
        }
    }
    c.set_status(t.id, "done")?;
    c.add_event(t.id, "merge", what, "")?;
    Ok(())
}

/// Picks up pull requests you merged or closed on the forge itself. Returns what changed.
pub fn sync(c: &Company) -> Result<Vec<String>> {
    if forge(c)?.is_none() {
        return Ok(vec![]);
    }
    let mut changed = vec![];
    for t in c.tickets()?.iter().filter(|t| t.status == "review") {
        let Some(n) = t.pr_number else { continue };
        let repo = c.repo_of(t)?;
        let Some(f) = forge_for(c, &repo)? else { continue };
        let key = c.key(t.id);
        match f.state(&repo, n)? {
            PrState::Open => {}
            PrState::Merged => {
                landed(c, t, Some(f), &format!("#{n} was merged on {}", f.name()))?;
                changed.push(format!("{key} was merged on {}", f.name()));
                crate::work::release_waiting(c, t.id)?;
            }
            PrState::Closed => {
                c.set_status(t.id, "failed")?;
                c.add_event(t.id, "stop", &format!("#{n} was closed on {} without merging", f.name()), "")?;
                changed.push(format!("{key}'s {} was closed. Retry it: onecorp retry {key}", f.pr_word()));
            }
        }
    }
    Ok(changed)
}
