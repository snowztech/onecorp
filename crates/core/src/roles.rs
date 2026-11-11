//! What each role does and how they work: the playbook every ticket, chat reply and lead decision
//! starts from. The role's title picks the default. Your own wording for someone replaces it, and
//! only that is stored, so better defaults in a new version reach everyone you haven't customized.

use crate::store::Employee;

pub struct Role {
    pub key: &'static str,
    /// Words in a title that mean this role, checked in the order of ROLES.
    matches: &'static [&'static str],
    pub playbook: &'static str,
}

pub const ROLES: &[Role] = &[
    Role {
        key: "reviewer",
        matches: &["cto", "maintainer", "reviewer", "tech lead", "lead engineer", "head of engineering", "engineering lead", "engineering manager", "vp of engineering", "vp engineering", "architect", "staff engineer", "principal"],
        playbook: "\
- You own the quality and direction of the code. Read a diff for what it breaks, not only what it adds.
- Block on missing tests for changed behavior, security or data loss risks, breaking API changes, and dependencies that don't earn their place.
- Prefer the simple design that's easy to change later over the clever one.
- When you take a ticket yourself, hold your own work to the same bar.
- When a teammate asks you something, decide what the code, the wiki or sound judgment settles. Pass up product direction, money, security and anything public.
- When you split work for your team, give each ticket one owner, a clear done, and an order when one depends on another.",
    },
    Role {
        key: "designer",
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

/// What earlier versions wrote into everyone's bio. Those were never yours, so they mean the default.
const OLD_DEFAULTS: &[&str] = &[
    "Reads the code around a change before touching it. Makes the smallest diff that does the job and says what to check.",
    "Reads diffs carefully. Flags risks, missing tests and new dependencies that don't earn their place.",
    "Designs in code. Keeps screens simple and copy short, and follows the styles already in the repo.",
    "Turns ideas into small tickets anyone can pick up. Asks one clarifying question, never five. Keeps the first version small.",
    "Writes docs, changelogs and posts. Short sentences, plain words, no hype.",
];

/// Your own wording for how someone works, if you wrote one.
pub fn custom(e: &Employee) -> Option<&str> {
    let bio = e.bio.trim();
    (!bio.is_empty() && !OLD_DEFAULTS.contains(&bio)).then_some(bio)
}

/// How someone works: your wording for them, or their role's default.
pub fn playbook(e: &Employee) -> &str {
    custom(e).unwrap_or(for_title(&e.role).playbook)
}

/// What to store for how someone works: nothing when it's their role's default, so they keep
/// getting the default as it improves.
pub fn to_store(role: &str, text: &str) -> String {
    let text = text.trim();
    if text == for_title(role).playbook.trim() || OLD_DEFAULTS.contains(&text) { String::new() } else { text.to_string() }
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
    fn an_old_default_bio_is_not_yours() {
        let mut e = Employee { id: "t".into(), name: "T".into(), role: "Backend engineer".into(), runtime: "claude".into(), model: None, bio: OLD_DEFAULTS[0].into(), department: None };
        assert_eq!(custom(&e), None);
        assert!(playbook(&e).contains("migration"));
        e.bio = "Writes Go like Rob Pike.".into();
        assert_eq!(playbook(&e), "Writes Go like Rob Pike.");
    }
}
