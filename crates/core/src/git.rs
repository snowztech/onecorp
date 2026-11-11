use anyhow::{Result, bail};
use std::path::Path;
use std::process::{Command, Output};

type Env = [(String, String)];

// Parallel tickets share one repo, and git takes lock files (index.lock, packed-refs) while it
// works. One git command at a time inside OneCorp avoids "unable to lock" failures. They're fast.
static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn run(dir: &Path, args: &[&str], env: &Env) -> Result<Output> {
    let _guard = LOCK.lock().unwrap_or_else(|e| e.into_inner());
    Ok(Command::new("git").arg("-C").arg(dir).args(args).envs(env.iter().map(|(k, v)| (k, v))).output()?)
}

/// Runs git and returns trimmed stdout, or fails with git's own error message.
fn git(dir: &Path, args: &[&str], env: &Env) -> Result<String> {
    let out = run(dir, args, env)?;
    if !out.status.success() {
        bail!("git {} failed: {}", args.join(" "), String::from_utf8_lossy(&out.stderr).trim());
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn ok(dir: &Path, args: &[&str]) -> Result<bool> {
    Ok(run(dir, args, &[])?.status.success())
}

/// Creates a worktree on a fresh branch starting at `from` (a branch, or a remote one like
/// origin/main) and returns the branch name. If `branch` is already taken (an old company with the
/// same name, say), it tries branch-2, branch-3 and so on.
pub fn add_worktree(repo: &Path, path: &Path, branch: &str, from: &str) -> Result<String> {
    let mut name = branch.to_string();
    for n in 2.. {
        if !ok(repo, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{name}")])? {
            break;
        }
        name = format!("{branch}-{n}");
    }
    git(repo, &["worktree", "add", "-b", &name, &path.to_string_lossy(), from], &[])?;
    Ok(name)
}

/// Lets a ticket's worktree push only its own branch: a pre-push hook in a hooks folder that's this
/// worktree's alone, so your own checkout and other tickets are untouched. Deleting the branch on
/// the remote is refused too. It stops mistakes, not someone set on getting around it.
pub fn guard_pushes(repo: &Path, wt: &Path, branch: &str, hooks: &Path) -> Result<()> {
    // A core.worktree in the repo's shared config would have to move first. That's rare, so skip the guard then.
    if config(repo, "core.worktree").is_some() {
        return Ok(());
    }
    std::fs::create_dir_all(hooks)?;
    let hook = hooks.join("pre-push");
    std::fs::write(
        &hook,
        format!(
            "#!/bin/sh\n# OneCorp: this ticket pushes only its own branch.\nwhile read local_ref local_sha remote_ref remote_sha; do\n  \
             if [ \"$remote_ref\" != \"refs/heads/{branch}\" ] || [ \"$local_sha\" = \"0000000000000000000000000000000000000000\" ]; then\n    \
             echo \"OneCorp: push only {branch}. The founder's approval merges it into the base branch.\" >&2\n    exit 1\n  fi\ndone\nexit 0\n"
        ),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755))?;
    }
    git(repo, &["config", "extensions.worktreeConfig", "true"], &[])?;
    git(wt, &["config", "--worktree", "core.hooksPath", &hooks.to_string_lossy()], &[])?;
    Ok(())
}

/// Gives a new worktree the Rust builds already in your checkout, so a ticket doesn't recompile
/// every dependency from scratch. Each `target/` next to a tracked Cargo.toml is cloned with APFS
/// copy-on-write: instant, and no extra disk until something changes. Where cloning isn't
/// possible (not APFS, or not a Mac) nothing is copied, since a real copy can be gigabytes.
/// Returns the folders it warmed, like "core/target".
pub fn warm_builds(repo: &Path, wt: &Path) -> Vec<String> {
    let Ok(listed) = git(repo, &["ls-files", "--", "Cargo.toml", "*/Cargo.toml"], &[]) else { return vec![] };
    let mut warmed = vec![];
    for manifest in listed.lines() {
        let dir = Path::new(manifest).parent().unwrap_or(Path::new(""));
        let (from, to) = (repo.join(dir).join("target"), wt.join(dir).join("target"));
        // Only a target git ignores: the ticket's commit takes everything else in the worktree.
        // A path inside it, since `target/` in a .gitignore only matches a folder that exists.
        let ignored = ok(wt, &["check-ignore", "-q", &dir.join("target").join("x").to_string_lossy()]).unwrap_or(false);
        if !from.is_dir() || to.exists() || !wt.join(dir).is_dir() || !ignored {
            continue;
        }
        let cloned = std::process::Command::new("cp").arg("-cR").arg(&from).arg(&to).output().is_ok_and(|o| o.status.success());
        if cloned {
            warmed.push(dir.join("target").to_string_lossy().into_owned());
        } else {
            let _ = std::fs::remove_dir_all(&to);
        }
    }
    warmed
}

pub fn has_ref(repo: &Path, name: &str) -> bool {
    ok(repo, &["rev-parse", "--verify", "--quiet", name]).unwrap_or(false)
}

pub fn remote_url(repo: &Path, remote: &str) -> Option<String> {
    git(repo, &["remote", "get-url", remote], &[]).ok().filter(|u| !u.is_empty())
}

pub fn fetch(repo: &Path, branch: &str) -> Result<()> {
    git(repo, &["fetch", "--quiet", "origin", branch], &[])?;
    Ok(())
}

/// Pushes a ticket's branch to origin, creating it there the first time.
/// With a lease, since the employee may have rebased the branch: it's theirs, and the lease still
/// refuses to overwrite anything pushed since we last saw it.
pub fn push(wt: &Path, branch: &str) -> Result<()> {
    git(wt, &["push", "--quiet", "--force-with-lease", "-u", "origin", &format!("{branch}:{branch}")], &[])?;
    Ok(())
}

/// Best effort: the forge may already have deleted it after the merge.
pub fn delete_remote_branch(repo: &Path, branch: &str) {
    let _ = run(repo, &["push", "--quiet", "origin", "--delete", branch], &[]);
}

/// The branch new work starts from: the one origin points to, otherwise main or master, and only
/// then whatever is checked out. A feature branch you happen to be on is never the base.
pub fn default_branch(repo: &Path) -> Result<String> {
    let local = |b: &str| ok(repo, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{b}")]).unwrap_or(false);
    if let Ok(head) = git(repo, &["symbolic-ref", "--quiet", "--short", "refs/remotes/origin/HEAD"], &[])
        && let Some(b) = head.strip_prefix("origin/")
        && local(b)
    {
        return Ok(b.to_string());
    }
    for b in ["main", "master"] {
        if local(b) {
            return Ok(b.to_string());
        }
    }
    current_branch(repo)
}

pub fn current_branch(repo: &Path) -> Result<String> {
    match git(repo, &["symbolic-ref", "--quiet", "--short", "HEAD"], &[]) {
        Ok(b) => Ok(b),
        Err(_) => bail!("{} is on a detached HEAD. Check out a branch first", repo.display()),
    }
}

/// A git config value as git resolves it for this repo, like user.email. None when unset.
pub fn config(repo: &Path, key: &str) -> Option<String> {
    git(repo, &["config", "--get", key], &[]).ok().filter(|v| !v.is_empty())
}

/// The env that makes git commits show up under an employee's name.
pub fn identity(name: &str, email: &str) -> Vec<(String, String)> {
    ["GIT_AUTHOR_NAME", "GIT_COMMITTER_NAME"]
        .iter()
        .map(|k| (k.to_string(), name.to_string()))
        .chain(["GIT_AUTHOR_EMAIL", "GIT_COMMITTER_EMAIL"].iter().map(|k| (k.to_string(), email.to_string())))
        .collect()
}

/// Commits everything left in the worktree, including finishing a merge in progress.
/// Returns the short hash, or None if there was nothing to commit.
pub fn commit_all(wt: &Path, message: &str, identity: &Env) -> Result<Option<String>> {
    let merging = ok(wt, &["rev-parse", "--verify", "--quiet", "MERGE_HEAD"])?;
    if !merging && git(wt, &["status", "--porcelain"], identity)?.is_empty() {
        return Ok(None);
    }
    git(wt, &["add", "-A"], identity)?;
    // A merge keeps git's own "Merge branch 'main'" message, which says what it is.
    if merging {
        git(wt, &["commit", "-q", "--no-edit"], identity)?;
    } else {
        git(wt, &["commit", "-q", "-m", message], identity)?;
    }
    Ok(Some(git(wt, &["rev-parse", "--short", "HEAD"], identity)?))
}

pub enum Merge {
    Clean,
    /// The merge is left in progress so the employee can fix these files.
    Conflicts(Vec<String>),
}

/// Brings `base` into the ticket's worktree so the branch is up to date before it lands.
pub fn merge_base_into(wt: &Path, base: &str, identity: &Env) -> Result<Merge> {
    if run(wt, &["merge", "--no-edit", "-q", base], identity)?.status.success() {
        return Ok(Merge::Clean);
    }
    let files: Vec<String> = git(wt, &["diff", "--name-only", "--diff-filter=U"], identity)?.lines().map(String::from).collect();
    if files.is_empty() {
        // Failed for some other reason. Put the worktree back the way it was.
        let _ = run(wt, &["merge", "--abort"], &[]);
        bail!("couldn't merge {base} into the ticket branch");
    }
    Ok(Merge::Conflicts(files))
}

/// Moves `base` up to `branch`. Only a fast-forward: by now `branch` already contains `base`.
/// When `base` is checked out in the user's repo, git updates their files and refuses
/// rather than overwrite local changes.
pub fn fast_forward(repo: &Path, base: &str, branch: &str) -> Result<()> {
    if !ok(repo, &["merge-base", "--is-ancestor", base, branch])? {
        bail!("{branch} doesn't contain {base} yet");
    }
    if current_branch(repo).ok().as_deref() == Some(base) {
        if let Err(e) = git(repo, &["merge", "--ff-only", "-q", branch], &[]) {
            bail!("couldn't update {base} in {}. Commit or stash your local changes there, then approve again.\n{e}", repo.display());
        }
    } else {
        let old = git(repo, &["rev-parse", base], &[])?;
        let new = git(repo, &["rev-parse", branch], &[])?;
        // With the old value, the update fails instead of clobbering a ref that moved meanwhile.
        git(repo, &["update-ref", &format!("refs/heads/{base}"), &new, &old], &[])?;
    }
    Ok(())
}

/// One file a ticket changed. Binary files count as 0 lines.
pub struct Change {
    pub path: String,
    pub added: u32,
    pub removed: u32,
}

/// What `branch` changed since it left `base`. Three dots: later work on base doesn't show up here.
pub fn changes(repo: &Path, base: &str, branch: &str) -> Result<Vec<Change>> {
    let out = git(repo, &["diff", "--numstat", &format!("{base}...{branch}")], &[])?;
    Ok(out
        .lines()
        .filter_map(|l| {
            let mut parts = l.splitn(3, '\t');
            let (a, r, path) = (parts.next()?, parts.next()?, parts.next()?);
            Some(Change { path: path.to_string(), added: a.parse().unwrap_or(0), removed: r.parse().unwrap_or(0) })
        })
        .collect())
}

/// True when `a` and `b` hold the same files: a branch like that has nothing to merge.
pub fn same_files(repo: &Path, a: &str, b: &str) -> Result<bool> {
    Ok(git(repo, &["diff", "--name-only", a, b], &[])?.trim().is_empty())
}

/// True when everything on `branch` is already on `base`: merged, squashed in with the same
/// files, or never changed anything.
pub fn already_on(repo: &Path, base: &str, branch: &str) -> bool {
    git(repo, &["merge-base", "--is-ancestor", branch, base], &[]).is_ok() || same_files(repo, base, branch).unwrap_or(false)
}

/// (short hash, author, subject) for each commit on `branch` that isn't on `base`, oldest first.
pub fn commits(repo: &Path, base: &str, branch: &str) -> Result<Vec<(String, String, String)>> {
    let out = git(repo, &["log", "--reverse", "--format=%h%x09%an%x09%s", &format!("{base}..{branch}")], &[])?;
    Ok(out
        .lines()
        .filter_map(|l| {
            let mut p = l.splitn(3, '\t');
            Some((p.next()?.to_string(), p.next()?.to_string(), p.next()?.to_string()))
        })
        .collect())
}

/// The full patch, cut at `max` bytes so a huge change can't freeze the window. True when cut.
pub fn patch(repo: &Path, base: &str, branch: &str, max: usize) -> Result<(String, bool)> {
    let mut out = git(repo, &["diff", "--no-color", &format!("{base}...{branch}")], &[])?;
    if out.len() <= max {
        return Ok((out, false));
    }
    let cut = (0..=max).rev().find(|&i| out.is_char_boundary(i)).unwrap_or(0);
    out.truncate(cut);
    Ok((out, true))
}

/// Throws away a ticket's worktree and branch, whatever state they're in. For retrying from scratch.
/// `workspaces` is the company's own folder of worktrees: anything outside it is refused, so a
/// mistake can never delete your repo or anything else on disk.
pub fn discard(repo: &Path, wt: &Path, branch: &str, workspaces: &Path) {
    let inside = wt.canonicalize().ok().zip(workspaces.canonicalize().ok()).is_some_and(|(w, root)| w.starts_with(&root) && w != root);
    if !inside {
        return;
    }
    let _ = run(repo, &["worktree", "remove", "--force", &wt.to_string_lossy()], &[]);
    let _ = std::fs::remove_dir_all(wt);
    let _ = run(repo, &["worktree", "prune"], &[]);
    let _ = run(repo, &["branch", "-D", branch], &[]);
}

/// Removes a finished ticket's worktree and branch.
pub fn clean_up(repo: &Path, wt: &Path, branch: &str) -> Result<()> {
    git(repo, &["worktree", "remove", &wt.to_string_lossy()], &[])?;
    git(repo, &["branch", "-D", branch], &[])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_ticket_worktree_pushes_only_its_own_branch() {
        let root = std::env::temp_dir().join(format!("onecorp-guard-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let (remote, repo, wt, hooks) = (root.join("remote.git"), root.join("repo"), root.join("wt"), root.join("hooks"));
        let me = identity("Test", "test@example.com");
        let sh = |dir: &Path, args: &[&str]| git(dir, args, &me);
        std::fs::create_dir_all(&remote).unwrap();
        std::fs::create_dir_all(&repo).unwrap();
        sh(&remote, &["init", "-q", "--bare", "-b", "main"]).unwrap();
        sh(&repo, &["init", "-q", "-b", "main"]).unwrap();
        std::fs::write(repo.join("a.txt"), "one\n").unwrap();
        commit_all(&repo, "init", &me).unwrap();
        sh(&repo, &["remote", "add", "origin", &remote.to_string_lossy()]).unwrap();
        sh(&repo, &["push", "-q", "origin", "main"]).unwrap();

        let branch = add_worktree(&repo, &wt, "onecorp/acme/acm-1", "main").unwrap();
        guard_pushes(&repo, &wt, &branch, &hooks).unwrap();
        std::fs::write(wt.join("a.txt"), "two\n").unwrap();
        commit_all(&wt, "work", &me).unwrap();

        assert!(push(&wt, &branch).is_ok(), "its own branch, even with a lease");
        assert!(sh(&wt, &["push", "-q", "origin", &format!("{branch}:main")]).is_err(), "never the base branch");
        assert!(sh(&wt, &["push", "-q", "origin", "--delete", &branch]).is_err(), "nor deleting its own on the remote");
        // Your own checkout pushes as before: the guard is that worktree's alone.
        std::fs::write(repo.join("b.txt"), "mine\n").unwrap();
        commit_all(&repo, "yours", &me).unwrap();
        assert!(sh(&repo, &["push", "-q", "origin", "main"]).is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn merges_fast_forwards_and_reports_conflicts() {
        let root = std::env::temp_dir().join(format!("onecorp-git-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let me = identity("Test", "test@example.com");
        let sh = |dir: &Path, args: &[&str]| git(dir, args, &me).unwrap();
        let write = |p: &Path, s: &str| std::fs::write(p, s).unwrap();

        sh(&repo, &["init", "-q", "-b", "main"]);
        write(&repo.join("a.txt"), "one\n");
        commit_all(&repo, "init", &me).unwrap();

        // Ticket 1 changes a.txt while main stays put: clean merge and a fast-forward.
        let wt1 = root.join("wt1");
        let b1 = add_worktree(&repo, &wt1, "t/1", "main").unwrap();
        write(&wt1.join("a.txt"), "two\n");
        commit_all(&wt1, "t1", &me).unwrap();
        assert!(matches!(merge_base_into(&wt1, "main", &me).unwrap(), Merge::Clean));
        fast_forward(&repo, "main", &b1).unwrap();
        assert_eq!(std::fs::read_to_string(repo.join("a.txt")).unwrap(), "two\n");
        clean_up(&repo, &wt1, &b1).unwrap();
        assert!(!wt1.exists());

        // Discarding refuses anything outside the worktrees folder, like the repo itself.
        let workspaces = root.join("wt");
        std::fs::create_dir_all(&workspaces).unwrap();
        discard(&repo, &repo, "main", &workspaces);
        assert!(repo.join("a.txt").exists(), "the repo must survive a discard aimed at it");

        // A new worktree gets the checkout's Rust builds, cloned where the disk allows it.
        write(&repo.join("Cargo.toml"), "[package]\n");
        write(&repo.join(".gitignore"), "target/\n");
        commit_all(&repo, "rust", &me).unwrap();
        std::fs::create_dir_all(repo.join("target/debug")).unwrap();
        write(&repo.join("target/debug/built"), "yes");
        let wt3 = root.join("wt3");
        add_worktree(&repo, &wt3, "t/3", "main").unwrap();
        let warmed = warm_builds(&repo, &wt3);
        if cfg!(target_os = "macos") {
            assert_eq!(warmed, ["target"]);
            assert_eq!(std::fs::read_to_string(wt3.join("target/debug/built")).unwrap(), "yes");
        }
        assert!(warm_builds(&repo, &wt3).is_empty(), "never twice");

        // Ticket 2 started before a conflicting change landed on main.
        let wt2 = root.join("wt2");
        let b2 = add_worktree(&repo, &wt2, "t/2", "main").unwrap();
        write(&wt2.join("a.txt"), "ticket\n");
        commit_all(&wt2, "t2", &me).unwrap();
        write(&repo.join("a.txt"), "main\n");
        commit_all(&repo, "main moved", &me).unwrap();

        // What the reviewer sees: only the ticket's own change, even though main moved since.
        let ch = changes(&repo, "main", &b2).unwrap();
        assert_eq!((ch.len(), ch[0].path.as_str(), ch[0].added, ch[0].removed), (1, "a.txt", 1, 1));
        assert_eq!(commits(&repo, "main", &b2).unwrap().iter().map(|c| c.2.as_str()).collect::<Vec<_>>(), vec!["t2"]);
        let (p, cut) = patch(&repo, "main", &b2, 1_000_000).unwrap();
        assert!(p.contains("+ticket") && !cut);
        assert!(patch(&repo, "main", &b2, 10).unwrap().1);

        let Merge::Conflicts(files) = merge_base_into(&wt2, "main", &me).unwrap() else { panic!("expected a conflict") };
        assert_eq!(files, vec!["a.txt".to_string()]);
        assert!(fast_forward(&repo, "main", &b2).is_err(), "must not land before the conflict is fixed");

        // The employee fixes the file, OneCorp finishes the merge, and now it lands.
        write(&wt2.join("a.txt"), "main and ticket\n");
        assert!(commit_all(&wt2, "resolve", &me).unwrap().is_some());
        assert!(sh(&wt2, &["log", "-1", "--format=%s"]).starts_with("Merge branch 'main'"));
        fast_forward(&repo, "main", &b2).unwrap();
        assert_eq!(std::fs::read_to_string(repo.join("a.txt")).unwrap(), "main and ticket\n");

        let _ = std::fs::remove_dir_all(&root);
    }
}
