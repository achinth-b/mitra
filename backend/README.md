# Mitra Backend

Social prediction market API built with FastAPI, PostgreSQL, and Redis.

## Quick Start

### Local Development with Docker (Recommended)

1. **Start all services:**
   ```bash
   docker-compose up --build
   ```

2. **Create and apply initial migration:**
   ```bash
   docker-compose exec backend poetry run alembic revision --autogenerate -m "Initial migration"
   docker-compose exec backend poetry run alembic upgrade head
   ```

3. **Access the API:**
   - API: http://localhost:8000
   - Docs: http://localhost:8000/docs
   - PostgreSQL: localhost:5432
   - Redis: localhost:6379

### Local Development without Docker

1. **Install Poetry:**
   ```bash
   curl -sSL https://install.python-poetry.org | python3 -
   ```

2. **Install dependencies:**
   ```bash
   cd backend
   poetry install
   ```

3. **Setup environment:**
   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

4. **Run PostgreSQL and Redis:**
   ```bash
   # Using Homebrew on macOS:
   brew services start postgresql@15
   brew services start redis
   
   # Create database:
   createdb mitra
   ```

5. **Run migrations:**
   ```bash
   poetry run alembic revision --autogenerate -m "Initial migration"
   poetry run alembic upgrade head
   ```

6. **Start the server:**
   ```bash
   poetry run uvicorn app.main:app --reload
   ```

## Project Structure

```
backend/
├── app/
│   ├── main.py              # FastAPI app entry
│   ├── config.py            # Settings
│   ├── database.py          # DB connection
│   ├── redis_client.py      # Redis client
│   ├── models/              # SQLAlchemy models
│   │   ├── user.py
│   │   ├── group.py
│   │   ├── market.py
│   │   └── transaction.py
│   ├── schemas/             # Pydantic schemas (to be added)
│   ├── api/                 # API routes (to be added)
│   ├── services/            # Business logic (to be added)
│   └── utils/               # Utilities (to be added)
├── alembic/                 # Database migrations
├── pyproject.toml           # Poetry dependencies
├── Dockerfile
└── .env.example
```

## Poetry Commands

```bash
# Install dependencies
poetry install

# Add a new dependency
poetry add package-name

# Add a dev dependency
poetry add --group dev package-name

# Update dependencies
poetry update

# Show installed packages
poetry show

# Activate virtual environment
poetry shell

# Run a command in the virtual env
poetry run python script.py
```

## Database Models

- **User**: User accounts with balance and stats
- **AuthToken**: Magic link and session tokens
- **Group**: Friend groups for markets
- **GroupMember**: Group membership with roles
- **Market**: Prediction markets (binary/multi-outcome)
- **MarketState**: Current odds and share counts
- **Bet**: User bets on market outcomes
- **Resolution**: Market resolution proposals and votes
- **Transaction**: Wallet transaction history
- **UserStats**: User accuracy and betting stats

## Alembic Commands

```bash
# Create new migration
poetry run alembic revision --autogenerate -m "Description"

# Apply migrations
poetry run alembic upgrade head

# Rollback one migration
poetry run alembic downgrade -1

# Show current revision
poetry run alembic current

# Show migration history
poetry run alembic history
```

## Code Quality

```bash
# Format code with Black
poetry run black app/

# Lint with Ruff
poetry run ruff check app/

# Type check with mypy
poetry run mypy app/

# Run all checks
make format lint typecheck
```

## Testing

```bash
# Run all tests
poetry run pytest

# Run with coverage
poetry run pytest --cov=app tests/

# Run specific test file
poetry run pytest tests/test_auth.py

# Using Makefile
make test
```

## Environment Variables

See `.env.example` for all required configuration. Key variables:

- `DATABASE_URL`: PostgreSQL connection string
- `REDIS_URL`: Redis connection string
- `SECRET_KEY`: JWT signing key
- `RESEND_API_KEY`: Email service API key
- `FRONTEND_URL`: Frontend URL for CORS

## Development Tips

1. **Database changes**: Always use Alembic migrations
2. **Type hints**: Use strict typing throughout
3. **Async/await**: All database operations are async
4. **Dependency injection**: Use FastAPI's DI for db/redis clients
5. **Environment**: Use `.env` for local config, never commit secrets
6. **Poetry**: Use `poetry add` instead of pip install
7. **Virtual env**: Poetry creates isolated environments automatically

## Makefile Commands

```bash
make help           # Show all commands
make setup          # Initial setup
make install        # Install dependencies
make dev            # Start Docker environment
make migrate-create # Create migration
make migrate-up     # Apply migrations
make test           # Run tests
make format         # Format code
make lint           # Lint code
```

## Next Steps

After initial setup:

1. Implement API routes in `app/api/`
2. Add Pydantic schemas in `app/schemas/`
3. Create business logic in `app/services/`
4. Add tests in `tests/`
5. Implement authentication middleware

## Poetry vs pip

**Why Poetry?**
- ✅ Better dependency resolution
- ✅ Lock file for reproducible builds
- ✅ Automatic virtual environment management
- ✅ Modern Python packaging standard
- ✅ Easy dev vs prod dependencies
- ✅ Built-in build and publish tools

**Migration from requirements.txt:**
```bash
# Old way
pip install -r requirements.txt

# New way (Poetry handles it)
poetry install
```
