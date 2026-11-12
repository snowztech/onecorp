# Changelog

Notable changes to OneCorp. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Changed

- Wiki search highlights what you searched: in a few words under each page that mentions it, and in the page you open, which scrolls to the first match.

### Fixed

- CLI output now explains how to review new plans and see their tickets, and uses clearer wording for run summaries and report closure.

## [0.1.12] - 2026-11-12

Conversations with your team flow better: employees comment on tickets from Chat, keep answering without an @mention, and you can search the wiki.

### Added

- Employees can add a comment to a ticket when you ask them in Chat, like the text to paste into a file. It shows in the ticket's history, and whoever works on the ticket reads it. In Chat they're also told what they can't do from there, so they don't promise it.
- Changes in a review show old and new line numbers next to each line.
- In a channel, a message with no @mention goes to whoever replied to you last, so a conversation carries on without mentioning them every time. The hint under the message box says who will reply.
- Search the wiki: type in the box above the page list to see only the pages that mention it.

### Fixed

- Starting the tickets a report proposes closes it right away. Before, while OneCorp was checking pull requests, one ticket could run inside the click, leaving the report and its Start button in the Inbox until that ticket finished.
- Changing or removing an employee's photo shows in their profile right away, before you save.

## [0.1.11] - 2026-11-11

Your employees work on their own, up to your approval: they run what they need, own their branch, and stop getting stuck on merges.

### Added

- Your employees work on their own now. They run what they need in their own copy of the repo: builds, tests, installs and scripts. Their branch is theirs: they commit as they go, rebase onto main or merge it in, fix conflicts and push it. A guard keeps each ticket's pushes on its own branch, and nothing reaches main until you approve it. This is on for every company, existing ones included. If you'd rather approve each command, choose Only allowed under Commands in Settings, Company.
- Employees can open a draft pull request for their branch with gh or glab to see CI early, update it, and read its checks and CI logs. OneCorp links the ticket to it instead of opening a second one, and marks it ready when they finish. Merging, closing, approving and reopening pull requests stay yours, along with repo settings, releases, secrets and the GitHub API. Sudo and deleting folders stay off.
- A setup command per repo, under Settings, Company: OneCorp runs it in each new ticket's copy of the repo before the employee starts, like `npm ci`. If it fails, the employee is told and the ticket still runs.
- Rescan a repo to add the commands its files suggest now, keeping yours. Make another repo the primary one, so the old one can be removed.

### Changed

- With Only allowed, employees ask for every command they're missing in one request, and one Inbox card allows them all, or lets them run anything. A script on the list runs as `tools/x.sh` or `./tools/x.sh`, employees are told to run listed commands as written, and commands that can never run, like `git rebase`, can't be added.
- On Codex, running anything also lets installs reach the network, and lets employees commit and rebase on their branch.
- Settings suggests turning on branch protection for main on GitHub, since employees push their own branches.

### Fixed

- Merging on GitHub works on repos that turn merge commits off or require a linear history. OneCorp merges the way the repo allows, falls back to a squash, and says what to check if GitHub still refuses.
- A ticket whose work is already on main no longer gets stuck in review. Its Inbox card says there's nothing left to merge, and Close as done closes its pull request and deletes its branch, instead of failing on an empty merge. A run that ends with nothing new to merge comes back as a report.
- A pull request you close on GitHub after its work reached main another way marks the ticket done, instead of stopped.
- Employees know finishing needs no tool: when they end with their summary, the ticket goes to review. They no longer ask you to mark tickets done or delete branches, redo work that's waiting to merge, or file tickets about git and permissions.
- If an employee's commits reach main during a run, around your approval, the ticket says so, with the commits to check.
- Claude Code employees can open the wiki pages they're pointed to while working on a ticket.

## [0.1.10] - 2026-11-11

Your team works more like a real one: leads split work from chat, every role has a playbook, and Ask OneCorp answers questions about the app. Prompts are leaner, and Spending shows where the money goes.

### Added

- Ask someone in chat to fix or look into something, and they propose the tickets right under their reply. A lead splits the work across their team, like Odile giving the cache to Maya and the query to Tomás. Start them in one click, or open one to change who does it.
- Whoever leads a department knows what that means, whatever their title: their team's questions come to them first, they split the work and keep an eye on who's stuck.
- Every role comes with a playbook: what a backend, frontend or mobile engineer, tech lead, product manager, designer, writer, growth, DevOps or QA person owns and how they work, read before every ticket. The title picks it, so a Backend engineer and a Frontend engineer no longer work the same way, and you can pick another one in their profile. Add notes for one person on top, like "We write Go with table tests", and they still get better playbooks as OneCorp improves them. Rewrite the whole thing when you want, and switch back in a click. You see each person's playbook on the Team step of setup too.
- Ask OneCorp, under Help in Chat: ask how something works, whether OneCorp can do it, or if what you saw is a bug. It knows the docs for your version and your company as it is now, and drafts a GitHub issue you review before posting. It only runs when you ask.

### Changed

- Employees read the wiki like a new hire would: rules pages on every ticket, their own space in full, and everything else when the work needs it. Each page says whether it's a Rule or Reference, and the Wiki page tells you when the rules get long. Prompts are smaller, so runs cost less.
- Each ticket's log starts with how big its prompt was and what it's made of: wiki, playbook, ticket, notes, chat. A Claude Code ticket's log also shows how its tokens split: how many it read, how many of those came from cache, and how many it wrote.
- Chat replies read the last 12 messages instead of 20, and Ask OneCorp reads the longer docs only when a question needs them.
- Spending splits the money by kind of run: tickets, chat replies, lead decisions and Ask OneCorp, as colors in the two-week chart. Under it, switch the table between people, kind, projects and agents. Settings is wider to fit.
- Spending says its dollars are at API prices: what you pay on an API key, and what the same work would cost on a subscription. When Claude Code or Codex hits its plan's usage limit, Spending says how often that happened this week.
- The chat and lead decisions setting says it follows each person's agent, and names each model with its agent.
- The Codex cost setting is gone from Integrations. Codex spend is still estimated from its tokens, and the est. tag in Spending says how.

### Fixed

- A wiki page's byline shows the face of whoever wrote it last, next to their name, and your own photo when that's you. Who started the page comes after, as "first written by".
- Spending no longer says nothing was spent yet when all of it was today.

## [0.1.9] - 2026-11-11

The wiki opens with an Overview and a Glossary, and a stopped ticket can be retried with a note on what to do differently.

### Added

- The wiki's Overview page holds what a new hire reads first: why the company exists, what you're building and for whom, and where it stands today, from idea to live. Your team drafts the sections you leave empty, and never touches the ones you wrote.
- Retry a stopped ticket with a note, from its Inbox card or the ticket: tell them what to do differently this time. The note shows in the ticket's history. From the command line: `onecorp retry KEY --note "..."`.
- A Glossary page in Product, drafted from the code and app copy: the words that mean something specific in your product, so nobody mixes them up.

### Changed

- Mission and What we're building are one page now, the Overview. Existing wikis merge them on their own, and Redraft fills in Where we are and the Glossary.

### Fixed

- Ticket details show their Markdown, like lists and `code`, instead of raw dashes and backticks.
- Pages moved in from the old wiki show who wrote them, you or the employee who drafted it, instead of OneCorp.
- Editing a wiki page looks like the page: the title big, one wide writing area, and Write and Preview tabs instead of a half-empty split.
- Check for Updates… and Settings… in the OneCorp menu have icons on macOS 26, like the items around them.
- While your team drafts the wiki, the Wiki page says who is writing which pages, with a link to each run, until the drafts land.

## [0.1.8] - 2026-11-11

The wiki becomes a real company wiki, audits come back as reports instead of pull requests, and your team writes plainly. Your current wiki moves into pages on its own the first time this version opens.

### Added

- The wiki is pages in spaces, like Product/Customers and Engineering/Conventions, instead of one long document. Each page shows who wrote it and who changed it last, and its history keeps every version, ready to restore. (#42)
- Your team drafts the wiki from the repo without asking you anything: the engineering lead writes the Engineering pages and someone on the product side the Product ones, at the same time. Pages only you can write, like Now, wait for you with a hint. A company that starts empty gets a "Draft your wiki" card on the Wiki page once you've hired someone. (#42)
- Redraft from the repo, from the wiki's sidebar or with `onecorp wiki redraft`, by your team or one person you pick. (#42)
- Proposals from your team name the page they belong on, and show on that page. Ticking "Save to the wiki" when you answer a question adds it to Decisions. (#42)
- Audits, research and advice come back as reports, not pull requests. When a ticket changes no files, its reply is the work: you read it in the Inbox with the tickets it proposes, then start the ones you want or keep them for later. (#41)
- Check for Updates… in the OneCorp menu, under About. It checks right away and says how it went: up to date, downloading, or ready to restart. (#44)
- Settings… ⌘, in the OneCorp menu, where Mac apps keep it. (#44)

### Changed

- Employees write plainly: short sentences, no filler or recaps, and never semicolons, em dashes or double hyphens in prose. It applies to everyone, whatever their personality, in summaries, reports, chat, the wiki and their questions to you. If a model slips anyway, OneCorp turns those into commas and full stops before you see them. Code is never touched. (#43)
- Codex runs with low verbosity, so its replies are shorter and use fewer tokens. (#43)
- Tickets an employee creates while working show up with that work, so you see what they propose next to what they did. (#41)
- Diffs keep their colors when you scroll sideways, and added and removed lines are easier to tell apart. (#41)
- Your existing wiki becomes pages the first time the app opens. The old file stays next to them as `wiki-before-pages.md`. (#42)
- `onecorp wiki edit` takes a page, like `onecorp wiki edit Company/Now`. (#42)
- The download opens as a small window with OneCorp next to your Applications folder: drag one onto the other to install. (#40)

## [0.1.7] - 2026-11-11

OneCorp now updates itself, and blocked tickets always reach your Inbox. This is the last version to install by hand: run `brew upgrade --cask onecorp` or download the DMG once, and later versions come to you.

### Added

- OneCorp updates itself. New versions download in the background, and "Restart to update" in the sidebar applies them.

### Fixed

- A ticket waiting on you always has something in your Inbox to answer. When an employee marks a ticket blocked, what they need lands in your Inbox as a question, and answering it picks the ticket back up. Tickets already stuck this way get their question the next time the app opens. (#26)
- Codex errors and your change requests show in full in a ticket's history, instead of stopping mid-sentence. (#21)
- Chat stays where you are: at the latest message while new ones and photos arrive, or where you scrolled up to read. Sending a message brings you back to the end. (#20)
- Custom photos show as soon as the app opens, instead of the drawn face for a few seconds first. (#14)

## [0.1.6] - 2026-11-11

A calmer, more polished app: replies you can cancel, scrollbars out of the way, and chat that remembers what you've read.

### Changed

- Asking for changes, answering a question or rewording a wiki rule opens a small box under the card, with Cancel. ⌘↵ sends it and Esc closes it. A card shows one thing at a time: your reply, or the discard confirmation.
- Scrollbars stay out of sight until you point at something that scrolls, like on the rest of your Mac.
- On the Company page, Needs you and Live stay in view and scroll on their own. Needs you lists everything waiting, not just the first three.
- The extra face sets load only when you pick one, so the app opens with less to load.
- The app's bundle identifier is now `dev.onecorp.desktop`. macOS asks again before showing notifications, and the theme and face set go back to their defaults once. Your companies and everything in them stay as they are.

### Fixed

- Chat no longer shows dots on messages you've already read. What you've read is remembered per company, for direct messages too, and survives a restart.
- Long links in the Live feed wrap instead of scrolling sideways.
- The Send button in chat lines up with the message box.
- The window no longer scrolls as a whole, which left a stray scrollbar and a grey square in the corner.

## [0.1.5] - 2026-11-11

Faster tickets, readable logs, and employees who ask instead of getting stuck.

### Added

- Discard work you don't want, from its review in the Inbox or on the ticket. The pull request is closed, the branch deleted, and the ticket goes back to the backlog to start again or delete.
- An employee who needs a command they can't run asks for it in your Inbox, like `xcodegen generate` with the reason. Allow it and the whole team can run it in that repo, and the ticket picks back up. Git, root and pull request commands are never asked: OneCorp handles those.

### Changed

- Commits are yours by default once git knows your name and email. "As your team" is still one click away in Settings.
- Pull requests end with a small signature: who did it, at which company, made with OneCorp. Choose Plain in Settings for none.
- Inbox cards keep all their actions on one row, and summaries show `code` as code.
- Run logs show what employees say in full, and whole commands and errors, wrapped under themselves instead of cut to one line.
- The Repos section in Settings is calmer: a card per repo with its commands, Add a repo at the top, and shorter paths.
- Long questions, wiki proposals and review summaries in the Inbox show a few lines, with Show all for the rest.
- Every employee knows how OneCorp works: tickets live in OneCorp, OneCorp does the git work and decides what they can run.
- Each repo's commands in Settings are now a list you can edit one by one: remove one with ×, or type and press Enter to add one. Pasting a list from a Claude Code settings file works, so `Bash(make build:*)` becomes `make build`.
- Employees know OneCorp decides which commands they can run. They no longer ask you to create `.claude/settings.json`.

### Fixed

- A review no longer shows other people's merged work as part of the ticket when your local main is behind GitHub.
- Run logs show `code` as code, and long temporary paths as just the file name.
- Tickets in Rust repos stop rebuilding everything from scratch. A new ticket reuses the builds already in your checkout, cloned instantly on your Mac's disk.
- A docs-only change no longer runs the full build and test suite. Employees run the checks that cover what they changed.
- A long command in a run log shows "still running, 3 min" instead of looking stuck.
- Asked in chat about the ticket they're on, employees see their latest steps and their worktree, so they no longer say they haven't changed anything when they have.
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

[0.1.12]: https://github.com/snowztech/onecorp/releases/tag/v0.1.12
[0.1.11]: https://github.com/snowztech/onecorp/releases/tag/v0.1.11
[0.1.10]: https://github.com/snowztech/onecorp/releases/tag/v0.1.10
[0.1.9]: https://github.com/snowztech/onecorp/releases/tag/v0.1.9
[0.1.8]: https://github.com/snowztech/onecorp/releases/tag/v0.1.8
[0.1.7]: https://github.com/snowztech/onecorp/releases/tag/v0.1.7
[0.1.6]: https://github.com/snowztech/onecorp/releases/tag/v0.1.6
[0.1.5]: https://github.com/snowztech/onecorp/releases/tag/v0.1.5
[0.1.4]: https://github.com/snowztech/onecorp/releases/tag/v0.1.4
[0.1.3]: https://github.com/snowztech/onecorp/releases/tag/v0.1.3
[0.1.2]: https://github.com/snowztech/onecorp/releases/tag/v0.1.2
[0.1.1]: https://github.com/snowztech/onecorp/releases/tag/v0.1.1
[0.1.0]: https://github.com/snowztech/onecorp/releases/tag/v0.1.0
