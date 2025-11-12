# Poetry Migration Complete ✅

Successfully migrated from `requirements.txt` to Poetry for dependency management.

## What Changed

### Files Added
- ✅ `backend/pyproject.toml` - Backend dependencies and configuration
- ✅ `ml/pyproject.toml` - ML dependencies and configuration

### Files Removed
- ❌ `backend/requirements.txt` - Replaced by pyproject.toml
- ❌ `ml/requirements.txt` - Replaced by pyproject.toml

### Files Updated
- 🔄 `backend/Dockerfile` - Now uses Poetry
- 🔄 `docker-compose.yml` - Updated command to use Poetry
- 🔄 `Makefile` - Added Poetry commands
- 🔄 `backend/README.md` - Updated with Poetry instructions
- 🔄 `QUICKSTART.md` - Updated setup guide
- 🔄 `README.md` - Added Poetry info
- 🔄 `.gitignore` - Added Poetry-specific ignores
- 🔄 `verify_setup.sh` - Added Poetry check

## Benefits of Poetry

### 1. Better Dependency Resolution
```bash
# Old way - manual conflict resolution
pip install fastapi==0.109.0
pip install sqlalchemy==2.0.25
# Conflicts? You figure it out...

# New way - automatic resolution
poetry add fastapi sqlalchemy
# Poetry handles all conflicts
```

### 2. Lock File for Reproducibility
```bash
# poetry.lock ensures everyone uses same versions
poetry install  # Installs exact versions from lock file
```

### 3. Automatic Virtual Environment
```bash
# Old way
python -m venv venv
source venv/bin/activate
pip install -r requirements.txt

# New way
poetry install  # Creates venv automatically
poetry shell    # Activates it
```

### 4. Separate Dev Dependencies
```toml
[tool.poetry.dependencies]
fastapi = "^0.109.0"  # Production

[tool.poetry.group.dev.dependencies]
pytest = "^7.4.4"  # Development only
```

### 5. Modern Tooling Integration
```toml
[tool.black]
line-length = 100

[tool.ruff]
select = ["E", "W", "F"]

[tool.mypy]
strict = true
```

## New Workflow

### Installation

**First time setup:**
```bash
# Install Poetry
curl -sSL https://install.python-poetry.org | python3 -

# Install dependencies
cd backend
poetry install
```

**Docker (no local Poetry needed):**
```bash
docker-compose up --build
# Poetry is installed in container
```

### Adding Dependencies

**Production dependency:**
```bash
cd backend
poetry add package-name

# Or with version
poetry add "fastapi>=0.109.0"
```

**Development dependency:**
```bash
poetry add --group dev pytest
```

**Remove dependency:**
```bash
poetry remove package-name
```

### Running Commands

**With Poetry:**
```bash
poetry run uvicorn app.main:app --reload
poetry run alembic upgrade head
poetry run pytest
poetry run black app/
```

**Or activate shell:**
```bash
poetry shell  # Activates venv
uvicorn app.main:app --reload
alembic upgrade head
pytest
```

**In Docker:**
```bash
# Commands automatically use Poetry
docker-compose exec backend poetry run pytest
```

### Updating Dependencies

**Update all:**
```bash
poetry update
```

**Update specific package:**
```bash
poetry update fastapi
```

**Show outdated:**
```bash
poetry show --outdated
```

## Configuration Files

### backend/pyproject.toml

```toml
[tool.poetry]
name = "mitra-backend"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"
fastapi = "^0.109.0"
# ... other deps

[tool.poetry.group.dev.dependencies]
pytest = "^7.4.4"
black = "^24.1.1"
ruff = "^0.1.14"

[tool.black]
line-length = 100

[tool.ruff]
line-length = 100
```

### ml/pyproject.toml

```toml
[tool.poetry]
name = "mitra-ml"
version = "0.1.0"

[tool.poetry.dependencies]
python = "^3.11"
numpy = "^1.26.3"
scikit-learn = "^1.4.0"
# ... other deps
```

## Docker Integration

### Dockerfile Changes

**Before:**
```dockerfile
COPY requirements.txt .
RUN pip install -r requirements.txt
```

**After:**
```dockerfile
# Install Poetry
RUN curl -sSL https://install.python-poetry.org | python3 -

# Install dependencies
COPY pyproject.toml poetry.lock* ./
RUN poetry install --no-root

# Install project
COPY . .
RUN poetry install
```

### docker-compose.yml Changes

**Before:**
```yaml
command: uvicorn app.main:app --host 0.0.0.0 --reload
```

**After:**
```yaml
command: poetry run uvicorn app.main:app --host 0.0.0.0 --reload
```

## Makefile Commands

New/updated commands:

```bash
make install        # Install deps with Poetry locally
make format         # Format code with Black
make lint           # Lint with Ruff
make typecheck      # Type check with mypy
make migrate-create # Uses Poetry
make migrate-up     # Uses Poetry
```

## Common Tasks

### Check what's installed
```bash
poetry show
poetry show --tree  # With dependencies
```

### Export to requirements.txt (if needed)
```bash
poetry export -f requirements.txt > requirements.txt
poetry export -f requirements.txt --without-hashes > requirements.txt
```

### Update Poetry itself
```bash
poetry self update
```

### Clear cache
```bash
poetry cache clear pypi --all
```

### Use specific Python version
```bash
poetry env use python3.11
poetry env use /usr/local/bin/python3.11
```

### Check for security vulnerabilities
```bash
poetry check
```

## Troubleshooting

### Poetry command not found

**Solution:**
```bash
# Install Poetry
curl -sSL https://install.python-poetry.org | python3 -

# Add to PATH
export PATH="$HOME/.local/bin:$PATH"

# Add to shell config (~/.zshrc or ~/.bashrc)
echo 'export PATH="$HOME/.local/bin:$PATH"' >> ~/.zshrc
```

### Lock file conflicts

**Solution:**
```bash
poetry lock --no-update
poetry install
```

### Dependency resolution taking forever

**Solution:**
```bash
# Use newer Poetry resolver
poetry config experimental.new-installer true
```

### Want to use system Python

**Solution:**
```bash
poetry config virtualenvs.prefer-active-python true
```

### Multiple Python versions

**Solution:**
```bash
# List environments
poetry env list

# Remove old environments
poetry env remove python3.10

# Use specific version
poetry env use python3.11
```

## Migration Checklist

✅ Created `backend/pyproject.toml`
✅ Created `ml/pyproject.toml`
✅ Removed `backend/requirements.txt`
✅ Removed `ml/requirements.txt`
✅ Updated `Dockerfile` to use Poetry
✅ Updated `docker-compose.yml`
✅ Updated `Makefile` with Poetry commands
✅ Updated all documentation
✅ Added Poetry checks to `verify_setup.sh`
✅ Configured Black, Ruff, mypy in pyproject.toml
✅ Added dev dependencies
✅ Updated `.gitignore` for Poetry

## Next Steps

1. **First build will be slower** (Poetry installs in Docker)
2. **poetry.lock will be generated** on first `poetry install`
3. **Commit poetry.lock** to git (ensures reproducibility)
4. **Use `poetry add`** instead of editing pyproject.toml manually

## Resources

- Poetry docs: https://python-poetry.org/docs/
- Poetry CLI: https://python-poetry.org/docs/cli/
- Dependency specification: https://python-poetry.org/docs/dependency-specification/
- Configuration: https://python-poetry.org/docs/configuration/

---

**Migration complete! You now have modern Python dependency management. 🎉**

