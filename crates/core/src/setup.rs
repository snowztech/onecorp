//! Creating a company in one step, so the CLI and the app can't drift apart: the company itself,
//! its starting team, and how its work ships.

use crate::store::{Company, Employee};
use crate::templates::{self, Template};
use crate::ship;
use anyhow::Result;
use std::path::Path;

/// `brief` is what you're building. With a brief and a team, planning starts right away.
pub fn new_company(name: &str, repo: &Path, mission: &str, team: Option<&Template>, runtime: &str, brief: &str) -> Result<(Company, Vec<Employee>)> {
    let c = Company::create(name, repo, mission)?;
    c.set_ship_mode(ship::default_mode(&c.repo()?))?;
    let hired = match team {
        Some(t) => templates::apply(&c, t, runtime)?,
        None => vec![],
    };
    crate::store::set_current(&c.name)?;
    if !brief.trim().is_empty() {
        c.write_brief(brief)?;
        if !hired.is_empty() {
            crate::work::kickoff(&c, None)?;
        }
    }
    Ok((c, hired))
}
