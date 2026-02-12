SHELL := /bin/bash

# Detect host architecture for musl target
MUSL_TARGET := $(shell uname -m | sed 's/x86_64/x86_64-unknown-linux-musl/' | sed 's/aarch64/aarch64-unknown-linux-musl/')

.PHONY: build build-debug build-static test test-unit test-e2e test-e2e-all test-e2e-browser test-perf test-e2e-podman test-compose test-compose-validate coverage run fmt clippy check docker-build docker-build-cross docker-build-multiarch docker-run compose-up compose-down hash-password clean security-scan test-all quick-start init completions manpage bench changelog

build:
	cargo build --release

build-debug:
	cargo build

build-static:
	rustup target add $(MUSL_TARGET) 2>/dev/null || true
	cargo build --release --target $(MUSL_TARGET)
	@echo "Static binary: target/$(MUSL_TARGET)/release/s5"

test:
	cargo test --all-targets

test-unit:
	cargo test --lib

test-e2e:
	cargo test --test '*'

test-e2e-all:
	cargo test --test '*' -- --include-ignored

test-e2e-browser:
	@command -v podman >/dev/null 2>&1 || { echo "Error: podman is required for browser E2E tests"; exit 1; }
	@podman image exists docker.io/chromedp/headless-shell:latest 2>/dev/null || \
		{ echo "Pulling chromedp/headless-shell..."; podman pull docker.io/chromedp/headless-shell:latest; }
	@status=0; \
	cargo test --test e2e_browser_dashboard -- --ignored --nocapture || status=$$?; \
	podman ps -aq --filter "name=s5-chrome" | xargs -r podman stop 2>/dev/null || true; \
	exit $$status

test-perf:
	cargo test --test e2e_performance -- --ignored --nocapture

test-e2e-podman:
	./scripts/test-e2e-podman.sh

test-compose:
	./scripts/test-compose.sh

test-compose-validate:
	podman-compose config

test-all: test security-scan

coverage:
	cargo llvm-cov --all-targets

run:
	cargo run -- --config config.example.toml

fmt:
	cargo fmt

clippy:
	cargo clippy --all-targets -- -D warnings

check:
	cargo check --all-targets

security-scan:
	./scripts/security-scan.sh

docker-build:
	podman build -t s5:latest .

docker-build-cross:
	./scripts/build-multiarch-cross.sh

docker-build-multiarch:
	./scripts/build-multiarch-qemu.sh

docker-run:
	podman run --rm -p 2222:2222 -p 1080:1080 -v ./config.example.toml:/etc/s5/config.toml:ro s5:latest

compose-up:
	podman-compose up -d

compose-down:
	podman-compose down

hash-password:
	@read -sp "Enter password: " pass && echo && \
	hash=$$(cargo run --quiet -- hash-password --password "$$pass") && \
	echo "" && \
	echo "password_hash = \"$$hash\""

quick-start:
	cargo run -- quick-start --password demo

init:
	cargo run -- init --password demo --output config.toml

completions:
	@mkdir -p completions
	cargo run --quiet -- completions bash > completions/s5.bash
	cargo run --quiet -- completions zsh > completions/_s5
	cargo run --quiet -- completions fish > completions/s5.fish
	@echo "Shell completions generated in completions/"

manpage:
	@mkdir -p man
	cargo run --quiet -- manpage > man/s5.1
	@echo "Man page generated: man/s5.1"

bench:
	cargo bench

changelog:
	git-cliff --output CHANGELOG.md

clean:
	cargo clean
