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
pub const MISSION: &str = "Company/Mission";
pub const BUILDING: &str = "Company/What we're building";
pub const NOW: &str = "Company/Now";
pub const DECISIONS: &str = "Company/Decisions";

/// The pages employees draft from the repo, and what goes in each.
pub const DRAFTS: [(&str, &str); 7] = [
    (BUILDING, "the product, who it's for and what it does today, in a short paragraph. The team plans from it."),
    ("Product/Customers", "who uses it, what they're trying to get done, and what they care about."),
    ("Product/Rules", "rules that are easy to break without knowing: limits, money, privacy, legal, anything users were promised."),
    ("Product/Voice", "how the product talks to people: tone, words it uses and avoids, languages."),
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
    let order = [MISSION, BUILDING, NOW, DECISIONS].into_iter().chain(DRAFTS.iter().map(|(p, _)| *p)).position(|p| p == path).unwrap_or(99);
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
    let known = [MISSION, BUILDING, NOW, DECISIONS].into_iter().chain(DRAFTS.iter().map(|(p, _)| *p));
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
        write(c, &path, if now.is_empty() { text.to_string() } else { format!("{now}\n\n{text}") }.trim(), "onecorp", "moved from the old wiki")?;
    }
    for p in [MISSION, BUILDING, NOW, DECISIONS] {
        seed(c, p)?;
    }
    fs::rename(&old, c.dir.join("wiki-before-pages.md"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pages_keep_their_history_and_old_wikis_move_in() {
        let dir = std::env::temp_dir().join(format!("onecorp-wiki-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("wiki.md"), "# acme wiki\n\n## Mission\n\nNotes for writers.\n\n## Codebase\n\n- Rust\n\n## Decisions\n\n- No ORMs\n").unwrap();
        let c = Company::open_dir("acme", dir.clone()).unwrap();

        // The old sections are pages now, in the space they belong to.
        assert_eq!(read(&c, MISSION).unwrap(), "Notes for writers.");
        assert_eq!(read(&c, "Engineering/Codebase").unwrap(), "- Rust");
        assert_eq!(find(&c, "Conventions").unwrap(), "Engineering/Conventions");
        assert!(dir.join("wiki-before-pages.md").exists() && !dir.join("wiki.md").exists());
        let order: Vec<String> = pages(&c).unwrap().into_iter().map(|p| p.path).collect();
        assert_eq!(&order[..2], [MISSION, BUILDING]);

        // Who wrote what, and any version comes back.
        append(&c, DECISIONS, "Tokens are hashed", "maya", "proposed it").unwrap();
        write(&c, DECISIONS, "- Nothing", "founder", "").unwrap();
        write(&c, DECISIONS, "- Nothing", "founder", "").unwrap();
        let h = history(&c, DECISIONS).unwrap();
        assert_eq!(h.iter().map(|e| e.author.as_str()).collect::<Vec<_>>(), ["founder", "maya", "onecorp"], "an unchanged save isn't logged");
        restore(&c, h[1].id, "founder").unwrap();
        assert_eq!(read(&c, DECISIONS).unwrap(), "- No ORMs\n- Tokens are hashed");

        // Renaming keeps the history, and pages can't escape the wiki folder.
        rename(&c, DECISIONS, "Product/Decisions", "founder").unwrap();
        assert_eq!(history(&c, "Product/Decisions").unwrap().len(), 5);
        assert!(clean("../etc/passwd").is_err() && clean("Product/../x").is_err() && clean("Product").is_err());

        // A writer's reply is split into its pages.
        let got = parse("Here you go.\n# Product/Customers\n\nClinics.\n\n# `Engineering / Codebase`\n- Next.js\n```\n");
        assert_eq!(got, vec![("Product/Customers".to_string(), "Clinics.".to_string()), ("Engineering/Codebase".to_string(), "- Next.js".to_string())]);
        assert!(render(&c).unwrap().starts_with("# Company / Mission\n\nNotes for writers."));
        let _ = fs::remove_dir_all(&dir);
    }
}
