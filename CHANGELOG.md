# Changelog

Notable changes to OneCorp. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Fixed

- Tickets in Rust repos stop rebuilding everything from scratch. A new ticket reuses the builds already in your checkout, cloned instantly on your Mac's disk.
- A docs-only change no longer runs the full build and test suite. Employees run the checks that cover what they changed.
- A long command in a run log shows "still running, 3 min" instead of looking stuck.
- Asked in chat about the ticket they're on, employees see their latest steps and their worktree, so they no longer say they haven't changed anything when they have.

### Changed

- Run logs show what employees say in full, and whole commands and errors, wrapped under themselves instead of cut to one line.
- The Repos section in Settings is calmer: a card per repo with its commands, Add a repo at the top, and shorter paths.

### Added

- An employee who needs a command they can't run asks for it in your Inbox, like `xcodegen generate` with the reason. Allow it and the whole team can run it in that repo, and the ticket picks back up. Git, root and pull request commands are never asked: OneCorp handles those.

### Changed

- Long questions, wiki proposals and review summaries in the Inbox show a few lines, with Show all for the rest.
- Every employee knows how OneCorp works: tickets live in OneCorp, OneCorp does the git work and decides what they can run.
- Each repo's commands in Settings are now a list you can edit one by one: remove one with ×, or type and press Enter to add one. Pasting a list from a Claude Code settings file works, so `Bash(make build:*)` becomes `make build`.
- Employees know OneCorp decides which commands they can run. They no longer ask you to create `.claude/settings.json`.

### Fixed

- Wiki writing now saves and completes after a retry or usage-limit resume. Older wiki drafts stuck in review show the document and a Save wiki action instead of asking to merge a nonexistent branch. Completed wiki tasks link to the wiki; plan actions say they start a plan.

## [0.1.4] - 2026-11-11

Employees talk like they work at your company: they read the tickets you name and know who's doing what.

### Added

- Click the faces in a channel's header to see who's in it, what each person is up to, and who leads it. Mention someone or open your DM with them from there.

### Changed

- Employees know their company like a colleague would. They know who leads which team, who's working on what or stuck, and who's free. They also know what was said lately in their other channels, so something decided in #engineering is known in a DM too.

### Fixed

- Name a ticket in chat, like MYC-2, and the employee replying reads it: what it asks, who has it, where it stands and their last update. Before, they only knew its title and asked you to paste it.

## [0.1.3] - 2026-11-11

Teams start with a model per role, and the Inbox lets you set things aside.

### Added

- Dismiss a question in the Inbox that doesn't block anyone. The employee already carried on, so nothing is posted or sent back. A stopped ticket can be deleted right from its Inbox card.

### Changed

- Starting teams come with models that fit each role: the strongest for the CTO or maintainer who reviews, a balanced one for engineers, PMs and designers, and the cheapest for writers. On Claude Code that's Opus 5.5, Sonnet 5.5 and Haiku 5.5. Each person's model shows on the Team step, where you can change it, and follows them if you switch their agent.
- Default always says which model it runs, like Default · Opus 5.5, read from your own Claude Code or Codex settings. Desks and chips show model names instead of ids.

### Fixed

- In chat, employees know which tickets they worked on. Asked about the wiki they wrote, they no longer say someone else did.
- Employees stop repeating a long reply they think got cut off. Shortened messages in chat history are now marked as shortened.

## [0.1.2] - 2026-11-11

A smoother first launch: setup fills the window, and tools connect in one click.

### Changed

- Connecting your tools takes one click: Sign in and Install open Terminal with the command already running, and the step updates by itself when you come back. Claude Code signs in with `claude auth login` instead of a full session. Check again is back.

### Fixed

- The welcome and setup screens fill the window again after you folded the sidebar. They were squeezed into a thin column on the left.
- The GitHub CLI fix is a command you can run: `brew install gh && gh auth login`.

## [0.1.1] - 2026-11-10

Your team builds and tests in many more kinds of repo.

### Changed

- Releases publish as soon as the build is signed and notarized, and the Homebrew cask follows in the same run.

### Fixed

- Employees on Claude Code can build and test in many more repos: Python with only loose test files, uv, Poetry, Ruby, Java and Kotlin (Maven, Gradle), .NET, Swift, Elixir, PHP, Dart and Flutter, Deno, Zig, C and C++ (CMake, Meson), Haskell, OCaml, Clojure, just and Task. On a Mac, python3 works too, and test output can be piped through tail or grep.
- With no build or test commands found, an employee's summary says the work is untested and where to add commands, instead of calling the worktree read-only.

## [0.1.0] - 2026-11-10

The first public release. Run a company of AI employees on Claude Code or Codex, on your Mac.

### Added

- A company around your repos: departments, leads and a team you hire from a template or one by one. Each employee has a role, an agent, a model and a personality. A company can span several repos, like an app and its api.
- Tickets and projects. Describe a project and your planner breaks it into tickets with owners and an order, for you to approve. Tickets can wait on each other and start by themselves.
- Each ticket runs in its own git worktree and branch, starting from main. Employees build and test with your repo's own commands, so work comes back tested. Commits carry the employee as author and you as co-author, or you alone.
- You approve what ships. Finished work opens a pull request on GitHub or GitLab, or merges on your Mac. Send it back with feedback and the same session picks it up.
- One inbox for plans, questions with a default answer, reviews and stopped runs. Questions go to the department lead first, who settles what they can.
- Team chat: #general, a channel per department and direct messages. Leads' decisions show up in their team's channel. Markdown like Slack.
- A shared wiki and personal memory. Your lead writes the first page from the repo, and everyone reads it before every ticket.
- Spending you control: a $10 daily budget to start, caps per person, a cheaper model for chat, and spend by person, project and agent. When a plan's usage limit is hit, work waits and resumes by itself.
- OneCorp keeps working from the menu bar when you close the window, with what needs you counted next to it. Notifications and a Dock badge cover all your companies.
- A setup that does the work: it checks your agents and GitHub, finds your repos or clones one from GitHub, and lands you in a company that already knows its codebase.
- Light and dark, three face sets, and an About pane with your version and a bug report link.
- The `onecorp` command does everything the app does.
- Signed and notarized. Install with `brew install --cask snowztech/tap/onecorp` or from onecorp.dev.

[0.1.4]: https://github.com/snowztech/onecorp/releases/tag/v0.1.4
[0.1.3]: https://github.com/snowztech/onecorp/releases/tag/v0.1.3
[0.1.2]: https://github.com/snowztech/onecorp/releases/tag/v0.1.2
[0.1.1]: https://github.com/snowztech/onecorp/releases/tag/v0.1.1
[0.1.0]: https://github.com/snowztech/onecorp/releases/tag/v0.1.0
