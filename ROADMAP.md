# Roadmap

## North star

One founder runs real companies staffed by AI employees, from a desktop app that's a pleasure to open every day.

- **Real output.** Employees write real code in real repos, and it lands under their own names.
- **The founder stays in charge.** The app makes decisions fast: answer, review, approve. It doesn't try to replace you.
- **Small teams that ship.** A few narrow roles with real tools, not an org chart of VPs.
- **Always clear.** You see at a glance who is doing what and what it costs.
- **Great DX.** Up and running in two minutes. Every error says what to do next.
- **A real identity.** Faces, personalities and a look of its own, so it never feels vibecoded.

The clickable prototype shows where this is going: https://claude.ai/artifact/EqjxeApnFJi7TNUmFna3Fy (source in `../crew-hq/prototype.html`).

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

## Now: the desktop app

Tauri 2, on top of the `onecorp` library and `serve`. Built from the prototype's design.

- [ ] Tauri shell that starts `serve` in the background and reads the company database
- [ ] Office floor: desks, live status, speech bubbles
- [ ] Inbox: answer, approve and review in one click
- [ ] Board, Chat, and the employee drawer with the live terminal
- [ ] New company and hiring flows
- [ ] Company switcher (`Cmd+1` to `Cmd+9`)

## Next

- [ ] Faces and personality from the prototype (generated avatars, moods, voice in chat)
- [ ] Budgets and payroll: a daily cap per employee and per company, pause at the cap
- [ ] Org chart, where questions go up to leads before they reach you
- [ ] Pi runtime
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
