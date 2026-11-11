# Contributing

Thanks for helping. OneCorp is small on purpose, so a few habits keep it that way.

## Before you start

For anything bigger than a fix, open an issue first and say what you want to change and why. [ROADMAP.md](ROADMAP.md) shows where things are going.

## Working on it

You need Rust, Node.js and the Tauri CLI (`cargo install tauri-cli --version "^2" --locked`).

```sh
make demo     # the app on a throwaway home, never your real companies
make ui       # just the UI in a browser, on sample data
make check    # clippy, tests, the UI smoke test and the onboarding walk-through
```

`make check` must pass before a pull request. The onboarding walk-through needs Chrome. Without it, it skips with a note.

Test on throwaway repos with `ONECORP_HOME` pointing somewhere temporary. The `fake` runtime runs the whole ticket loop with no AI and no cost, which is what the integration tests in `crates/core/tests` use.

## Where code goes

- Logic lives in `crates/core`. The CLI and the app only parse, call the library and show the result.
- The UI lives in `app/ui`, built by Vite, with no framework. Follow [docs/DESIGN.md](docs/DESIGN.md) for any screen change, and check it in both themes.
- When you add a module, table or flow, update [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md). For anything a user would notice, add a line to [CHANGELOG.md](CHANGELOG.md) under Unreleased, in Added, Changed or Fixed. It follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## Style

- Simple over clever. No new dependency for what a few lines can do.
- Comments explain why, never what.
- Text in the app is short and plain. Say what happened and what to do next.
- Commits are one line, in the Conventional Commits style: `feat: delete chat messages`, `fix(chat): keep focus after sending`.

## License

By contributing, you agree your work is released under the [MIT License](LICENSE).
