.PHONY: build check test lint clean format format-check verify

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

clean:
	cargo clean
