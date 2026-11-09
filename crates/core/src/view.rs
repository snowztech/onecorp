//! What the screens show, built from the store. The desktop app serializes this as is, so every
//! "who is doing what" rule lives here, once, and can be tested without a window.

use crate::runtime::first_line;
use crate::store::{Company, Ticket};
use crate::work::spend;
use anyhow::Result;
use serde_json::{Value, json};

/// Everything the Office and the Inbox need for one company.
pub fn snapshot(c: &Company) -> Result<Value> {
    let tickets = c.tickets()?;
    let team = c.employees()?;
    let name_of = |id: &str| team.iter().find(|e| e.id == id).map(|e| e.name.clone()).unwrap_or_else(|| id.to_string());

    let employees: Vec<Value> = team
        .iter()
        .map(|e| {
            let theirs: Vec<&Ticket> = tickets.iter().filter(|t| t.assignee.as_deref() == Some(&e.id)).collect();
            let first = |s: &str| theirs.iter().find(|t| t.status == s).copied();
            // What a person is doing right now: working beats blocked beats waiting in line.
            let (status, ticket) = match (first("doing"), first("blocked"), first("queued")) {
                (Some(t), _, _) => ("working", Some(t)),
                (None, Some(t), _) => ("blocked", Some(t)),
                (None, None, Some(t)) => ("queued", Some(t)),
                _ => ("idle", None),
            };
            let bubble = match (status, ticket) {
                ("working", Some(t)) => last_activity(c, t.id).unwrap_or_default(),
                ("blocked", Some(t)) => format!("Need you on {}", c.key(t.id)),
                ("queued", Some(t)) => format!("Up next: {}", c.key(t.id)),
                _ => String::new(),
            };
            let cost = theirs.iter().fold(0.0, |s, t| s + t.cost_usd);
            let tokens: i64 = theirs.iter().map(|t| t.tokens).sum();
            json!({
                "id": e.id, "name": e.name, "role": e.role, "runtime": e.runtime, "model": e.model,
                "status": status, "bubble": bubble, "spend": spend(cost, tokens), "department": e.department,
                "ticket": ticket.map(|t| json!({"key": c.key(t.id), "title": t.title})),
            })
        })
        .collect();

    let inbox: Vec<Value> = c
        .inbox()?
        .iter()
        .map(|q| {
            let options: Vec<&str> = q.options.split(", ").filter(|o| !o.is_empty()).collect();
            json!({
                "id": q.id, "kind": q.kind, "key": q.ticket_id.map(|t| c.key(t)),
                "employee": q.employee, "name": name_of(&q.employee), "question": q.question,
                // For a handbook proposal, `options` holds the section it goes under.
                "section": if q.kind == "handbook" { q.options.clone() } else { String::new() },
                "options": if q.kind == "handbook" { vec![] } else { options },
                "default": q.default_answer, "blocking": q.blocking,
            })
        })
        .collect();

    let reviews: Vec<Value> = tickets
        .iter()
        .filter(|t| t.status == "review")
        .map(|t| {
            let who = t.assignee.clone().unwrap_or_default();
            json!({
                "key": c.key(t.id), "title": t.title, "employee": who, "name": name_of(&who),
                "summary": last_said(c, t.id).unwrap_or_default(), "spend": spend(t.cost_usd, t.tokens),
            })
        })
        .collect();

    let feed: Vec<Value> = c
        .recent_events(40)?
        .into_iter()
        .map(|(at, ticket, who, kind, summary)| {
            json!({"at": at, "key": c.key(ticket), "employee": who, "name": name_of(&who), "kind": kind, "summary": summary})
        })
        .collect();

    // The org chart: departments with their people, plus anyone not in one yet.
    let mut departments: Vec<Value> = c
        .departments()?
        .iter()
        .map(|d| {
            let members: Vec<&str> = team.iter().filter(|e| e.department == Some(d.id)).map(|e| e.id.as_str()).collect();
            json!({"id": d.id, "name": d.name, "lead": d.lead, "members": members})
        })
        .collect();
    let loose: Vec<&str> = team.iter().filter(|e| e.department.is_none()).map(|e| e.id.as_str()).collect();
    if !loose.is_empty() {
        departments.push(json!({"id": null, "name": "No department yet", "lead": null, "members": loose}));
    }

    let cost = tickets.iter().fold(0.0, |s, t| s + t.cost_usd);
    let tokens: i64 = tickets.iter().map(|t| t.tokens).sum();
    Ok(json!({
        "departments": departments,
        "company": c.name,
        "serve": crate::serve::running(),
        "employees": employees,
        "tickets": tickets.iter().map(|t| json!({
            "key": c.key(t.id), "title": t.title, "assignee": t.assignee, "status": t.status,
            "spend": spend(t.cost_usd, t.tokens),
        })).collect::<Vec<_>>(),
        "inbox": inbox,
        "reviews": reviews,
        "feed": feed,
        "spend": { "cost": cost, "tokens": tokens, "label": spend(cost, tokens) },
    }))
}

/// The latest thing a working employee said or did, short enough for a speech bubble.
fn last_activity(c: &Company, ticket: i64) -> Option<String> {
    let events = c.events(ticket).ok()?;
    let (_, kind, summary) = events.iter().rev().find(|(_, k, _)| k == "say" || k == "tool")?;
    let text = if kind == "tool" { summary.clone() } else { first_line(summary) };
    Some(text.chars().take(70).collect())
}

fn last_said(c: &Company, ticket: i64) -> Option<String> {
    let events = c.events(ticket).ok()?;
    events.iter().rev().find(|(_, k, _)| k == "say").map(|(_, _, s)| s.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Employee;

    #[test]
    fn shows_who_is_doing_what() {
        let dir = std::env::temp_dir().join(format!("onecorp-view-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let c = Company::open_dir("snowztech", dir.clone()).unwrap();
        for (id, name) in [("maya", "Maya Okafor"), ("tomas", "Tomás Reis"), ("lina", "Lina Park")] {
            let e = Employee { id: id.into(), name: name.into(), role: "Engineer".into(), runtime: "claude".into(), model: None, bio: String::new(), department: None };
            c.hire(&e).unwrap();
        }
        let m = c.add_ticket("magic links", "maya").unwrap();
        c.set_status(m, "doing").unwrap();
        c.add_event(m, "tool", "Edit src/auth.rs", "").unwrap();
        let t = c.add_ticket("pricing", "tomas").unwrap();
        c.set_status(t, "blocked").unwrap();

        let v = snapshot(&c).unwrap();
        let by = |id: &str| v["employees"].as_array().unwrap().iter().find(|e| e["id"] == id).unwrap().clone();
        assert_eq!((by("maya")["status"].as_str(), by("maya")["bubble"].as_str()), (Some("working"), Some("Edit src/auth.rs")));
        assert_eq!(by("tomas")["status"], "blocked");
        assert_eq!(by("tomas")["bubble"], "Need you on SNO-2");
        assert_eq!(by("lina")["status"], "idle");
        assert_eq!(v["feed"][0]["name"], "Maya Okafor");
        // Nobody is in a department yet, so they show up together until you place them.
        assert_eq!(v["departments"][0]["name"], "No department yet");
        let eng = c.add_department("Engineering").unwrap();
        c.set_department("maya", eng.id).unwrap();
        let v = snapshot(&c).unwrap();
        assert_eq!((v["departments"][0]["name"].as_str(), v["departments"][0]["lead"].as_str()), (Some("Engineering"), Some("maya")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
