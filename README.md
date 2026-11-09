# onecorp

Run a company of AI employees. Hire a team, give them tickets, watch them work in real repos, approve what ships.

Early days. Right now there's one employee (Claude Code) and a CLI. The idea, product and architecture notes live in `../crew-hq/`.

## Try it

```sh
cargo build
./target/debug/onecorp new snowztech --repo ~/dev/some-repo
./target/debug/onecorp run "add a --version flag"
./target/debug/onecorp status
./target/debug/onecorp logs SNO-1
```

`run` opens a ticket, creates a git worktree on branch `onecorp/sno-1`, and runs Claude Code headless inside it. Every event and the final cost go into `~/.onecorp/companies/<name>/company.db`. Your main checkout is never touched.

For now the agent can edit files in its worktree and run git, nothing else. No builds, no tests, no network commands.

Set `ONECORP_HOME` to keep data somewhere other than `~/.onecorp`.
