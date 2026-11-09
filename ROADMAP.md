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
- [x] Company MCP server: `ask_human`, `post_message`, `update_ticket`, `propose_wiki`, `remember`
- [x] Inbox, and answering resumes the same agent session (`inbox`, `answer`)
- [x] Review: `diff`, `changes`, `approve`. Conflicts go back to the employee to fix
- [x] Team chat: #general and a channel per department, agents read it before each ticket, @mentions get replies (`chat`, `say`)
- [x] Company wiki everyone reads before each ticket, with agent proposals (`wiki`)
- [x] Personal memory per employee, learned from your reviews (`memory`)
- [x] `serve` runs queued tickets in parallel, one ticket per employee at a time
- [x] Departments with leads (`dept`, `hire --dept`)
- [x] Fake runtime with no AI, and an end-to-end test of the whole ticket loop
- [x] Stuck tickets get noticed and marked failed, and `retry` picks them back up
- [x] Stop a running ticket or cancel a queued one (`stop`)
- [x] `doctor` checks git, the agent CLIs and every company, with the fix for each problem
- [x] Agents are found even when the app is opened from the Dock, with its bare PATH
- [x] Workspace layout: `crates/core` holds the logic, `crates/cli` and `app` are thin shells

## Now: the desktop app (v0.1.0 shipped)

Tauri 2, on top of the `onecorp` library and `serve`. Built from the prototype's design.

One Company page instead of separate Office, Team and Org screens: you on top, departments beneath as rooms of live desks. Concept: [docs/vision/hq-concept.html](docs/vision/hq-concept.html).

- [x] Tauri shell that runs `serve` in the background, on the core library
- [x] Company page: founder card, departments, desks with live status and bubbles
- [x] Inbox: answer, approve, send back and wiki proposals in one click
- [x] Hire into a department, add a department, move someone between departments
- [x] Person drawer: what they're doing now, memory, tickets
- [x] Company switcher in the sidebar
- [x] Board: every ticket by status, with a ticket drawer for its log and actions
- [x] Editable starting teams, and starting a new repo (optionally on GitHub) from the new company flow
- [x] Kickoff from an idea: a brief, a plan of tickets from your PM, review and start it
- [x] Real tickets: details, a backlog, tickets that wait for others, and `create_ticket` for the team
- [x] Founder profile: your name and photo, no account, and your team knows who they work for
- [x] Ship as pull requests on GitHub or GitLab, merged on approve
- [x] Chat page: channels, live typing, @mention anyone to ask them
- [x] Reviews in the Inbox show the real changes: commits, files, diff
- [x] Wiki page: read, edit and accept proposals in the app
- [x] New company flow in the app: pick the repo, name it, pick a starting team
- [x] Questions go to the department lead before they reach you, and you can step in anytime
- [x] Settings in sections (Company, Spending, Agents, You) with instant saves: look, ship mode, budget, agent health, delete a company, your profile
- [x] Wiki as a readable company doc
- [x] Projects: plan a feature or goal into tickets from the Board, with progress per project
- [ ] ⌘K command bar for everything

## v1 for first users

Everything a solo founder needs from download to merged work, with no dead ends.

- [x] Download a DMG; the release workflow drafts one for every tag
- [x] Onboarding as one journey: You, Company (name, logo, mission), Repo with a check of your Mac, Team from templates, Launch
- [x] Getting started checklist to the first merged work
- [x] Pause a company: running work finishes, nothing new starts
- [x] Dock badge and notifications when your team needs you
- [x] Hire, edit, make lead and let go; agents and models picked, not typed
- [x] Wiki as a Markdown wiki, started from what's in the repo
- [x] Delete finished tickets; delete a company without touching the repo
- [x] Our mascot, the OneCorp badge, and an app icon
- [x] An automated walk through onboarding in Chrome, in `make check` and CI
- [ ] Signed and notarized releases, so the app opens without the right-click step (the very last step before announcing)
- [x] Notifications and the badge for every company, not only the one you're in

## Next

- [ ] Agents answer each other's @mentions, with a cap so two of them can't chat forever
- [ ] Ask for a ticket from chat: "@rui plan the export feature" turns into a ticket
- [ ] Faces and personality from the prototype (generated avatars, moods, voice in chat)
- [x] Budgets: a daily budget per company and a cap per person, a pause at the budget, and spend by person, project and agent
- [ ] Pi runtime
- [ ] Extra MCP servers per employee, picked at hiring (GitHub, Figma, Notion…), next to the company tools
- [ ] Let Claude employees build and test, in a sandbox
- [ ] `whats_happening` tool, so employees see what teammates are working on

## Later

- [ ] Real accounts per employee (GitHub bot, Slack or Discord)
- [ ] Replay a day, weekly report card, daily standup
- [ ] Shareable company templates
- [ ] Several repos per company: each ticket picks its repo
- [x] Install script (`scripts/install.sh`) and OneCorp.app
- [ ] Name and brand (onecorp.dev for now)

## How we work

- Small slices, each one committed and tested end to end before the next.
- Update this file in the same commit as the work it describes.
