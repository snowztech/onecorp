// The OneCorp desktop app. A thin shell: every command opens the company and calls the library,
// the same functions the CLI uses. The UI in ui/ polls `snapshot` and calls the actions.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use onecorp::store::{self, Company};
use onecorp::work::{self, Job};
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
    Ok(json!({ "list": list, "current": current }))
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

#[tauri::command(async)]
fn new_ticket(company: String, title: String, employee: String) -> Res<()> {
    let c = open(&company)?;
    let title = title.trim();
    if title.is_empty() {
        return Err("Say what needs doing.".into());
    }
    if c.employee(&employee).map_err(err)?.is_none() {
        return Err(format!("Nobody called {employee} works at {company}."));
    }
    let id = c.add_ticket(title, &employee).map_err(err)?;
    work::dispatch(&c, id, Job::Start).map_err(err)
}

#[tauri::command(async)]
fn answer(company: String, id: i64, text: String) -> Res<()> {
    work::answer(&open(&company)?, id, &text).map_err(err)
}

#[tauri::command(async)]
fn approve(company: String, key: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::approve(&c, &t).map_err(err)
}

#[tauri::command(async)]
fn request_changes(company: String, key: String, feedback: String) -> Res<()> {
    let c = open(&company)?;
    let t = ticket(&c, &key)?;
    work::request_changes(&c, &t, &feedback).map_err(err)
}

/// What the new company flow needs: the starting teams, and which runtimes are installed here.
#[tauri::command(async)]
fn setup_info() -> Res<Value> {
    let runtimes = onecorp::doctor::installed_runtimes();
    let teams: Vec<Value> = onecorp::templates::TEMPLATES
        .iter()
        .map(|t| {
            json!({
                "key": t.key, "name": t.name, "description": t.description, "runtime": t.runtime,
                "hires": t.hires.iter().map(|h| {
                    let p = onecorp::templates::preset(h.preset);
                    json!({"name": h.name, "role": h.role.or(p.map(|p| p.role)), "dept": h.dept})
                }).collect::<Vec<_>>(),
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
    Ok(json!({
        "ok": true, "path": full, "branch": branch, "suggested": onecorp::templates::suggest_name(&folder), "used_by": used_by,
    }))
}

#[tauri::command(async)]
fn create_company(name: String, repo: String, mission: String, template: String, runtime: String) -> Res<String> {
    let t = onecorp::templates::template(&template).ok_or_else(|| format!("No starting team called {template}."))?;
    if !onecorp::runtime::RUNTIMES.contains(&runtime.as_str()) {
        return Err(format!("{runtime} isn't supported yet."));
    }
    let c = Company::create(name.trim(), std::path::Path::new(repo.trim()), &mission).map_err(err)?;
    onecorp::templates::apply(&c, t, &runtime).map_err(err)?;
    store::set_current(&c.name).map_err(err)?;
    Ok(c.name)
}

#[tauri::command(async)]
fn handbook(company: String) -> Res<String> {
    open(&company)?.handbook().map_err(err)
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
    c.hire(&store::Employee { id: id.clone(), name, role, runtime, model, bio, department }).map_err(err)?;
    Ok(id)
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
        .invoke_handler(tauri::generate_handler![
            companies,
            use_company,
            snapshot,
            person,
            review,
            retry,
            stop,
            handbook,
            save_handbook,
            add_to_handbook,
            setup_info,
            check_repo,
            create_company,
            new_ticket,
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
