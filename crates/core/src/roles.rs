//! How each person works, in three layers. Their role's playbook, picked from their title unless
//! you pick another. What leading asks of them, when they lead a department. And your notes for
//! them on top. Only your choices are stored, so better playbooks in a new version reach everyone.

use crate::store::{Company, Employee};
use anyhow::Result;

pub struct Role {
    pub key: &'static str,
    /// What the picker calls it.
    pub label: &'static str,
    /// Words in a title that mean this role, checked in the order of ROLES.
    matches: &'static [&'static str],
    pub playbook: &'static str,
}

pub const ROLES: &[Role] = &[
    Role {
        key: "reviewer",
        label: "Tech lead",
        matches: &["cto", "maintainer", "reviewer", "tech lead", "lead engineer", "head of engineering", "engineering lead", "engineering manager", "vp of engineering", "vp engineering", "architect", "staff engineer", "principal"],
        playbook: "\
- You own the quality and direction of the code. Read a diff for what it breaks, not only what it adds.
- Block on missing tests for changed behavior, security or data loss risks, breaking API changes, and dependencies that don't earn their place.
- Prefer the simple design that's easy to change later over the clever one.
- When you take a ticket yourself, hold your own work to the same bar.
- Say what you'd change and why in a sentence each. Approve what's good enough, not only what's perfect.",
    },
    Role {
        key: "designer",
        label: "Designer",
        matches: &["design", "ux", "ui/ux"],
        playbook: "\
- Design in code, inside the product's existing styles, components and spacing.
- Keep screens simple: one main action, short copy, a clear hierarchy.
- Design every state: first use, empty, loading, error and long text.
- Accessible by default: contrast, visible focus, labels and comfortable hit areas.
- Check it in light and dark, and on small screens.
- In your summary, say what changed on screen and where to look.",
    },
    Role {
        key: "pm",
        label: "Product manager",
        matches: &["product manager", "product owner", "product lead", "head of product", "project manager", "cpo", "pm", "program manager"],
        playbook: "\
- Turn goals into small tickets one person can finish in a sitting, each with a clear done.
- Keep the first version small. Cut scope before you add people.
- Ask one clarifying question when it truly matters, with a default. Never five.
- Give each ticket to the person whose role fits, and order tickets that depend on each other.
- Write tickets someone new could pick up: what, why, and how to check it.
- Say what you left out on purpose, so it reads as a decision, not a gap.",
    },
    Role {
        key: "qa",
        label: "QA",
        matches: &["qa", "quality", "test engineer", "tester", "sdet"],
        playbook: "\
- Start from what the user does, then the edges: empty, huge, slow, offline, twice in a row.
- Turn each bug you find into a failing test first.
- Test at the lowest level that catches the bug. Keep end-to-end tests few and stable.
- A flaky test is a bug. Fix it or remove it, never just retry it.
- Report a bug with exact steps, what you expected and what happened.",
    },
    Role {
        key: "devops",
        label: "DevOps",
        matches: &["devops", "infra", "platform", "sre", "reliability", "cloud", "ops"],
        playbook: "\
- Change infrastructure through code and config in the repo, never by hand.
- Every change can be rolled back. Say how in your summary.
- Keep secrets out of the repo, the logs and the CI output.
- Keep CI fast and reliable: cache what you can, no flaky steps, fail with a clear message.
- Prefer boring, managed services over clever setups.
- Call out anything that costs money, needs access you don't have, or has to be done in a dashboard.",
    },
    Role {
        key: "mobile",
        label: "Mobile engineer",
        matches: &["mobile", "ios", "android", "swift", "kotlin", "flutter", "react native"],
        playbook: "\
- Follow the platform's conventions and the patterns already in the app.
- Handle no network, a slow network, and the app going to the background and back.
- Respect safe areas, text size settings, dark mode, and small and large devices.
- Keep the main thread free so startup and scrolling stay smooth.
- Ask for a permission when it's needed, with the reason.
- Say when a change needs a store review, a new permission or a version bump.",
    },
    Role {
        key: "backend",
        label: "Backend engineer",
        matches: &["backend", "back-end", "back end", "api", "server", "data engineer", "database"],
        playbook: "\
- Read the handlers, models and tests around a change first, and follow how errors, logging and config are already done.
- Keep APIs backward compatible. If a change would break a client, say so and ask before making it.
- Schema changes go in a new migration that's safe on live data. Never edit an old one.
- Validate input at the edges, and never log secrets, tokens or personal data.
- Watch for queries in loops, missing indexes and lists with no limit.
- Every endpoint or job you change gets a test, the failure path included.
- In your summary, call out migrations, config or environment changes someone has to apply.",
    },
    Role {
        key: "frontend",
        label: "Frontend engineer",
        matches: &["frontend", "front-end", "front end", "web", "ui engineer", "ui developer"],
        playbook: "\
- Reuse the components, styles and tokens already in the repo before making new ones.
- Handle every state: loading, empty, error and long content, not only the happy path.
- Keep it accessible: real buttons and labels, the keyboard works, focus shows, contrast holds in light and dark.
- Check small and large screens. No layout shift and no sideways scrolling.
- Keep copy short and in the product's voice from the wiki.
- No new library for what a few lines can do.
- Test the logic you changed, and say in your summary which screens to look at.",
    },
    Role {
        key: "growth",
        label: "Growth",
        matches: &["growth", "market", "seo", "acquisition"],
        playbook: "\
- Know who the customer is from the wiki before you write a word.
- Lead with the problem you solve, in the customer's words, not a list of features.
- Every claim is true and specific. No superlatives you can't back up.
- Pages load fast, read well on a phone and have one clear call to action.
- Use the analytics and SEO setup already in the repo, and add events for what you'd want to measure.
- Say in your summary how you'd know it worked.",
    },
    Role {
        key: "writer",
        label: "Writer",
        matches: &["writ", "docs", "documentation", "content", "copy", "editor"],
        playbook: "\
- Write for a reader in a hurry: the point first, short sentences, plain words.
- Check facts against the code before you write them. Commands and examples must work as written.
- No hype, no filler, no recap at the end.
- Follow the voice in the wiki, and use the words from the Glossary.
- Update what's already there before adding a new page.
- Say in your summary what you changed and what might need a second look.",
    },
    Role {
        key: "engineer",
        label: "Engineer",
        matches: &[],
        playbook: "\
- Read the code around a change before you touch it, and follow the patterns already there.
- Make the smallest change that does the job. No new dependency unless it clearly earns its place.
- Fix the cause, not the symptom. When a bug comes from a shared function, fix it there.
- Add or update a test for what you changed, and run the build and tests before you finish.
- Leave the code easier to read than you found it, without refactoring what the ticket doesn't need.
- Say in your summary what you tested, what you didn't, and what the reviewer should look at.",
    },
];

/// The role a title means: "Backend engineer", "CTO", "Growth writer". Anything else is an engineer.
pub fn for_title(title: &str) -> &'static Role {
    let t = format!(" {} ", title.to_lowercase().replace(|c: char| !c.is_alphanumeric() && c != '-' && c != '/', " "));
    // Short words like "pm" or "ops" only count on their own, so "development" isn't a PM.
    let has = |m: &str| if m.len() <= 3 { t.contains(&format!(" {m} ")) } else { t.contains(m) };
    ROLES.iter().find(|r| r.matches.iter().any(|m| has(m))).unwrap_or(&ROLES[ROLES.len() - 1])
}

/// The wiki spaces a role works in every day, read in full with every ticket.
pub fn spaces(role: &Role) -> &'static [&'static str] {
    match role.key {
        "pm" | "designer" | "writer" | "growth" => &["Product"],
        _ => &["Engineering"],
    }
}

/// What earlier versions wrote into everyone's bio. Those were never yours, so they're no notes.
const OLD_DEFAULTS: &[&str] = &[
    "Reads the code around a change before touching it. Makes the smallest diff that does the job and says what to check.",
    "Reads diffs carefully. Flags risks, missing tests and new dependencies that don't earn their place.",
    "Designs in code. Keeps screens simple and copy short, and follows the styles already in the repo.",
    "Turns ideas into small tickets anyone can pick up. Asks one clarifying question, never five. Keeps the first version small.",
    "Writes docs, changelogs and posts. Short sentences, plain words, no hype.",
];

pub fn by_key(key: &str) -> Option<&'static Role> {
    ROLES.iter().find(|r| r.key == key)
}

/// Their playbook's role: the one you picked, or the one their title means.
pub fn role_of(c: &Company, e: &Employee) -> Result<&'static Role> {
    Ok(c.setting(&format!("kind.{}", e.id))?.and_then(|k| by_key(&k)).unwrap_or_else(|| for_title(&e.role)))
}

/// Your notes for someone, added to their playbook. The `bio` column has always held them.
pub fn notes(e: &Employee) -> Option<&str> {
    let bio = e.bio.trim();
    (!bio.is_empty() && !OLD_DEFAULTS.contains(&bio)).then_some(bio)
}

/// Your own playbook for someone, when you rewrote theirs instead of adding notes.
pub fn rewrite(c: &Company, e: &Employee) -> Result<Option<String>> {
    Ok(c.setting(&format!("playbook.{}", e.id))?.filter(|p| !p.trim().is_empty()))
}

/// The playbook they follow: your rewrite, or their role's.
pub fn playbook(c: &Company, e: &Employee) -> Result<String> {
    Ok(rewrite(c, e)?.unwrap_or_else(|| role_of(c, e).map(|r| r.playbook).unwrap_or_default().to_string()))
}

/// Saves which playbook someone follows and how. A role that matches their title isn't stored,
/// so it follows the title. A rewrite equal to the role's playbook isn't either, so it follows
/// the defaults as they improve.
pub fn set(c: &Company, id: &str, title: &str, kind: Option<&str>, rewrite: Option<&str>) -> Result<()> {
    let role = match kind.and_then(by_key) {
        Some(r) if r.key != for_title(title).key => {
            c.set_setting(&format!("kind.{id}"), r.key)?;
            r
        }
        _ => {
            c.delete_setting(&format!("kind.{id}"))?;
            for_title(title)
        }
    };
    match rewrite.map(str::trim).filter(|t| !t.is_empty() && *t != role.playbook.trim()) {
        Some(text) => c.set_setting(&format!("playbook.{id}"), text),
        None => c.delete_setting(&format!("playbook.{id}")),
    }
}

/// What leading means, for anyone who leads a department, whatever their title.
pub fn lead_part(c: &Company, e: &Employee) -> Result<Option<String>> {
    let led: Vec<String> = c.departments()?.into_iter().filter(|d| d.lead.as_deref() == Some(e.id.as_str())).map(|d| d.name).collect();
    if led.is_empty() {
        return Ok(None);
    }
    let team: Vec<String> = c.employees()?.into_iter().filter(|o| o.id != e.id && c.lead_of(&o.id).ok().flatten().is_some_and(|l| l.id == e.id)).map(|o| o.name).collect();
    Ok(Some(format!(
        "You lead {}{}.\n         - Questions from your team come to you first. Decide what the code, the wiki or sound judgment settles. Pass up product direction, money, security and anything public.\n         - When you split work, give each ticket one owner on your team, a clear done, and an order when one depends on another.\n         - Keep an eye on your team: who's stuck, what waits for review, what nobody owns.",
        led.join(" and "),
        if team.is_empty() { String::new() } else { format!(": {}", team.join(", ")) }
    )))
}

/// Everything about how someone works, for the top of their prompts: playbook, then what leading
/// asks of them, then your notes, which come last so they win.
pub fn brief(c: &Company, e: &Employee) -> Result<String> {
    let mut out = format!("How you work:\n{}", playbook(c, e)?);
    if let Some(lead) = lead_part(c, e)? {
        out.push_str(&format!("\n\n{lead}"));
    }
    if let Some(n) = notes(e) {
        out.push_str(&format!("\n\nThe founder's notes for you, which come first:\n{n}"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles_pick_their_role() {
        let key = |t| for_title(t).key;
        assert_eq!(key("Backend engineer"), "backend");
        assert_eq!(key("Frontend engineer"), "frontend");
        assert_eq!(key("CTO"), "reviewer");
        assert_eq!(key("Growth writer"), "growth");
        assert_eq!(key("Docs writer"), "writer");
        assert_eq!(key("Product designer"), "designer");
        assert_eq!(key("Product manager"), "pm");
        assert_eq!(key("PM"), "pm");
        assert_eq!(key("Product lead"), "pm");
        assert_eq!(key("Engineering manager"), "reviewer");
        assert_eq!(key("Development lead"), "engineer", "pm inside a word doesn't count");
        assert_eq!(key("iOS engineer"), "mobile");
        assert_eq!(key("DevOps"), "devops");
        assert_eq!(key("Engineer"), "engineer");
        assert_eq!(key("Wizard"), "engineer");
    }

    #[test]
    fn an_old_default_bio_is_no_notes() {
        let mut e = Employee { id: "t".into(), name: "T".into(), role: "Backend engineer".into(), runtime: "claude".into(), model: None, bio: OLD_DEFAULTS[0].into(), department: None };
        assert_eq!(notes(&e), None);
        e.bio = "Writes Go like Rob Pike.".into();
        assert_eq!(notes(&e), Some("Writes Go like Rob Pike."));
    }

}
