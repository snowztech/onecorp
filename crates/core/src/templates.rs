//! Role presets and starting teams, shared by the CLI and the app, so a new company is one step
//! from having people in it.

use crate::store::{Company, Employee, slug};
use anyhow::Result;

pub struct Preset {
    pub key: &'static str,
    pub role: &'static str,
}

pub const PRESETS: &[Preset] = &[
    Preset {
        key: "engineer",
        role: "Engineer",
    },
    Preset {
        key: "reviewer",
        role: "Reviewer",
    },
    Preset {
        key: "designer",
        role: "Product designer",
    },
    Preset {
        key: "pm",
        role: "Product manager",
    },
    Preset {
        key: "writer",
        role: "Writer",
    },
];

/// The model tier a role starts on: the strongest for whoever reviews and decides, the cheapest
/// for short writing, a balanced one for everyone else.
pub fn tier_of(preset: &str) -> &'static str {
    match preset {
        "reviewer" => "strongest",
        "writer" => "cheapest",
        _ => "balanced",
    }
}

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
    /// Set when the template only makes sense on one runtime, like the stand-in demo.
    pub runtime: Option<&'static str>,
    /// Kept out of the app: the stand-in demo is for tests and trying the CLI, since an employee
    /// without AI can't do real work.
    pub hidden: bool,
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
        name: "Solo builder",
        description: "One engineer who ships. Hire more when you need them.",
        runtime: None,
        hidden: false,
        hires: &[Hire { name: "Maya Okafor", preset: "engineer", role: None, dept: Some("Engineering") }],
    },
    Template {
        key: "startup",
        name: "Startup",
        description: "A CTO, frontend and backend engineers, a PM, a designer, and a writer for the launch.",
        runtime: None,
        hidden: false,
        hires: &[
            Hire { name: "Odile Martin", preset: "reviewer", role: Some("CTO"), dept: Some("Engineering") },
            Hire { name: "Maya Okafor", preset: "engineer", role: Some("Frontend engineer"), dept: Some("Engineering") },
            Hire { name: "Tomás Reis", preset: "engineer", role: Some("Backend engineer"), dept: Some("Engineering") },
            Hire { name: "Rui Alves", preset: "pm", role: None, dept: Some("Product") },
            Hire { name: "Lina Park", preset: "designer", role: None, dept: Some("Product") },
            Hire { name: "Sam Duarte", preset: "writer", role: Some("Growth writer"), dept: Some("Growth") },
        ],
    },
    Template {
        key: "oss",
        name: "Open source",
        description: "A maintainer who reviews every change, an engineer, and a writer for docs and changelogs.",
        runtime: None,
        hidden: false,
        hires: &[
            Hire { name: "Odile Martin", preset: "reviewer", role: Some("Maintainer"), dept: Some("Engineering") },
            Hire { name: "Maya Okafor", preset: "engineer", role: None, dept: Some("Engineering") },
            Hire { name: "Sam Duarte", preset: "writer", role: Some("Docs writer"), dept: Some("Docs") },
        ],
    },
    Template {
        key: "indie",
        name: "Indie maker",
        description: "Build it and launch it yourself: an engineer, a designer, and a growth marketer.",
        runtime: None,
        hidden: false,
        hires: &[
            Hire { name: "Maya Okafor", preset: "engineer", role: None, dept: Some("Product") },
            Hire { name: "Lina Park", preset: "designer", role: None, dept: Some("Product") },
            Hire { name: "Sam Duarte", preset: "writer", role: Some("Growth marketer"), dept: Some("Growth") },
        ],
    },
    Template {
        key: "agency",
        name: "Agency",
        description: "Client work: a PM who plans, two engineers, a designer, and a QA reviewer who checks every change.",
        runtime: None,
        hidden: false,
        hires: &[
            Hire { name: "Rui Alves", preset: "pm", role: Some("Project manager"), dept: Some("Delivery") },
            Hire { name: "Maya Okafor", preset: "engineer", role: None, dept: Some("Delivery") },
            Hire { name: "Tomás Reis", preset: "engineer", role: None, dept: Some("Delivery") },
            Hire { name: "Lina Park", preset: "designer", role: None, dept: Some("Delivery") },
            Hire { name: "Odile Martin", preset: "reviewer", role: Some("QA reviewer"), dept: Some("Delivery") },
        ],
    },
    Template { key: "empty", name: "Start empty", description: "No one yet. Add departments and hire them yourself.", runtime: None, hidden: false, hires: &[] },
    Template {
        key: "small",
        name: "Product team",
        description: "A CTO who reviews, an engineer, a product manager who plans, and a designer.",
        runtime: None,
        hidden: true,
        hires: SMALL_TEAM,
    },
    Template {
        key: "demo",
        name: "Stand-in team",
        description: "The product team on a stand-in with no AI, for tests and trying the CLI.",
        runtime: Some("fake"),
        hidden: true,
        hires: SMALL_TEAM,
    },
];

pub fn template(key: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.key.eq_ignore_ascii_case(key.trim()))
}

/// Someone to hire: what a template suggests, after you've edited it.
#[derive(Clone, Debug, PartialEq)]
pub struct Person {
    pub name: String,
    pub role: String,
    pub bio: String,
    pub runtime: String,
    pub model: Option<String>,
    pub department: Option<String>,
    /// A personality key. None means the one their role starts with.
    pub voice: Option<String>,
}

impl Template {
    /// The template's people, as a list you can change before hiring. `runtime` is used unless
    /// the template insists on its own, like the free demo.
    pub fn people(&self, runtime: &str) -> Vec<Person> {
        let runtime = self.runtime.unwrap_or(runtime);
        self.hires
            .iter()
            .map(|h| {
                let p = preset(h.preset);
                Person {
                    name: h.name.into(),
                    role: h.role.or(p.map(|p| p.role)).unwrap_or("Engineer").into(),
                    bio: String::new(),
                    runtime: runtime.into(),
                    model: crate::runtime::tier_model(runtime, tier_of(h.preset)).map(|(id, _)| id),
                    department: h.dept.map(String::from),
                    voice: None,
                }
            })
            .collect()
    }
}

/// Problems with a team before anything is created: missing names, two people who would get
/// the same id, runtimes that don't exist.
pub fn check(people: &[Person]) -> Result<()> {
    let mut ids: Vec<(String, &str)> = vec![];
    for p in people {
        let id = slug(&p.name);
        if id.is_empty() {
            anyhow::bail!("everyone needs a name with at least one letter or digit");
        }
        if let Some((_, other)) = ids.iter().find(|(i, _)| *i == id) {
            anyhow::bail!("{other} and {} would both be {id}. Give one of them a different first name", p.name);
        }
        if !crate::runtime::RUNTIMES.contains(&p.runtime.as_str()) {
            anyhow::bail!("{} works on {}, which isn't supported. Pick one of: {}", p.name, p.runtime, crate::runtime::RUNTIMES.join(", "));
        }
        ids.push((id, &p.name));
    }
    Ok(())
}

/// Hires everyone into a company, creating their departments. The first person into a
/// department leads it.
pub fn hire(c: &Company, people: &[Person]) -> Result<Vec<Employee>> {
    check(people)?;
    let mut hired = vec![];
    for p in people {
        let department = match p.department.as_deref().map(str::trim).filter(|d| !d.is_empty()) {
            Some(name) => Some(match c.department(name)? {
                Some(d) => d.id,
                None => c.add_department(name)?.id,
            }),
            None => None,
        };
        let e = Employee {
            id: slug(&p.name),
            name: p.name.trim().into(),
            role: if p.role.trim().is_empty() { "Engineer".into() } else { p.role.trim().into() },
            runtime: p.runtime.clone(),
            model: p.model.clone().filter(|m| !m.trim().is_empty()),
            bio: crate::roles::to_store(&p.role, &p.bio),
            department,
        };
        c.hire(&e)?;
        if let Some(v) = p.voice.as_deref().filter(|v| !v.is_empty()) {
            crate::voice::set(c, &e.id, v)?;
        }
        hired.push(e);
    }
    Ok(hired)
}

/// Hires a template as it is.
pub fn apply(c: &Company, t: &Template, runtime: &str) -> Result<Vec<Employee>> {
    hire(c, &t.people(runtime))
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
    fn roles_start_on_a_model_that_fits() {
        let startup = TEMPLATES.iter().find(|t| t.key == "startup").unwrap().people("claude");
        let model = |name: &str| startup.iter().find(|p| p.name.starts_with(name)).unwrap().model.clone();
        assert_eq!(model("Odile").as_deref(), Some("claude-opus-5-5"), "the CTO reviews and decides");
        assert_eq!(model("Maya").as_deref(), Some("claude-sonnet-5-5"));
        assert_eq!(model("Sam").as_deref(), Some("claude-haiku-5-5"), "short writing");
        // The stand-in has no models, so its people keep none.
        assert!(TEMPLATES.iter().find(|t| t.key == "startup").unwrap().people("fake").iter().all(|p| p.model.is_none()));
    }

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

        // An edited team: duplicates are caught before anything is created.
        let mut team = template("small").unwrap().people("claude");
        team[1].name = "Odile Two".into();
        assert!(check(&team).unwrap_err().to_string().contains("would both be odile"));
        team[1].name = "Inês".into();
        team[1].role = "Growth writer".into();
        assert!(check(&team).is_ok());

        // Editing keeps the id. Letting a lead go hands the department to the next person in it.
        let mut odile = c.employee("odile").unwrap().unwrap();
        odile.role = "CTO and reviewer".into();
        c.update_employee(&odile).unwrap();
        assert_eq!(c.employee("odile").unwrap().unwrap().role, "CTO and reviewer");
        c.fire("odile").unwrap();
        assert_eq!(c.department("Engineering").unwrap().unwrap().lead.as_deref(), Some("maya"));

        // What OneCorp reads from the repo replaces its page instead of piling up.
        c.set_wiki_section("codebase", "old").unwrap();
        c.set_wiki_section("codebase", "- Stack: Rust").unwrap();
        assert_eq!(crate::wiki::read(&c, "Engineering/Codebase").unwrap(), "- Stack: Rust");

        assert_eq!(suggest_name("My Notes App"), "my-notes-app");
        assert_eq!(suggest_name("  snowztech.com "), "snowztech-com");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
