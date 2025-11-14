.PHONY: all build check test lint format format-check clean run dev docker-up docker-down help ci

# Default target - format first, then lint, then check
all: format lint check

# Build the project
build:
	cargo build --workspace

# Build release version
build-release:
	cargo build --workspace --release

# Quick check without building
check:
	cargo check --workspace

# Run tests
test:
	cargo test --workspace

# Run clippy linter (assumes code is already formatted)
lint:
	cargo clippy --workspace -- -D warnings

# Auto-format code (run this before linting)
format:
	cargo fmt --all

# Check formatting without modifying files (for CI)
format-check:
	cargo fmt --all -- --check

# Clean build artifacts
clean:
	cargo clean

# Run the server (development mode)
run:
	cargo run --bin serv

# Run the server with auto-reload (requires cargo-watch)
dev:
	cargo watch -x 'run --bin serv'

# Start PostgreSQL with Docker Compose
docker-up:
	docker compose -f compose.dev.yaml up -d

# Stop PostgreSQL
docker-down:
	docker compose -f compose.dev.yaml down

# Full CI check (format-check instead of format to avoid modifying files)
ci: format-check lint check test

# Install development tools
install-tools:
	cargo install cargo-watch
	cargo install sqlx-cli --no-default-features --features postgres

# Help command
help:
	@echo "Available targets:"
	@echo "  make all           - Format, lint, and check (default)"
	@echo "  make build         - Build the project"
	@echo "  make build-release - Build optimized release version"
	@echo "  make check         - Quick compilation check"
	@echo "  make test          - Run all tests"
	@echo "  make lint          - Run clippy linter"
	@echo "  make format        - Auto-format code with rustfmt"
	@echo "  make format-check  - Check formatting without changes"
	@echo "  make clean         - Remove build artifacts"
	@echo "  make run           - Run the server"
	@echo "  make dev           - Run with auto-reload (needs cargo-watch)"
	@echo "  make docker-up     - Start PostgreSQL"
	@echo "  make docker-down   - Stop PostgreSQL"
	@echo "  make ci            - Run all CI checks (format-check, lint, check, test)"
	@echo "  make install-tools - Install dev dependencies"