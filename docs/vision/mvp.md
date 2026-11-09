# MVP

Goal: one person can hire 3 employees, give them a repo, and get a merged PR by the end of the day, while watching the whole thing and knowing what it cost.

## In

- Tauri app with Office, Board, Chat and Inbox
- 3 employees from templates: PM, engineer, reviewer
- Claude Code and Codex adapters
- Company MCP server with the core tools
- One worktree per ticket, live terminal view
- Budget per employee, "Pause company" button
- One approval gate: merge
- Generated faces and moods (cheap, and it's the hook)
- SQLite event log

## Later

- Hiring flow with candidates and test tasks
- Pi and built-in runtimes
- Personality traits, speech bubbles, celebrations
- Real accounts (GitHub bot, Slack, Discord)
- Replay, report cards, templates
- Sandboxing beyond worktrees

## Build order

1. Rust core: spawn one Claude Code process in a worktree, parse its stream, log events and cost. No UI.
2. MCP server with `get_ticket`, `update_ticket`, `ask_human`. Run one ticket end to end from the CLI.
3. Second adapter (Codex) to prove the runtime abstraction.
4. Tauri shell with Board and the live terminal.
5. Office floor, Inbox, budgets.
6. Three employees working in parallel, with a scheduler that respects rate limits.

## Open questions

- Who orchestrates by default: the PM agent, or a plain scheduler with rules?
- How much memory per employee before it hurts more than it helps?
- Cost tracking for subscription logins, where there's no per-token bill.
- Open source core and paid sync, or fully open source?
- Name.
