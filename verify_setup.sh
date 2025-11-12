#!/bin/bash

# Verification script for Mitra setup

echo "🔍 Verifying Mitra Setup..."
echo ""

# Check if Poetry is installed
if ! command -v poetry &> /dev/null; then
    echo "⚠️  Poetry not found. Install from: https://python-poetry.org/docs/#installation"
    echo "   curl -sSL https://install.python-poetry.org | python3 -"
else
    echo "✅ Poetry is installed ($(poetry --version))"
fi

# Check if Docker is running
if ! docker info > /dev/null 2>&1; then
    echo "❌ Docker is not running. Please start Docker first."
    exit 1
fi
echo "✅ Docker is running"

# Check if docker-compose exists
if ! command -v docker-compose &> /dev/null; then
    echo "❌ docker-compose not found. Please install Docker Compose."
    exit 1
fi
echo "✅ docker-compose found"

# Check if .env exists
if [ ! -f "backend/.env" ]; then
    echo "⚠️  backend/.env not found"
    echo "   Run: cp backend/.env.example backend/.env"
else
    echo "✅ backend/.env exists"
fi

# Check if pyproject.toml exists
if [ ! -f "backend/pyproject.toml" ]; then
    echo "❌ backend/pyproject.toml not found"
else
    echo "✅ backend/pyproject.toml exists"
fi

# Check directory structure
echo ""
echo "📁 Checking directory structure..."

required_files=(
    "backend/app/main.py"
    "backend/app/config.py"
    "backend/app/database.py"
    "backend/app/models/__init__.py"
    "backend/pyproject.toml"
    "backend/Dockerfile"
    "docker-compose.yml"
    "ml/pyproject.toml"
)

for file in "${required_files[@]}"; do
    if [ -f "$file" ]; then
        echo "  ✅ $file"
    else
        echo "  ❌ $file (missing)"
    fi
done

echo ""
echo "📦 Checking Python syntax..."

if command -v python3 &> /dev/null; then
    python3 -m py_compile backend/app/main.py 2>/dev/null
    if [ $? -eq 0 ]; then
        echo "✅ Python syntax is valid"
    else
        echo "⚠️  Python syntax check failed (this might be ok if you don't have Python 3.11+)"
    fi
else
    echo "⚠️  Python not found locally (will use Docker)"
fi

echo ""
echo "🎯 Next Steps:"
echo ""
echo "1. Install Poetry (if not already):"
echo "   curl -sSL https://install.python-poetry.org | python3 -"
echo ""
echo "2. Ensure backend/.env is configured:"
echo "   - Set SECRET_KEY (generate with: openssl rand -hex 32)"
echo "   - Set RESEND_API_KEY (get from https://resend.com)"
echo ""
echo "3. (Optional) Install dependencies locally:"
echo "   make install"
echo ""
echo "4. Start services with Docker:"
echo "   make dev"
echo ""
echo "5. Create database migration (in another terminal):"
echo "   make migrate-create msg=\"Initial schema\""
echo "   make migrate-up"
echo ""
echo "6. Access API:"
echo "   http://localhost:8000/docs"
echo ""
echo "✨ Setup verification complete!"
