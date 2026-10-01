.PHONY: build check test lint clean format format-check verify eval

build:
	cargo build --release

check:
	cargo check
	cargo clippy --all-targets --all-features -- -D warnings

test:
	cargo test

format:
	cargo fmt --all

format-check:
	cargo fmt --all -- --check

lint: format-check
	cargo clippy --all-targets --all-features -- -D warnings

verify: format-check
	cargo clippy --all-targets --all-features -- -D warnings
	cargo test

eval:
	cargo build -p intent
	uv run --project evals pytest evals/ -s

clean:
	cargo clean
