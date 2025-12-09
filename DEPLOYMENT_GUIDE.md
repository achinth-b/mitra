# Mitra Deployment Guide

Complete step-by-step guide to deploy mitra.markets to Solana Devnet.

**Time Required:** ~30 minutes  
**Cost:** FREE (Devnet uses test tokens)

---

## Prerequisites Checklist

Before starting, ensure you have:

- [ ] macOS (you have this ✓)
- [ ] Docker Desktop installed ([download here](https://www.docker.com/products/docker-desktop/))
- [ ] Solana CLI installed (you have this ✓)
- [ ] Rust toolchain installed (you have this ✓)
- [ ] Anchor CLI installed (you have this ✓)

---

## Part 1: Database Setup

### Step 1.1: Start Docker Desktop

1. Open **Docker Desktop** from your Applications folder
2. Wait for it to fully start (whale icon in menu bar should be stable)
3. Verify it's running:

```bash
docker info > /dev/null 2>&1 && echo "Docker is running ✓"
```

### Step 1.2: Create PostgreSQL Container

```bash
docker run -d \
  --name mitra-postgres \
  -p 5432:5432 \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=mitra \
  postgres:15
```

### Step 1.3: Wait for Database to Start

```bash
# Wait 5 seconds for PostgreSQL to initialize
sleep 5

# Verify it's running
docker ps | grep mitra-postgres
```

You should see output like:
```
abc123  postgres:15  ...  Up 5 seconds  0.0.0.0:5432->5432/tcp  mitra-postgres
```

### Step 1.4: Run Database Migrations

```bash
cd /Users/achinth/Desktop/Code/mitra

# Create tables
docker exec -i mitra-postgres psql -U postgres -d mitra < backend/migrations/001_init_schema.sql

# Create indexes
docker exec -i mitra-postgres psql -U postgres -d mitra < backend/migrations/002_add_indices.sql
```

### Step 1.5: Verify Tables Created

```bash
docker exec mitra-postgres psql -U postgres -d mitra -c "\dt"
```

Expected output:
```
          List of relations
 Schema |     Name      | Type  |  Owner
--------+---------------+-------+----------
 public | bets          | table | postgres
 public | events        | table | postgres
 public | friend_groups | table | postgres
 public | group_members | table | postgres
 public | users         | table | postgres
(5 rows)
```

✅ **Database is ready!**

---

## Part 2: Solana Devnet Setup

### Step 2.1: Configure Solana CLI for Devnet

```bash
# Add Solana to PATH (run this in every new terminal, or add to ~/.zshrc)
export PATH="/Users/achinth/.local/share/solana/install/active_release/bin:$PATH"

# Switch to devnet
solana config set --url devnet

# Verify configuration
solana config get
```

Expected output should show:
```
RPC URL: https://api.devnet.solana.com
```

### Step 2.2: Create or Check Your Wallet

```bash
# Check if you have a wallet
ls ~/.config/solana/id.json

# If no wallet exists, create one:
solana-keygen new --outfile ~/.config/solana/id.json

# View your wallet address
solana address
```

⚠️ **Save your wallet address!** You'll need it later.

### Step 2.3: Get Free Devnet SOL

```bash
# Request 2 free SOL (you can do this multiple times)
solana airdrop 2

# Check your balance
solana balance
```

You should see: `2 SOL`

If airdrop fails (rate limited), try:
```bash
# Alternative: Use web faucet
open https://faucet.solana.com
# Paste your wallet address and request SOL
```

### Step 2.4: Deploy Solana Programs

```bash
cd /Users/achinth/Desktop/Code/mitra/solana

# Build programs (if not already built)
anchor build

# Deploy to devnet
anchor deploy --provider.cluster devnet
```

This will output something like:
```
Deploying program "friend_groups"...
Program Id: Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS

Deploying program "events"...
Program Id: GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F

Deploying program "treasury"...
Program Id: 38uX65g1HHMyoJ7WdtqqjrTrJEjD23WxZnLai6NUnUNB
```

### Step 2.5: Save Program IDs

Copy the program IDs from the output above. You'll need them for the backend configuration.

✅ **Solana programs deployed!**

---

## Part 3: Backend Configuration

### Step 3.1: Create Environment File

```bash
cat > /Users/achinth/Desktop/Code/mitra/backend/.env << 'EOF'
# Database
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/mitra

# Server Ports
GRPC_PORT=50051
WS_PORT=8080

# Environment
ENVIRONMENT=development
LOG_LEVEL=debug
RUST_LOG=info,mitra_backend=debug

# Solana Configuration (DEVNET)
SOLANA_RPC_URL=https://api.devnet.solana.com

# Program IDs (update these with your deployed program IDs)
EVENTS_PROGRAM_ID=GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F
FRIEND_GROUPS_PROGRAM_ID=A4hEysUGCcMWtuiWMCUZr8nw6mL8WDkTsKXjifTttCQJ	
TREASURY_PROGRAM_ID=38uX65g1HHMyoJ7WdtqqjrTrJEjD23WxZnLai6NUnUNB

# Backend Authority Keypair (path to your Solana wallet)
BACKEND_KEYPAIR_PATH=/Users/achinth/.config/solana/id.json

# ML Service (optional, can be disabled for PoC)
ML_SERVICE_URL=http://localhost:8000
ML_ENABLED=false
EOF
```

### Step 3.2: Update Program IDs (If Different)

If your deployed program IDs are different from the defaults, edit the `.env` file:

```bash
nano /Users/achinth/Desktop/Code/mitra/backend/.env
# Or use any text editor
```

---

## Part 4: Run the Backend

### Step 4.1: Build the Backend

```bash
cd /Users/achinth/Desktop/Code/mitra/backend

# Build in release mode (faster runtime)
SQLX_OFFLINE=true cargo build --release
```

⚠️ **Note:** First build may take 3-5 minutes.

### Step 4.2: Start the Backend Server

```bash
cd /Users/achinth/Desktop/Code/mitra/backend

# Run the server
SQLX_OFFLINE=true cargo run --release
```

You should see:
```
╔══════════════════════════════════════════════════════════╗
║           Mitra Backend Service Starting                  ║
╚══════════════════════════════════════════════════════════╝
✓ Database connected
✓ Solana client initialized
✓ gRPC server started on 0.0.0.0:50051
✓ WebSocket server started on 0.0.0.0:8080
╔══════════════════════════════════════════════════════════╗
║           Mitra Backend Service Ready!                    ║
╚══════════════════════════════════════════════════════════╝
```

✅ **Backend is running!**

---

## Part 5: Verify Everything Works

### Step 5.1: Test gRPC Connection

Open a **new terminal** and run:

```bash
# Install grpcurl if not already installed
brew install grpcurl

# List available services
grpcurl -plaintext localhost:50051 list
```

Expected output:
```
grpc.reflection.v1alpha.ServerReflection
mitra.MitraService
```

### Step 5.2: Test WebSocket Connection

```bash
# Install websocat if not already installed
brew install websocat

# Connect to WebSocket
echo '{"type":"ping"}' | websocat ws://localhost:8080
```

### Step 5.3: Test Creating a User (via gRPC)

```bash
grpcurl -plaintext -d '{
  "wallet_address": "YourWalletAddressHere"
}' localhost:50051 mitra.MitraService/CreateUser
```

---

## Part 6: Running Tests

### Step 6.1: Run Solana Contract Tests

```bash
cd /Users/achinth/Desktop/Code/mitra/solana

# Start local validator with programs
export PATH="/Users/achinth/.local/share/solana/install/active_release/bin:$PATH"
rm -rf test-ledger
solana-test-validator \
  --bpf-program A4hEysUGCcMWtuiWMCUZr8nw6mL8WDkTsKXjifTttCQJ target/deploy/friend_groups.so \
  --bpf-program GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F target/deploy/events.so \
  --bpf-program 38uX65g1HHMyoJ7WdtqqjrTrJEjD23WxZnLai6NUnUNB target/deploy/treasury.so &

# Wait for validator to start
sleep 10

# Run tests
anchor test --skip-deploy --skip-local-validator
```

### Step 6.2: Run Backend Tests

```bash
cd /Users/achinth/Desktop/Code/mitra/backend

# Run unit tests
SQLX_OFFLINE=true cargo test
```

---

## Quick Reference: Daily Commands

### Start Everything

```bash
# Terminal 1: Start Docker (if not running)
open -a Docker

# Terminal 2: Start database
docker start mitra-postgres

# Terminal 3: Start backend
cd /Users/achinth/Desktop/Code/mitra/backend
SQLX_OFFLINE=true cargo run
```

### Stop Everything

```bash
# Stop backend
# Press Ctrl+C in the backend terminal

# Stop database
docker stop mitra-postgres
```

### View Logs

```bash
# Database logs
docker logs mitra-postgres

# Backend logs are in the terminal where it's running
```

---

## Troubleshooting

### Database Connection Failed

```bash
# Check if PostgreSQL is running
docker ps | grep mitra-postgres

# If not running, start it
docker start mitra-postgres

# If container doesn't exist, recreate it (Step 1.2)
```

### Solana Airdrop Failed

```bash
# Rate limited - wait 30 seconds and try again
sleep 30 && solana airdrop 2

# Or use the web faucet
open https://faucet.solana.com
```

### Backend Build Failed

```bash
# Clean and rebuild
cd /Users/achinth/Desktop/Code/mitra/backend
cargo clean
SQLX_OFFLINE=true cargo build
```

### Port Already in Use

```bash
# Find what's using the port
lsof -i :50051  # gRPC port
lsof -i :8080   # WebSocket port

# Kill the process
kill -9 <PID>
```

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Your Machine                             │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   ┌─────────────────┐     ┌─────────────────┐                   │
│   │  Backend (Rust) │────▶│   PostgreSQL    │                   │
│   │  Port: 50051    │     │   Port: 5432    │                   │
│   │  (gRPC)         │     │   (Docker)      │                   │
│   │                 │     └─────────────────┘                   │
│   │  Port: 8080     │                                           │
│   │  (WebSocket)    │                                           │
│   └────────┬────────┘                                           │
│            │                                                     │
└────────────┼─────────────────────────────────────────────────────┘
             │
             ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Solana Devnet                               │
│                 (Free, Public Blockchain)                        │
├─────────────────────────────────────────────────────────────────┤
│  friend_groups.so  │  events.so  │  treasury.so                 │
└─────────────────────────────────────────────────────────────────┘
```

---

## What's Next?

After completing this guide, you have:

- ✅ PostgreSQL database running locally
- ✅ Solana programs deployed to devnet
- ✅ Backend server running with gRPC + WebSocket

**To build a full product, you still need:**

1. **Web App UI** - React/Next.js frontend
2. **Wallet Integration** - Phantom/Solflare connection
3. **Cloud Deployment** - Move from localhost to hosted services

---

## Useful Links

- [Solana Devnet Explorer](https://explorer.solana.com/?cluster=devnet)
- [Solana Faucet](https://faucet.solana.com)
- [Anchor Documentation](https://www.anchor-lang.com/)
- [Docker Desktop](https://www.docker.com/products/docker-desktop/)

