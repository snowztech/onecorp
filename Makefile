# Everyday commands. Run `make` to see them.

DEMO_HOME := /tmp/onecorp-demo

.PHONY: help app fresh demo demo-reset serve doctor test lint check install install-cli bundle

help:
	@echo "make app          open the desktop app on your real companies (~/.onecorp)"
	@echo "make fresh        open the app as a first user would: empty home, full onboarding"
	@echo "make demo         open the app on a throwaway home, to try things safely"
	@echo "make demo-reset   wipe the throwaway home, to see first-launch setup again"
	@echo "make serve        run queued work in this terminal, without the app"
	@echo "make doctor       check git, Claude Code, Codex and your companies"
	@echo "make test         Rust tests, the UI smoke test, and a walk through onboarding in Chrome"
	@echo "make lint         clippy, warnings as errors"
	@echo "make check        lint and test, what CI runs"
	@echo "make install      install the onecorp command and OneCorp.app"
	@echo "make install-cli  install only the onecorp command"
	@echo "make bundle       build OneCorp.app and a DMG without installing it"

app:
	cargo run -p onecorp-app

demo:
	ONECORP_HOME=$(DEMO_HOME) cargo run -p onecorp-app

# The app on a brand-new, empty home: see onboarding exactly as a first user does. Your real
# companies aren't touched. Run it again for another fresh start.
fresh:
	rm -rf $(CURDIR)/target/fresh-home && ONECORP_HOME=$(CURDIR)/target/fresh-home cargo run -p onecorp-app

demo-reset:
	rm -rf $(DEMO_HOME)

serve:
	cargo run -q -p onecorp-cli -- serve

doctor:
	cargo run -q -p onecorp-cli -- doctor

test:
	cargo test -p onecorp -p onecorp-cli
	node app/tests/ui-smoke.js
	node app/tests/onboarding.mjs

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
