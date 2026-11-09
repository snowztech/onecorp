# onecorp

Run a company of AI employees. Hire a team, give them tickets, watch them work in real repos, approve what ships.

Early days. Right now employees run on Claude Code and everything happens in the CLI. The idea, product and architecture notes live in `../crew-hq/`.

## Try it

```sh
cargo build
alias onecorp=./target/debug/onecorp

onecorp new snowztech --repo ~/dev/some-repo
onecorp hire                                  # asks name, job, how they work, model
onecorp hire "Maya Okafor" --as engineer --model sonnet
onecorp team
onecorp run "add a --version flag" --to maya  # --to is optional with one employee
onecorp status
onecorp logs SNO-1
onecorp fire maya
```

Presets for `--as`: `engineer`, `reviewer`, `designer`, `writer`. Override the title with `--role` and how they work with `--bio`.

`run` opens a ticket, creates a git worktree on branch `onecorp/<company>/sno-1`, and runs the employee's agent inside it. The prompt is built from their name, role and bio. Every event and the final cost go into `~/.onecorp/companies/<name>/company.db`. Your main checkout is never touched.

For now agents can edit files in their worktree and run git, nothing else. No builds, no tests, no network commands.

Set `ONECORP_HOME` to keep data somewhere other than `~/.onecorp`.
