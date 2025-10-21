.PHONY: help build install test lint format clean docker-up docker-down init-db release

help:
	@echo "Cudgel - Code Indexing Tool"
	@echo ""
	@echo "Available commands:"
	@echo "  make build        - Build Rust binary"
	@echo "  make install      - Install cudgel binary"
	@echo "  make test         - Run Rust tests"
	@echo "  make lint         - Run clippy"
	@echo "  make format       - Format code with rustfmt"
	@echo "  make clean        - Remove build artifacts"
	@echo "  make docker-up    - Start PostgreSQL and Temporal with Docker"
	@echo "  make docker-down  - Stop Docker services"
	@echo "  make init-db      - Initialize database schema"
	@echo "  make release      - Build optimized release binary"

build:
	cargo build

release:
	cargo build --release

install:
	cargo install --path .

test:
	cargo test

lint:
	cargo clippy -- -D warnings

format:
	cargo fmt

clean:
	cargo clean
	rm -rf target/
	rm -rf .pytest_cache/
	find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	find . -type f -name "*.pyc" -delete

docker-up:
	docker-compose up -d
	@echo "Waiting for PostgreSQL to be ready..."
	@sleep 5
	@echo "Services started:"
	@echo "  PostgreSQL: localhost:5432"
	@echo "  Temporal: localhost:7233"
	@echo "  Temporal UI: http://localhost:8080"

docker-down:
	docker-compose down

init-db:
	cargo run --release -- init-db

all: format lint test build
