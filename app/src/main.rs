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
    Ok(json!({ "list": list, "current": current, "looks": looks }))
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
#[tauri::command(async)]
fn new_ticket(company: String, title: String, employee: Option<String>, description: Option<String>, after: Option<String>, project: Option<String>, later: bool) -> Res<String> {
    let c = open(&company)?;
    let employee = employee.filter(|e| !e.is_empty());
    if employee.is_none() && !later {
        return Err("Pick who does it, or save it for later.".into());
    }
    let after = after.filter(|a| !a.is_empty()).map(|k| ticket(&c, &k).map(|t| t.id)).transpose()?;
    // Part of a project: the ticket counts toward its progress and shows under it on the Board.
    let plan = project.filter(|p| !p.is_empty()).map(|k| ticket(&c, &k)).transpose()?.filter(|t| t.kind == "plan").map(|t| t.id);
    let new = store::NewTicket { title, description: description.unwrap_or_default(), assignee: employee, after, plan, ..Default::default() };
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
    Ok(json!({
        "name": c.name, "repo": repo, "data": c.dir, "home": store::home(),
        "ship": c.ship_mode().map_err(err)?, "forge": detected.map(|f| f.mode()), "forge_name": detected.map(|f| f.name()),
        "origin": onecorp::git::remote_url(&repo, "origin"),
        "tickets": c.tickets().map_err(err)?.len(), "people": c.employees().map_err(err)?.len(),
        "version": env!("CARGO_PKG_VERSION"),
    }))
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
        out.push(json!({ "company": name, "inbox": s["inbox"], "reviews": s["reviews"], "failed": s["failed"] }));
    }
    Ok(json!(out))
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
                "hires": t.people("claude").iter().map(|p| json!({"name": p.name, "role": p.role, "dept": p.department})).collect::<Vec<_>>(),
            })
        })
        .collect();
    Ok(json!({ "templates": teams, "runtimes": runtimes }))
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
    let branch = match onecorp::git::current_branch(&full) {
        Ok(b) => b,
        Err(_) => return problem("The repo is on a detached HEAD. Check out a branch, like main, then try again."),
    };
    let folder = full.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let used_by: Vec<String> = store::list()
        .map_err(err)?
        .into_iter()
        .filter(|c| Company::open(c).ok().and_then(|c| c.repo().ok()).is_some_and(|r| r == full))
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
#[tauri::command(async)]
fn create_company(name: String, repo: String, mission: String, people: Vec<Value>, brief: Option<String>, color: Option<String>, logo: Option<String>) -> Res<String> {
    let s = |v: &Value, k: &str| v[k].as_str().unwrap_or("").trim().to_string();
    let people: Vec<onecorp::templates::Person> = people
        .iter()
        .map(|p| onecorp::templates::Person {
            name: s(p, "name"),
            role: s(p, "role"),
            bio: onecorp::templates::bio_for(&s(p, "role")).into(),
            runtime: s(p, "runtime"),
            model: None,
            department: Some(s(p, "dept")).filter(|d| !d.is_empty()),
        })
        .collect();
    let (c, _) = onecorp::setup::new_company(name.trim(), std::path::Path::new(repo.trim()), &mission, &people, brief.as_deref().unwrap_or("")).map_err(err)?;
    // The look is a nicety: the company exists either way, so a bad logo file shouldn't undo it.
    let logo = logo.filter(|l| !l.is_empty()).map(std::path::PathBuf::from);
    let _ = c.set_look(color.as_deref().unwrap_or("#2b55c9"), logo.as_deref(), logo.is_none());
    Ok(c.name)
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

/// Deletes a merged, stopped or unstarted ticket from anywhere, not just a plan.
#[tauri::command(async)]
fn remove_ticket(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::delete(&c, &t).map_err(err)
}

#[tauri::command(async)]
fn handbook(company: String) -> Res<String> {
    open(&company)?.handbook().map_err(err)
}

#[tauri::command(async)]
fn chat(company: String, channel: String) -> Res<Value> {
    onecorp::view::chat(&open(&company)?, &channel).map_err(err)
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
fn save_handbook(company: String, text: String) -> Res<()> {
    open(&company)?.write_handbook(&text).map_err(err)
}

#[tauri::command(async)]
fn add_to_handbook(company: String, section: String, text: String) -> Res<()> {
    if text.trim().is_empty() {
        return Err("Write the rule first.".into());
    }
    open(&company)?.add_to_handbook(section.trim(), text.trim()).map_err(err)
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
fn retry(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::retry(&c, &t).map_err(err)
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
fn hire(company: String, name: String, role: String, bio: String, runtime: String, model: Option<String>, department: Option<i64>) -> Res<String> {
    let c = open(&company)?;
    let id = store::slug(&name);
    if id.is_empty() {
        return Err("A name needs at least one letter or digit.".into());
    }
    if !onecorp::runtime::RUNTIMES.contains(&runtime.as_str()) {
        return Err(format!("{runtime} isn't supported yet."));
    }
    let model = model.filter(|m| !m.trim().is_empty());
    // An empty bio gets one that fits the role, like the CLI and templates do.
    let bio = if bio.trim().is_empty() { onecorp::templates::bio_for(&role).to_string() } else { bio };
    c.hire(&store::Employee { id: id.clone(), name, role, runtime, model, bio, department }).map_err(err)?;
    Ok(id)
}

#[tauri::command(async)]
fn update_employee(company: String, id: String, name: String, role: String, runtime: String, model: Option<String>, bio: String) -> Res<()> {
    let c = open(&company)?;
    let e = c.employee(&id).map_err(err)?.ok_or_else(|| format!("Nobody called {id} works here."))?;
    if !onecorp::runtime::RUNTIMES.contains(&runtime.as_str()) {
        return Err(format!("{runtime} isn't supported yet."));
    }
    let bio = if bio.trim().is_empty() { onecorp::templates::bio_for(&role).to_string() } else { bio };
    c.update_employee(&store::Employee { name, role, runtime, model: model.filter(|m| !m.trim().is_empty()), bio, ..e }).map_err(err)
}

/// Rewrites the handbook's Codebase section from what's in the repo now.
#[tauri::command(async)]
fn learn_repo(company: String) -> Res<()> {
    let c = open(&company)?;
    let notes = onecorp::setup::repo_notes(&c.repo().map_err(err)?).ok_or("The repo has nothing to read yet: no README, build files or folders.")?;
    c.set_handbook_section("Codebase", &notes).map_err(err)
}

#[tauri::command(async)]
fn set_paused(company: String, paused: bool) -> Res<()> {
    open(&company)?.set_paused(paused).map_err(err)
}

/// Someone's photo, fetched only when it changes (the snapshot carries a stamp, not the image).
#[tauri::command(async)]
fn employee_photo(company: String, id: String) -> Res<Option<String>> {
    Ok(open(&company)?.avatar(&id).map_err(err)?.1.as_deref().and_then(data_url))
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            companies,
            use_company,
            chat,
            say,
            settings,
            set_ship,
            health,
            readiness,
            attention,
            delete_company,
            forget_founder,
            snapshot,
            person,
            review,
            retry,
            stop,
            ticket_detail,
            handbook,
            kickoff,
            plan_project,
            remove_ticket,
            update_employee,
            fire,
            employee_photo,
            set_avatar,
            set_paused,
            learn_repo,
            image_preview,
            set_look,
            make_lead,
            save_brief,
            dismiss_plan,
            delete_ticket,
            save_handbook,
            add_to_handbook,
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
        .run(tauri::generate_context!())
        .expect("couldn't start the OneCorp window");
}

#[cfg(test)]
mod tests {
    #[test]
    fn base64_matches_the_standard() {
        assert_eq!(super::base64(b""), "");
        assert_eq!(super::base64(b"f"), "Zg==");
        assert_eq!(super::base64(b"fo"), "Zm8=");
        assert_eq!(super::base64(b"foo"), "Zm9v");
        assert_eq!(super::base64(b"foobar"), "Zm9vYmFy");
    }
}
