.PHONY: help setup dev down migrate-create migrate-up clean install

help:
	@echo "Mitra Development Commands:"
	@echo ""
	@echo "  make setup          - Initial project setup"
	@echo "  make install        - Install dependencies with Poetry"
	@echo "  make dev            - Start development environment"
	@echo "  make down           - Stop all services"
	@echo "  make migrate-create - Create new database migration"
	@echo "  make migrate-up     - Apply database migrations"
	@echo "  make clean          - Clean up containers and volumes"
	@echo "  make logs           - Show logs from all services"
	@echo "  make shell          - Open shell in backend container"
	@echo "  make format         - Format code with black"
	@echo "  make lint           - Lint code with ruff"
	@echo "  make test           - Run tests"

setup:
	@echo "Setting up Mitra development environment..."
	@echo "Checking Poetry installation..."
	@command -v poetry >/dev/null 2>&1 || { echo "Poetry not found. Install from https://python-poetry.org/docs/#installation"; exit 1; }
	@echo "✓ Poetry is installed"
	@echo ""
	@echo "Creating .env file..."
	@test -f backend/.env || cp backend/.env.example backend/.env
	@echo "✓ .env file created (edit with your configuration)"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Edit backend/.env with your configuration"
	@echo "  2. Run 'make install' to install dependencies locally"
	@echo "  3. Run 'make dev' to start services"
	@echo "  4. Run 'make migrate-create' to create initial migration"
	@echo "  5. Run 'make migrate-up' to apply migrations"

install:
	@echo "Installing backend dependencies..."
	cd backend && poetry install
	@echo ""
	@echo "Installing ML dependencies..."
	cd ml && poetry install
	@echo "✓ Dependencies installed"

dev:
	docker-compose up --build

down:
	docker-compose down

migrate-create:
	docker-compose exec backend poetry run alembic revision --autogenerate -m "$(if $(msg),$(msg),Auto migration)"

migrate-up:
	docker-compose exec backend poetry run alembic upgrade head

migrate-down:
	docker-compose exec backend poetry run alembic downgrade -1

clean:
	docker-compose down -v
	@echo "✓ Cleaned up containers and volumes"

logs:
	docker-compose logs -f

shell:
	docker-compose exec backend /bin/bash

backend-shell:
	docker-compose exec backend poetry run python

test:
	docker-compose exec backend poetry run pytest

format:
	cd backend && poetry run black app/
	cd ml && poetry run black .

lint:
	cd backend && poetry run ruff check app/

typecheck:
	cd backend && poetry run mypy app/

# Local development (without Docker)
local-install:
	cd backend && poetry install && poetry run alembic upgrade head

local-run:
	cd backend && poetry run uvicorn app.main:app --reload

local-test:
	cd backend && poetry run pytest
