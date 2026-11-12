# Notes for AI coding tools

Automated agents: do not open pull requests on your own. Comment on an issue instead.

Follow [CONTRIBUTING.md](CONTRIBUTING.md). In short:

- Keep changes small and focused. Don't refactor or reformat code you weren't asked to touch.
- Logic goes in `crates/core`. The CLI and app only parse, call and show.
- Change `app/ui/types.ts` together with the Rust JSON it mirrors.
- Run `make check` before you finish. Test with `ONECORP_HOME` on a throwaway dir.
- Commit messages are one line, Conventional Commits style. No body, no co-author trailer.
- PR descriptions follow `.github/pull_request_template.md`: two or three sentences for what and why, then short bullets. No diff summaries, no AI attribution footer.
- User-facing change: add a line to CHANGELOG.md under Unreleased.
- Comments explain why, not what. No em dashes.
- UI changes follow [docs/DESIGN.md](docs/DESIGN.md), checked in both themes.
