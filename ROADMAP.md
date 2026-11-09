# Roadmap

## North star

One founder runs real companies staffed by AI employees, from a desktop app that's a pleasure to open every day.

- **Real output.** Employees write real code in real repos, and it lands under their own names.
- **The founder stays in charge.** The app makes decisions fast: answer, review, approve. It doesn't try to replace you.
- **Small teams that ship.** A few narrow roles with real tools, not an org chart of VPs.
- **Always clear.** You see at a glance who is doing what and what it costs.
- **Great DX.** Up and running in two minutes. Every error says what to do next.
- **A real identity.** Faces, personalities and a look of its own, so it never feels vibecoded.

The clickable prototype shows where this is going: https://claude.ai/artifact/EqjxeApnFJi7TNUmFna3Fy (source in [docs/vision/prototype.html](docs/vision/prototype.html)).

## Done

The whole company loop works in the CLI, tested end to end on throwaway repos.

- [x] Companies around a repo, switch between them (`new`, `use`, `companies`)
- [x] Hire and fire, with presets and an interactive form (`hire`, `team`, `fire`)
- [x] Tickets in their own git worktree and branch, every event and cost logged (`run`, `status`, `logs`)
- [x] Claude Code and Codex runtimes behind one small agent layer
- [x] Work committed under the employee's name, like `Tomás Reis <tomas@snowztech.onecorp>`
- [x] Company MCP server: `ask_human`, `post_message`, `update_ticket`, `propose_handbook`, `remember`
- [x] Inbox, and answering resumes the same agent session (`inbox`, `answer`)
- [x] Review: `diff`, `changes`, `approve`. Conflicts go back to the employee to fix
- [x] Team chat (`chat`)
- [x] Company handbook everyone reads before each ticket, with agent proposals (`handbook`)
- [x] Personal memory per employee, learned from your reviews (`memory`)
- [x] `serve` runs queued tickets in parallel, one ticket per employee at a time
- [x] Departments with leads (`dept`, `hire --dept`)
- [x] Fake runtime with no AI, and an end-to-end test of the whole ticket loop
- [x] Stuck tickets get noticed and marked failed, and `retry` picks them back up
- [x] `doctor` checks git, the agent CLIs and every company, with the fix for each problem
- [x] Agents are found even when the app is opened from the Dock, with its bare PATH
- [x] Workspace layout: `crates/core` holds the logic, `crates/cli` and `app` are thin shells

## Now: the desktop app

Tauri 2, on top of the `onecorp` library and `serve`. Built from the prototype's design.

One Company page instead of separate Office, Team and Org screens: you on top, departments beneath as rooms of live desks. Concept: [docs/vision/hq-concept.html](docs/vision/hq-concept.html).

- [x] Tauri shell that runs `serve` in the background, on the core library
- [x] Company page: founder card, departments, desks with live status and bubbles
- [x] Inbox: answer, approve, send back and handbook proposals in one click
- [x] Hire into a department, add a department, move someone between departments
- [x] Person drawer: what they're doing now, memory, tickets
- [x] Company switcher in the sidebar
- [ ] Board and Chat pages
- [x] Reviews in the Inbox show the real changes: commits, files, diff
- [x] Handbook page: read, edit and accept proposals in the app
- [ ] New company flow in the app (today it's `onecorp new`)
- [ ] Questions go to the department lead before they reach you
- [ ] ⌘K command bar for everything

## Next

- [ ] Faces and personality from the prototype (generated avatars, moods, voice in chat)
- [ ] Budgets and payroll: a daily cap per employee and per company, pause at the cap
- [ ] Pi runtime
- [ ] Extra MCP servers per employee, picked at hiring (GitHub, Figma, Notion…), next to the company tools
- [ ] Let Claude employees build and test, in a sandbox
- [ ] `whats_happening` tool, so employees see what teammates are working on

## Later

- [ ] Real accounts per employee (GitHub bot, Slack or Discord)
- [ ] Replay a day, weekly report card, daily standup
- [ ] Shareable company templates
- [ ] Several repos per company
- [ ] Install script and releases
- [ ] Name and brand (onecorp.dev for now)

## How we work

- Small slices, each one committed and tested end to end before the next.
- Update this file in the same commit as the work it describes.
