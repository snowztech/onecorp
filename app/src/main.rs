// The OneCorp desktop app. A thin shell: every command opens the company and calls the library,
// the same functions the CLI uses. The UI in ui/ polls `snapshot` and calls the actions.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use onecorp::store::{self, Company};
use onecorp::work;
use serde_json::{Value, json};

type Res<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

fn open(company: &str) -> Res<Company> {
    Company::open(company).map_err(err)
}

#[tauri::command(async)]
fn companies() -> Res<Value> {
    let list = store::list().map_err(err)?;
    let current = store::current().ok().filter(|c| list.contains(c)).or_else(|| list.first().cloned());
    // Each company's badge: its color, and its logo inlined so the webview can show it.
    let looks: serde_json::Map<String, Value> = list
        .iter()
        .filter_map(|n| {
            let (color, logo) = Company::open(n).ok()?.look().ok()?;
            Some((n.clone(), json!({ "color": color, "logo": logo.as_deref().and_then(data_url) })))
        })
        .collect();
    // A dev build says so, and which home it runs on, so it's never mistaken for the installed app.
    let dev = cfg!(debug_assertions).then(|| store::home().display().to_string());
    Ok(json!({ "list": list, "current": current, "looks": looks, "dev": dev }))
}

/// An image file as a data URL, so the webview can show it without file access.
fn data_url(p: &std::path::Path) -> Option<String> {
    let bytes = std::fs::read(p).ok()?;
    let mime = match p.extension()?.to_string_lossy().to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg".to_string(),
        "svg" => "image/svg+xml".to_string(),
        other => format!("image/{other}"),
    };
    Some(format!("data:{mime};base64,{}", base64(&bytes)))
}

/// A preview of an image you just picked, before anything is saved.
#[tauri::command(async)]
fn image_preview(path: String) -> Res<String> {
    let p = std::path::Path::new(&path);
    let ok = p.extension().is_some_and(|e| ["png", "jpg", "jpeg", "webp", "svg", "gif"].contains(&e.to_string_lossy().to_lowercase().as_str()));
    if !ok || std::fs::metadata(p).map_err(|e| e.to_string())?.len() > 4 * 1024 * 1024 {
        return Err("Pick an image under 4 MB: png, jpg, webp, svg or gif.".into());
    }
    data_url(p).ok_or_else(|| "Couldn't read that image.".into())
}

/// A company's color, and its logo: a new file, removed, or kept as it is.
#[tauri::command(async)]
fn set_look(company: String, color: String, logo: Option<String>, remove_logo: bool) -> Res<()> {
    let path = logo.filter(|l| !l.is_empty()).map(std::path::PathBuf::from);
    open(&company)?.set_look(&color, path.as_deref(), path.is_none() && !remove_logo).map_err(err)
}

#[tauri::command(async)]
fn use_company(name: String) -> Res<()> {
    open(&name)?;
    store::set_current(&name).map_err(err)
}

#[tauri::command(async)]
fn snapshot(company: String) -> Res<Value> {
    let c = open(&company)?;
    c.recover_interrupted().map_err(err)?;
    onecorp::view::snapshot(&c).map_err(err)
}

/// Creates a ticket. `later` saves it to the backlog, otherwise it starts now (or waits for `after`).
// Tauri maps each named field from the app to an argument, so the list is the form.
#[allow(clippy::too_many_arguments)]
#[tauri::command(async)]
fn new_ticket(company: String, title: String, employee: Option<String>, description: Option<String>, after: Option<String>, project: Option<String>, repo: Option<String>, later: bool) -> Res<String> {
    let c = open(&company)?;
    let employee = employee.filter(|e| !e.is_empty());
    if employee.is_none() && !later {
        return Err("Pick who does it, or save it for later.".into());
    }
    let after = after.filter(|a| !a.is_empty()).map(|k| ticket(&c, &k).map(|t| t.id)).transpose()?;
    // Part of a project: the ticket counts toward its progress and shows under it on the Board.
    let plan = project.filter(|p| !p.is_empty()).map(|k| ticket(&c, &k)).transpose()?.filter(|t| t.kind == "plan").map(|t| t.id);
    let new = store::NewTicket { title, description: description.unwrap_or_default(), assignee: employee, after, plan, repo, ..Default::default() };
    let id = c.new_ticket(&new, "backlog").map_err(err)?;
    if !later {
        work::start(&c, &ticket(&c, &c.key(id))?).map_err(err)?;
    }
    Ok(c.key(id))
}

#[tauri::command(async)]
fn start_ticket(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::start(&c, &t).map_err(err)
}

#[tauri::command(async)]
fn assign(company: String, key: String, employee: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    c.assign(t.id, &employee).map_err(err)
}

#[tauri::command(async)]
fn answer(company: String, id: i64, text: String) -> Res<()> {
    work::answer(&open(&company)?, id, &text).map_err(err)
}

/// Closes a report you've read without starting what it proposed.
#[tauri::command(async)]
fn close_report(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::finish_report(&c, &t, false).map_err(err)?;
    work::release_waiting(&c, t.id).map_err(err)
}

#[tauri::command(async)]
fn dismiss_question(company: String, id: i64) -> Res<()> {
    work::dismiss(&open(&company)?, id).map_err(err)
}

#[tauri::command(async)]
fn approve(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    onecorp::ship::approve(&c, &t).map_err(err)
}

#[tauri::command(async)]
fn request_changes(company: String, key: String, feedback: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::request_changes(&c, &t, &feedback).map_err(err)
}

/// What the new company flow needs: the starting teams, and which runtimes are installed here.
/// The founder profile, with the photo inlined so the UI can show it without serving files.
#[tauri::command(async)]
fn founder() -> Res<Value> {
    let Some(f) = onecorp::founder::load() else { return Ok(Value::Null) };
    let photo = f.photo.as_deref().and_then(data_url);
    Ok(json!({ "name": f.name, "photo": photo }))
}

/// `photo`: a file to use. `remove_photo`: back to the generated face. Neither: keep what's there.
#[tauri::command(async)]
fn save_founder(name: String, photo: Option<String>, remove_photo: bool) -> Res<()> {
    use onecorp::founder::Photo;
    let path = photo.filter(|p| !p.trim().is_empty()).map(std::path::PathBuf::from);
    let choice = match (&path, remove_photo) {
        (Some(p), _) => Photo::Use(p),
        (None, true) => Photo::Remove,
        (None, false) => Photo::Keep,
    };
    onecorp::founder::save(&name, choice).map(|_| ()).map_err(err)
}

/// Standard base64, for inlining the founder's photo. Small enough not to need a dependency.
fn base64(bytes: &[u8]) -> String {
    const ABC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        for i in 0..4 {
            out.push(if i <= chunk.len() { ABC[(n >> (18 - 6 * i) & 63) as usize] as char } else { '=' });
        }
    }
    out
}

/// What the Settings page shows about a company.
#[tauri::command(async)]
fn settings(company: String) -> Res<Value> {
    let c = open(&company)?;
    let repo = c.repo().map_err(err)?;
    let detected = onecorp::forge::Forge::detect(&repo);
    // The agents this team runs on, so the talk choice names only the models that apply.
    let runtimes: Vec<String> = c.employees().map_err(err)?.into_iter().map(|e| e.runtime).collect();
    let talk_models: serde_json::Map<String, Value> = ["cheapest", "balanced"]
        .iter()
        .map(|t| {
            let names: Vec<String> = ["claude", "codex"].iter().filter(|r| runtimes.iter().any(|x| x == *r)).filter_map(|r| onecorp::runtime::talk_model(r, t).map(|(_, n)| n)).collect();
            (t.to_string(), json!(names))
        })
        .collect();
    Ok(json!({
        "name": c.name, "repo": repo, "data": c.dir, "home": store::home(),
        "ship": c.ship_mode().map_err(err)?, "forge": detected.map(|f| f.mode()), "forge_name": detected.map(|f| f.name()),
        "origin": onecorp::git::remote_url(&repo, "origin"),
        "tickets": c.tickets().map_err(err)?.len(), "people": c.employees().map_err(err)?.len(),
        "version": env!("CARGO_PKG_VERSION"),
        "repos": repos_json(&c).map_err(err)?,
        "commits": c.setting("commits").map_err(err)?.unwrap_or_else(|| if onecorp::git::config(&repo, "user.name").is_some() { "you" } else { "team" }.into()),
        "pr_sign": c.setting("pr_sign").map_err(err)?.as_deref() != Some("off"),
        "talk": onecorp::work::talk_tier(&c).map_err(err)?,
        // The model names each choice means, for the agents this team uses.
        "talk_models": talk_models,
        "git_you": onecorp::git::config(&repo, "user.name").zip(onecorp::git::config(&repo, "user.email")).map(|(n, e)| json!({"name": n, "email": e})),
    }))
}

/// Each repo with the build and test commands employees may run in it.
fn repos_json(c: &Company) -> anyhow::Result<Vec<Value>> {
    c.repos()?
        .iter()
        .map(|r| Ok(json!({"name": r.name, "path": r.path, "main": r.main, "commands": onecorp::commands::list(c, r)?, "custom": onecorp::commands::custom(c, r)?})))
        .collect()
}

/// A repo's build and test commands. No text goes back to what the repo suggests.
#[tauri::command(async)]
fn set_commands(company: String, repo: String, text: Option<String>) -> Res<Vec<Value>> {
    let c = open(&company)?;
    let r = c.repo_named(&repo).map_err(err)?;
    match text {
        Some(t) => onecorp::commands::set(&c, &r, &t),
        None => onecorp::commands::reset(&c, &r),
    }
    .map_err(err)?;
    repos_json(&c).map_err(err)
}

/// Which model answers in chat and decides as a lead: "light" (a smaller one) or "same".
#[tauri::command(async)]
fn set_talk(company: String, mode: String) -> Res<()> {
    if !["cheapest", "balanced", "same"].contains(&mode.as_str()) {
        return Err(format!("{mode} isn't a choice. Pick cheapest, balanced or same."));
    }
    open(&company)?.set_setting("talk", &mode).map_err(err)
}

/// Who commits are by: "team" (the employee, with you as committer and co-author) or "you".
#[tauri::command(async)]
fn set_commits(company: String, mode: String) -> Res<()> {
    if !["team", "you"].contains(&mode.as_str()) {
        return Err(format!("{mode} isn't a choice. Pick team or you."));
    }
    open(&company)?.set_setting("commits", &mode).map_err(err)
}

/// Whether pull requests end with who did the work, at which company, made with OneCorp.
#[tauri::command(async)]
fn set_pr_sign(company: String, on: bool) -> Res<()> {
    open(&company)?.set_setting("pr_sign", if on { "on" } else { "off" }).map_err(err)
}

#[tauri::command(async)]
fn add_repo(company: String, path: String, name: Option<String>) -> Res<Vec<Value>> {
    let c = open(&company)?;
    c.add_repo(std::path::Path::new(path.trim()), name.as_deref()).map_err(err)?;
    repos_json(&c).map_err(err)
}

#[tauri::command(async)]
fn remove_repo(company: String, name: String) -> Res<Vec<Value>> {
    let c = open(&company)?;
    c.remove_repo(&name).map_err(err)?;
    repos_json(&c).map_err(err)
}

#[tauri::command(async)]
fn set_ship(company: String, mode: String) -> Res<()> {
    onecorp::ship::set_mode(&open(&company)?, &mode).map_err(err)
}

fn checks(list: Vec<onecorp::doctor::Check>) -> Value {
    let level = |l: &onecorp::doctor::Level| match l {
        onecorp::doctor::Level::Ok => "ok",
        onecorp::doctor::Level::Warn => "warn",
        onecorp::doctor::Level::Fail => "fail",
    };
    json!(list.iter().map(|c| json!({"level": level(&c.level), "what": c.what, "detail": c.detail, "fix": c.fix})).collect::<Vec<_>>())
}

/// What waits on you in every company: questions, plans, reviews and stopped runs. Drives the
/// Dock badge and notifications, so another company's question doesn't go unnoticed.
#[tauri::command(async)]
fn attention() -> Res<Value> {
    let mut out = vec![];
    for name in store::list().map_err(err)? {
        let Ok(c) = Company::open(&name) else { continue };
        let Ok(s) = onecorp::view::snapshot(&c) else { continue };
        out.push(json!({ "company": name, "inbox": s["inbox"], "reviews": s["reviews"], "failed": s["failed"], "spend": s["spend"] }));
    }
    Ok(json!(out))
}

#[tauri::command(async)]
fn spend_report(company: String) -> Res<Value> {
    onecorp::view::spend_report(&open(&company)?).map_err(err)
}

/// The daily budget in dollars, 0 for none.
#[tauri::command(async)]
fn set_budget(company: String, dollars: f64) -> Res<()> {
    onecorp::spend::set_budget(&open(&company)?, dollars).map_err(err)
}

#[tauri::command(async)]
fn set_cap(company: String, id: String, dollars: f64) -> Res<()> {
    open(&company)?.set_daily_cap(&id, dollars).map_err(err)
}

/// Dollars per million tokens, for estimating agents that report only tokens.
#[tauri::command(async)]
fn set_price(company: String, runtime: String, per_million: f64) -> Res<()> {
    onecorp::spend::set_price(&open(&company)?, &runtime, per_million).map_err(err)
}

/// What you pay a month for an agent's plan, for every company. 0 means you pay per use.
#[tauri::command(async)]
fn set_plan(runtime: String, dollars: f64) -> Res<()> {
    onecorp::spend::set_plan(&runtime, dollars).map_err(err)
}

/// Go on past today's budget. It applies again tomorrow.
#[tauri::command(async)]
fn keep_going_today(company: String) -> Res<()> {
    onecorp::spend::resume_for_today(&open(&company)?).map_err(err)
}

/// The same checks as `onecorp doctor`.
#[tauri::command(async)]
fn health() -> Res<Value> {
    Ok(checks(onecorp::doctor::run()))
}

/// What this Mac needs before a company can work: git, an agent CLI, the GitHub CLI.
#[tauri::command(async)]
fn readiness() -> Res<Value> {
    Ok(checks(onecorp::doctor::machine()))
}

/// Opens Terminal running a setup command, like `claude auth login`. Only the fixes OneCorp's own
/// checks suggest, so the page can't ask for anything else.
#[tauri::command(async)]
fn in_terminal(command: String) -> Res<()> {
    let allowed = onecorp::doctor::machine().into_iter().filter_map(|c| c.fix).any(|f| f == command);
    if !allowed {
        return Err("OneCorp only runs its own setup commands.".into());
    }
    let script = format!("tell application \"Terminal\"\nactivate\ndo script \"{}\"\nend tell", command.replace('\\', "\\\\").replace('"', "\\\""));
    std::process::Command::new("osascript").arg("-e").arg(script).status().map_err(|e| format!("couldn't open Terminal: {e}"))?;
    Ok(())
}

#[tauri::command(async)]
fn delete_company(name: String) -> Res<()> {
    onecorp::setup::delete_company(&name).map_err(err)
}

#[tauri::command(async)]
fn forget_founder() -> Res<()> {
    onecorp::founder::remove().map_err(err)
}

#[tauri::command(async)]
fn setup_info() -> Res<Value> {
    let runtimes = onecorp::doctor::installed_runtimes();
    let teams: Vec<Value> = onecorp::templates::TEMPLATES
        .iter()
        .filter(|t| !t.hidden)
        .map(|t| {
            json!({
                "key": t.key, "name": t.name, "description": t.description, "runtime": t.runtime,
                "hires": t.people("claude").iter().zip(t.hires).map(|(p, h)| json!({"name": p.name, "role": p.role, "dept": p.department, "tier": onecorp::templates::tier_of(h.preset), "voice": onecorp::voice::for_role(&p.role, &store::slug(&p.name))})).collect::<Vec<_>>(),
            })
        })
        .collect();
    // Your other companies' teams, to start this one with the same people.
    let mut others = vec![];
    for name in store::list().unwrap_or_default() {
        let Ok(c) = Company::open(&name) else { continue };
        let depts = c.departments().unwrap_or_default();
        let hires: Vec<Value> = c
            .employees()
            .unwrap_or_default()
            .iter()
            .map(|e| {
                let dept = e.department.and_then(|id| depts.iter().find(|d| d.id == id)).map(|d| d.name.clone());
                json!({"name": e.name, "role": e.role, "dept": dept, "runtime": e.runtime, "model": e.model, "voice": onecorp::voice::of(&c, e).unwrap_or_default()})
            })
            .collect();
        if !hires.is_empty() {
            others.push(json!({"company": name, "hires": hires}));
        }
    }
    Ok(json!({ "templates": teams, "runtimes": runtimes, "voices": onecorp::voice::list(), "teams_from": others }))
}

/// Checks a folder before it becomes a company's repo, and suggests a name from it.
#[tauri::command(async)]
fn check_repo(path: String) -> Res<Value> {
    let path = std::path::PathBuf::from(path.trim());
    let problem = |p: &str| Ok(json!({ "ok": false, "problem": p }));
    let Ok(full) = path.canonicalize() else { return problem("That folder doesn't exist.") };
    if !full.join(".git").exists() {
        return problem("That folder isn't a git repo. Run git init in it first, or pick its top folder.");
    }
    let branch = match onecorp::git::default_branch(&full) {
        Ok(b) => b,
        Err(_) => return problem("The repo is on a detached HEAD. Check out a branch, like main, then try again."),
    };
    let folder = full.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let used_by: Vec<String> = store::list()
        .map_err(err)?
        .into_iter()
        .filter(|c| Company::open(c).ok().and_then(|c| c.repos().ok()).is_some_and(|rs| rs.iter().any(|r| r.path == full)))
        .collect();
    // How this company would ship, so the flow can say so before you create it.
    let forge = onecorp::forge::Forge::detect(&full);
    let signed_in = forge.map(|f| f.signed_in(&full));
    Ok(json!({
        "ok": true, "path": full, "branch": branch, "suggested": onecorp::templates::suggest_name(&folder), "used_by": used_by,
        "forge": forge.map(|f| f.name()), "pr_word": forge.map(|f| f.pr_word()),
        "signed_in": signed_in.as_ref().map(|s| s.is_ok()), "account": signed_in.and_then(|s| s.ok()),
    }))
}

/// `people` is the team as you edited it: name, role, runtime, department.
// Tauri maps each named field from the app to an argument, so the list is the form.
#[allow(clippy::too_many_arguments)]
#[tauri::command(async)]
fn create_company(name: String, repo: String, mission: String, people: Vec<Value>, brief: Option<String>, color: Option<String>, logo: Option<String>, repos: Option<Vec<Value>>, repo_name: Option<String>) -> Res<String> {
    let s = |v: &Value, k: &str| v[k].as_str().unwrap_or("").trim().to_string();
    let people: Vec<onecorp::templates::Person> = people
        .iter()
        .map(|p| onecorp::templates::Person {
            name: s(p, "name"),
            role: s(p, "role"),
            bio: String::new(),
            runtime: s(p, "runtime"),
            // A model you kept from another team, or the one the person's tier means on their agent.
            model: Some(s(p, "model")).filter(|m| !m.is_empty()).or_else(|| onecorp::runtime::tier_model(&s(p, "runtime"), &s(p, "tier")).map(|(id, _)| id)),
            department: Some(s(p, "dept")).filter(|d| !d.is_empty()),
            voice: Some(s(p, "voice")).filter(|v| !v.is_empty()),
        })
        .collect();
    let (c, _) = onecorp::setup::new_company(name.trim(), std::path::Path::new(repo.trim()), &mission, &people, brief.as_deref().unwrap_or("")).map_err(err)?;
    // What tickets and the planner call the primary repo, when you named it yourself.
    if let Some(n) = repo_name.map(|n| onecorp::templates::suggest_name(&n)).filter(|n| !n.is_empty()) {
        let _ = c.set_setting("repo.name", &n);
    }
    // The other repos were checked in the flow. One that went missing since is left out, and can
    // be added again from Settings.
    for r in repos.unwrap_or_default() {
        let path = s(&r, "path");
        if let Err(e) = c.add_repo(std::path::Path::new(&path), Some(&s(&r, "name"))) {
            eprintln!("couldn't add {path}: {e:#}");
        }
    }
    // The look is a nicety: the company exists either way, so a bad logo file shouldn't undo it.
    let logo = logo.filter(|l| !l.is_empty()).map(std::path::PathBuf::from);
    let _ = c.set_look(color.as_deref().unwrap_or("#27272a"), logo.as_deref(), logo.is_none());
    Ok(c.name)
}

/// Your GitHub repos, for cloning one in onboarding.
#[tauri::command(async)]
fn github_repos() -> Res<Value> {
    let list = onecorp::forge::Forge::GitHub.repos().map_err(err)?;
    Ok(list.into_iter().map(|(name, description, private)| json!({"name": name, "description": description, "private": private})).collect())
}

#[tauri::command(async)]
fn clone_repo(name: String, path: Option<String>) -> Res<String> {
    let path = path.map(|p| std::path::PathBuf::from(p.trim())).filter(|p| !p.as_os_str().is_empty());
    onecorp::setup::clone_from_github(&name, path.as_deref()).map(|p| p.display().to_string()).map_err(err)
}

/// Where a clone of owner/name would go, to show before cloning.
#[tauri::command]
fn clone_target(name: String) -> String {
    onecorp::setup::clone_target(&name).display().to_string()
}

/// Where clones and new repos go from now on.
#[tauri::command(async)]
fn set_projects_folder(path: String) -> Res<String> {
    onecorp::setup::set_default_parent(std::path::Path::new(path.trim())).map(|p| p.display().to_string()).map_err(err)
}

/// Where a new repo would go, and where you could also create it on GitHub.
#[tauri::command(async)]
fn new_repo_info() -> Res<Value> {
    let gh = onecorp::forge::Forge::GitHub;
    let owners = gh.owners().unwrap_or_default();
    Ok(json!({ "parent": onecorp::setup::default_parent(), "owners": owners }))
}

/// Starts a new repo. `owner` set: also create it on GitHub under that account or org.
#[tauri::command(async)]
fn create_repo(parent: String, folder: String, mission: String, owner: Option<String>, private: bool) -> Res<String> {
    let folder = folder.trim();
    if folder.is_empty() || folder.contains('/') {
        return Err("Name the folder, like notes-app.".into());
    }
    let path = std::path::Path::new(parent.trim()).join(folder);
    let full = owner.filter(|o| !o.is_empty()).map(|o| format!("{o}/{}", onecorp::templates::suggest_name(folder)));
    let remote = full.as_deref().map(|f| onecorp::setup::Remote { full_name: f, private });
    let made = onecorp::setup::create_repo(&path, folder, &mission, remote).map_err(err)?;
    Ok(made.to_string_lossy().into_owned())
}

/// Plans a project into tickets. Returns the plan ticket.
#[tauri::command(async)]
fn plan_project(company: String, name: String, goal: String, by: Option<String>) -> Res<String> {
    let c = open(&company)?;
    let id = work::plan_project(&c, &name, &goal, by.as_deref().filter(|b| !b.is_empty())).map_err(err)?;
    Ok(c.key(id))
}

/// Saves the brief and has someone plan it. Returns the plan ticket.
#[tauri::command(async)]
fn kickoff(company: String, brief: String, by: Option<String>) -> Res<String> {
    let c = open(&company)?;
    if brief.trim().is_empty() {
        return Err("Say what you're building first.".into());
    }
    c.write_brief(&brief).map_err(err)?;
    let id = work::kickoff(&c, by.as_deref().filter(|b| !b.is_empty())).map_err(err)?;
    Ok(c.key(id))
}

#[tauri::command(async)]
fn save_brief(company: String, brief: String) -> Res<()> {
    open(&company)?.write_brief(&brief).map_err(err)
}

#[tauri::command(async)]
fn dismiss_plan(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::dismiss_plan(&c, &t).map_err(err)
}

/// Removes a ticket that never started, like one you don't want from a plan.
#[tauri::command(async)]
fn delete_ticket(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    c.delete_ticket(t.id).map_err(err)
}

/// Throws away a ticket's work in review. The ticket stays, back in the backlog.
#[tauri::command(async)]
fn discard_ticket(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::discard(&c, &t).map_err(err)
}

/// Deletes a merged, stopped or unstarted ticket from anywhere, not just a plan.
#[tauri::command(async)]
fn remove_ticket(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::delete(&c, &t).map_err(err)
}

#[tauri::command(async)]
fn wiki(company: String) -> Res<Value> {
    onecorp::view::wiki(&open(&company)?).map_err(err)
}

#[tauri::command(async)]
fn page_history(company: String, path: String) -> Res<Value> {
    onecorp::view::page_history(&open(&company)?, &path).map_err(err)
}

/// Saves a wiki page as yours. With `from`, the page was renamed or moved to `path` too.
/// Returns where it is now.
#[tauri::command(async)]
fn save_page(company: String, path: String, text: String, from: Option<String>) -> Res<String> {
    let c = open(&company)?;
    let path = onecorp::wiki::clean(&path).map_err(err)?;
    match from.as_deref().filter(|f| !f.is_empty()) {
        None if onecorp::wiki::exists(&c, &path) => return Err(format!("There's already a page called {path}.")),
        Some(f) if f != path => onecorp::wiki::rename(&c, f, &path, "founder").map_err(err)?,
        _ => {}
    }
    onecorp::wiki::write(&c, &path, &text, "founder", "").map_err(err)?;
    Ok(path)
}

/// Has the team draft the wiki's pages from the repo again. Earlier versions stay in each page's history.
#[tauri::command(async)]
fn redraft_wiki(company: String, by: Option<String>) -> Res<()> {
    work::write_wiki(&open(&company)?, by.as_deref().filter(|b| !b.is_empty())).map(|_| ()).map_err(err)
}

#[tauri::command(async)]
fn delete_page(company: String, path: String) -> Res<()> {
    onecorp::wiki::remove(&open(&company)?, &path, "founder").map_err(err)
}

#[tauri::command(async)]
fn restore_page(company: String, id: i64) -> Res<String> {
    onecorp::wiki::restore(&open(&company)?, id, "founder").map_err(err)
}

#[tauri::command(async)]
fn chat(company: String, channel: String) -> Res<Value> {
    onecorp::view::chat(&open(&company)?, &channel).map_err(err)
}

/// Opens a web link in your browser. Only http and https, so nothing else can be launched this way.
/// The opener plugin's URL check refused long query strings like a prefilled bug report.
#[tauri::command(async)]
fn open_link(url: String) -> Res<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Only web links open from here.".into());
    }
    std::process::Command::new("open").arg(&url).status().map_err(|e| format!("couldn't open the link: {e}"))?;
    Ok(())
}

/// Each agent's models, for the model picker.
#[tauri::command(async)]
fn models() -> Value {
    let list = |r: &str| onecorp::runtime::models(r).into_iter().map(|(id, name, about)| json!({"id": id, "name": name, "about": about})).collect::<Vec<_>>();
    // What each tier means on each agent, and what "Default" runs, so the app names real models.
    let tier = |r: &str, t: &str| onecorp::runtime::tier_model(r, t).map(|(id, name)| json!({"id": id, "name": name}));
    let tiers = |r: &str| json!({"strongest": tier(r, "strongest"), "balanced": tier(r, "balanced"), "cheapest": tier(r, "cheapest")});
    json!({"claude": list("claude"), "codex": list("codex"),
        "tiers": {"claude": tiers("claude"), "codex": tiers("codex")},
        "defaults": {"claude": onecorp::runtime::default_model("claude"), "codex": onecorp::runtime::default_model("codex")}})
}

#[tauri::command]
fn voices() -> Value {
    onecorp::voice::list()
}

#[tauri::command(async)]
fn set_voice(company: String, id: String, voice: String) -> Res<()> {
    onecorp::voice::set(&open(&company)?, &id, &voice).map_err(err)
}

#[tauri::command(async)]
fn delete_message(company: String, id: i64) -> Res<()> {
    open(&company)?.delete_message(id).map_err(err)
}

/// Posts as the founder. Each @mentioned person replies on their own thread, so a slow reply
/// never holds up the app. The UI shows them typing until it lands.
#[tauri::command(async)]
fn say(company: String, channel: String, text: String) -> Res<()> {
    for id in onecorp::chat::say(&open(&company)?, &channel, &text).map_err(err)? {
        let company = company.clone();
        std::thread::spawn(move || {
            if let Err(e) = Company::open(&company).and_then(|c| onecorp::chat::reply(&c, id)) {
                eprintln!("chat reply: {e:#}");
            }
        });
    }
    Ok(())
}

#[tauri::command(async)]
fn add_to_wiki(company: String, page: String, text: String) -> Res<()> {
    if text.trim().is_empty() {
        return Err("Write the rule first.".into());
    }
    open(&company)?.add_to_wiki(page.trim(), text.trim(), "founder").map_err(err)
}

#[tauri::command(async)]
fn ticket_detail(company: String, key: String) -> Res<Value> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    onecorp::view::ticket(&c, &t).map_err(err)
}

#[tauri::command(async)]
fn stop(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::stop(&c, &t).map_err(err)
}

#[tauri::command(async)]
fn retry(company: String, key: String, note: Option<String>) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::retry(&c, &t, note.as_deref()).map_err(err)
}

#[tauri::command(async)]
fn review(company: String, key: String) -> Res<Value> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    onecorp::view::review(&c, &t).map_err(err)
}

#[tauri::command(async)]
fn person(company: String, id: String) -> Res<Value> {
    onecorp::view::person(&open(&company)?, &id).map_err(err)
}

#[tauri::command(async)]
fn add_department(company: String, name: String) -> Res<i64> {
    Ok(open(&company)?.add_department(&name).map_err(err)?.id)
}

#[tauri::command(async)]
fn move_to_department(company: String, employee: String, department: i64) -> Res<()> {
    open(&company)?.set_department(&employee, department).map_err(err)
}

/// Hires from the app's candidate cards. Returns the new employee's id.
#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
fn hire(company: String, name: String, role: String, bio: String, runtime: String, model: Option<String>, department: Option<i64>, voice: Option<String>, kind: Option<String>) -> Res<String> {
    let c = open(&company)?;
    let id = store::slug(&name);
    if id.is_empty() {
        return Err("A name needs at least one letter or digit.".into());
    }
    if !onecorp::runtime::RUNTIMES.contains(&runtime.as_str()) {
        return Err(format!("{runtime} isn't supported yet."));
    }
    let model = model.filter(|m| !m.trim().is_empty());
    onecorp::roles::set(&c, &id, &role, kind.as_deref(), None).map_err(err)?;
    c.hire(&store::Employee { id: id.clone(), name, role, runtime, model, bio: bio.trim().into(), department }).map_err(err)?;
    if let Some(v) = voice.filter(|v| !v.is_empty()) {
        onecorp::voice::set(&c, &id, &v).map_err(err)?;
    }
    Ok(id)
}

// Tauri maps each named field from the app to an argument, so the list is the form.
#[allow(clippy::too_many_arguments)]
#[tauri::command(async)]
fn update_employee(company: String, id: String, name: String, role: String, runtime: String, model: Option<String>, bio: String, voice: Option<String>, kind: Option<String>, playbook: Option<String>) -> Res<()> {
    let c = open(&company)?;
    let e = c.employee(&id).map_err(err)?.ok_or_else(|| format!("Nobody called {id} works here."))?;
    if !onecorp::runtime::RUNTIMES.contains(&runtime.as_str()) {
        return Err(format!("{runtime} isn't supported yet."));
    }
    if let Some(v) = voice.filter(|v| !v.is_empty()) {
        onecorp::voice::set(&c, &id, &v).map_err(err)?;
    }
    onecorp::roles::set(&c, &id, &role, kind.as_deref(), playbook.as_deref()).map_err(err)?;
    c.update_employee(&store::Employee { name, role, runtime, model: model.filter(|m| !m.trim().is_empty()), bio: bio.trim().into(), ..e }).map_err(err)
}

/// Every playbook, for the picker in a profile.
#[tauri::command]
fn roles() -> Value {
    serde_json::json!(onecorp::roles::ROLES.iter().map(|r| serde_json::json!({"key": r.key, "label": r.label, "playbook": r.playbook})).collect::<Vec<_>>())
}

/// The playbook a title means, so the form follows the role as you type it.
#[tauri::command]
fn role_for(title: String) -> String {
    onecorp::roles::for_title(&title).key.to_string()
}

/// Rewrites the wiki's Codebase section from what's in the repo now.
#[tauri::command(async)]
fn learn_repo(company: String) -> Res<()> {
    let c = open(&company)?;
    let notes = onecorp::setup::repo_notes(&c.repo().map_err(err)?).ok_or("The repo has nothing to read yet: no README, build files or folders.")?;
    c.set_wiki_section("Codebase", &notes).map_err(err)
}

#[tauri::command(async)]
fn set_paused(company: String, paused: bool) -> Res<()> {
    open(&company)?.set_paused(paused).map_err(err)
}

/// How someone looks: a face seed, a new photo, or back to their drawn face.
#[tauri::command(async)]
fn set_avatar(company: String, id: String, face: Option<String>, photo: Option<String>, remove_photo: bool) -> Res<()> {
    let c = open(&company)?;
    c.set_face(&id, face.as_deref()).map_err(err)?;
    if let Some(p) = photo.filter(|p| !p.is_empty()) {
        c.set_photo(&id, Some(std::path::Path::new(&p))).map_err(err)?;
    } else if remove_photo {
        c.set_photo(&id, None).map_err(err)?;
    }
    Ok(())
}

#[tauri::command(async)]
fn fire(company: String, id: String) -> Res<()> {
    open(&company)?.fire(&id).map_err(err)
}

#[tauri::command(async)]
fn make_lead(company: String, id: String) -> Res<()> {
    let c = open(&company)?;
    let e = c.employee(&id).map_err(err)?.ok_or_else(|| format!("Nobody called {id} works here."))?;
    let dept = e.department.ok_or_else(|| format!("{} isn't in a department yet.", e.name))?;
    c.set_lead(dept, &id).map_err(err)
}

fn ticket(c: &Company, key: &str) -> Res<store::Ticket> {
    let id = c.parse_key(key).map_err(err)?;
    c.ticket(id).map_err(err)?.ok_or_else(|| format!("No ticket {key}."))
}

fn main() {
    // Agents start the company MCP server by re-running this binary with `mcp ...`.
    let args: Vec<String> = std::env::args().collect();
    if let Some(result) = onecorp::mcp::from_args(&args) {
        if let Err(e) = result {
            eprintln!("error: {e:#}");
            std::process::exit(1);
        }
        return;
    }

    // The team works while the app is open. If a standalone `onecorp serve` is already running,
    // this one stops right away and the app queues work for that one instead.
    std::thread::spawn(|| {
        if let Err(e) = onecorp::serve::serve(3) {
            eprintln!("serve: {e:#}");
        }
    });

    tauri::Builder::default()
        .setup(|app| {
            // Photos load straight from their files, and only from the companies folder.
            tauri::Manager::asset_protocol_scope(app).allow_directory(store::home().join("companies"), true)?;
            if cfg!(debug_assertions)
                && let Some(w) = tauri::Manager::get_webview_window(app, "main")
            {
                let _ = w.set_title("OneCorp Dev");
            }
            menu_bar(app)?;
            help_menu(app)?;
            Ok(())
        })
        // Closing the window hides it: the team keeps working, and the menu bar brings it back.
        // Quit (⌘Q, or from the menu bar) is what stops them.
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                still_working_note(tauri::Manager::app_handle(window));
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .invoke_handler(tauri::generate_handler![
            in_terminal,
            dismiss_question,
            close_report,
            companies,
            use_company,
            chat,
            delete_message,
            set_voice,
            set_commands,
            set_pr_sign,
            discard_ticket,
            add_repo,
            set_commits,
            set_talk,
            remove_repo,
            github_repos,
            clone_repo,
            set_projects_folder,
            clone_target,
            voices,
            models,
            open_link,
            set_tray_count,
            say,
            settings,
            set_ship,
            health,
            readiness,
            attention,
            spend_report,
            set_plan,
            set_budget,
            set_cap,
            set_price,
            keep_going_today,
            delete_company,
            forget_founder,
            snapshot,
            person,
            review,
            retry,
            roles,
            role_for,
            stop,
            ticket_detail,
            wiki,
            kickoff,
            plan_project,
            remove_ticket,
            update_employee,
            fire,
            set_avatar,
            set_paused,
            learn_repo,
            image_preview,
            set_look,
            make_lead,
            save_brief,
            dismiss_plan,
            delete_ticket,
            save_page,
            delete_page,
            redraft_wiki,
            restore_page,
            page_history,
            add_to_wiki,
            setup_info,
            founder,
            save_founder,
            check_repo,
            create_company,
            new_repo_info,
            create_repo,
            new_ticket,
            start_ticket,
            assign,
            answer,
            approve,
            request_changes,
            add_department,
            move_to_department,
            hire
        ])
        .build(tauri::generate_context!())
        .expect("couldn't start the OneCorp window")
        // Clicking the Dock icon with the window hidden shows it again.
        .run(|app, event| {
            if let tauri::RunEvent::Reopen { .. } = event {
                show_window(app);
            }
        });
}

fn show_window(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}

/// The menu bar icon: open the window, or quit and stop the team.
fn menu_bar(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::TrayIconBuilder;
    let open = MenuItem::with_id(app, "open", "Open OneCorp", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit OneCorp", true, Some("CmdOrCtrl+Q"))?;
    let menu = Menu::with_items(app, &[&open, &PredefinedMenuItem::separator(app)?, &quit])?;
    TrayIconBuilder::with_id("main")
        .icon(tauri::image::Image::from_bytes(include_bytes!("../icons/tray.png"))?)
        .icon_as_template(true)
        .tooltip("OneCorp")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

/// The Mac menu as usual (About OneCorp shows the version), plus Check for Updates and Settings
/// under About, and Help holding the ways to reach us.
fn help_menu(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{HELP_SUBMENU_ID, Menu, MenuItem, MenuItemKind, PredefinedMenuItem};
    let menu = Menu::default(app.handle())?;
    // The app menu comes first on a Mac, and starts with About.
    if let Some(MenuItemKind::Submenu(main)) = menu.items()?.into_iter().next() {
        main.insert(&MenuItem::with_id(app, "check-updates", "Check for Updates…", true, None::<&str>)?, 1)?;
        main.insert(&PredefinedMenuItem::separator(app)?, 2)?;
        main.insert(&MenuItem::with_id(app, "settings", "Settings…", true, Some("CmdOrCtrl+,"))?, 3)?;
    }
    if let Some(MenuItemKind::Submenu(help)) = menu.get(HELP_SUBMENU_ID) {
        help.append(&MenuItem::with_id(app, "report-bug", "Report a Bug…", true, None::<&str>)?)?;
        help.append(&MenuItem::with_id(app, "github", "OneCorp on GitHub", true, None::<&str>)?)?;
    }
    app.set_menu(menu)?;
    #[cfg(target_os = "macos")]
    menu_symbols();
    app.on_menu_event(|app, event| {
        // These act in the window, so it comes forward first in case it was closed.
        if let id @ ("check-updates" | "settings") = event.id.as_ref() {
            if let Some(w) = tauri::Manager::get_webview_window(app, "main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
            let _ = tauri::Emitter::emit(app, "menu", id);
            return;
        }
        let url = match event.id.as_ref() {
            "report-bug" => bug_url(&app.package_info().version.to_string()),
            "github" => "https://github.com/snowztech/onecorp".into(),
            _ => return,
        };
        let _ = std::process::Command::new("open").arg(url).status();
    });
    Ok(())
}

/// macOS 26 draws an icon next to its own menu items (About, Hide, Quit) but not next to ones
/// apps add. Check for Updates and Settings get the symbols Apple uses for them, so the menu
/// lines up. Older macOS shows no menu icons, so neither do these.
#[cfg(target_os = "macos")]
fn menu_symbols() {
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::{MainThreadMarker, NSProcessInfo, NSString};
    let Some(mtm) = MainThreadMarker::new() else { return };
    if NSProcessInfo::processInfo().operatingSystemVersion().majorVersion < 26 {
        return;
    }
    let Some(menu) = NSApplication::sharedApplication(mtm).mainMenu().and_then(|m| m.itemAtIndex(0)).and_then(|i| i.submenu()) else { return };
    for (title, symbol) in [("Check for Updates…", "arrow.triangle.2.circlepath"), ("Settings…", "gearshape")] {
        if let Some(item) = menu.itemWithTitle(&NSString::from_str(title)) {
            let image = NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(symbol), None);
            item.setImage(image.as_deref());
        }
    }
}

/// A new GitHub issue with the questions to answer and the version already in it.
fn bug_url(version: &str) -> String {
    let body = format!("**What happened**\n\n\n**What you expected**\n\n\n**Steps to get there**\n1. \n\n---\nOneCorp {version} on macOS");
    let enc: String = body
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect();
    format!("https://github.com/snowztech/onecorp/issues/new?labels=bug&body={enc}")
}

/// The first time the window closes, say the team is still at it, so nobody thinks it quit.
fn still_working_note(app: &tauri::AppHandle) {
    use std::sync::atomic::{AtomicBool, Ordering};
    use tauri_plugin_notification::NotificationExt;
    static SAID: AtomicBool = AtomicBool::new(false);
    if !SAID.swap(true, Ordering::Relaxed) {
        let _ = app.notification().builder().title("OneCorp is still working").body("Your team carries on in the background. Open it from the menu bar, or quit with ⌘Q.").show();
    }
}

/// How many things wait on you, next to the menu bar icon. Nothing when it's zero.
#[tauri::command]
fn set_tray_count(app: tauri::AppHandle, count: u32) {
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_title(if count > 0 { Some(count.to_string()) } else { None });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn bug_reports_carry_the_version() {
        let url = super::bug_url("0.1.0");
        assert!(url.starts_with("https://github.com/snowztech/onecorp/issues/new?labels=bug&body="));
        assert!(url.contains("OneCorp%200.1.0%20on%20macOS") && !url.contains('*') && !url.contains(' '), "{url}");
    }

    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foo"), "Zm9v");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }
}
