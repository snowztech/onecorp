# onecorp

Run a company of AI employees. Hire a team, give them tickets, watch them work in real repos, approve what ships.

Early days. Employees run on Claude Code or Codex, and everything happens in the CLI. The idea, product and architecture notes live in `../crew-hq/`.

## Try it

```sh
cargo build
alias onecorp=./target/debug/onecorp

onecorp new snowztech --repo ~/dev/some-repo
onecorp hire                                  # asks name, job, how they work, model
onecorp hire "Maya Okafor" --as engineer --model sonnet
onecorp hire "Tomás Reis" --as engineer --runtime codex
onecorp team
onecorp run "add a --version flag" --to maya  # --to is optional with one employee
onecorp status
onecorp logs SNO-1
onecorp inbox                                 # questions your team is waiting on
onecorp answer 1 "go with 15 minutes"
onecorp chat eng                              # what the team posted
onecorp fire maya
```

Presets for `--as`: `engineer`, `reviewer`, `designer`, `writer`. Override the title with `--role` and how they work with `--bio`.

`run` opens a ticket, creates a git worktree on branch `onecorp/<company>/sno-1`, and runs the employee's agent inside it. The prompt is built from their name, role and bio. When the agent finishes, OneCorp commits the work under the employee's name, like `Tomás Reis <tomas@snowztech.onecorp>`. Every event and the final cost go into `~/.onecorp/companies/<name>/company.db`. Your main checkout is never touched.

Each agent gets the company's MCP server (`onecorp mcp`, started by the agent over stdio, scoped to one ticket) with three tools:

- `ask_human` puts a question in your inbox. With a default the agent carries on. With `blocking` it stops and the ticket shows as blocked.
- `post_message` posts to a team channel.
- `update_ticket` marks the ticket blocked, with a reason.

Answering picks the work back up on its own. `onecorp answer` resumes the agent's session in the same worktree when:

- the question was blocking and it was the ticket's last open one, or
- the agent went ahead with a default and your answer is different.

Only this MCP server loads for agents. Your own Claude Code and Codex MCP servers and config stay out of it.

What agents can run depends on the runtime:

- Claude Code: edits in the worktree and read-only git. No builds or tests yet.
- Codex: any command in its sandbox (worktree writes only, no network), so it can build and test.

Codex reports tokens, not dollars, so its spend shows as tokens.

Set `ONECORP_HOME` to keep data somewhere other than `~/.onecorp`.
