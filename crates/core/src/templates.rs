//! Role presets and starting teams, shared by the CLI and the app, so a new company is one step
//! from having people in it.

use crate::store::{Company, Employee, slug};
use anyhow::Result;

pub struct Preset {
    pub key: &'static str,
    pub role: &'static str,
    pub bio: &'static str,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        key: "engineer",
        role: "Engineer",
        bio: "Reads the code around a change before touching it. Makes the smallest diff that does the job and says what to check.",
    },
    Preset {
        key: "reviewer",
        role: "Reviewer",
        bio: "Reads diffs carefully. Flags risks, missing tests and new dependencies that don't earn their place.",
    },
    Preset {
        key: "designer",
        role: "Product designer",
        bio: "Designs in code. Keeps screens simple and copy short, and follows the styles already in the repo.",
    },
    Preset {
        key: "pm",
        role: "Product manager",
        bio: "Turns ideas into small tickets anyone can pick up. Asks one clarifying question, never five. Keeps the first version small.",
    },
    Preset {
        key: "writer",
        role: "Writer",
        bio: "Writes docs, changelogs and posts. Short sentences, plain words, no hype.",
    },
];

pub fn preset(key: &str) -> Option<&'static Preset> {
    PRESETS.iter().find(|p| p.key.eq_ignore_ascii_case(key.trim()))
}

pub struct Hire {
    pub name: &'static str,
    pub preset: &'static str,
    /// A title other than the preset's, like CTO for a reviewer.
    pub role: Option<&'static str>,
    pub dept: Option<&'static str>,
}

pub struct Template {
    pub key: &'static str,
    pub name: &'static str,
    pub description: &'static str,
    /// Set when the template only makes sense on one runtime, like the free demo.
    pub runtime: Option<&'static str>,
    pub hires: &'static [Hire],
}

const SMALL_TEAM: &[Hire] = &[
    // First into Engineering, so she leads it and answers the team's questions first.
    Hire { name: "Odile Martin", preset: "reviewer", role: Some("CTO"), dept: Some("Engineering") },
    Hire { name: "Maya Okafor", preset: "engineer", role: None, dept: Some("Engineering") },
    // First into Product, so he leads it and plans the work from your brief.
    Hire { name: "Rui Alves", preset: "pm", role: None, dept: Some("Product") },
    Hire { name: "Lina Park", preset: "designer", role: None, dept: Some("Product") },
];

pub const TEMPLATES: &[Template] = &[
    Template {
        key: "solo",
        name: "Just me and one engineer",
        description: "One engineer to start. Hire the rest when you need them.",
        runtime: None,
        hires: &[Hire { name: "Maya Okafor", preset: "engineer", role: None, dept: Some("Engineering") }],
    },
    Template {
        key: "small",
        name: "Small product team",
        description: "A CTO who leads Engineering and reviews, an engineer, and a product manager and a designer in Product.",
        runtime: None,
        hires: SMALL_TEAM,
    },
    Template {
        key: "demo",
        name: "Try it for free",
        description: "The small team on a stand-in with no AI. Nothing to install, nothing to pay. Rehire on a real runtime anytime.",
        runtime: Some("fake"),
        hires: SMALL_TEAM,
    },
    Template { key: "empty", name: "Start empty", description: "No one yet. Add departments and hire yourself.", runtime: None, hires: &[] },
];

pub fn template(key: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.key.eq_ignore_ascii_case(key.trim()))
}

/// Hires a template's people into a fresh company, creating their departments. `runtime` is
/// used unless the template insists on its own.
pub fn apply(c: &Company, t: &Template, runtime: &str) -> Result<Vec<Employee>> {
    let runtime = t.runtime.unwrap_or(runtime);
    let mut hired = vec![];
    for h in t.hires {
        let p = preset(h.preset).ok_or_else(|| anyhow::anyhow!("no preset called {}", h.preset))?;
        let department = match h.dept {
            Some(name) => Some(match c.department(name)? {
                Some(d) => d.id,
                None => c.add_department(name)?.id,
            }),
            None => None,
        };
        let e = Employee {
            id: slug(h.name),
            name: h.name.into(),
            role: h.role.unwrap_or(p.role).into(),
            runtime: runtime.into(),
            model: None,
            bio: p.bio.into(),
            department,
        };
        c.hire(&e)?;
        hired.push(e);
    }
    Ok(hired)
}

/// A company name from a folder name: "My Notes App" becomes my-notes-app.
pub fn suggest_name(folder: &str) -> String {
    let mut out = String::new();
    for ch in folder.trim().to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else if !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_set_up_a_team() {
        let dir = std::env::temp_dir().join(format!("onecorp-templates-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = Company::open_dir("acme", dir.clone()).unwrap();

        let hired = apply(&c, template("demo").unwrap(), "claude").unwrap();
        assert_eq!(hired.len(), 4);
        assert!(hired.iter().all(|e| e.runtime == "fake"), "the demo always runs on the stand-in");
        let eng = c.department("Engineering").unwrap().unwrap();
        assert_eq!(eng.lead.as_deref(), Some("odile"));
        assert_eq!(c.employee("odile").unwrap().unwrap().role, "CTO");
        assert_eq!(c.department("product").unwrap().unwrap().lead.as_deref(), Some("rui"));

        assert_eq!(suggest_name("My Notes App"), "my-notes-app");
        assert_eq!(suggest_name("  snowztech.com "), "snowztech-com");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
