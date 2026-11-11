//! The company wiki: pages in spaces, like Product/Customers, each a Markdown file in the
//! company's wiki folder. Every save is logged with who made it and what it saved, so a page
//! shows who wrote it, and any earlier version can come back.

use crate::store::Company;
use anyhow::{Result, bail};
use rusqlite::params;
use std::fs;
use std::path::PathBuf;

/// The spaces OneCorp starts with, in the order they show. You can add more.
pub const SPACES: [&str; 3] = ["Company", "Product", "Engineering"];
/// Why we exist, what we're building and where we are: what a new hire reads first.
pub const OVERVIEW: &str = "Company/Overview";
/// Its sections, in order. What we're building is the brief the planner works from.
pub const WHY: &str = "Why we exist";
pub const BUILDING: &str = "What we're building";
pub const WHERE: &str = "Where we are";
pub const NOW: &str = "Company/Now";
pub const DECISIONS: &str = "Company/Decisions";

/// The pages employees draft from the repo, and what goes in each.
pub const DRAFTS: [(&str, &str); 8] = [
    (OVERVIEW, "three short sections. ## Why we exist: a line or two. ## What we're building: the product, who it's for and what it does today, in a short paragraph. ## Where we are: the stage it's at, like an idea, in development, in beta or live, and what's shipped, from the README, changelog, version and deploy setup. Sections the founder already wrote are kept as they are."),
    ("Product/Customers", "who uses it, what they're trying to get done, and what they care about."),
    ("Product/Rules", "rules that are easy to break without knowing: limits, money, privacy, legal, anything users were promised."),
    ("Product/Voice", "how the product talks to people: tone, words it uses and avoids, languages."),
    ("Product/Glossary", "words that mean something specific here, one line each, from the code, data model and app copy. Like what counts as a customer, a booking or a reminder. Only terms a newcomer could get wrong."),
    ("Engineering/Codebase", "what isn't obvious from a quick look: the stack in a line, where the important pieces live, how a request or job flows, outside services. Not a folder-by-folder list."),
    ("Engineering/Build and test", "the exact commands to set up, run, test and lint, and what to run before calling work done."),
    ("Engineering/Conventions", "what the code already does: naming, structure, style, testing. Only what's really there."),
];

pub struct Page {
    pub path: String,
    pub text: String,
}

impl Page {
    pub fn space(&self) -> &str {
        self.path.split_once('/').map_or("", |(s, _)| s)
    }
    pub fn title(&self) -> &str {
        self.path.split_once('/').map_or(&self.path, |(_, t)| t)
    }
}

/// One save of a page: who, why, when, and the text it saved. Empty text means it was deleted.
pub struct Edit {
    pub id: i64,
    pub page: String,
    pub author: String,
    pub note: String,
    pub at: String,
    pub text: String,
}

pub fn dir(c: &Company) -> PathBuf {
    c.dir.join("wiki")
}

/// "Product / Customers" as "Product/Customers". Refuses anything that isn't a space and a title,
/// so a page can never land outside the wiki folder.
pub fn clean(path: &str) -> Result<String> {
    let Some((space, title)) = path.split_once('/') else { bail!("a page is a space and a title, like Product/Customers") };
    let (space, title) = (space.trim(), title.trim().trim_end_matches(".md").trim());
    let ok = |s: &str| !s.is_empty() && s.chars().count() <= 60 && !s.starts_with('.') && !s.contains(['/', '\\', ':']) && !s.chars().any(char::is_control);
    if !ok(space) || !ok(title) {
        bail!("\"{path}\" isn't a page name. Use a space and a title, like Product/Customers");
    }
    Ok(format!("{space}/{title}"))
}

fn file(c: &Company, path: &str) -> PathBuf {
    dir(c).join(format!("{path}.md"))
}

/// Spaces in their usual order, then the rest by name. In a space, its main pages come first.
fn rank(path: &str) -> (usize, String, usize, String) {
    let (space, title) = path.split_once('/').unwrap_or(("", path));
    let s = SPACES.iter().position(|x| *x == space).unwrap_or(SPACES.len());
    let order = [OVERVIEW, NOW, DECISIONS].into_iter().chain(DRAFTS.iter().map(|(p, _)| *p)).position(|p| p == path).unwrap_or(99);
    (s, space.to_lowercase(), order, title.to_lowercase())
}

pub fn pages(c: &Company) -> Result<Vec<Page>> {
    let mut out = vec![];
    let Ok(spaces) = fs::read_dir(dir(c)) else { return Ok(out) };
    for space in spaces.flatten().filter(|e| e.path().is_dir()) {
        for f in fs::read_dir(space.path())?.flatten() {
            let p = f.path();
            if p.extension().is_some_and(|e| e == "md") {
                let path = format!("{}/{}", space.file_name().to_string_lossy(), p.file_stem().unwrap_or_default().to_string_lossy());
                out.push(Page { path, text: fs::read_to_string(&p)?.trim().to_string() });
            }
        }
    }
    out.sort_by_key(|p| rank(&p.path));
    Ok(out)
}

/// A page's text, or empty when there's no such page.
pub fn read(c: &Company, path: &str) -> Result<String> {
    match fs::read_to_string(file(c, &clean(path)?)) {
        Ok(s) => Ok(s.trim().to_string()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e.into()),
    }
}

pub fn exists(c: &Company, path: &str) -> bool {
    clean(path).is_ok_and(|p| file(c, &p).exists())
}

/// Saves a page as `by`: "founder", an employee's id, or "onecorp". An unchanged page isn't
/// logged again.
pub fn write(c: &Company, path: &str, text: &str, by: &str, note: &str) -> Result<()> {
    let path = clean(path)?;
    let text = text.trim();
    if exists(c, &path) && read(c, &path)? == text {
        return Ok(());
    }
    let f = file(c, &path);
    fs::create_dir_all(f.parent().expect("a page lives in its space's folder"))?;
    fs::write(&f, if text.is_empty() { String::new() } else { format!("{text}\n") })?;
    log(c, &path, by, note, text)
}

/// Adds a line to the end of a page as a bullet, creating the page if it's new.
pub fn append(c: &Company, path: &str, line: &str, by: &str, note: &str) -> Result<()> {
    let line = line.trim();
    if line.is_empty() {
        bail!("write what everyone should know first");
    }
    let bullet = if line.starts_with("- ") { line.to_string() } else { format!("- {line}") };
    let now = read(c, path)?;
    write(c, path, &if now.is_empty() { bullet } else { format!("{now}\n{bullet}") }, by, note)
}

/// Makes an empty page, for the founder to fill in. Nothing is logged: nobody wrote it yet.
pub fn seed(c: &Company, path: &str) -> Result<()> {
    let f = file(c, &clean(path)?);
    if !f.exists() {
        fs::create_dir_all(f.parent().expect("a page lives in its space's folder"))?;
        fs::write(f, "")?;
    }
    Ok(())
}

pub fn remove(c: &Company, path: &str, by: &str) -> Result<()> {
    let path = clean(path)?;
    let f = file(c, &path);
    if !f.exists() {
        bail!("there's no page called {path}");
    }
    fs::remove_file(&f)?;
    // An empty space goes with its last page.
    if let Some(space) = f.parent() {
        let _ = fs::remove_dir(space);
    }
    log(c, &path, by, "deleted the page", "")
}

/// Moves a page to a new name or space, keeping its history.
pub fn rename(c: &Company, from: &str, to: &str, by: &str) -> Result<()> {
    let (from, to) = (clean(from)?, clean(to)?);
    if from == to {
        return Ok(());
    }
    if exists(c, &to) {
        bail!("there's already a page called {to}");
    }
    let text = read(c, &from)?;
    fs::create_dir_all(file(c, &to).parent().expect("a page lives in its space's folder"))?;
    fs::rename(file(c, &from), file(c, &to))?;
    if let Some(space) = file(c, &from).parent() {
        let _ = fs::remove_dir(space);
    }
    c.db.execute("UPDATE wiki_log SET page = ?2 WHERE page = ?1", params![from, to])?;
    log(c, &to, by, &format!("renamed it from {from}"), &text)
}

fn log(c: &Company, path: &str, by: &str, note: &str, text: &str) -> Result<()> {
    c.db.execute("INSERT INTO wiki_log (page, author, note, text) VALUES (?1, ?2, ?3, ?4)", params![path, by, note, text])?;
    Ok(())
}

/// A page's saves, newest first.
pub fn history(c: &Company, path: &str) -> Result<Vec<Edit>> {
    edits(c, "WHERE page = ?1 ORDER BY id DESC", &clean(path)?)
}

/// The latest saves across the wiki, newest first.
pub fn recent(c: &Company, n: usize) -> Result<Vec<Edit>> {
    let mut all = edits(c, "WHERE ?1 = ?1 ORDER BY id DESC", "")?;
    all.truncate(n);
    Ok(all)
}

fn edits(c: &Company, filter: &str, arg: &str) -> Result<Vec<Edit>> {
    let mut st = c.db.prepare(&format!("SELECT id, page, author, note, at, text FROM wiki_log {filter}"))?;
    let rows = st.query_map([arg], |r| Ok(Edit { id: r.get(0)?, page: r.get(1)?, author: r.get(2)?, note: r.get(3)?, at: r.get(4)?, text: r.get(5)? }))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Puts an earlier version of a page back, as a new save.
pub fn restore(c: &Company, edit: i64, by: &str) -> Result<String> {
    let Some(e) = edits(c, "WHERE id = ?1", &edit.to_string())?.into_iter().next() else { bail!("that version is gone") };
    write(c, &e.page, &e.text, by, &format!("restored the version from {}", e.at))?;
    Ok(e.page)
}

/// The whole wiki as one document, for prompts and the CLI. Empty pages are left out.
pub fn render(c: &Company) -> Result<String> {
    let parts: Vec<String> = pages(c)?.iter().filter(|p| !p.text.is_empty()).map(|p| format!("# {} / {}\n\n{}", p.space(), p.title(), p.text)).collect();
    Ok(parts.join("\n\n"))
}

/// The page a loose name means: a full "Space/Title", a page's title, or one of the sections
/// the wiki had before it had pages, like "Conventions". Anything else is a new Company page.
pub fn find(c: &Company, name: &str) -> Result<String> {
    let name = name.trim();
    if name.contains('/') {
        return clean(name);
    }
    if let Some(p) = pages(c)?.into_iter().find(|p| p.title().eq_ignore_ascii_case(name)) {
        return Ok(p.path);
    }
    // The mission and the product brief were pages of their own before the Overview held them.
    if [WHY, BUILDING, WHERE, "Mission"].iter().any(|s| s.eq_ignore_ascii_case(name)) {
        return Ok(OVERVIEW.into());
    }
    let known = [OVERVIEW, NOW, DECISIONS].into_iter().chain(DRAFTS.iter().map(|(p, _)| *p));
    match known.into_iter().find(|p| p.split_once('/').is_some_and(|(_, t)| t.eq_ignore_ascii_case(name))) {
        Some(p) => Ok(p.to_string()),
        None => clean(&format!("Company/{}", if name.is_empty() { "Decisions" } else { name })),
    }
}

/// Pages from a reply: each starts with a "# Space/Title" line. Anything before the first is dropped.
pub fn parse(reply: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = vec![];
    for line in reply.lines() {
        let heading = line.strip_prefix("# ").map(|h| h.trim().trim_matches('`').trim()).filter(|h| h.contains('/')).and_then(|h| clean(h).ok());
        match (heading, out.last_mut()) {
            (Some(path), _) => out.push((path, String::new())),
            (None, Some((_, body))) => {
                body.push_str(line);
                body.push('\n');
            }
            (None, None) => {}
        }
    }
    let fence = |b: &str| b.trim().trim_end_matches("```").trim().to_string();
    out.into_iter().map(|(p, b)| (p, fence(&b))).filter(|(_, b)| !b.is_empty()).collect()
}

/// A wiki from before pages, one wiki.md with ## sections, becomes pages. The old file stays
/// next to them as wiki-before-pages.md.
pub fn migrate(c: &Company) -> Result<()> {
    move_old(c)?;
    credit_moved(c)?;
    merge_overview(c)
}

/// The text under a page's `## name`, without the heading. Empty when there's no such section.
pub fn section(doc: &str, name: &str) -> String {
    let mut lines = doc.lines().skip_while(|l| !l.strip_prefix("## ").is_some_and(|h| h.trim().eq_ignore_ascii_case(name)));
    lines.next();
    lines.take_while(|l| !l.starts_with("## ")).collect::<Vec<_>>().join("\n").trim().to_string()
}

/// The page with `## name` holding `body`: replaced where it is, or put in the Overview's order.
/// An empty body takes the section out.
pub fn with_section(doc: &str, name: &str, body: &str) -> String {
    let mut parts: Vec<(String, String)> = vec![];
    let mut intro = String::new();
    for line in doc.lines() {
        match line.strip_prefix("## ") {
            Some(h) => parts.push((h.trim().to_string(), String::new())),
            None => match parts.last_mut() {
                Some((_, b)) => b.push_str(&format!("{line}\n")),
                None => intro.push_str(&format!("{line}\n")),
            },
        }
    }
    let order = |h: &str| [WHY, BUILDING, WHERE].iter().position(|s| s.eq_ignore_ascii_case(h)).unwrap_or(9);
    match parts.iter().position(|(h, _)| h.eq_ignore_ascii_case(name)) {
        Some(i) => parts[i].1 = body.to_string(),
        None => {
            let at = parts.iter().position(|(h, _)| order(h) > order(name)).unwrap_or(parts.len());
            parts.insert(at, (name.to_string(), body.to_string()));
        }
    }
    let mut out = intro.trim().to_string();
    for (h, b) in parts.iter().filter(|(_, b)| !b.trim().is_empty()) {
        out.push_str(&format!("\n\n## {h}\n\n{}", b.trim()));
    }
    out.trim().to_string()
}

/// A draft's Overview fills only the sections you haven't written: what you wrote always stays.
pub fn fill_overview(now: &str, draft: &str) -> String {
    [WHY, BUILDING, WHERE].iter().fold(now.to_string(), |doc, s| {
        let (mine, theirs) = (section(&doc, s), section(draft, s));
        if mine.is_empty() && !theirs.is_empty() { with_section(&doc, s, &theirs) } else { doc }
    })
}

/// Mission and What we're building were two pages. They're sections of the Overview now, and
/// it's credited to whoever wrote the product part, since that's most of it.
fn merge_overview(c: &Company) -> Result<()> {
    let (mission, building) = ("Company/Mission", "Company/What we're building");
    if !exists(c, mission) && !exists(c, building) {
        return Ok(());
    }
    let mut doc = read(c, OVERVIEW)?;
    for (old, name) in [(mission, WHY), (building, BUILDING)] {
        if section(&doc, name).is_empty() {
            doc = with_section(&doc, name, &read(c, old)?);
        }
    }
    let by = [building, mission].iter().find_map(|p| history(c, p).ok()?.into_iter().next()).map(|e| e.author).unwrap_or_else(|| "founder".into());
    write(c, OVERVIEW, &doc, &by, "merged Mission and What we're building into one page")?;
    for old in [mission, building] {
        let _ = fs::remove_file(file(c, old));
    }
    Ok(())
}

/// The old wiki.md never said who wrote what, but the company knows: you wrote the mission, what
/// we're building and the decisions, and whoever had the last "Write the wiki" ticket wrote the
/// rest. Also fixes pages an earlier version moved in under OneCorp's name.
fn credit_moved(c: &Company) -> Result<()> {
    const MOVED: &str = "moved from the old wiki";
    let moved: Vec<Edit> = edits(c, "WHERE author = 'onecorp' AND note = ?1", MOVED)?;
    if moved.is_empty() {
        return Ok(());
    }
    let writer = c.tickets()?.into_iter().rev().find(|t| t.kind == "wiki" && t.status == "done").and_then(|t| t.assignee);
    for e in moved {
        let yours = [OVERVIEW, "Company/Mission", "Company/What we're building", DECISIONS].contains(&e.page.as_str());
        let by = if yours { "founder" } else { writer.as_deref().unwrap_or("founder") };
        c.db.execute("UPDATE wiki_log SET author = ?2 WHERE id = ?1", params![e.id, by])?;
    }
    Ok(())
}

fn move_old(c: &Company) -> Result<()> {
    let old = c.dir.join("wiki.md");
    if dir(c).exists() || !old.exists() {
        return Ok(());
    }
    let doc = fs::read_to_string(&old)?;
    let mut sections: Vec<(String, String)> = vec![];
    for line in doc.lines() {
        match line.strip_prefix("## ") {
            Some(h) => sections.push((h.trim().to_string(), String::new())),
            None if !line.starts_with("# ") => {
                if let Some((_, body)) = sections.last_mut() {
                    body.push_str(line);
                    body.push('\n');
                }
            }
            None => {}
        }
    }
    fs::create_dir_all(dir(c))?;
    for (name, body) in sections {
        let path = find(c, &name)?;
        let text = if name.eq_ignore_ascii_case("Mission") && body.trim() == "What the company is for, in a sentence or two." { "" } else { body.trim() };
        let now = read(c, &path)?;
        let doc = match (path == OVERVIEW, name.eq_ignore_ascii_case("Mission")) {
            (true, true) => with_section(&now, WHY, text),
            (true, false) => with_section(&now, &name, text),
            _ if now.is_empty() => text.to_string(),
            _ => format!("{now}\n\n{text}"),
        };
        write(c, &path, doc.trim(), "onecorp", "moved from the old wiki")?;
    }
    for p in [OVERVIEW, NOW, DECISIONS] {
        seed(c, p)?;
    }
    fs::rename(&old, c.dir.join("wiki-before-pages.md"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mission_and_brief_become_one_overview() {
        let dir = std::env::temp_dir().join(format!("onecorp-overview-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("wiki/Company")).unwrap();
        fs::write(dir.join("wiki/Company/Mission.md"), "SMS appointment reminders\n").unwrap();
        fs::write(dir.join("wiki/Company/What we're building.md"), "Reminders for clinics.\n").unwrap();
        let c = Company::open_dir("acme", dir.clone()).unwrap();

        // Pages from 0.1.8 merge into the Overview, in order, and the old ones go.
        assert_eq!(read(&c, OVERVIEW).unwrap(), "## Why we exist\n\nSMS appointment reminders\n\n## What we're building\n\nReminders for clinics.");
        assert!(!exists(&c, "Company/Mission") && !exists(&c, "Company/What we're building"));
        assert_eq!(find(&c, "Mission").unwrap(), OVERVIEW);

        // A draft fills only what's missing: what you wrote stays.
        let draft = "## Why we exist\n\nSomething else.\n\n## Where we are\n\nIn beta with 12 clinics.";
        let filled = fill_overview(&read(&c, OVERVIEW).unwrap(), draft);
        assert!(filled.starts_with("## Why we exist\n\nSMS appointment reminders") && filled.ends_with("## Where we are\n\nIn beta with 12 clinics."));
        // A section lands in its place, and an empty one goes.
        assert_eq!(with_section("## Where we are\n\nBeta", WHY, "Why"), "## Why we exist\n\nWhy\n\n## Where we are\n\nBeta");
        assert_eq!(with_section("## Why we exist\n\nWhy", WHY, ""), "");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn pages_keep_their_history_and_old_wikis_move_in() {
        let dir = std::env::temp_dir().join(format!("onecorp-wiki-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("wiki.md"), "# acme wiki\n\n## Mission\n\nNotes for writers.\n\n## Codebase\n\n- Rust\n\n## Decisions\n\n- No ORMs\n").unwrap();
        let c = Company::open_dir("acme", dir.clone()).unwrap();

        // The old sections are pages now, in the space they belong to.
        assert_eq!(read(&c, OVERVIEW).unwrap(), "## Why we exist\n\nNotes for writers.");
        assert_eq!(read(&c, "Engineering/Codebase").unwrap(), "- Rust");
        assert_eq!(find(&c, "Conventions").unwrap(), "Engineering/Conventions");
        assert!(dir.join("wiki-before-pages.md").exists() && !dir.join("wiki.md").exists());
        let order: Vec<String> = pages(&c).unwrap().into_iter().map(|p| p.path).collect();
        assert_eq!(&order[..3], [OVERVIEW, NOW, DECISIONS]);

        // Who wrote what, and any version comes back.
        append(&c, DECISIONS, "Tokens are hashed", "maya", "proposed it").unwrap();
        write(&c, DECISIONS, "- Nothing", "founder", "").unwrap();
        write(&c, DECISIONS, "- Nothing", "founder", "").unwrap();
        let h = history(&c, DECISIONS).unwrap();
        assert_eq!(h.iter().map(|e| e.author.as_str()).collect::<Vec<_>>(), ["founder", "maya", "founder"], "an unchanged save isn't logged, and you wrote the old decisions");
        assert_eq!(history(&c, "Engineering/Codebase").unwrap()[0].author, "founder", "with no wiki ticket on record, the old pages are yours");
        restore(&c, h[1].id, "founder").unwrap();
        assert_eq!(read(&c, DECISIONS).unwrap(), "- No ORMs\n- Tokens are hashed");

        // Renaming keeps the history, and pages can't escape the wiki folder.
        rename(&c, DECISIONS, "Product/Decisions", "founder").unwrap();
        assert_eq!(history(&c, "Product/Decisions").unwrap().len(), 5);
        assert!(clean("../etc/passwd").is_err() && clean("Product/../x").is_err() && clean("Product").is_err());

        // A writer's reply is split into its pages.
        let got = parse("Here you go.\n# Product/Customers\n\nClinics.\n\n# `Engineering / Codebase`\n- Next.js\n```\n");
        assert_eq!(got, vec![("Product/Customers".to_string(), "Clinics.".to_string()), ("Engineering/Codebase".to_string(), "- Next.js".to_string())]);
        assert!(render(&c).unwrap().starts_with("# Company / Overview\n\n## Why we exist\n\nNotes for writers."));
        let _ = fs::remove_dir_all(&dir);
    }
}
