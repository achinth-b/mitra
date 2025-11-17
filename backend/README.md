# Mitra Backend Service

Backend service for the Mitra prediction market platform, built with Rust, PostgreSQL, and gRPC.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Database Setup](#database-setup)
- [Environment Variables](#environment-variables)
- [Connection String Format](#connection-string-format)
- [Migrations](#migrations)
- [Running the Service](#running-the-service)
- [Testing](#testing)
- [Project Structure](#project-structure)
- [Development](#development)

## Prerequisites

- **Rust** (1.70+): [Install Rust](https://www.rust-lang.org/tools/install)
- **PostgreSQL** (14+): [Install PostgreSQL](https://www.postgresql.org/download/)
- **Cargo**: Comes with Rust installation
- **SQLx CLI** (optional, for manual migrations): `cargo install sqlx-cli`

## Database Setup

### 1. Install PostgreSQL

**macOS (Homebrew):**
```bash
brew install postgresql@14
brew services start postgresql@14
```

**Ubuntu/Debian:**
```bash
sudo apt update
sudo apt install postgresql postgresql-contrib
sudo systemctl start postgresql
```

**Docker:**
```bash
docker run --name mitra-postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=mitra_db \
  -p 5432:5432 \
  -d postgres:14
```

### 2. Create Database

Connect to PostgreSQL and create the database:

```bash
# Connect to PostgreSQL
psql -U postgres

# Create database
CREATE DATABASE mitra_db;

# Create test database (optional, for testing)
CREATE DATABASE mitra_test;

# Exit psql
\q
```

### 3. Set Up User (Optional)

For production, create a dedicated user:

```sql
CREATE USER mitra_user WITH PASSWORD 'your_secure_password';
GRANT ALL PRIVILEGES ON DATABASE mitra_db TO mitra_user;
GRANT ALL PRIVILEGES ON DATABASE mitra_test TO mitra_user;
```

## Environment Variables

Create a `.env` file in the `backend/` directory with the following variables:

### Required Variables

```bash
# Database connection URL (required)
DATABASE_URL=postgresql://user:password@localhost:5432/mitra_db
```

### Optional Variables

```bash
# Database Connection Pool Settings
DATABASE_MAX_CONNECTIONS=10              # Max pool size (default: 10)
DATABASE_ACQUIRE_TIMEOUT_SECS=30        # Connection acquire timeout (default: 30)
DATABASE_IDLE_TIMEOUT_SECS=600          # Idle connection timeout in seconds (default: 600 = 10 min)
DATABASE_MAX_LIFETIME_SECS=1800         # Max connection lifetime in seconds (default: 1800 = 30 min)
DATABASE_TEST_BEFORE_ACQUIRE=true       # Test connections before use (default: true)

# Application Settings
LOG_LEVEL=info                           # Log level: trace, debug, info, warn, error (default: info)
GRPC_PORT=50051                          # gRPC server port (default: 50051)
HTTP_PORT=8080                           # HTTP server port (optional)
ENVIRONMENT=development                  # Environment: development, staging, production (default: development)
```

### Example `.env` File

```bash
# Development Environment
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/mitra_db
DATABASE_MAX_CONNECTIONS=10
LOG_LEVEL=debug
GRPC_PORT=50051
ENVIRONMENT=development
```

### Production Environment Variables

For production, set these via your deployment platform (e.g., Kubernetes secrets, AWS Secrets Manager, etc.):

```bash
DATABASE_URL=postgresql://mitra_user:secure_password@db.example.com:5432/mitra_db
DATABASE_MAX_CONNECTIONS=20
LOG_LEVEL=info
GRPC_PORT=50051
ENVIRONMENT=production
```

## Connection String Format

The `DATABASE_URL` follows the PostgreSQL connection string format:

```
postgresql://[user[:password]@][host][:port][/database][?parameter_list]
```

### Format Components

- **`postgresql://`** - Protocol prefix (required)
- **`user`** - Database username (required)
- **`password`** - Database password (required if authentication is enabled)
- **`host`** - Database hostname or IP address (default: localhost)
- **`port`** - Database port (default: 5432)
- **`database`** - Database name (required)
- **`?parameter_list`** - Optional query parameters

### Examples

**Local Development:**
```
postgresql://postgres:postgres@localhost:5432/mitra_db
```

**With SSL:**
```
postgresql://user:password@localhost:5432/mitra_db?sslmode=require
```

**Remote Server:**
```
postgresql://mitra_user:secure_pass@db.example.com:5432/mitra_db
```

**With Connection Pooling Parameters:**
```
postgresql://user:password@localhost:5432/mitra_db?connect_timeout=10&application_name=mitra_backend
```

### Common Query Parameters

- `sslmode` - SSL mode: `disable`, `require`, `verify-ca`, `verify-full`
- `connect_timeout` - Connection timeout in seconds
- `application_name` - Application name for PostgreSQL logging
- `pool_size` - Connection pool size (handled by sqlx, not URL)

## Migrations

Migrations are automatically run when the service starts. The migration files are located in `backend/migrations/`.

### Automatic Migrations

Migrations run automatically on service startup:

```bash
cargo run
```

The service will:
1. Connect to the database
2. Check which migrations have been applied
3. Run any pending migrations in order
4. Start the service

### Manual Migration Commands

If you need to run migrations manually:

**Using SQLx CLI:**
```bash
# Install SQLx CLI (if not already installed)
cargo install sqlx-cli

# Set database URL
export DATABASE_URL=postgresql://postgres:postgres@localhost:5432/mitra_db

# Run pending migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert

# Create new migration
sqlx migrate add migration_name
```

**Using SQL directly:**
```bash
# Connect to database
psql -U postgres -d mitra_db

# Run migration file
\i migrations/001_init_schema.sql
\i migrations/002_add_indices.sql
```

### Migration Files

- `001_init_schema.sql` - Creates core tables (users, friend_groups, group_members, events, bets)
- `002_add_indices.sql` - Adds indexes for query optimization

### Checking Migration Status

```bash
# Using SQLx CLI
sqlx migrate info

# Using SQL
psql -U postgres -d mitra_db -c "SELECT * FROM _sqlx_migrations ORDER BY installed_on;"
```

## Running the Service

### Development Mode

```bash
# Navigate to backend directory
cd backend

# Load environment variables and run
cargo run

# Or with explicit environment file
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/mitra_db cargo run
```

### Release Mode

```bash
# Build optimized release binary
cargo build --release

# Run release binary
./target/release/mitra-backend
```

### With Docker

```bash
# Build Docker image
docker build -t mitra-backend .

# Run container
docker run --env-file .env -p 50051:50051 mitra-backend
```

## Testing

### Prerequisites for Testing

1. Create a test database:
```bash
createdb mitra_test
```

2. Set test database URL (optional, has default):
```bash
export TEST_DATABASE_URL=postgresql://postgres:postgres@localhost:5432/mitra_test
```

### Running Tests

```bash
# Run all tests
cargo test

# Run with output
cargo test -- --nocapture

# Run specific test
cargo test test_user_create

# Run tests in a specific module
cargo test database_test

# Run tests with verbose output
cargo test -- --nocapture --test-threads=1
```

### Test Structure

- **`tests/database_test.rs`** - Integration tests for database operations
- **`tests/helpers.rs`** - Test utilities and fixtures

### Test Coverage

The test suite includes:
- Connection pool tests
- Migration tests
- CRUD operations for all repositories
- Transaction handling
- Error cases (constraints, not found, etc.)

## Project Structure

```
backend/
├── Cargo.toml              # Rust dependencies and project config
├── build.rs                 # Build script for gRPC code generation
├── README.md                # This file
├── migrations/              # Database migration files
│   ├── 001_init_schema.sql
│   └── 002_add_indices.sql
├── src/
│   ├── main.rs             # Application entry point
│   ├── config.rs           # Configuration management
│   ├── error.rs            # Error types and handling
│   ├── database/           # Database connection and pool management
│   │   ├── mod.rs
│   │   └── pool.rs
│   ├── models/             # Domain models/entities
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── friend_group.rs
│   │   ├── group_member.rs
│   │   ├── event.rs
│   │   └── bet.rs
│   └── repositories/       # Data access layer
│       ├── mod.rs
│       ├── user_repository.rs
│       ├── friend_group_repository.rs
│       ├── group_member_repository.rs
│       ├── event_repository.rs
│       └── bet_repository.rs
└── tests/                   # Integration tests
    ├── database_test.rs
    └── helpers.rs
```

## Development

### Adding a New Migration

1. Create migration file:
```bash
sqlx migrate add migration_name
```

2. Edit the generated SQL file in `migrations/`

3. Test the migration:
```bash
# Run on test database
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/mitra_test sqlx migrate run
```

### Adding a New Repository

1. Create repository file in `src/repositories/`
2. Add module export in `src/repositories/mod.rs`
3. Add tests in `tests/database_test.rs`
4. Update `AppState` in `src/main.rs` if needed

### Code Style

- Follow Rust naming conventions
- Use `cargo fmt` to format code
- Use `cargo clippy` to check for linting issues

```bash
# Format code
cargo fmt

# Check linting
cargo clippy

# Fix auto-fixable issues
cargo clippy --fix
```

## Troubleshooting

### Database Connection Issues

**Error: "connection refused"**
- Check PostgreSQL is running: `pg_isready`
- Verify host and port in `DATABASE_URL`
- Check firewall settings

**Error: "authentication failed"**
- Verify username and password in `DATABASE_URL`
- Check PostgreSQL authentication settings in `pg_hba.conf`

**Error: "database does not exist"**
- Create the database: `createdb mitra_db`
- Verify database name in `DATABASE_URL`

### Migration Issues

**Error: "migration already applied"**
- This is normal if migrations were run before
- Check migration status: `sqlx migrate info`

**Error: "migration failed"**
- Check PostgreSQL logs for detailed error
- Verify migration SQL syntax
- Ensure database user has necessary permissions

### Build Issues

**Error: "failed to resolve: use of undeclared crate"**
- Run `cargo build` to download dependencies
- Check `Cargo.toml` for correct dependencies

**Error: "cannot find macro `sqlx::query_as!`"**
- Ensure `sqlx` is in dependencies with `postgres` feature
- Run `cargo clean && cargo build`

## Additional Resources

- [SQLx Documentation](https://docs.rs/sqlx/)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
- [Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Documentation](https://tokio.rs/)

## License

[Your License Here]

