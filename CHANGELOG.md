# Changelog

Notable changes to OneCorp. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

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

[0.1.0]: https://github.com/snowztech/onecorp/releases/tag/v0.1.0
