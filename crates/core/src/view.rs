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
            // How they look: a chosen face seed, and a stamp for their photo so the app knows when to reload it.
            let (seed, photo) = c.avatar(&e.id).unwrap_or_default();
            let photo = photo.and_then(|p| std::fs::metadata(&p).ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs().to_string());
            json!({
                "id": e.id, "name": e.name, "role": e.role, "runtime": e.runtime, "model": e.model, "face": seed, "photo": photo,
                "today": c.spent_since(0, Some(&e.id)).map(|s| s.0).unwrap_or(0.0), "cap": c.daily_cap(&e.id).unwrap_or(0.0),
                "status": status, "bubble": bubble, "spend": spend(cost, tokens), "department": e.department,
                "ticket": ticket.map(|t| json!({"key": c.key(t.id), "title": t.title})),
            })
        })
        .collect();

    let question = |q: &crate::store::Question| {
            let options: Vec<&str> = q.options.split(", ").filter(|o| !o.is_empty()).collect();
            json!({
                "id": q.id, "kind": q.kind, "key": q.ticket_id.map(|t| c.key(t)),
                "employee": q.employee, "name": name_of(&q.employee), "question": q.question,
                // For a wiki proposal, `options` holds the section it goes under.
                "section": if q.kind == "wiki" { q.options.clone() } else { String::new() },
                "options": if q.kind == "wiki" { vec![] } else { options },
                "default": q.default_answer, "blocking": q.blocking,
                "via": q.via, "via_name": q.via.as_deref().map(name_of), "note": q.note,
            })
    };
    let inbox: Vec<Value> = c.inbox()?.iter().map(question).collect();
    // Questions a lead is still looking at. You can step in and answer them yourself.
    let with_leads: Vec<Value> = c
        .with_leads()?
        .iter()
        .map(|q| {
            let mut v = question(q);
            v["lead_name"] = json!(c.lead_of(&q.employee).ok().flatten().map(|l| l.name));
            v
        })
        .collect();

    let reviews: Vec<Value> = tickets
        .iter()
        .filter(|t| t.status == "review")
        .map(|t| {
            let who = t.assignee.clone().unwrap_or_default();
            // A plan's review is its tickets, which you can change before starting it.
            let tickets: Vec<Value> = if t.kind == "plan" {
                c.plan_tickets(t.id).unwrap_or_default().iter().map(|p| {
                    let who = p.assignee.clone().unwrap_or_default();
                    json!({"key": c.key(p.id), "title": p.title, "description": p.description, "assignee": p.assignee, "name": name_of(&who), "after": p.after.map(|a| c.key(a)), "status": p.status})
                }).collect()
            } else {
                vec![]
            };
            json!({
                "key": c.key(t.id), "title": t.title, "employee": who, "name": name_of(&who), "kind": t.kind, "tickets": tickets,
                "summary": last_said(c, t.id).unwrap_or_default(), "spend": spend(t.cost_usd, t.tokens),
                "pr_number": t.pr_number, "pr_url": t.pr_url,
            })
        })
        .collect();

    // Tickets that stopped or failed, with the last thing that went wrong, so they can be retried.
    let failed: Vec<Value> = tickets
        .iter()
        .filter(|t| t.status == "failed")
        .map(|t| {
            let who = t.assignee.clone().unwrap_or_default();
            let events = c.events(t.id).unwrap_or_default();
            let why = events.iter().rev().find(|(_, k, _)| k == "error" || k == "done").map(|(_, _, s)| s.clone()).unwrap_or_default();
            json!({"key": c.key(t.id), "title": t.title, "employee": who, "name": name_of(&who), "why": why})
        })
        .collect();

    let feed: Vec<Value> = c
        .recent_events(40)?
        .into_iter()
        .map(|(at, ticket, who, kind, summary, title)| {
            // "claude (opus), started" reads as just "started" in the feed.
            let summary = if kind == "start" { summary.rsplit(", ").next().unwrap_or("started").to_string() } else { summary };
            // One short line, whatever the event carries.
            let line = summary.lines().next().unwrap_or("").trim();
            let summary = if line.chars().count() > 90 { format!("{}…", line.chars().take(90).collect::<String>().trim_end()) } else { line.to_string() };
            json!({"at": at, "key": c.key(ticket), "employee": who, "name": name_of(&who), "kind": kind, "summary": summary, "title": title})
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

    let (chat_cost, chat_tokens) = c.chat_spend()?;
    let today = c.spent_since(0, None)?;
    let cost = tickets.iter().fold(chat_cost, |s, t| s + t.cost_usd);
    let tokens: i64 = chat_tokens + tickets.iter().map(|t| t.tokens).sum::<i64>();
    let all_messages = c.messages(None)?;
    // A direct message thread with each person, with its latest message for unread dots.
    let dms: Vec<Value> = team
        .iter()
        .map(|e| {
            let ch = crate::chat::dm(&e.id);
            json!({"id": e.id, "channel": ch, "last": all_messages.iter().filter(|m| m.channel == ch).map(|m| m.id).max()})
        })
        .collect();
    let channels: Vec<Value> = crate::chat::channels(c)?
        .into_iter()
        .map(|ch| {
            let last = all_messages.iter().filter(|m| m.channel == ch.name).map(|m| m.id).max();
            json!({"name": ch.name, "title": ch.title, "department": ch.department, "last": last})
        })
        .collect();
    let forge = crate::ship::forge(c)?;
    // Kickoff: what you're building, who'd plan it, and any planning happening now.
    let planning: Vec<Value> = tickets
        .iter()
        .filter(|t| t.kind == "plan" && ["todo", "queued", "doing"].contains(&t.status.as_str()))
        .map(|t| json!({"key": c.key(t.id), "name": name_of(t.assignee.as_deref().unwrap_or("")), "title": t.title}))
        .collect();
    // Projects: each plan, with how far its tickets got.
    let projects: Vec<Value> = tickets
        .iter()
        .filter(|t| t.kind == "plan" && t.status != "failed")
        .map(|p| {
            let mine: Vec<&Ticket> = tickets.iter().filter(|t| t.plan == Some(p.id)).collect();
            json!({
                "key": c.key(p.id), "title": p.title, "goal": p.description, "status": p.status,
                "planner": p.assignee, "planner_name": name_of(p.assignee.as_deref().unwrap_or("")),
                "total": mine.len(), "done": mine.iter().filter(|t| t.status == "done").count(),
            })
        })
        .collect();
    let kickoff = json!({
        "brief": c.brief()?,
        "planner": crate::work::pick_planner(c, None).ok().map(|e| e.id),
        "planned": tickets.iter().any(|t| t.kind == "plan"),
        "planning": planning,
    });
    Ok(json!({
        "kickoff": kickoff,
        "projects": projects,
        "paused": c.paused()?,
        "ship": {"mode": c.ship_mode()?, "forge": forge.map(|f| f.name()), "pr_word": forge.map(|f| f.pr_word())},
        "departments": departments,
        "company": c.name,
        "serve": crate::serve::running(),
        "employees": employees,
        "tickets": tickets.iter().map(|t| json!({
            "key": c.key(t.id), "title": t.title, "assignee": t.assignee, "status": t.status,
            "spend": spend(t.cost_usd, t.tokens), "after": t.after.map(|a| c.key(a)), "kind": t.kind,
            "created_by": (t.created_by != "founder").then(|| name_of(&t.created_by)),
            "plan": t.plan.map(|p| c.key(p)),
        })).collect::<Vec<_>>(),
        "inbox": inbox,
        "with_leads": with_leads,
        "reviews": reviews,
        "failed": failed,
        "feed": feed,
        "channels": channels,
        "dms": dms,
        "spend": {
            "cost": cost, "tokens": tokens, "label": spend(cost, tokens),
            "today": today.0, "today_label": spend(today.0, today.1), "budget": crate::spend::budget(c)?,
            "warn": crate::spend::warned_today(c)?, "paused_by_budget": crate::spend::paused_by_budget(c)?,
        },
    }))
}

/// Where the money goes: today, this week and all time, the last 14 days, and by person,
/// project and agent. Dollars are estimated from tokens where the agent doesn't report a cost.
pub fn spend_report(c: &Company) -> Result<Value> {
    let rows = c.spend_rows()?;
    let day = |n: i64| c.spent_day(n);
    let (today, week_start) = (day(0)?, day(6)?);
    let sum = |f: &dyn Fn(&crate::store::SpendRow) -> bool| {
        let (mut d, mut t) = (0.0, 0i64);
        for r in rows.iter().filter(|r| f(r)) {
            d += r.4;
            t += r.5;
        }
        json!({"cost": d, "tokens": t, "label": spend(d, t)})
    };
    let windows = |f: &dyn Fn(&crate::store::SpendRow) -> bool| {
        json!({
            "today": sum(&|r| f(r) && r.0 == today),
            "week": sum(&|r| f(r) && r.0 >= week_start),
            "all": sum(&|r| f(r)),
        })
    };
    let mut days = vec![];
    for n in (0..14).rev() {
        let d = day(n)?;
        let cost: f64 = rows.iter().filter(|r| r.0 == d).map(|r| r.4).sum();
        days.push(json!({"day": d, "cost": cost}));
    }
    let team = c.employees()?;
    let people: Vec<Value> = team
        .iter()
        .map(|e| {
            let mut v = windows(&|r| r.1 == e.id);
            v["id"] = json!(e.id);
            v["name"] = json!(e.name);
            v["runtime"] = json!(e.runtime);
            v["cap"] = json!(c.daily_cap(&e.id).unwrap_or(0.0));
            v
        })
        .collect();
    let projects: Vec<Value> = c
        .tickets()?
        .iter()
        .filter(|t| t.kind == "plan")
        .map(|p| {
            let mut v = windows(&|r| r.3 == Some(p.id));
            v["key"] = json!(c.key(p.id));
            v["title"] = json!(p.title);
            v
        })
        .collect();
    let mut runtimes: Vec<&str> = rows.iter().map(|r| r.2.as_str()).collect();
    runtimes.sort();
    runtimes.dedup();
    let agents: Vec<Value> = runtimes
        .iter()
        .map(|rt| {
            let mut v = windows(&|r| r.2 == *rt);
            v["runtime"] = json!(rt);
            v["estimated"] = json!(rows.iter().any(|r| r.2 == *rt && r.6));
            v["price"] = json!(crate::spend::price(c, rt).unwrap_or(0.0));
            v
        })
        .collect();
    Ok(json!({
        "totals": windows(&|_| true), "budget": crate::spend::budget(c)?, "days": days,
        "people": people, "projects": projects, "agents": agents,
        "prices": crate::spend::DEFAULT_PRICES.iter().map(|(rt, _)| json!({"runtime": rt, "price": crate::spend::price(c, rt).unwrap_or(0.0)})).collect::<Vec<_>>(),
        "estimated": rows.iter().any(|r| r.6),
    }))
}

/// One channel's messages, newest last, for the Chat page.
pub fn chat(c: &Company, channel: &str) -> Result<Value> {
    const SHOWN: usize = 200;
    let msgs = c.messages(Some(channel))?;
    let founder = crate::founder::name();
    let shown: Vec<Value> = msgs
        .iter()
        .skip(msgs.len().saturating_sub(SHOWN))
        .map(|m| {
            let name = if m.author == "founder" { founder.clone() } else { c.employee(&m.author).ok().flatten().map(|e| e.name).unwrap_or_else(|| m.author.clone()) };
            json!({
                "id": m.id, "at": m.at, "author": m.author, "name": name, "text": m.text,
                "key": m.ticket.map(|t| c.key(t)), "pending": m.pending, "founder": m.author == "founder",
            })
        })
        .collect();
    Ok(json!({"channel": channel, "messages": shown}))
}

/// One ticket for the ticket drawer: who, where, what it cost, and everything that happened.
pub fn ticket(c: &Company, t: &Ticket) -> Result<Value> {
    let who = t.assignee.clone().unwrap_or_default();
    let name = c.employee(&who)?.map(|e| e.name).unwrap_or_else(|| who.clone());
    let log: Vec<Value> = c.events(t.id)?.into_iter().map(|(at, kind, summary)| json!({"at": at, "kind": kind, "summary": summary})).collect();
    Ok(json!({
        "key": c.key(t.id), "title": t.title, "status": t.status, "employee": who, "name": name,
        "branch": t.branch, "base": t.base, "spend": spend(t.cost_usd, t.tokens), "log": log,
        "pr_number": t.pr_number, "pr_url": t.pr_url,
        "description": t.description, "after": t.after.map(|a| c.key(a)),
        "created_by": (t.created_by != "founder").then(|| c.employee(&t.created_by).ok().flatten().map(|e| e.name).unwrap_or_else(|| t.created_by.clone())),
    }))
}

/// Everything a reviewer needs for one ticket: the files, the commits and the patch.
pub fn review(c: &Company, ticket: &Ticket) -> Result<Value> {
    let key = c.key(ticket.id);
    let branch = ticket.branch.clone().ok_or_else(|| anyhow::anyhow!("{key} hasn't started, so there's nothing to review"))?;
    let repo = c.repo()?;
    let base = match &ticket.base {
        Some(b) => b.clone(),
        None => crate::git::current_branch(&repo)?,
    };
    let files = crate::git::changes(&repo, &base, &branch)?;
    let (patch, cut) = crate::git::patch(&repo, &base, &branch, 200_000)?;
    Ok(json!({
        "key": key, "title": ticket.title, "base": base, "branch": branch,
        "added": files.iter().map(|f| f.added).sum::<u32>(), "removed": files.iter().map(|f| f.removed).sum::<u32>(),
        "files": files.iter().map(|f| json!({"path": f.path, "added": f.added, "removed": f.removed})).collect::<Vec<_>>(),
        "commits": crate::git::commits(&repo, &base, &branch)?.iter().map(|(h, a, s)| json!({"hash": h, "author": a, "subject": s})).collect::<Vec<_>>(),
        "patch": patch, "truncated": cut,
    }))
}

/// One employee, for the person drawer: their tickets, what they're doing, what they've learned.
pub fn person(c: &Company, id: &str) -> Result<Value> {
    let e = c.employee(id)?.ok_or_else(|| anyhow::anyhow!("nobody called {id} works at {}", c.name))?;
    let mine: Vec<Ticket> = c.tickets()?.into_iter().filter(|t| t.assignee.as_deref() == Some(id)).collect();
    let current = mine.iter().rev().find(|t| ["doing", "blocked", "queued", "review"].contains(&t.status.as_str()));
    let activity: Vec<Value> = match current {
        Some(t) => c.events(t.id)?.into_iter().rev().take(40).rev().map(|(at, kind, summary)| json!({"at": at, "kind": kind, "summary": summary})).collect(),
        None => vec![],
    };
    let memory = c.memory(id)?;
    let notes: Vec<&str> = memory.lines().filter(|l| !l.trim().is_empty()).map(|l| l.trim_start_matches("- ")).collect();
    Ok(json!({
        "id": e.id, "name": e.name, "role": e.role, "runtime": e.runtime, "model": e.model, "bio": e.bio, "department": e.department,
        "voice": crate::voice::of(c, &e)?,
        "voices": crate::voice::list(),
        "current": current.map(|t| json!({"key": c.key(t.id), "title": t.title, "status": t.status})),
        "activity": activity,
        "tickets": mine.iter().rev().map(|t| json!({"key": c.key(t.id), "title": t.title, "status": t.status, "spend": spend(t.cost_usd, t.tokens)})).collect::<Vec<_>>(),
        "memory": notes,
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
        c.add_event(m, "start", "claude (opus), started", "").unwrap();
        c.add_event(m, "tool", "Edit src/auth.rs", "").unwrap();
        let t = c.add_ticket("pricing", "tomas").unwrap();
        c.set_status(t, "blocked").unwrap();

        let v = snapshot(&c).unwrap();
        let by = |id: &str| v["employees"].as_array().unwrap().iter().find(|e| e["id"] == id).unwrap().clone();
        assert_eq!((by("maya")["status"].as_str(), by("maya")["bubble"].as_str()), (Some("working"), Some("Edit src/auth.rs")));
        assert_eq!(by("tomas")["status"], "blocked");
        assert_eq!(by("tomas")["bubble"], "Need you on SNO-2");
        assert_eq!(by("lina")["status"], "idle");
        assert_eq!((v["feed"][0]["name"].as_str(), v["feed"][0]["summary"].as_str()), (Some("Maya Okafor"), Some("started")));
        assert_eq!(v["feed"].as_array().unwrap().len(), 1, "tool calls stay out of the feed");
        // Nobody is in a department yet, so they show up together until you place them.
        assert_eq!(v["departments"][0]["name"], "No department yet");
        let eng = c.add_department("Engineering").unwrap();
        c.set_department("maya", eng.id).unwrap();
        let v = snapshot(&c).unwrap();
        assert_eq!((v["departments"][0]["name"].as_str(), v["departments"][0]["lead"].as_str()), (Some("Engineering"), Some("maya")));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
