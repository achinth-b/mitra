# Quick Start Guide

Get Mitra running in under 5 minutes with Poetry.

## Prerequisites

- Docker & Docker Compose
- Poetry (optional for local development)

## Option 1: Docker (Easiest - Recommended)

### 1. Setup Environment

```bash
# Copy environment template
cp backend/.env.example backend/.env

# Edit with your configuration
vim backend/.env
```

**Required changes in `.env`:**
- `SECRET_KEY`: Generate with `openssl rand -hex 32`
- `RESEND_API_KEY`: Get from https://resend.com (or use dummy for now)

### 2. Start Services

```bash
# Start PostgreSQL, Redis, and Backend
docker-compose up --build
```

Wait for services to be healthy (check logs for "Application startup complete").

### 3. Initialize Database

In a new terminal:

```bash
# Create initial migration
docker-compose exec backend poetry run alembic revision --autogenerate -m "Initial schema"

# Apply migration
docker-compose exec backend poetry run alembic upgrade head
```

### 4. Verify Everything Works

```bash
# Check API health
curl http://localhost:8000/health

# Open API docs in browser
open http://localhost:8000/docs
```

## Option 2: Local Development (No Docker)

### 1. Install Poetry

```bash
curl -sSL https://install.python-poetry.org | python3 -
```

### 2. Install Dependencies

```bash
# Backend
cd backend
poetry install

# ML (optional)
cd ../ml
poetry install
```

### 3. Setup Services

```bash
# Start PostgreSQL and Redis (using Homebrew on macOS)
brew services start postgresql@15
brew services start redis

# Create database
createdb mitra
```

### 4. Configure Environment

```bash
cd backend
cp .env.example .env
# Edit DATABASE_URL to use localhost
# DATABASE_URL=postgresql+asyncpg://YOUR_USER@localhost:5432/mitra
```

### 5. Run Migrations

```bash
cd backend
poetry run alembic revision --autogenerate -m "Initial schema"
poetry run alembic upgrade head
```

### 6. Start Backend

```bash
poetry run uvicorn app.main:app --reload
```

## Using Makefile (Easiest)

```bash
# Setup (creates .env from template)
make setup

# Install dependencies locally (optional)
make install

# Start development (Docker)
make dev

# In another terminal:
make migrate-create msg="Initial schema"
make migrate-up

# View logs
make logs

# Stop everything
make down
```

## Next Steps

1. **Test the API**: Visit http://localhost:8000/docs
2. **Format code**: `make format` or `cd backend && poetry run black app/`
3. **Lint code**: `make lint` or `cd backend && poetry run ruff check app/`
4. **Run tests**: `make test` or `cd backend && poetry run pytest`

## Poetry Quick Reference

```bash
# Activate virtual environment
cd backend
poetry shell

# Add a package
poetry add package-name

# Add dev dependency
poetry add --group dev package-name

# Update all packages
poetry update

# Show installed packages
poetry show

# Run a command
poetry run python script.py
```

## Troubleshooting

### Port already in use
```bash
# Check what's using ports
lsof -i :5432  # PostgreSQL
lsof -i :6379  # Redis
lsof -i :8000  # Backend

# Stop conflicting services
brew services stop postgresql
brew services stop redis
```

### Poetry not found
```bash
# Install Poetry
curl -sSL https://install.python-poetry.org | python3 -

# Add to PATH (add to ~/.zshrc or ~/.bashrc)
export PATH="$HOME/.local/bin:$PATH"
```

### Database connection issues
```bash
# Check PostgreSQL is running
docker-compose ps

# View PostgreSQL logs
docker-compose logs postgres

# Recreate database
make clean  # Warning: deletes all data
make dev
```

### Poetry lock issues
```bash
cd backend
poetry lock --no-update
poetry install
```

## Development Workflow

1. **Make code changes** in `backend/app/`
2. **FastAPI auto-reloads** (check logs)
3. **Format code**: `poetry run black app/`
4. **Lint**: `poetry run ruff check app/`
5. **Add database changes**:
   ```bash
   make migrate-create msg="Add new field"
   make migrate-up
   ```
6. **Test**: `poetry run pytest`
7. **Test in API docs**: http://localhost:8000/docs

## Database Access

```bash
# Connect to PostgreSQL
docker-compose exec postgres psql -U mitra -d mitra

# Common queries
SELECT * FROM users;
SELECT * FROM markets;
\dt  -- List tables
\d users  -- Describe table
```

## Redis Access

```bash
# Connect to Redis
docker-compose exec redis redis-cli

# Common commands
KEYS *
GET magic_link:*
FLUSHDB  -- Clear all data (development only!)
```

## Clean Slate

```bash
# Remove everything and start fresh
make clean
make dev
make migrate-create msg="Initial schema"
make migrate-up
```

## What You Have Now

✅ FastAPI backend with Poetry dependency management
✅ PostgreSQL database with all tables
✅ Redis for caching and sessions
✅ Alembic migrations configured
✅ Docker environment for consistency
✅ Auto-reload for rapid development
✅ Interactive API docs at /docs
✅ Modern Python tooling (Black, Ruff, mypy)

## What's Next

See Phase 1 plan for the full 6-week implementation.

**Week 1 remaining tasks:**
- [ ] Implement auth endpoints (magic link)
- [ ] User CRUD operations
- [ ] Group management
- [ ] Invite system

Ready to code with Poetry! 🚀
