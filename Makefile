.PHONY: help install dev-install test lint format typecheck clean docker-up docker-down init-db

help:
	@echo "Cudgel - Code Indexing Tool"
	@echo ""
	@echo "Available commands:"
	@echo "  make install      - Install cudgel"
	@echo "  make dev-install  - Install cudgel with development dependencies"
	@echo "  make test         - Run tests"
	@echo "  make lint         - Run linter (ruff)"
	@echo "  make format       - Format code with black"
	@echo "  make typecheck    - Run type checker (mypy)"
	@echo "  make clean        - Remove build artifacts"
	@echo "  make docker-up    - Start PostgreSQL and Temporal with Docker"
	@echo "  make docker-down  - Stop Docker services"
	@echo "  make init-db      - Initialize database schema"

install:
	pip install -e .

dev-install:
	pip install -e ".[dev]"

test:
	pytest tests/ -v

lint:
	ruff check src/

format:
	black src/

typecheck:
	mypy src/

clean:
	rm -rf build/
	rm -rf dist/
	rm -rf *.egg-info
	rm -rf .pytest_cache/
	rm -rf .mypy_cache/
	rm -rf .ruff_cache/
	find . -type d -name __pycache__ -exec rm -rf {} +
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
	cudgel init-db

all: format lint typecheck test
