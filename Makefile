# Everyday commands. Run `make` to see them.

DEMO_HOME := /tmp/onecorp-demo

.PHONY: help app demo demo-reset serve doctor test lint check install install-cli bundle

help:
	@echo "make app          open the desktop app on your real companies (~/.onecorp)"
	@echo "make demo         open the app on a throwaway home, to try things safely"
	@echo "make demo-reset   wipe the throwaway home, to see first-launch setup again"
	@echo "make serve        run queued work in this terminal, without the app"
	@echo "make doctor       check git, Claude Code, Codex and your companies"
	@echo "make test         Rust tests, the end-to-end test, and the UI smoke test"
	@echo "make lint         clippy, warnings as errors"
	@echo "make check        lint and test, what CI runs"
	@echo "make install      install the onecorp command and OneCorp.app"
	@echo "make install-cli  install only the onecorp command"
	@echo "make bundle       build OneCorp.app without installing it"

app:
	cargo run -p onecorp-app

demo:
	ONECORP_HOME=$(DEMO_HOME) cargo run -p onecorp-app

demo-reset:
	rm -rf $(DEMO_HOME)

serve:
	cargo run -q -p onecorp-cli -- serve

doctor:
	cargo run -q -p onecorp-cli -- doctor

test:
	cargo test -p onecorp -p onecorp-cli
	node app/tests/ui-smoke.js

lint:
	cargo clippy -p onecorp -p onecorp-cli -p onecorp-app --all-targets -- -D warnings

check: lint test

install:
	scripts/install.sh

install-cli:
	scripts/install.sh --cli

bundle:
	cd app && cargo tauri build --bundles app,dmg
	@echo "Built target/release/bundle/macos/OneCorp.app and target/release/bundle/dmg/"
