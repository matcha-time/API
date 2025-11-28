.PHONY: all build check test lint format format-check clean run dev docker-up docker-down help ci
.PHONY: prod-build prod-deploy prod-up prod-down prod-logs prod-backup prod-restore monitoring-up

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

# ============================================================================
# Production Commands
# ============================================================================

# Build production Docker image
prod-build:
	docker build -t matcha-time-api:latest .

# Full production deployment (build, backup, deploy, health check)
prod-deploy:
	./scripts/deploy.sh

# Start production services
prod-up:
	docker-compose -f compose.prod.yaml --env-file .env.production up -d

# Stop production services
prod-down:
	docker-compose -f compose.prod.yaml down

# View production logs
prod-logs:
	docker-compose -f compose.prod.yaml logs -f api

# Create database backup
prod-backup:
	./scripts/backup.sh

# Restore database from backup
prod-restore:
	@echo "Usage: make prod-restore BACKUP=<backup_file.sql.gz>"
	@echo "Example: make prod-restore BACKUP=./backups/postgres/matcha_db_20250128_120000.sql.gz"
ifdef BACKUP
	./scripts/restore.sh $(BACKUP)
endif

# Start monitoring stack (Prometheus + Grafana)
monitoring-up:
	docker-compose -f compose.prod.yaml --profile monitoring --env-file .env.production up -d

# Help command
help:
	@echo "Available targets:"
	@echo ""
	@echo "Development:"
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
	@echo "  make docker-up     - Start PostgreSQL (development)"
	@echo "  make docker-down   - Stop PostgreSQL"
	@echo "  make ci            - Run all CI checks"
	@echo "  make install-tools - Install dev dependencies"
	@echo ""
	@echo "Production:"
	@echo "  make prod-build    - Build production Docker image"
	@echo "  make prod-deploy   - Full deployment (recommended)"
	@echo "  make prod-up       - Start production services"
	@echo "  make prod-down     - Stop production services"
	@echo "  make prod-logs     - View production logs"
	@echo "  make prod-backup   - Create database backup"
	@echo "  make prod-restore  - Restore from backup (requires BACKUP=path)"
	@echo "  make monitoring-up - Start Prometheus + Grafana"