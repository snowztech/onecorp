# onecorp

Run a company of AI employees. Hire a team, give them tickets, watch them work in real repos, approve what ships.

Early days. Employees run on Claude Code or Codex, and everything happens in the CLI. The idea, product and architecture notes live in `../crew-hq/`.

## Try it

```sh
cargo build
alias onecorp=./target/debug/onecorp

onecorp new snowztech --repo ~/dev/some-repo --mission "Open source tools for AI agents"
onecorp hire                                  # asks name, job, how they work, model
onecorp hire "Maya Okafor" --as engineer --model sonnet
onecorp hire "Tomás Reis" --as engineer --runtime codex
onecorp team
onecorp run "add a --version flag" --to maya  # --to is optional with one employee
onecorp status
onecorp logs SNO-1
onecorp diff SNO-1                            # what the ticket changed
onecorp changes SNO-1 "use a match instead"   # send it back, same session picks it up
onecorp approve SNO-1                         # merge into the branch it started from
onecorp inbox                                 # questions your team is waiting on
onecorp answer 1 "go with 15 minutes"
onecorp chat eng                              # what the team posted
onecorp handbook                              # what every employee reads before a ticket
onecorp handbook add "No ORMs"                # under Decisions, or --section Conventions
onecorp handbook edit                         # open it in $EDITOR
onecorp fire maya
```

Presets for `--as`: `engineer`, `reviewer`, `designer`, `writer`. Override the title with `--role` and how they work with `--bio`.

`run` opens a ticket, creates a git worktree on branch `onecorp/<company>/sno-1`, and runs the employee's agent inside it. The prompt is built from their name, role and bio. When the agent finishes, OneCorp commits the work under the employee's name, like `Tomás Reis <tomas@snowztech.onecorp>`. Every event and the final cost go into `~/.onecorp/companies/<name>/company.db`. Your main checkout is never touched.

Each agent gets the company's MCP server (`onecorp mcp`, started by the agent over stdio, scoped to one ticket) with these tools:

- `ask_human` puts a question in your inbox. With a default the agent carries on. With `blocking` it stops and the ticket shows as blocked.
- `post_message` posts to a team channel.
- `update_ticket` marks the ticket blocked, with a reason.
- `propose_handbook` suggests a decision or convention. It lands in your inbox, and `onecorp answer N yes` (or `no`, or your own wording) settles it.

## Shared knowledge

Each company has a handbook at `~/.onecorp/companies/<name>/handbook.md`: mission, decisions, conventions. It's plain markdown, so edit it however you like. Every ticket starts with the handbook and a list of who works here, generated from the team, so a decision made once is known by everyone. Approved work lands on the base branch, so the next ticket starts from everyone's merged code too.

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
