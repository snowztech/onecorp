//! The build and test commands employees may run in their worktree, so work comes back tested.
//! Claude Code only gets read-only git plus this list. Codex runs commands in its own sandbox.
//!
//! Worked out from the repo (Cargo.toml means cargo, a pnpm lockfile means pnpm...) until you
//! set your own list in Settings.

use crate::store::{Company, Repo, Ticket};
use anyhow::Result;
use std::path::Path;

/// What the repo looks like it builds and tests with, read from the files at its root.
/// Each entry is a prefix: "cargo test" also allows "cargo test --lib".
pub fn detect(repo: &Path) -> Vec<String> {
    let names = names_in(repo);
    let has = |f: &str| names.iter().any(|n| n == f);
    let ends = |e: &str| names.iter().any(|n| n.ends_with(e));
    let mut out: Vec<String> = vec![];
    let mut add = |cmds: &[&str]| out.extend(cmds.iter().map(|s| s.to_string()));

    if has("Cargo.toml") {
        add(&["cargo build", "cargo test", "cargo check", "cargo clippy", "cargo fmt"]);
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
        add(&[&format!("{pm} install"), &format!("{pm} test"), &format!("{pm} run")]);
    }
    if has("deno.json") || has("deno.jsonc") {
        add(&["deno test", "deno task", "deno check", "deno lint", "deno fmt"]);
    }
    if has("go.mod") {
        add(&["go build", "go test", "go vet"]);
    }
    if python(repo, &names) {
        if has("uv.lock") {
            add(&["uv sync", "uv run"]);
        }
        if has("poetry.lock") {
            add(&["poetry install", "poetry run"]);
        }
        if has("Pipfile") {
            add(&["pipenv install", "pipenv run"]);
        }
        if names.iter().any(|n| n.starts_with("requirements") && n.ends_with(".txt")) {
            add(&["pip install -r", "python -m pip install -r", "python3 -m pip install -r"]);
        }
        if has("tox.ini") {
            add(&["tox"]);
        }
        // macOS has python3 but no python, so both spellings.
        add(&["pytest", "python -m pytest", "python3 -m pytest", "python -m unittest", "python3 -m unittest"]);
    }
    if has("Gemfile") {
        add(&["bundle install", "bundle exec", "rake"]);
    }
    if has("pom.xml") {
        add(&[if has("mvnw") { "./mvnw" } else { "mvn" }]);
    }
    if ["build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts"].iter().any(|f| has(f)) {
        add(&[if has("gradlew") { "./gradlew" } else { "gradle" }]);
    }
    if has("build.sbt") {
        add(&["sbt"]);
    }
    if ends(".sln") || ends(".csproj") || ends(".fsproj") {
        add(&["dotnet restore", "dotnet build", "dotnet test", "dotnet format"]);
    }
    if has("Package.swift") {
        add(&["swift build", "swift test"]);
    }
    if ends(".xcodeproj") || ends(".xcworkspace") {
        add(&["xcodebuild"]);
    }
    if has("mix.exs") {
        add(&["mix deps.get", "mix compile", "mix test", "mix format"]);
    }
    if has("rebar.config") {
        add(&["rebar3"]);
    }
    if has("composer.json") {
        add(&["composer install", "composer test", "vendor/bin/phpunit", "vendor/bin/pest"]);
    }
    if has("pubspec.yaml") {
        let flutter = std::fs::read_to_string(repo.join("pubspec.yaml")).is_ok_and(|s| s.contains("flutter:"));
        let tool = if flutter { "flutter" } else { "dart" };
        add(&[&format!("{tool} pub get"), &format!("{tool} test"), &format!("{tool} analyze")]);
    }
    if has("build.zig") {
        add(&["zig build"]);
    }
    if has("CMakeLists.txt") {
        add(&["cmake", "ctest"]);
    }
    if has("meson.build") {
        add(&["meson", "ninja"]);
    }
    if has("stack.yaml") {
        add(&["stack build", "stack test"]);
    } else if ends(".cabal") {
        add(&["cabal build", "cabal test"]);
    }
    if has("dune-project") {
        add(&["dune build", "dune test"]);
    }
    if has("deps.edn") {
        add(&["clojure"]);
    }
    if has("project.clj") {
        add(&["lein"]);
    }
    if has("Makefile") || has("makefile") || has("GNUmakefile") {
        add(&["make"]);
    }
    if has("justfile") || has("Justfile") {
        add(&["just"]);
    }
    if has("Taskfile.yml") || has("Taskfile.yaml") {
        add(&["task"]);
    }
    let mut seen = vec![];
    out.retain(|c| !seen.contains(c) && { seen.push(c.clone()); true });
    out
}

fn names_in(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir).map(|d| d.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default()
}

// A Python project file, or just Python files: a repo of scripts with a test_x.py is still tested with pytest.
fn python(repo: &Path, names: &[String]) -> bool {
    let project = ["pyproject.toml", "setup.py", "setup.cfg", "Pipfile", "tox.ini", "uv.lock", "poetry.lock", "conftest.py"];
    names.iter().any(|n| project.contains(&n.as_str()) || (n.starts_with("requirements") && n.ends_with(".txt")) || n.ends_with(".py"))
        || ["tests", "test"].iter().any(|d| names_in(&repo.join(d)).iter().any(|n| n.ends_with(".py")))
}

// The main repo keeps the plain key, so lists saved before there were several repos still apply.
fn key(r: &Repo) -> String {
    if r.main { "commands".into() } else { format!("commands.{}", r.name) }
}

/// A repo's list: yours if you set one, otherwise what the repo looks like.
pub fn list(c: &Company, r: &Repo) -> Result<Vec<String>> {
    match c.setting(&key(r))? {
        Some(s) => Ok(parse(&s)),
        None => Ok(detect(&r.path)),
    }
}

/// Whether the list is one you set, rather than what the repo suggests.
pub fn custom(c: &Company, r: &Repo) -> Result<bool> {
    Ok(c.setting(&key(r))?.is_some())
}

/// The list for a ticket's own repo.
pub fn for_ticket(c: &Company, t: &Ticket) -> Result<Vec<String>> {
    list(c, &repo_of(c, t)?)
}

pub fn repo_of(c: &Company, t: &Ticket) -> Result<Repo> {
    match &t.repo {
        Some(name) => c.repo_named(name),
        None => Ok(c.repos()?.remove(0)),
    }
}

/// Adds a command to the ticket's repo, for everyone who works there from now on.
pub fn allow(c: &Company, t: &Ticket, cmd: &str) -> Result<()> {
    let r = repo_of(c, t)?;
    let mut all = list(c, &r)?;
    // A request can hold several commands, one per line.
    all.extend(cmd.lines().map(str::trim).filter(|l| !l.is_empty() && !all.iter().any(|a| a == l)).map(String::from).collect::<Vec<_>>());
    set(c, &r, &all.join("\n"))
}

/// Why a command can never be allowed, whoever asks: it reaches past the worktree, or it's
/// OneCorp's job. None when it's fine to ask the founder.
pub fn never(cmd: &str) -> Option<&'static str> {
    let first = cmd.split_whitespace().next().unwrap_or("");
    match first {
        "git" => Some("OneCorp does the git work: commits, branches and pushes. You can read with git status, diff and log"),
        "sudo" | "su" | "doas" => Some("nothing runs as root"),
        "rm" if cmd.contains("-r") || cmd.contains(" /") || cmd.contains("~") => Some("deleting folders is too easy to get wrong. Delete files one by one with your edit tools"),
        "gh" | "glab" => Some("OneCorp opens and merges pull requests"),
        _ => None,
    }
}

/// Saves your own list. One command per line or comma. An empty list means none at all.
/// Commands that could never run, like `git rebase`, are left out.
pub fn set(c: &Company, r: &Repo, text: &str) -> Result<()> {
    let keep: Vec<String> = parse(text).into_iter().filter(|cmd| never(cmd).is_none()).collect();
    c.set_setting(&key(r), &keep.join("\n"))
}

/// Looks at the repo again: what its files suggest now is added to your list, and yours stays.
pub fn rescan(c: &Company, r: &Repo) -> Result<()> {
    if !custom(c, r)? {
        return Ok(());
    }
    let mut all = list(c, r)?;
    for cmd in detect(&r.path) {
        if !all.contains(&cmd) {
            all.push(cmd);
        }
    }
    set(c, r, &all.join("\n"))
}

/// Whether the team runs any command in its worktrees, or only the listed ones. New companies run
/// anything: you approve the work at review, and each ticket has its own copy of the repo.
pub fn run_any(c: &Company) -> Result<bool> {
    Ok(c.setting("run")?.as_deref() == Some("anything"))
}

pub fn set_run_any(c: &Company, on: bool) -> Result<()> {
    c.set_setting("run", if on { "anything" } else { "listed" })
}

/// Back to what the repo looks like.
pub fn reset(c: &Company, r: &Repo) -> Result<()> {
    c.delete_setting(&key(r))
}

/// Also takes a list pasted from a Claude Code settings file: `"Bash(make build:*)",` is `make build`.
/// Git reads are always allowed, so they're left out.
fn parse(text: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![];
    for item in text.split(['\n', ',']) {
        let mut cmd = item.trim().trim_matches(|ch| ch == '"' || ch == '\'' || ch == '`').trim();
        if let Some(inner) = cmd.strip_prefix("Bash(").and_then(|s| s.strip_suffix(')')) {
            cmd = inner.trim_end_matches('*').trim_end_matches(':').trim();
        }
        let cmd = cmd.split_whitespace().collect::<Vec<_>>().join(" ");
        if ["git status", "git diff", "git log"].contains(&cmd.as_str()) {
            continue;
        }
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

    #[test]
    fn knows_the_usual_ecosystems() {
        let repo = |name: &str, files: &[&str]| {
            let dir = std::env::temp_dir().join(format!("onecorp-detect-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            for f in files {
                let path = dir.join(f);
                std::fs::create_dir_all(path.parent().unwrap()).unwrap();
                std::fs::write(path, if *f == "pubspec.yaml" { "dependencies:\n  flutter:\n" } else { "" }).unwrap();
            }
            let found = detect(&dir);
            let _ = std::fs::remove_dir_all(&dir);
            found
        };
        // Loose Python files with no project file, the case a first user hit.
        assert!(repo("loose", &["calc.py", "test_calc.py"]).contains(&"pytest".to_string()));
        assert!(repo("testsdir", &["tests/test_api.py"]).contains(&"pytest".to_string()));
        assert!(repo("uv", &["pyproject.toml", "uv.lock"]).starts_with(&["uv sync".to_string(), "uv run".to_string()]));
        assert_eq!(repo("gradle", &["build.gradle.kts", "gradlew"]), ["./gradlew"]);
        assert_eq!(repo("maven", &["pom.xml"]), ["mvn"]);
        assert!(repo("dotnet", &["App.csproj"]).contains(&"dotnet test".to_string()));
        assert!(repo("ruby", &["Gemfile"]).contains(&"bundle exec".to_string()));
        assert!(repo("elixir", &["mix.exs"]).contains(&"mix test".to_string()));
        assert!(repo("php", &["composer.json"]).contains(&"vendor/bin/phpunit".to_string()));
        assert!(repo("flutter", &["pubspec.yaml"]).contains(&"flutter test".to_string()));
        assert!(repo("swift", &["Package.swift"]).contains(&"swift test".to_string()));
        assert_eq!(repo("make", &["Makefile", "justfile"]), ["make", "just"]);

        assert!(repo("empty", &["README.md"]).is_empty());
    }

    #[test]
    fn some_commands_are_never_allowed() {
        assert!(never("git push origin main").is_some() && never("sudo make install").is_some() && never("rm -rf build").is_some());
        assert!(never("make sqlc").is_none() && never("xcodegen generate").is_none() && never("rm build.log").is_none());
    }

    #[test]
    fn a_pasted_claude_list_reads_as_commands() {
        let pasted = "\"Bash(make build:*)\",\n\"Bash(npm ci:)\", Bash(xcodegen generate)\nBash(git status:*)\nnpm run lint\nmake build";
        assert_eq!(parse(pasted), ["make build", "npm ci", "xcodegen generate", "npm run lint"]);
    }
}
