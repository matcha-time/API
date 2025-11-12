all: build lint format

lint:
	cargo fmt

build:
	cargo build

format:
	cargo fmt -- --check
	cargo clippy --locked --all-targets --all-features -- -D warnings --no-deps
	cargo clippy --tests --no-deps -- -D warnings