# Changelog

## Unreleased

- Know when your team needs you, even with the app in the background: the dock icon shows how much waits on you, and each new question, plan or review sends a macOS notification.
- Pause a company: your off switch for spending. Running work finishes, nothing new starts, leads don't consult and nobody replies in chat. A bar on every page says so, with Resume. From the Live section, or `onecorp pause` and `onecorp resume`.
- Hire anyone you like: the hire drawer is a form (name, role, runtime, model, how they work) with three suggestions that fill it in one click, and a face that follows the name. + Hire is back on each department and + Department on your card. In someone's drawer you can edit them, make them lead, give them a ticket, or let them go; a lead who leaves hands the department to the next person in it.
- Delete tickets that are merged, stopped or never started, from the ticket drawer or `onecorp rm`. Merged work stays in your repo, and a stopped ticket's branch is cleaned up.
- Calmer all-clear states, a Cancel on the department form, and ⌘B also folds the sidebar.
- Needs you is always on the Company page: the top three things waiting on you, most urgent first, each opening the Inbox right at that card. With nothing waiting it says so: all clear. When your whole team is free, a line under your name offers to plan a project or give someone a ticket.
- The sidebar folds into a rail of icons (⌘\\ or the button in the top bar), and remembers how you left it. The top bar names the page you're on, and the org chart ends with a quiet + New department.
- Projects, like epics that plan themselves. Plan a project from the Board (or `onecorp plan "Stripe billing" --goal "..."`): your planner breaks it into tickets with owners and order, and you approve it in the Inbox. The Board shows each project as a chip with its progress, and picking one shows only its tickets. The first version is simply your first project. The Company page is the office again: your team at their desks, with planning shown only while it happens.
- A lighter top bar and a cleaner company switcher.
- A polish pass so every screen feels like the same app. Settings left the sidebar: your face at the bottom opens your menu (Edit profile, Settings ⌘,, Forget my profile), and Settings is a sheet over the app with Company and Setup check tabs. The top bar only says where you are, with company facts and spend on the Company page. Pages have icons, empty states are friendly, and small animations make it feel alive: pages ease in, menus and sheets open smoothly, buttons press, all off with Reduce Motion.
- Settings (⌘,): the company's repo and data folder, how finished work ships, your profile, and the same health checks as `onecorp doctor`, with the fix for each. A danger zone deletes a company (type its name to confirm) or forgets your profile. Deleting stops running work and removes OneCorp's worktrees, and never touches your repo or its branches. Also `onecorp delete <company>` and `onecorp founder --forget`.
- The Handbook reads like a company doc: a title, a table of contents, your team's proposals at the top, and edit controls only when you reach for them.
- One company switcher at the top of the sidebar instead of a list. The plan card says what it does: describe the product once, your team plans it into tickets. A single task is a New ticket.
- One calm look across the app, written down in `docs/DESIGN.md`: one type scale, one card style, one label style, three radii, and no attention rings. The Company page lost its nested boxes: the brief becomes a one-line Building strip once planned, you're a header instead of a card, and Needs you and Live are plain sections. Live only shows moments that matter (started, finished, asked, merged) with the ticket's title, and opens the ticket on click. Handbook sections fold their add field away.
- A first launch that feels like a welcome, not a dashboard. Setting up your profile and creating a company are full screen, with nothing around them yet. Your drawn face follows your name as you type. Editing your profile later is a small popover from your name in the sidebar. On a Mac the title bar is gone and the window controls sit on the app. Fixed a blank window on a brand new install.
- Team chat you can see and join. #general for the company and a channel per department, made for you as departments appear. Employees post to their own team by default and read recent chat before each ticket. Post from the Chat page or `onecorp say engineering "@maya how does login work?"`, and whoever you @mention replies, read-only, with the cost counted in your spend. Times now show in your own time zone.
- A calmer app. One New ticket button (⌘K) replaces the busy top bar form. The Board has five columns, with everything that needs you in one, and only shows cost once something was spent. Inbox cards lead with their one-click answers and fold free text away. Desks drop their runtime chips and hire buttons, since the empty desk already does that.
- Start from just an idea: the new company flow can create the repo, with git, a README and a first commit, and also create it on GitHub under you or one of your orgs (`onecorp new --init --github`). Starting teams are editable before you create the company: names, roles, departments and runtimes.
- Kickoff from an idea: write a brief (in the app, `onecorp brief`, or when creating a company), and your product manager plans it into small tickets with owners and an order. Review the plan in the Inbox, change owners or remove tickets, then start it. Plan again with feedback, or dismiss it. The small and demo teams now include a product manager.
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
