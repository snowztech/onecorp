# How OneCorp works

This is the map of the code as it is today. For why OneCorp exists and where it's going, see the [roadmap](../ROADMAP.md). The early design notes in [vision/architecture.md](vision/architecture.md) are a first draft and differ from the code in places. How employees work together, every path and the gaps, is in [COLLABORATION.md](COLLABORATION.md).

## The idea in one paragraph

You run companies. Each company has a repo, a team of AI employees grouped into departments, a wiki and an inbox. An employee is a name, a role and a bio on top of a **runtime**, the agent CLI that does the work (Claude Code or Codex). When you give someone a ticket, OneCorp makes them a private git worktree, starts their runtime in it with a prompt built from who they are and what the company knows, and gives them company tools to ask you things and talk to the team. When they finish, OneCorp commits the work under their name and puts it in your inbox for review. Approving merges it, and the next ticket starts from there.

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
| `store.rs` | The company database: employees, departments, tickets, events, inbox, chat, jobs. Also the wiki and memory files. |
| `work.rs` | The ticket lifecycle: start, resume, answer, request changes, approve. Builds the prompts. |
| `serve.rs` | Runs queued jobs in parallel. |
| `runtime/` | The agent CLIs. `mod.rs` is what they share, `claude.rs` and `codex.rs` build each command and read its JSON. `fake.rs` is a stand-in with no AI, for tests and trying things out. |
| `setup.rs` › `repo_notes` | Reads a repo like a newcomer would (stack, commands, layout, README) for the wiki's Codebase section. No AI. |
| `backup.rs` | Copies of a company in `backups/<time>[-why]/`: the database through `VACUUM INTO`, which copies consistently while others write, plus the wiki, notes, faces and logo. Taken every hour while the database changed (from serve's minute loop), when a new version first opens the company (before it migrates), and by hand. Every copy from the last day is kept, then the newest of each day, up to 14 days. Restoring copies the company as it is first, then writes the copy back through SQLite's backup API so open connections see it; it's refused while work runs. |
| `chat.rs` | Team chat: #general plus a channel per department. Resolves channel names, finds @mentions, runs a mentioned employee's reply (read-only, no company tools: `TICKET:` lines in the reply propose tickets, `COMMENT:` blocks add a `comment` event to a ticket, which goes into its prompt, `REVIEW:` lines start the replier's review of work in review), and builds the recent chat that goes into each ticket prompt. |
| `mcp.rs` | The company tools agents call: ask you, post to chat, propose to the wiki, remember, ask for a review, mark blocked. |
| `ship.rs` | How approved work lands: a local merge, or a pull request on GitHub or GitLab. Publishing, approving, and noticing PRs merged or closed on the forge. |
| `forge.rs` | The only code that talks to `gh` and `glab`: open, look at and merge a pull request, check the login. |
| `founder.rs` | You: a name and an optional photo in `~/.onecorp/founder.json`, shared by every company. Prompts use the name. No account. |
| `setup.rs` | Deleting a company safely (stops work, removes OneCorp's worktrees, never the repo or its branches), and creating one in one step: a new repo if you start from nothing (optionally on GitHub), the company, its team as you edited it, how it ships, and the first plan. |
| `git.rs` | Worktrees, commits under an employee's name, diffs, merges and conflicts. |
| Repos | A company's main repo lives in `meta` (`repo`). Others are in the `repos` table (name, path), and each ticket has an optional `repo` name (empty means the main one). `Company::repo_of(ticket)` is where its worktree, diff, review and merge happen. New work branches from `git::default_branch` (origin's HEAD, then main, then master), never from whatever the founder has checked out. The ship mode is per company, but a repo whose `origin` isn't on that forge merges locally (`ship::forge_for`). Planning, the wiki and chat replies run read-only with the other repos added (`--add-dir` for Claude Code). |
| Commits | `work::signature` decides who a commit is by. Setting `commits`: "team" (default) makes the employee the author and the founder the committer, with a `Co-authored-by` trailer. "you" makes the founder both. The founder is the repo's git `user.name` and `user.email`. |
| Background | Closing the window hides it (`on_window_event`), so `serve` keeps running. A menu bar icon (`menu_bar`) opens the window or quits, and shows what waits on you (`set_tray_count`). |
| Cached system part | A run can pass `Opts::system`: what stays the same from one run to the next. Claude Code gets it with `--append-system-prompt` and caches it across runs. It's the company's base, the same for everyone so the whole team reads one cached copy: for tickets, how ticket runs work plus `company_rules` for their side of the wiki (Product or Engineering); for chat, how chat works plus the rules; for Ask OneCorp, the docs. Who they are, the ticket, who's doing what and the conversation are the prompt. Ticket runs also pass `--exclude-dynamic-system-prompt-sections`, since each runs in its own worktree, which Claude Code would otherwise name in the system prompt. Chat replies and Ask OneCorp cache their conversation for 5 minutes instead of an hour (`chat::one_off_cache`), since writing the hour-long cache costs twice the input price and a one-off answer is never read again. Ticket runs read at most 8,000 characters of a successful command's output inline (`bashOutputMaxChars`); the rest is saved to a file they can read. Codex gets the system part at the start of the prompt. |
| Recent work | Every chat reply and Ask OneCorp gets `chat::recent_work`: the 20 tickets the team finished last, newest first, each with who, when and the first sentence of its summary, in one query (`Store::recently_done`). It's in the prompt, not the cached system part, since it changes with every merge. |
| Chat memory | A reply sees the last 12 messages, the last four in full, plus a summary of the conversation before that. Each reply writes the next summary on a last `SUMMARY:` line that's taken out before it's posted, and kept in `meta` as `chat.summary.<channel>`. The cost stays the same however long the conversation runs. While it's written, a reply's text is saved every 400ms (`Store::write_pending`, from Claude Code's `--include-partial-messages` deltas, `Event::Delta`), so it shows as it comes. |
| Talk model | Chat replies and lead decisions use `work::talk_model`: setting `talk` picks "balanced" (default, Sonnet or Codex's workhorse), "cheapest" (Haiku or Codex's fast model) or "same" (the person's own), through `runtime::talk_model`. Tickets always use the person's model. |
| `commands.rs` | The build and test commands employees may run, detected from the repo's files or set in Settings (`commands` in `meta`). Per repo: `commands` for the main one, `commands.<name>` for the others. Claude Code gets them as `Bash(<cmd>:*)` next to read-only git, and the ticket prompt lists them. |
| `limits.rs` | Plan usage limits. Reads Claude's and Codex's "limit reached" messages and the reset time in them, keeps it in `~/.onecorp/limits/<runtime>` (a limit is per Mac, not per company), and lists who can't work right now so claim, dispatch and chat skip them. A run that hits the limit is queued again, not failed. |
| `voice.rs` | Personalities: eight ways of writing (terse, warm, thorough, dry...). Each employee starts with one that fits their role (a lead is calm, a reviewer blunt, a PM curious, a designer warm, a writer dry, engineers vary), or the one you picked (stored in `meta` as `voice.<id>`). It goes into every prompt next to their bio. |
| `templates.rs` | Role presets (engineer, reviewer, designer, product manager, writer) and starting teams. A template becomes a list of `Person` you can edit, checked for duplicates before anyone is hired. |
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
        ├── wiki.md        mission, decisions, conventions
        ├── memory/maya.md     one file per employee
        └── wt/sno-12/         one git worktree per ticket
```

Several processes write `company.db` at once: the CLI, `serve`, the app and every agent's MCP server. WAL mode and a five second busy timeout let them take turns. Columns added after a release get created on open by `migrate()` in `store.rs`, so older company files keep working.

The main tables: `employees` (with `department` and `fired_at`), `departments` (with `lead`), `tickets` (status, branch, base, worktree, session id, cost, tokens), `events` (everything an agent did, with its raw JSON), `inbox` (questions and wiki proposals), `messages` (team chat: each reply points at the message it answers with `reply_to`, and a channel is read newest first through an index, a page at a time) and `jobs` (the queue).

## Life of a ticket

1. **You give the ticket.** `onecorp run "add a --version flag" --to maya`, or the box in the app's top bar. A ticket has a title, optional details, who created it (you, or an employee through `create_ticket`), and optionally another ticket it waits for. Saved for later, it sits in `backlog`. `work::start` starts it: it needs someone to do it, and if it waits for a ticket that isn't merged yet, it goes to `waiting` and `work::release_waiting` starts it once that one lands.
2. **Now or later.** `work::dispatch` checks whether `serve` is running. If not, it runs the ticket right there. If it is, it adds a row in `jobs`, marks the ticket `queued` and returns straight away.
3. **A private workspace.** `work::start` records the base branch (usually `main`) and has `git` create a worktree on a fresh branch, `onecorp/snowztech/sno-12`. Your own checkout is never touched.
4. **The prompt.** It's built from the employee's name, role and bio, the ticket, what their runtime can do, who the founder is, the company wiki, the list of who works here, and their own memory.
5. **The runtime starts.** `runtime::run` picks `claude.rs`, `codex.rs` or `fake.rs`, which start the agent CLI headless in the worktree. It runs with `runtime::path()`: your current PATH plus your login shell's PATH and the usual install folders, because an app opened from the Dock gets a bare PATH and wouldn't find `claude` or `codex`. They pass it two things: the git identity of the employee, so any commit is theirs, and an MCP config pointing at `onecorp mcp --company snowztech --ticket 12 --employee maya`. Only that MCP server loads. Your personal ones stay out.
6. **Watching it work.** The runtime prints JSON lines. Each runtime's `parse` turns them into a small set of `Event`s: start, say, tool, error, done. `work::drive` stores every event, prints it, and keeps the session id so the ticket can be picked back up later.
7. **Company tools.** While it works, the agent can call the MCP tools. `ask_human` puts a question in your inbox, and a blocking one marks the ticket `blocked`. `post_message` posts to chat. `ask_teammate` asks a teammate and waits for their chat reply. `propose_wiki` suggests a rule. `remember` adds to its own memory. `create_ticket` puts a follow-up or a piece of a plan in the backlog, for the founder to start.
8. **Done.** `git::commit_all` commits whatever the agent left, as `Maya Okafor <maya@snowztech.onecorp>`. The ticket becomes `review`, unless the agent marked itself blocked.

### Shipping

Each company has a shipping mode, stored in its database: `local`, `github` or `gitlab`. A new company picks a forge when its `origin` remote is on GitHub or GitLab and you're signed in there with `gh` or `glab`, and local otherwise. Change it with `onecorp ship`.

In GitHub or GitLab mode:

- A ticket starts from the latest `origin/<base>` instead of your local checkout (`ship::start_point`).
- When the run finishes, `ship::publish` pushes the branch and opens a pull request with the agent's summary, or pushes to the PR the ticket already has. If that fails, say you're signed out, the ticket stays in review with the reason, and approving tries again.
- `ship::approve` brings in the latest base (conflicts go back to the employee), merges the PR with a merge commit so every employee's commits keep their name, fast-forwards your local base when it can, and deletes the branch.
- `ship::sync`, called by serve about once a minute, notices PRs merged or closed on the forge itself.

Landing a ticket does the git work first (delete the remote branch, update your local base, remove the worktree) and marks the ticket done last. If OneCorp stops halfway, the ticket is still in review and the next sync finishes the job, since each step is safe to repeat.

This was tested against real GitHub with `gh`: opening a PR, updating it with changes, merging it on approve with each employee's commits keeping their name, and a PR merged directly on GitHub being picked up.

OneCorp has no account of its own. `gh` and `glab` use the login you already have, and PRs are opened by your account.

### After that

- **You answer a question.** `work::answer` saves it. If it unblocks the ticket, or the agent went with a different default, the same session continues in the same worktree with your answer. That's `claude --resume` or `codex exec resume`.
- **You ask for changes.** `work::request_changes` sends your feedback back to the same session and also saves it to that employee's memory, so they learn how you like things.
- **You approve.** `work::approve` first merges the base branch into the ticket branch. If that conflicts, the ticket goes back to its employee with the list of files, they fix the markers and you approve again. Then the base fast-forwards to the ticket branch, the ticket becomes `done`, and its worktree and branch are removed.

## From an idea to a plan

Each company has a brief in `brief.md`: what you're building, who it's for, what the first version needs. Everyone reads it at the start of every ticket, so the whole team knows the product.

A **project** is a plan: a ticket of kind `plan` whose title names it ("Stripe billing") and whose description says what it should do. `work::plan_project` creates one; `work::kickoff` is the first project, planned from the brief alone. The Board groups tickets by project through `tickets.plan`. A plan is a ticket which keeps planning visible on the Board, costed and logged, and lets it be stopped, retried and sent back like any work. `work::pick_planner` picks who plans: the one you choose, or a product person, or a department lead.

The planner runs **read-only on your repo itself** (no worktree, no commit, no pull request) and creates tickets with `create_ticket`. Tickets created while working on a plan ticket belong to it (`tickets.plan`). The plan then waits for you:

- Approving a plan (`ship::approve` sends it to `work::start_plan`) starts every ticket with someone on it. Tickets that wait for others go to `waiting` and start in order.
- Sending it back with changes removes its tickets and has the planner plan again with your feedback.
- `work::dismiss_plan` removes its tickets and stops it. Retrying plans again from scratch.

Ticket ids use `AUTOINCREMENT`, so a removed ticket's key is never given to a new one. `git::discard`, which force-removes a worktree, refuses any path outside the company's `wt/` folder, so it can never touch your repo.

## Leads answer first

When someone in a department asks a question with `ask_human`, it goes to their department's lead, not to you (`store::lead_of`). The question is stored with stage `lead`, and a `consult` job is queued for it.

`work::consult` runs the lead's runtime **read-only** in the asker's worktree: Claude Code without accepting edits, Codex in its read-only sandbox. The prompt has the question, the asker's default, the wiki and the team, and asks the lead to end with `DECISION: …` or `FOUNDER: why`. `work::verdict` reads that line, and anything unclear goes up to you, the safe side.

- A decision is saved as the lead's answer, and `work::follow_up` resumes the asker exactly like your own answer would. Their prompt says the answer came from their lead.
- A pass-up moves the question to stage `founder`, with the lead's reason. It shows in your inbox as "Odile passed this to you: …".
- Leads, people without a department and wiki proposals go straight to you. Questions still with a lead show under "With your leads", and you can answer them yourself.

The consult job waits until the asker's own run is done. Without `serve`, `work::drain_consults` runs them right after the ticket finishes.

## Leads review first

Work in review gets a teammate's review before the founder's, usually the author's lead (`work::reviewer_for`). When a run ends in review, `work::review_on_finish` starts it with nobody asking: from whoever the author named with the `request_review` tool, or else from their lead, unless the company turned leads' reviews off (setting `lead_review`, on by default, `work::leads_review`). Work sent back and redone is reviewed again. Authors without a lead, like the leads themselves, come straight to the founder. A review can also be asked for by hand: the founder from the Inbox or `onecorp ask-review`, or a lead from chat with a `REVIEW: <key>` line in their reply. The ticket prompt tells each employee who reviews their work, and that asking in chat doesn't start a review.

`work::ask_review` queues a `review` job, the reviewer's id in its label. It's queued with `enqueue`, not `dispatch`, so the ticket stays in `review`. `work::review` runs the reviewer's own model **read-only** in the author's worktree, with the ticket, the author's summary, the diff against its base (`view::review_range`, cut at 60,000 bytes) and the build and test commands they may run. The reviewer ends with `VERDICT: GOOD` or `VERDICT: CHANGES`, read by `work::review_verdict`. No clear verdict counts as changes, the safe side.

The verdict is a `lead-review` event on the ticket, with the notes in its raw JSON, and a note to the author in their team channel. Good work stays in review for the founder, with the verdict on its Inbox card until the work changes again. Changes go straight back (`work::send_back`): the notes become a `comment` event on the ticket, go into the author's memory, and resume the author's session with a `Resume` job through `dispatch`, so the ticket shows Up next while the author is busy and Working once they're on it. When the redo ends, `review_on_finish` reviews it again. A reviewer sends the same work back at most `MAX_REVIEW_ROUNDS` times (2) since the founder last asked for changes (`work::sent_back`). Past that the work goes to the founder with the verdict and the count, and merging or sending it back stays their call. A review interrupted mid-run leaves the ticket in review with an error event, never failed. A redo started from inside a review, when serve isn't running, is reviewed too: `store::other_review` doesn't count the review job of the same process.

## Shared knowledge

What employees know comes from four places, all assembled in `work::start`:

- **Merged code.** Approved work is on the base branch, and every ticket starts from it.
- **The wiki**, a markdown file per page under the company's `wiki/` folder (`wiki.rs`). Rules pages (Overview, Now, Decisions, Product/Rules, Engineering/Conventions) are in every prompt in full, and so is the space the person works in: Engineering for engineers, Product for product, design and writing (`roles::spaces`). Every other page is a line with its file, in a copy under `reading/wiki` the agent can read but whose edits change nothing (`wiki::for_prompt`, `reading_copy`). Plans and wiki drafts get it all. The full part is cut at 12,000 characters. You edit pages on the app's Wiki page, with `onecorp wiki`, or by accepting a proposal.
- **The team list**, generated from the employees table, so it never goes stale.
- **Personal memory**, one file per employee, newest notes kept when it gets long. Agents add with `remember`, and review feedback goes in automatically.

## Running in parallel

`onecorp serve` is a loop. Every second it rewrites the heartbeat file, drops finished workers, and claims queued jobs from every company until `--max` are running (default 3). Each job runs on its own thread with its own database connection. The claim query only takes a job when its ticket isn't already running **and its employee is free**, so a person works one ticket at a time. Git commands inside OneCorp go through one lock, so parallel tickets don't trip over git's lock files.

Every run is a row in `jobs` with the process id running it, inline runs included. `recover_interrupted` checks whether that process still exists, and when it's gone (Ctrl-C, a crash, the app closed mid-ticket) marks the job and its ticket failed, with an `interrupted` event. When serve starts, `work::resume_interrupted` queues those tickets again in their own session, unless the company is paused or the ticket can't resume (a plan, or no session yet). It runs in serve's loop, in `onecorp status` and on every app refresh, and never touches a run that's still alive. `work::retry` then picks the same session back up if there is one, or starts over in a clean worktree.

A run whose process is alive but stuck is the watchdog's (`work::quiet_runs`, `stop_quiet`, from serve's loop). Quiet means nothing logged on the ticket since its run started or its last event. A third of the way to the limit (setting `watch.stop`, 30 minutes unless changed, 0 for never) the Inbox shows it with Keep waiting, which logs an event and so restarts the clock. At the limit it's stopped the way Stop does, with an error event saying how long and the last step, so Retry picks up the same session.

Each run also records its agent's process id on the ticket (`tickets.agent_pid`, set through `runtime::Opts::on_spawn`). `work::stop` uses it to end a run you want to stop: a polite TERM, then KILL if the agent is still there after 3 seconds. A queued ticket just has its job cancelled. Either way it ends up failed, with a `stop` event, so `retry` resumes it later.

The desktop app runs serve on a background thread while it's open.

## The desktop app

`app/` is a Tauri 2 app. `app/src/main.rs` exposes one command per action (`snapshot`, `answer`, `approve`, `hire`, `review`, `save_wiki` and so on), each a few lines that open the company and call the library. Commands run off the main thread, so the window never freezes.

`app/ui/` is HTML, CSS and TypeScript modules with no framework, built by Vite into `app/dist`, which Tauri embeds. `types.ts` describes what every command returns, matching the `json!` in `app/src/main.rs` and `crates/core/src/view.rs`, so `invoke("snapshot", …)` comes back typed and `npm run check` catches a renamed field. `cargo tauri dev` serves it from Vite instead, so changes show without a rebuild. In the company workspace, `render.js` polls `snapshot` every 1.5 seconds, re-renders only when something changed, and never re-renders over a field you're typing in. Onboarding has no polling timer: actions and completed setup checks update its screens. Entering setup stops workspace polling; leaving it starts a single timer again. Opened in a normal browser (`make ui`), it runs on built-in sample data, which is the quickest way to work on the design.

Agents start the company MCP server by re-running the current binary with `mcp ...`. Inside the app, that binary is the app itself, so `main` first checks `onecorp::mcp::from_args` before opening a window.

Updates: on launch and every six hours, the app asks `releases/latest/download/latest.json` on GitHub for a newer version. The updater plugin downloads it, checks its signature against the public key in `tauri.conf.json`, and installs it. "Restart to update" in the sidebar relaunches on it. Dev builds never update. Each release publishes `OneCorp.app.tar.gz` and `latest.json`, signed in CI with the `TAURI_SIGNING_PRIVATE_KEY` secret. Lose that key and installed copies can't update anymore: keep it backed up.

## Testing

Each test covers the risky logic in one module:

- the runtime parsers, fed recorded JSON from Claude Code and Codex
- the MCP protocol and every tool, against a temporary company
- merges, fast-forwards and conflicts against a real throwaway git repo
- the job queue claim rules and recovery after a crash
- departments, wiki section editing and the screen snapshot
- reading a lead's verdict, including markdown like `**DECISION:**`

On top of those, `crates/core/tests/flow.rs` runs the whole ticket loop on a real git repo with the fake runtime: two tickets, commits under each employee's name, approve and merge, changes saved to memory, a conflict sent back and resolved, a failure, a retry, and questions answered by a lead, passed up by a lead, and unblocking a ticket. `crates/core/tests/kickoff.rs` goes from a brief to a plan, plans again, starts the plan in order, and dismisses and retries one, checking your checkout is never touched. It takes under a second.

`app/tests/ui-smoke.js` imports the app's UI in Node with a stand-in DOM and its sample data, and renders every screen, including each step of the new company flow.

`crates/core/tests/pull_requests.rs` does the same for shipping as pull requests, with a bare repo standing in for GitHub and a small script standing in for `gh`: opening, updating and merging a PR, your local base following, and PRs merged or closed on GitHub itself.

Run everything with `make test`. Real agents are tested by hand on a throwaway repo, with `ONECORP_HOME` pointing somewhere temporary.

## Where to add things

- **A new runtime**, like Pi: add `runtime/pi.rs` with a `run` and a `parse` that produce the same `Event`s, add it to `runtime::run`, `RUNTIMES` and `abilities`. Nothing else needs to change.
- **A new company tool**: add it to `tools()` and `call()` in `mcp.rs`, and extend the test there.
- **A new command**: put the logic in `work.rs` or `store.rs`, then call it from `crates/cli/src/main.rs` and, if it belongs in the app, from a Tauri command in `app/src/main.rs`.
- **Something new on screen**: add it to `view.rs` first, so it's tested and the app only draws it.
