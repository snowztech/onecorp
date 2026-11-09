> Early design notes, written before the code. For how OneCorp works today, see [../ARCHITECTURE.md](../ARCHITECTURE.md).

# Architecture

## Stack

- **Tauri 2.** Rust core, web UI (React or Svelte), native window. Around 10MB, one codebase for Mac, Windows and Linux, auto-updates.
- **Rust core** runs the scheduler, agent processes, the company MCP server, the event log and cost tracking.
- **SQLite** for tickets, messages, events and memory. One file per company.
- **Git worktrees** for isolation. One worktree per ticket.

## Project layout

A Cargo workspace with three pieces. Logic lives in the library, and the two binaries are thin shells over it.

```
onecorp/
├── Cargo.toml        workspace
├── crates/core/      lib `onecorp`: store, work, serve, mcp, git, runtime/ (Claude Code, Codex)
├── crates/cli/       bin `onecorp`: the CLI, including `onecorp serve` and `onecorp mcp`
└── app/              Tauri 2 desktop app, plain HTML/CSS/JS UI, no bundler
```

How it fits together today:

- Each company is one SQLite file. The CLI, `serve`, the app and each agent's MCP server all read and write it directly, with WAL mode so they can share it.
- `serve` claims queued jobs and runs them in parallel. When it's up, the CLI and the app queue work instead of running it in place, so they never block.
- The app runs `serve` in-process, so closing the window stops the team. A standalone `onecorp serve` keeps working without the app.

Ideas from earlier drafts that may still come back: a local HTTP API with SSE, so a web or phone view could connect. Today the shared SQLite file is the API. The original reasons for a separate daemon:

- Agents keep working when you close the app window.
- The CLI and the app always show the same state.
- The UI talks plain HTTP, so a web UI or a phone view later is cheap.

The desktop app ships the `onecorp` binary as a Tauri sidecar and starts the daemon if it isn't running.

Crates: `tokio`, `axum`, `rusqlite` (bundled), `portable-pty`, `clap`, `serde`, `rmcp` (MCP), `tauri`. Shell out to `git` and `gh` instead of linking git libraries.

## CLI

```
onecorp new snowztech --repo github.com/snowztech/vikusha
onecorp use tvcask
onecorp hire "senior rust engineer" --runtime codex
onecorp ticket "Add Discord transport" --to maya
onecorp status
onecorp logs maya -f
onecorp approve 39
onecorp pause --all
```

## Companies

You can run several companies (snowztech, tvcask, autoremind, pingbuddy) and switch between them like Slack workspaces.

- Each company has one or more repos, its own team, payroll, memory, secrets and channels.
- Employees don't cross companies. No leaking memory or secrets from one to another.
- One global inbox and one global payroll view across all companies, so you see everything that needs you in one place.
- Switching: a sidebar of company icons, `Cmd+1` to `Cmd+9`, `onecorp use <name>` in the CLI.

On disk:

```
~/.onecorp/
├── config.toml
└── companies/
    ├── snowztech/
    │   ├── company.yaml    roles, repos, budgets, approvals (can be its own git repo)
    │   ├── company.db      tickets, messages, events, memory
    │   └── wt/             worktrees, one per ticket
    └── tvcask/
```

## Performance, honestly

The app itself will be light. Tauri plus a Rust core idles at tens of MB and keeps the UI smooth while streaming events from many agents.

What it can't make lighter is the agents. Each CLI runtime is its own process, and Claude Code or Codex can each take a few hundred MB. Six engineers can mean a couple of GB of RAM. That's the cost of using real agent CLIs, and it's the same for Paperclip or anything else.

The real limits users will hit:

- **Rate limits.** Subscriptions throttle parallel sessions. The scheduler needs to queue work, not start everything at once.
- **Tokens.** The biggest cost lever is less chatter, smaller context and prompt caching, not the language of the app.
- **Human review time.** See product.md.

So Rust is a good choice for a small, fast, reliable desktop app. It's not what makes the agents fast.

## How employees run

Every employee is a profile on top of a **runtime**, the agent CLI that does the actual work.

| Runtime | How the app drives it |
|---|---|
| Claude Code | `claude -p --output-format stream-json` |
| Codex | `codex exec --json` |
| Pi | RPC / JSON mode |
| Built-in | Direct model API call. For PM or marketing roles that don't need a terminal. |

Each runtime gets a small adapter: start a session, send the prompt, parse the event stream (tool calls, tokens, cost), stop or resume. Switching an employee from Codex to Claude Code is a dropdown, because the ticket and their memory carry the context, not the CLI session.

People use their own logins and subscriptions. The app doesn't resell tokens.

## When a ticket is assigned

1. Create a worktree and branch, for example `wt/acm-12` on `acm-12-magic-link`.
2. Build the prompt from role, memory, ticket and the relevant thread.
3. Spawn the runtime in a PTY inside that worktree, with the employee's permissions.
4. Stream events. Log them, update cost, show the live terminal.
5. When the agent is done or blocked, it says so through company tools. The app moves the ticket, pings the reviewer, or adds an inbox item.

## Company tools over MCP

The app runs an MCP server every runtime connects to. That's how any CLI agent becomes an employee without knowing anything about the app.

- `get_ticket`, `update_ticket`, `create_ticket`
- `post_message`, `read_thread`
- `request_review`, `attach_artifact`
- `ask_human` (lands in your inbox)
- `remember` (writes to the employee's memory)

## Terminal and permissions

- Commands run inside the employee's worktree only.
- Network blocked by default except package registries.
- `git push` and merges go through approval.
- Secrets injected per employee, never written into the worktree.
- Later: sandbox-exec on macOS, containers or bubblewrap on Linux.

## Company as code

```yaml
# company.yaml
name: Acme Notes
budget_per_day: 12
repo: ./acme-notes
approvals: [merge, deploy, public_post, budget]

employees:
  - id: maya
    name: Maya Okafor
    role: Senior engineer
    runtime: codex
    budget_per_day: 3
    terminal: true
    network: false
    personality: { chatty: 0.3, bold: 0.6, picky: 0.9 }
```
