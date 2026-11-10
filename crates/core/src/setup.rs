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
    if let Some(notes) = repo_notes(&c.repo()?) {
        c.set_wiki_section("Codebase", &notes)?;
    }
    let hired = templates::hire(&c, people)?;
    crate::store::set_current(&c.name)?;
    if hired.iter().any(|e| e.runtime != "fake") {
        crate::work::write_wiki(&c, None)?;
    }
    if !brief.trim().is_empty() {
        c.write_brief(brief)?;
        if !hired.is_empty() {
            crate::work::kickoff(&c, None)?;
        }
    }
    Ok((c, hired))
}

/// Deletes a company: its tickets, chat, wiki and memory, everything OneCorp keeps for it.
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

/// What a newcomer would want to know about a repo, read from its files: the stack, how to build
/// and test it, how it's laid out, and what its README says. Free and local, no AI. None for a
/// repo with nothing to say yet, like a brand new one.
pub fn repo_notes(repo: &Path) -> Option<String> {
    let has = |f: &str| repo.join(f).exists();
    let read = |f: &str| std::fs::read_to_string(repo.join(f)).unwrap_or_default();
    let mut stack = vec![];
    for (file, name) in [("Cargo.toml", "Rust"), ("go.mod", "Go"), ("pyproject.toml", "Python"), ("requirements.txt", "Python"), ("Gemfile", "Ruby"),
        ("pom.xml", "Java"), ("build.gradle", "Java or Kotlin"), ("build.gradle.kts", "Kotlin"), ("Package.swift", "Swift"), ("composer.json", "PHP"), ("mix.exs", "Elixir"), ("deno.json", "Deno")] {
        if has(file) && !stack.contains(&name) {
            stack.push(name);
        }
    }
    if has("package.json") {
        stack.push(if has("tsconfig.json") { "TypeScript" } else { "JavaScript" });
    }

    let mut commands: Vec<String> = vec![];
    for line in read("Makefile").lines() {
        let Some((target, _)) = line.split_once(':') else { continue };
        if !target.is_empty() && !target.starts_with(['.', '\t', ' ', '#']) && target.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_') && !line.contains(":=") {
            commands.push(format!("make {target}"));
        }
    }
    if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&read("package.json"))
        && let Some(scripts) = pkg["scripts"].as_object()
    {
        let npm = if has("pnpm-lock.yaml") { "pnpm" } else if has("yarn.lock") { "yarn" } else if has("bun.lockb") { "bun" } else { "npm run" };
        commands.extend(scripts.keys().map(|k| format!("{npm} {k}")));
    }
    if has("Cargo.toml") && commands.is_empty() {
        commands.extend(["cargo build".to_string(), "cargo test".to_string()]);
    }
    commands.truncate(10);

    let mut folders: Vec<String> = std::fs::read_dir(repo)
        .map(|d| d.flatten().filter(|e| e.path().is_dir()).map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| !n.starts_with('.') && !["node_modules", "target", "dist", "build", "vendor"].contains(&n.as_str())).collect())
        .unwrap_or_default();
    folders.sort();
    folders.truncate(12);

    let readme = ["README.md", "readme.md", "README"].iter().map(|f| read(f)).find(|t| !t.is_empty()).unwrap_or_default();
    let about: String = readme
        .split("\n\n")
        .map(str::trim)
        .find(|p| !p.is_empty() && !p.starts_with(['#', '!', '[', '<', '`', '|', '-']))
        .unwrap_or("")
        .replace('\n', " ")
        .chars()
        .take(400)
        .collect();

    let mut out = vec![];
    if !stack.is_empty() {
        out.push(format!("- Stack: {}", stack.join(", ")));
    }
    if !commands.is_empty() {
        out.push(format!("- Build and test with: {}", commands.iter().map(|c| format!("`{c}`")).collect::<Vec<_>>().join(", ")));
    }
    if !folders.is_empty() {
        out.push(format!("- Layout: {}", folders.iter().map(|f| format!("`{f}/`")).collect::<Vec<_>>().join(", ")));
    }
    if !about.is_empty() {
        out.push(format!("- The README says: {about}"));
    }
    (!out.is_empty()).then(|| out.join("\n"))
}

/// Where new repos go by default: ~/dev when it exists, otherwise your home folder.
/// Clones one of your GitHub repos next to your other projects, or uses the folder if it's
/// already there. Returns where it is.
pub fn clone_from_github(full_name: &str, dest: Option<&Path>) -> Result<PathBuf> {
    let dest = dest.map(Path::to_path_buf).unwrap_or_else(|| clone_target(full_name));
    if dest.exists() {
        // Only the same repo counts as already there. Anything else in that folder stays untouched.
        if is_clone_of(&dest, full_name) {
            return Ok(dest);
        }
        bail!("{} already exists and isn't {full_name}. Pick another folder", dest.display());
    }
    crate::forge::Forge::GitHub.clone_repo(full_name, &dest)?;
    Ok(dest)
}

/// Where a clone goes by default: projects/app, or projects/owner-app when that folder holds
/// something else.
pub fn clone_target(full_name: &str) -> PathBuf {
    clone_target_in(&default_parent(), full_name)
}

fn clone_target_in(parent: &Path, full_name: &str) -> PathBuf {
    let (owner, name) = full_name.split_once('/').unwrap_or(("", full_name));
    let plain = parent.join(name);
    if !plain.exists() || is_clone_of(&plain, full_name) {
        return plain;
    }
    parent.join(format!("{owner}-{name}"))
}

/// Whether a folder is a clone of owner/name, from its origin remote.
fn is_clone_of(dir: &Path, full_name: &str) -> bool {
    let want = full_name.to_lowercase();
    crate::git::remote_url(dir, "origin")
        .map(|u| u.to_lowercase().trim_end_matches(".git").trim_end_matches('/').ends_with(&want))
        .unwrap_or(false)
}

/// Where new and cloned repos go: the folder you picked, or the one that already holds your projects.
pub fn default_parent() -> PathBuf {
    if let Ok(saved) = std::fs::read_to_string(projects_file())
        && Path::new(saved.trim()).is_dir()
    {
        return PathBuf::from(saved.trim());
    }
    guess_parent(&PathBuf::from(std::env::var("HOME").unwrap_or_default()))
}

/// Remembers where you keep your projects, for every clone and new repo after this one.
pub fn set_default_parent(dir: &Path) -> Result<PathBuf> {
    let dir = dir.canonicalize().with_context(|| format!("{} doesn't exist", dir.display()))?;
    anyhow::ensure!(dir.is_dir(), "{} isn't a folder", dir.display());
    std::fs::create_dir_all(crate::store::home())?;
    std::fs::write(projects_file(), dir.to_string_lossy().as_bytes())?;
    Ok(dir)
}

fn projects_file() -> PathBuf {
    crate::store::home().join("projects-folder")
}

/// The usual places people keep code, picking the one with the most git repos in it already.
/// The first that exists wins a tie, and home is the fallback.
fn guess_parent(home: &Path) -> PathBuf {
    const USUAL: [&str; 9] = ["Developer", "dev", "code", "Code", "Projects", "projects", "src", "workspace", "repos"];
    let repos_in = |d: &Path| std::fs::read_dir(d).map(|it| it.flatten().take(500).filter(|e| e.path().join(".git").exists()).count()).unwrap_or(0);
    USUAL
        .iter()
        .map(|n| home.join(n))
        .filter(|d| d.is_dir())
        .map(|d| (repos_in(&d), d))
        .fold(None::<(usize, PathBuf)>, |best, (n, d)| match best {
            Some((b, _)) if b >= n => best,
            _ => Some((n, d)),
        })
        .map(|(_, d)| d)
        .unwrap_or_else(|| home.to_path_buf())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clones_never_land_on_another_repo() {
        let dir = std::env::temp_dir().join(format!("onecorp-clone-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(clone_target_in(&dir, "tvcask/app"), dir.join("app"), "a free folder keeps the plain name");
        let other = dir.join("app");
        std::fs::create_dir_all(&other).unwrap();
        let git = |args: &[&str]| assert!(Command::new("git").arg("-C").arg(&other).args(args).status().unwrap().success());
        git(&["init", "-q"]);
        git(&["remote", "add", "origin", "git@github.com:someone/app.git"]);
        assert_eq!(clone_target_in(&dir, "tvcask/app"), dir.join("tvcask-app"), "someone else's app is left alone");
        assert_eq!(clone_target_in(&dir, "someone/app"), dir.join("app"), "the same repo is used as it is");
        assert!(clone_from_github("tvcask/app", Some(&other)).is_err(), "cloning onto another repo is refused");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn guesses_where_your_projects_live() {
        let home = std::env::temp_dir().join(format!("onecorp-home-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(&home).unwrap();
        assert_eq!(guess_parent(&home), home, "nothing usual: home");
        std::fs::create_dir_all(home.join("dev")).unwrap();
        std::fs::create_dir_all(home.join("code/a/.git")).unwrap();
        std::fs::create_dir_all(home.join("code/b/.git")).unwrap();
        assert_eq!(guess_parent(&home), home.join("code"), "the folder with your repos, not just the first that exists");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn reads_a_repo_like_a_newcomer() {
        let dir = std::env::temp_dir().join(format!("onecorp-notes-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        std::fs::create_dir_all(dir.join("node_modules")).unwrap();
        std::fs::write(dir.join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        std::fs::write(dir.join("Makefile"), ".PHONY: test\nVERSION := 1\ntest:\n\tcargo test\nrelease-build: test\n").unwrap();
        std::fs::write(dir.join("README.md"), "# X\n\n![badge](b.svg)\n\nA tiny tool that\nsyncs notes.\n\n## More\n").unwrap();

        let notes = repo_notes(&dir).unwrap();
        assert!(notes.contains("Stack: Rust"));
        assert!(notes.contains("`make test`, `make release-build`") && !notes.contains("VERSION") && !notes.contains("PHONY"));
        assert!(notes.contains("`src/`") && !notes.contains("node_modules"));
        assert!(notes.contains("The README says: A tiny tool that syncs notes."));

        let empty = dir.join("empty");
        std::fs::create_dir_all(&empty).unwrap();
        assert!(repo_notes(&empty).is_none(), "a new repo has nothing to say yet");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
