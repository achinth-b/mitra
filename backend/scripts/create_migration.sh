#!/bin/bash

# Script to create initial database migration

set -e

echo "Creating initial database migration..."

# Generate migration
poetry run alembic revision --autogenerate -m "Initial schema"

echo "✓ Migration created successfully!"
echo ""
echo "To apply the migration, run:"
echo "  poetry run alembic upgrade head"
