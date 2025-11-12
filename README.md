# Mitra

Social prediction market for friend groups. Bet on events about people you mutually know.

## Project Structure

```
mitra/
├── backend/          # FastAPI backend (Python + Poetry)
├── frontend/         # Next.js frontend (TypeScript) - Coming soon
├── ml/              # ML components (AMM, recommendations, analytics)
└── docker-compose.yml
```

## Quick Start

### Prerequisites

- Docker & Docker Compose
- Poetry (optional for local development)

### Setup

1. **Install Poetry (if developing locally):**
   ```bash
   curl -sSL https://install.python-poetry.org | python3 -
   ```

2. **Setup environment:**
   ```bash
   make setup
   # Or manually: cp backend/.env.example backend/.env
   # Edit backend/.env with your configuration
   ```

3. **Start services:**
   ```bash
   make dev
   # Or: docker-compose up --build
   ```

4. **Create and apply database migrations:**
   ```bash
   make migrate-create msg="Initial schema"
   make migrate-up
   ```

5. **Access the application:**
   - API: http://localhost:8000
   - API Docs: http://localhost:8000/docs
   - PostgreSQL: localhost:5432
   - Redis: localhost:6379

## All Commands Reference

### Setup Commands

```bash
# Initial setup (creates .env from template)
make setup

# Install Poetry (one-time, if not using Docker)
curl -sSL https://install.python-poetry.org | python3 -

# Install dependencies locally (optional)
make install
cd backend && poetry install
cd ml && poetry install

# Verify setup
./verify_setup.sh
```

### Development Commands (Make)

```bash
# Show all available commands
make help

# Start development environment (Docker)
make dev
docker-compose up --build

# Stop all services
make down
docker-compose down

# View logs from all services
make logs
docker-compose logs -f

# View logs for specific service
docker-compose logs -f backend
docker-compose logs -f postgres

# Clean up containers and volumes (WARNING: deletes data)
make clean
```

### Database & Migration Commands

```bash
# Create new migration
make migrate-create msg="Description of changes"
docker-compose exec backend poetry run alembic revision --autogenerate -m "Description"

# Apply migrations
make migrate-up
docker-compose exec backend poetry run alembic upgrade head

# Rollback one migration
make migrate-down
docker-compose exec backend poetry run alembic downgrade -1

# Show current migration version
docker-compose exec backend poetry run alembic current

# Show migration history
docker-compose exec backend poetry run alembic history

# Connect to PostgreSQL
docker-compose exec postgres psql -U mitra -d mitra

# PostgreSQL commands (inside psql)
\dt                 # List all tables
\d users            # Describe users table
SELECT * FROM users;
\q                  # Quit
```

### Redis Commands

```bash
# Connect to Redis
docker-compose exec redis redis-cli

# Redis commands (inside redis-cli)
KEYS *              # List all keys
GET key_name        # Get value
SET key value       # Set value
DEL key             # Delete key
FLUSHDB             # Clear all data (dev only!)
```

### Code Quality Commands

```bash
# Format code with Black
make format
cd backend && poetry run black app/

# Lint code with Ruff
make lint
cd backend && poetry run ruff check app/

# Lint with auto-fix
cd backend && poetry run ruff check --fix app/

# Type check with mypy
make typecheck
cd backend && poetry run mypy app/

# Run all quality checks
make format lint typecheck
```

### Testing Commands

```bash
# Run all tests
make test
docker-compose exec backend poetry run pytest

# Run tests with coverage
docker-compose exec backend poetry run pytest --cov=app

# Run specific test file
docker-compose exec backend poetry run pytest tests/test_auth.py

# Run tests matching pattern
docker-compose exec backend poetry run pytest -k "test_user"

# Run tests with verbose output
docker-compose exec backend poetry run pytest -v

# Run tests locally (without Docker)
cd backend && poetry run pytest
```

### Poetry Commands

```bash
# Install all dependencies
cd backend
poetry install

# Add a production dependency
poetry add package-name
poetry add "fastapi>=0.109.0"

# Add a development dependency
poetry add --group dev pytest

# Remove a dependency
poetry remove package-name

# Update all dependencies
poetry update

# Update specific package
poetry update fastapi

# Show installed packages
poetry show
poetry show --tree

# Show outdated packages
poetry show --outdated

# Activate virtual environment
poetry shell

# Run command in virtual environment
poetry run python script.py
poetry run uvicorn app.main:app --reload

# Export to requirements.txt (if needed)
poetry export -f requirements.txt > requirements.txt

# Check for security issues
poetry check

# Update Poetry itself
poetry self update

# Clear cache
poetry cache clear pypi --all
```

### Docker Commands

```bash
# Build and start all services
docker-compose up --build

# Start in detached mode (background)
docker-compose up -d

# Stop services
docker-compose down

# Stop and remove volumes (WARNING: deletes data)
docker-compose down -v

# View running containers
docker-compose ps

# View all logs
docker-compose logs -f

# View specific service logs
docker-compose logs -f backend

# Restart a service
docker-compose restart backend

# Execute command in running container
docker-compose exec backend bash
docker-compose exec backend poetry run python

# Build without cache
docker-compose build --no-cache

# Pull latest images
docker-compose pull
```

### Container Shell Access

```bash
# Open bash shell in backend container
make shell
docker-compose exec backend bash

# Open Python shell in backend container
make backend-shell
docker-compose exec backend poetry run python

# Run one-off command in backend
docker-compose exec backend poetry run alembic current
```

### Local Development (Without Docker)

```bash
# Start PostgreSQL (Homebrew on macOS)
brew services start postgresql@15

# Start Redis
brew services start redis

# Create database
createdb mitra

# Install dependencies
cd backend
poetry install

# Run migrations
poetry run alembic upgrade head

# Start backend server
poetry run uvicorn app.main:app --reload

# Run tests
poetry run pytest

# Stop services
brew services stop postgresql@15
brew services stop redis
```

### Troubleshooting Commands

```bash
# Check what's using a port
lsof -i :5432  # PostgreSQL
lsof -i :6379  # Redis
lsof -i :8000  # Backend API

# Kill process on port
kill -9 $(lsof -ti:8000)

# Check Docker status
docker info
docker-compose ps

# Check Poetry installation
poetry --version
which poetry

# Add Poetry to PATH
export PATH="$HOME/.local/bin:$PATH"

# Rebuild containers from scratch
docker-compose down -v
docker-compose build --no-cache
docker-compose up

# View Docker volumes
docker volume ls

# Remove all unused Docker resources
docker system prune -a --volumes

# Check backend logs for errors
docker-compose logs backend | grep -i error

# Check database connection
docker-compose exec postgres pg_isready -U mitra
```

### Git Commands (Common Workflow)

```bash
# Check status
git status

# Stage changes
git add .
git add backend/app/models/

# Commit
git commit -m "Add user authentication"

# Push to remote
git push origin main

# Pull latest changes
git pull origin main

# Create new branch
git checkout -b feature/market-betting

# Switch branches
git checkout main

# View commit history
git log --oneline

# Stash changes
git stash
git stash pop
```

### Quick Reference

**Start development:**
```bash
make dev
# Wait for services to start
make migrate-create msg="Initial schema"
make migrate-up
```

**Make changes:**
```bash
# Edit code in backend/app/
make format
make lint
make test
```

**Database changes:**
```bash
# Modify models in backend/app/models/
make migrate-create msg="Add new field"
make migrate-up
```

**Access services:**
```bash
# API docs: http://localhost:8000/docs
# Backend: http://localhost:8000
# PostgreSQL: localhost:5432
# Redis: localhost:6379
```

## Tech Stack

### Backend
- **FastAPI** - Async Python web framework
- **PostgreSQL** - Main database
- **Redis** - Caching and sessions
- **SQLAlchemy** - ORM with async support
- **Alembic** - Database migrations
- **Poetry** - Dependency management

### ML
- **scikit-learn** - Collaborative filtering
- **NetworkX** - Social graph analysis
- **NumPy/Pandas** - Data processing

### Development Tools
- **Black** - Code formatting
- **Ruff** - Fast linting
- **mypy** - Static type checking
- **pytest** - Testing framework

### Frontend (Coming Soon)
- **Next.js 14** - React framework
- **TypeScript** - Type safety
- **TailwindCSS** - Styling
- **shadcn/ui** - Component library

## Project Status

✅ **Phase 1 - Foundation (Current)**
- [x] Backend project structure with Poetry
- [x] Database models and migrations
- [x] Docker development environment
- [x] Code quality tools (Black, Ruff, mypy)
- [ ] Authentication system (magic links)
- [ ] User and group management
- [ ] Market creation and betting
- [ ] Resolution system
- [ ] ML components (AMM, recommendations)
- [ ] Frontend application

## Documentation

- [QUICKSTART.md](QUICKSTART.md) - Get running in 5 minutes
- [SETUP_COMPLETE.md](SETUP_COMPLETE.md) - What's been built
- [Backend README](backend/README.md) - API development guide
- [ML README](ml/README.md) - ML components documentation

## Database Schema

Core tables:
- **users** - User accounts with balance
- **auth_tokens** - Magic link and session tokens
- **groups** - Friend groups
- **group_members** - Group membership
- **markets** - Prediction markets
- **market_state** - Current odds and shares
- **bets** - User bets
- **resolutions** - Market resolution
- **transactions** - Wallet transactions
- **user_stats** - User accuracy and stats

## Development Workflow

1. **Start services**: `make dev`
2. **Make code changes** in `backend/app/`
3. **Format code**: `make format`
4. **Lint code**: `make lint`
5. **Run tests**: `make test`
6. **Create migration**: `make migrate-create msg="Description"`
7. **Apply migration**: `make migrate-up`
8. **Test in browser**: http://localhost:8000/docs

## Why Poetry?

Poetry provides modern Python dependency management:

- ✅ **Better dependency resolution** - Handles conflicts automatically
- ✅ **Lock file** - Reproducible builds with `poetry.lock`
- ✅ **Virtual environments** - Automatic, isolated environments
- ✅ **Dev dependencies** - Separate dev/prod dependencies
- ✅ **Modern standard** - PEP 517/518 compliant
- ✅ **Easy commands** - `poetry add`, `poetry update`

## Contributing

This is a Phase 1 MVP. Focus on:
- Simple, type-safe code
- Minimal surface area
- Developer-friendly APIs
- Fast iteration
- Modern Python best practices

## Useful Commands

### Poetry
```bash
poetry add package-name        # Add dependency
poetry add --group dev pkg     # Add dev dependency
poetry update                  # Update all packages
poetry show                    # List installed packages
poetry shell                   # Activate virtual environment
```

### Docker
```bash
docker-compose up             # Start services
docker-compose down           # Stop services
docker-compose logs -f        # Follow logs
docker-compose exec backend bash  # Shell into backend
```

### Database
```bash
# Connect to PostgreSQL
docker-compose exec postgres psql -U mitra -d mitra

# Connect to Redis
docker-compose exec redis redis-cli
```

## Troubleshooting

### Poetry not found
```bash
curl -sSL https://install.python-poetry.org | python3 -
export PATH="$HOME/.local/bin:$PATH"
```

### Port conflicts
```bash
lsof -i :5432  # PostgreSQL
lsof -i :6379  # Redis
lsof -i :8000  # Backend
```

### Fresh start
```bash
make clean  # Removes all containers and volumes
make dev    # Start fresh
```

## Next Steps

After backend foundation:

**Week 1 (Days 3-7):**
- Implement magic link authentication
- User and group management APIs
- Invite system

**Week 2:**
- Market creation and betting
- LMSR automated market maker
- Wallet and transaction system

**Week 3:**
- Resolution system (manual + democratic)
- Payout calculations
- Analytics tracking

**Week 4-5:**
- Frontend application
- ML recommendations
- User insights dashboard

**Week 6:**
- Testing and deployment
- Beta launch

## Resources

- Poetry: https://python-poetry.org/docs/
- FastAPI: https://fastapi.tiangolo.com
- SQLAlchemy: https://docs.sqlalchemy.org/en/20/
- Alembic: https://alembic.sqlalchemy.org/
- Ruff: https://docs.astral.sh/ruff/

## License

Private project.

---

**Ready to code with modern Python tooling! 🚀**
