# onecorp

Run a company of AI employees. Hire a team, give them tickets, watch them work in real repos, approve what ships.

Early days. Employees run on Claude Code or Codex, and everything happens in the CLI. The idea, product and brand notes live in [docs/vision](docs/vision). Where this is going and what is next: [ROADMAP.md](ROADMAP.md). How it works inside: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Layout

```
crates/core/     the `onecorp` library: everything that runs a company
  src/store.rs     companies, employees, tickets, inbox, jobs (one SQLite file per company)
  src/work.rs      start, resume and approve tickets
  src/serve.rs     runs queued work in parallel
  src/mcp.rs       the company tools agents call
  src/git.rs       worktrees, merges, commits under each employee's name
  src/runtime/     the agent CLIs: shared stream loop, Claude Code, Codex
crates/cli/      the `onecorp` command: parses arguments and prints, nothing else
app/             the desktop app: plain HTML/CSS/JS in app/ui, Rust commands in app/src
docs/vision/     product, architecture, brand and the clickable prototype
```

Logic lives in `crates/core`. The CLI and the app are thin shells over it, so both always behave the same.

## Try it

```sh
cargo build
alias onecorp=./target/debug/onecorp

onecorp doctor                                # checks git, Claude Code, Codex and your companies

onecorp new snowztech --repo ~/dev/some-repo --mission "Open source tools for AI agents"
onecorp hire                                  # asks name, job, how they work, model
onecorp hire "Maya Okafor" --as engineer --model sonnet
onecorp hire "Tomás Reis" --as engineer --runtime codex
onecorp hire Demo --runtime fake              # a stand-in with no AI, free, for trying things out
onecorp team
onecorp run "add a --version flag" --to maya  # --to is optional with one employee
onecorp status
onecorp logs SNO-1
onecorp diff SNO-1                            # what the ticket changed
onecorp changes SNO-1 "use a match instead"   # send it back, same session picks it up
onecorp approve SNO-1                         # merge into the branch it started from
onecorp retry SNO-4                           # try a ticket that stopped or failed again
onecorp inbox                                 # questions your team is waiting on
onecorp answer 1 "go with 15 minutes"
onecorp chat eng                              # what the team posted
onecorp handbook                              # what every employee reads before a ticket
onecorp handbook add "No ORMs"                # under Decisions, or --section Conventions
onecorp handbook edit                         # open it in $EDITOR
onecorp memory maya                           # what Maya has learned (--edit to change it)
onecorp fire maya
onecorp serve                                 # run queued work, several employees at a time
```

Presets for `--as`: `engineer`, `reviewer`, `designer`, `writer`. Override the title with `--role` and how they work with `--bio`.

`run` opens a ticket, creates a git worktree on branch `onecorp/<company>/sno-1`, and runs the employee's agent inside it. The prompt is built from their name, role and bio. When the agent finishes, OneCorp commits the work under the employee's name, like `Tomás Reis <tomas@snowztech.onecorp>`. Every event and the final cost go into `~/.onecorp/companies/<name>/company.db`. Your main checkout is never touched.

Each agent gets the company's MCP server (`onecorp mcp`, started by the agent over stdio, scoped to one ticket) with these tools:

- `ask_human` puts a question in your inbox. With a default the agent carries on. With `blocking` it stops and the ticket shows as blocked.
- `post_message` posts to a team channel.
- `update_ticket` marks the ticket blocked, with a reason.
- `remember` saves a one-line note to the employee's own memory.
- `propose_handbook` suggests a decision or convention. It lands in your inbox, and `onecorp answer N yes` (or `no`, or your own wording) settles it.

## Desktop app

```sh
cargo run -p onecorp-app
```

It opens your current company: you on top, departments and their desks beneath, everything that needs you in the Inbox. The team keeps working while the window is open, since the app runs `serve` for you. Create companies with `onecorp new` for now.

To tweak the design without the app, open `app/ui/index.html` in a browser. It runs on sample data.

## Running in the background

Without `onecorp serve`, every command runs its work right there in your terminal. With it running, `run`, `answer`, `changes` and a conflicting `approve` queue the work and return straight away, and serve picks it up:

- Up to `--max` employees work at the same time (default 3), across all your companies.
- Each employee works one ticket at a time, like a person. Their next ticket waits.
- Every line serve prints is tagged with the ticket and the employee, like `SNO-3 tomas`.
- If serve is stopped mid-ticket, the next start marks that ticket failed instead of leaving it stuck.

## Leads answer first

Questions from someone in a department go to their lead first. The lead decides what the handbook and the code settle, and passes up the rest with a reason. Product, money, anything public and security come to you. Set a lead with `onecorp dept lead Engineering odile`.

## Shared knowledge

Each company has a handbook at `~/.onecorp/companies/<name>/handbook.md`: mission, decisions, conventions. It's plain markdown, so edit it however you like. Every ticket starts with the handbook and a list of who works here, generated from the team, so a decision made once is known by everyone. Approved work lands on the base branch, so the next ticket starts from everyone's merged code too.

Each employee also has their own memory at `~/.onecorp/companies/<name>/memory/<id>.md`: one dated line per note, read at the start of each of their tickets. They add to it with `remember`, and when you ask for changes, your feedback goes in automatically, so they pick up how you like things done. Firing someone keeps the file, so rehiring brings their memory back.

Answering picks the work back up on its own. `onecorp answer` resumes the agent's session in the same worktree when:

- the question was blocking and it was the ticket's last open one, or
- the agent went ahead with a default and your answer is different.

`approve` brings the base branch into the ticket first. If that conflicts, the ticket goes back to its employee with the list of files, they fix the markers, OneCorp finishes the merge, and you approve again. Then the base fast-forwards, so if it's checked out in your repo, git updates your files or refuses rather than overwrite local changes. The ticket's worktree and branch are removed once it's merged.

Only this MCP server loads for agents. Your own Claude Code and Codex MCP servers and config stay out of it.

What agents can run depends on the runtime:

- Claude Code: edits in the worktree and read-only git. No builds or tests yet.
- Codex: any command in its sandbox (worktree writes only, no network), so it can build and test.

Codex reports tokens, not dollars, so its spend shows as tokens.

Set `ONECORP_HOME` to keep data somewhere other than `~/.onecorp`.
