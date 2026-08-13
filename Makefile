.PHONY: build check test lint clean format

build:
	cargo build --release

check:
	cargo check
	cargo clippy -- -D warnings

test:
	cargo test

lint: format check

format:
	cargo fmt --all

clean:
	cargo clean
