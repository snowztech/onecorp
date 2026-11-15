//! Copies of a company, so its history survives a bad update, a disk problem or a mistake. A copy
//! is a folder in `backups/` named by when it was taken: the database, copied consistently while
//! the team works, and the files that are the company too (the wiki, everyone's notes, faces and
//! the logo). Worktrees aren't copied: they're the repo's, and git keeps them.

use crate::store::Company;
use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};

/// How often a company is copied while something changes in it.
const EVERY: &str = "-1 hour";
/// Every copy from the last day is kept, then one a day, up to two weeks. Hourly copies kept for
/// two weeks would be 336 of them.
const KEEP_ALL: &str = "-1 day";
const KEEP: &str = "-14 days";
/// The folders and files copied along with the database.
const FILES: [&str; 4] = ["wiki", "memory", "avatars", "logo.png"];

/// A copy: its folder name, which starts with when it was taken, and why if it wasn't the hourly one.
#[derive(Debug, Clone, PartialEq)]
pub struct Copy {
    pub name: String,
    pub at: String,
    pub why: String,
}

fn dir(c: &Company) -> PathBuf {
    c.dir.join("backups")
}

/// "2026-11-15_14-00-05", in local time, for now or `offset` from now like "-1 hour".
fn stamp(c: &Company, offset: &str) -> Result<String> {
    c.local_time("%Y-%m-%d_%H-%M-%S", offset)
}

/// Copies the company now. `why` names a copy taken for a reason, like "before-0.1.14".
pub fn take(c: &Company, why: &str) -> Result<PathBuf> {
    let mut name = stamp(c, "")?;
    if !why.is_empty() {
        name.push('-');
        name.push_str(why);
    }
    let to = dir(c).join(&name);
    if to.exists() {
        return Ok(to);
    }
    fs::create_dir_all(&to)?;
    c.copy_database(&to.join("company.db"))?;
    for f in FILES {
        let from = c.dir.join(f);
        if from.exists() {
            copy_all(&from, &to.join(f))?;
        }
    }
    Ok(to)
}

/// Copies the company if an hour has passed since the last copy and something changed since,
/// then drops copies older than two weeks. Cheap when there's nothing to do: called every minute.
pub fn maybe(c: &Company) -> Result<Option<PathBuf>> {
    let newest = list(c)?.into_iter().next();
    let due = match &newest {
        None => true,
        Some(last) => last.name[..19] < *stamp(c, EVERY)? && changed_since(c, &dir(c).join(&last.name)),
    };
    let taken = if due { Some(take(c, "")?) } else { None };
    prune(c)?;
    Ok(taken)
}

/// Whether the database was written after `copy` was taken.
fn changed_since(c: &Company, copy: &Path) -> bool {
    let modified = |p: &Path| fs::metadata(p).and_then(|m| m.modified()).ok();
    let Some(taken) = modified(copy) else { return true };
    ["company.db", "company.db-wal"].iter().filter_map(|f| modified(&c.dir.join(f))).any(|m| m > taken)
}

/// The copies, newest first.
pub fn list(c: &Company) -> Result<Vec<Copy>> {
    let Ok(entries) = fs::read_dir(dir(c)) else { return Ok(vec![]) };
    let mut out: Vec<Copy> = entries
        .flatten()
        .filter(|e| e.path().join("company.db").is_file())
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            let (when, why) = (name.get(..19)?, name.get(20..).unwrap_or(""));
            // Only folders this module made: a timestamp, then maybe why.
            let ok = when.len() == 19 && when.chars().enumerate().all(|(i, ch)| match i {
                4 | 7 => ch == '-',
                10 => ch == '_',
                13 | 16 => ch == '-',
                _ => ch.is_ascii_digit(),
            });
            let (at, why) = (format!("{} {}", &when[..10], when[11..].replace('-', ":")), why.to_string());
            ok.then_some(Copy { at, why, name })
        })
        .collect();
    out.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(out)
}

/// Keeps every copy from the last day, the newest of each day before that, and nothing older than
/// two weeks. The newest copy always stays, however old.
fn prune(c: &Company) -> Result<()> {
    let (all, cutoff) = (stamp(c, KEEP_ALL)?, stamp(c, KEEP)?);
    let mut days_kept: Vec<String> = vec![];
    for (i, copy) in list(c)?.iter().enumerate() {
        let when = &copy.name[..19];
        let day = copy.name[..10].to_string();
        let keep = i == 0 || *when >= *all || (*when >= *cutoff && !days_kept.contains(&day));
        if keep {
            days_kept.push(day);
        } else {
            fs::remove_dir_all(dir(c).join(&copy.name))?;
        }
    }
    Ok(())
}

/// Puts a copy back. The company as it is now is copied first, so a restore can be undone by
/// restoring that one. Refused while work is running: a run writing to the database midway would
/// mix the two.
pub fn restore(c: &Company, name: &str) -> Result<()> {
    let copies = list(c)?;
    let copy = match copies.iter().find(|x| x.name == name) {
        Some(x) => x.clone(),
        None => match copies.iter().filter(|x| x.at == name).collect::<Vec<_>>()[..] {
            [one] => one.clone(),
            [] => bail!("no copy called {name}. See them with: onecorp restore"),
            ref many => bail!("{} copies were taken at {name}. Name one: {}", many.len(), many.iter().map(|x| x.name.as_str()).collect::<Vec<_>>().join(", ")),
        },
    };
    if c.running_jobs()? > 0 {
        bail!("work is running. Stop it or wait for it to finish, then restore");
    }
    take(c, "before-restore")?;
    let from = dir(c).join(&copy.name);
    c.restore_database(&from.join("company.db"))?;
    for f in FILES {
        let (here, there) = (c.dir.join(f), from.join(f));
        if here.is_dir() {
            fs::remove_dir_all(&here)?;
        } else if here.exists() {
            fs::remove_file(&here)?;
        }
        if there.exists() {
            copy_all(&there, &here)?;
        }
    }
    Ok(())
}

fn copy_all(from: &Path, to: &Path) -> Result<()> {
    if from.is_file() {
        fs::copy(from, to)?;
        return Ok(());
    }
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)?.flatten() {
        let path = entry.path();
        // Never follow a link out of the company's folder.
        if entry.file_type()?.is_symlink() {
            continue;
        }
        copy_all(&path, &to.join(entry.file_name()))?;
    }
    Ok(())
}
