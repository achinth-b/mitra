#!/bin/bash

# Script to initialize database with migrations

set -e

echo "Applying database migrations..."

# Apply migrations
poetry run alembic upgrade head

echo "✓ Database initialized successfully!"
