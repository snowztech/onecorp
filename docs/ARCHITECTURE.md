# How OneCorp works

This is the map of the code as it is today. For why OneCorp exists and where it's going, see the [roadmap](../ROADMAP.md). The early design notes in [vision/architecture.md](vision/architecture.md) are a first draft and differ from the code in places.

## The idea in one paragraph

You run companies. Each company has a repo, a team of AI employees grouped into departments, a handbook and an inbox. An employee is a name, a role and a bio on top of a **runtime**, the agent CLI that does the work (Claude Code or Codex). When you give someone a ticket, OneCorp makes them a private git worktree, starts their runtime in it with a prompt built from who they are and what the company knows, and gives them company tools to ask you things and talk to the team. When they finish, OneCorp commits the work under their name and puts it in your inbox for review. Approving merges it, and the next ticket starts from there.

## The pieces

```
                 you
                  │
     ┌────────────┴────────────┐
     │                         │
  onecorp (CLI)          OneCorp (desktop app)
  crates/cli             app/  (Rust commands + ui/)
     │                         │
     └────────────┬────────────┘
                  │  both call the same library
                  ▼
           crates/core  (lib `onecorp`)
   ┌──────────┬─────────┬──────────┬─────────┐
 store      work      serve      view      git
 SQLite     tickets   job queue  screens   worktrees, merges
   │          │
   │          ▼
   │      runtime/  ── starts ──▶  claude  or  codex   (agent CLIs, one per ticket)
   │                                   │
   │                                   │ calls company tools over MCP
   │                                   ▼
   └──────────────────────────── mcp  (`onecorp mcp`, one per running agent)
```

Logic lives only in `crates/core`. The CLI and the app parse input, call the library and show the result. That's why they always behave the same, and why the app needed no logic of its own.

| Module | What it owns |
|---|---|
| `store.rs` | The company database: employees, departments, tickets, events, inbox, chat, jobs. Also the handbook and memory files. |
| `work.rs` | The ticket lifecycle: start, resume, answer, request changes, approve. Builds the prompts. |
| `serve.rs` | Runs queued jobs in parallel. |
| `runtime/` | The agent CLIs. `mod.rs` is what they share, `claude.rs` and `codex.rs` build each command and read its JSON. `fake.rs` is a stand-in with no AI, for tests and trying things out. |
| `mcp.rs` | The company tools agents call: ask you, post to chat, propose to the handbook, remember, mark blocked. |
| `git.rs` | Worktrees, commits under an employee's name, diffs, merges and conflicts. |
| `doctor.rs` | The checks behind `onecorp doctor`: git, agent CLIs, home folder, each company's repo and team. |
| `view.rs` | What the screens show: who's doing what, the feed, the inbox, a review's diff, one person's details. |

## On disk

```
~/.onecorp/                    (or $ONECORP_HOME)
├── current                    the company commands use by default
├── serve.heartbeat            rewritten every second while serve runs
└── companies/
    └── snowztech/
        ├── company.db         SQLite, WAL mode
        ├── handbook.md        mission, decisions, conventions
        ├── memory/maya.md     one file per employee
        └── wt/sno-12/         one git worktree per ticket
```

Several processes write `company.db` at once: the CLI, `serve`, the app and every agent's MCP server. WAL mode and a five second busy timeout let them take turns. Columns added after a release get created on open by `migrate()` in `store.rs`, so older company files keep working.

The main tables: `employees` (with `department` and `fired_at`), `departments` (with `lead`), `tickets` (status, branch, base, worktree, session id, cost, tokens), `events` (everything an agent did, with its raw JSON), `inbox` (questions and handbook proposals), `messages` (team chat) and `jobs` (the queue).

## Life of a ticket

1. **You give the ticket.** `onecorp run "add a --version flag" --to maya`, or the box in the app's top bar. `store` adds a row in `tickets` with status `todo`.
2. **Now or later.** `work::dispatch` checks whether `serve` is running. If not, it runs the ticket right there. If it is, it adds a row in `jobs`, marks the ticket `queued` and returns straight away.
3. **A private workspace.** `work::start` records the base branch (usually `main`) and has `git` create a worktree on a fresh branch, `onecorp/snowztech/sno-12`. Your own checkout is never touched.
4. **The prompt.** It's built from the employee's name, role and bio, the ticket, what their runtime can do, the company handbook, the list of who works here, and their own memory.
5. **The runtime starts.** `runtime::run` picks `claude.rs`, `codex.rs` or `fake.rs`, which start the agent CLI headless in the worktree. It runs with `runtime::path()`: your current PATH plus your login shell's PATH and the usual install folders, because an app opened from the Dock gets a bare PATH and wouldn't find `claude` or `codex`. They pass it two things: the git identity of the employee, so any commit is theirs, and an MCP config pointing at `onecorp mcp --company snowztech --ticket 12 --employee maya`. Only that MCP server loads. Your personal ones stay out.
6. **Watching it work.** The runtime prints JSON lines. Each runtime's `parse` turns them into a small set of `Event`s: start, say, tool, error, done. `work::drive` stores every event, prints it, and keeps the session id so the ticket can be picked back up later.
7. **Company tools.** While it works, the agent can call the MCP tools. `ask_human` puts a question in your inbox, and a blocking one marks the ticket `blocked`. `post_message` posts to chat. `propose_handbook` suggests a rule. `remember` adds to its own memory.
8. **Done.** `git::commit_all` commits whatever the agent left, as `Maya Okafor <maya@snowztech.onecorp>`. The ticket becomes `review`, unless the agent marked itself blocked.

### After that

- **You answer a question.** `work::answer` saves it. If it unblocks the ticket, or the agent went with a different default, the same session continues in the same worktree with your answer. That's `claude --resume` or `codex exec resume`.
- **You ask for changes.** `work::request_changes` sends your feedback back to the same session and also saves it to that employee's memory, so they learn how you like things.
- **You approve.** `work::approve` first merges the base branch into the ticket branch. If that conflicts, the ticket goes back to its employee with the list of files, they fix the markers and you approve again. Then the base fast-forwards to the ticket branch, the ticket becomes `done`, and its worktree and branch are removed.

## Shared knowledge

What employees know comes from four places, all assembled in `work::start`:

- **Merged code.** Approved work is on the base branch, and every ticket starts from it.
- **The handbook**, one markdown file per company, read in full at the start of every ticket (cut at 12,000 characters). You edit it with `onecorp handbook`, on the app's Handbook page, or by accepting a proposal.
- **The team list**, generated from the employees table, so it never goes stale.
- **Personal memory**, one file per employee, newest notes kept when it gets long. Agents add with `remember`, and review feedback goes in automatically.

## Running in parallel

`onecorp serve` is a loop. Every second it rewrites the heartbeat file, drops finished workers, and claims queued jobs from every company until `--max` are running (default 3). Each job runs on its own thread with its own database connection. The claim query only takes a job when its ticket isn't already running **and its employee is free**, so a person works one ticket at a time. Git commands inside OneCorp go through one lock, so parallel tickets don't trip over git's lock files.

Every run is a row in `jobs` with the process id running it, inline runs included. `recover_interrupted` checks whether that process still exists, and when it's gone (Ctrl-C, a crash, the app closed mid-ticket) marks the job and its ticket failed. It runs in serve's loop, in `onecorp status` and on every app refresh, and never touches a run that's still alive. `work::retry` then picks the same session back up if there is one, or starts over in a clean worktree.

The desktop app runs serve on a background thread while it's open.

## The desktop app

`app/` is a Tauri 2 app. `app/src/main.rs` exposes one command per action (`snapshot`, `answer`, `approve`, `hire`, `review`, `save_handbook` and so on), each a few lines that open the company and call the library. Commands run off the main thread, so the window never freezes.

`app/ui/` is plain HTML, CSS and JavaScript, with no build step. `app.js` polls `snapshot` every 1.5 seconds, re-renders only when something changed, and never re-renders over a field you're typing in. Opened in a normal browser, it runs on built-in sample data, which is the quickest way to work on the design.

Agents start the company MCP server by re-running the current binary with `mcp ...`. Inside the app, that binary is the app itself, so `main` first checks `onecorp::mcp::from_args` before opening a window.

## Testing

Each test covers the risky logic in one module:

- the runtime parsers, fed recorded JSON from Claude Code and Codex
- the MCP protocol and every tool, against a temporary company
- merges, fast-forwards and conflicts against a real throwaway git repo
- the job queue claim rules and recovery after a crash
- departments, handbook section editing and the screen snapshot

On top of those, `crates/core/tests/flow.rs` runs the whole ticket loop on a real git repo with the fake runtime: two tickets, commits under each employee's name, approve and merge, changes saved to memory, a conflict sent back and resolved, a failure, and a retry. It takes under a second.

Run everything with `cargo test`. Real agents are tested by hand on a throwaway repo, with `ONECORP_HOME` pointing somewhere temporary.

## Where to add things

- **A new runtime**, like Pi: add `runtime/pi.rs` with a `run` and a `parse` that produce the same `Event`s, add it to `runtime::run`, `RUNTIMES` and `abilities`. Nothing else needs to change.
- **A new company tool**: add it to `tools()` and `call()` in `mcp.rs`, and extend the test there.
- **A new command**: put the logic in `work.rs` or `store.rs`, then call it from `crates/cli/src/main.rs` and, if it belongs in the app, from a Tauri command in `app/src/main.rs`.
- **Something new on screen**: add it to `view.rs` first, so it's tested and the app only draws it.
