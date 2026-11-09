# Changelog

## Unreleased

- Real tickets: details, a backlog (`--later`, `start`, `assign`), tickets that wait for another one to be merged and then start by themselves, and a `create_ticket` tool so leads and PMs can plan work. The app's Board has a Backlog column and a New ticket drawer.
- Founder profile: your name and photo, set on first launch or with `onecorp founder`. It shows on the org chart, and employees know who they work for. No account, it stays on your Mac.
- Ship as pull requests: finished work opens a PR on GitHub or GitLab, sending it back updates it, and approving merges it. PRs merged or closed on the forge are picked up. `onecorp ship` switches modes.

## 0.1.0

The first version you can use every day.

- Companies around a git repo, each with departments, a handbook and an inbox. Create them in the app or with `onecorp new`, from a starting team or empty.
- Employees on Claude Code, Codex, or a free stand-in with no AI. Hire them into departments. The first one in leads.
- Every ticket gets its own git worktree and branch, and the work is committed under the employee's name.
- Company tools for agents: ask the founder, post to chat, propose to the handbook, remember, mark blocked.
- Questions go to the department lead first. The lead decides, or passes it up with a reason.
- Review in the inbox with the real diff: approve and merge, or send it back with feedback that goes into the employee's memory. Conflicts go back to the employee to fix.
- A shared handbook everyone reads before each ticket, and personal memory per employee.
- `serve` runs work in parallel, one ticket per person. The app runs it for you.
- Stop a run, cancel a queued ticket, retry one that stopped. Stuck tickets are noticed and marked.
- The desktop app: Company page, Board, Inbox, Handbook, hiring, departments and new company flow.
- `onecorp doctor` checks your setup. `scripts/install.sh` installs the command and the app.
