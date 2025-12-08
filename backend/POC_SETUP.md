# Mitra Backend - PoC Setup Guide

This document explains how to get the Mitra backend PoC running.

## What Was Implemented

### 1. **Real SolanaClient** (`src/solana_client/anchor_client.rs`)
- Full configuration support via environment variables
- PDA derivation functions matching Solana contracts (using Keccak256)
- Transaction building for `commit_merkle_root` and `settle_event`
- Support for keypair loading from file or environment
- Fallback to simulation mode when no keypair is configured

### 2. **gRPC Service** (`src/grpc_service.rs`)
- Full implementation of the MitraService trait
- Handlers for:
  - `CreateFriendGroup`
  - `InviteMember`
  - `CreateEvent`
  - `PlaceBet`
  - `GetEventPrices`
  - `SettleEvent`
- Proper error handling with tonic Status codes
- Auth signature verification

### 3. **Server Startup** (`src/main.rs`)
- gRPC server on configurable port (default: 50051)
- WebSocket server on HTTP port for real-time updates
- Background tasks:
  - Committer (merkle root commitments every 10s)
  - ML Poller (price updates every 3s)
- Graceful shutdown handling

### 4. **Fixed Issues**
- ML Poller type conversion bug
- Various Rust ownership/borrow issues

## Prerequisites

### Required
1. **PostgreSQL** - Database for off-chain state
2. **Rust** (1.75+) - For building the backend

### Optional (for full functionality)
3. **protoc** - Protocol Buffers compiler for gRPC
   ```bash
   # macOS
   brew install protobuf
   
   # Ubuntu/Debian
   sudo apt install protobuf-compiler
   ```

4. **Solana CLI + Local Validator** - For on-chain interactions
   ```bash
   sh -c "$(curl -sSfL https://release.solana.com/v2.0.0/install)"
   ```

## Environment Variables

Create a `.env` file in the backend directory:

```bash
# Database
DATABASE_URL=postgresql://postgres:postgres@localhost/mitra

# Server
GRPC_PORT=50051
HTTP_PORT=8080
ENVIRONMENT=development
LOG_LEVEL=info

# Solana
SOLANA_RPC_URL=http://localhost:8899
# SOLANA_RPC_URL=https://api.devnet.solana.com  # For devnet

# Program IDs (from Anchor.toml)
EVENTS_PROGRAM_ID=GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F
FRIEND_GROUPS_PROGRAM_ID=A4hEysUGCcMWtuiWMCUZr8nw6mL8WDkTsKXjifTttCQJ
TREASURY_PROGRAM_ID=38uX65g1HHMyoJ7WdtqqjrTrJEjD23WxZnLai6NUnUNB

# Backend keypair (optional - for signing transactions)
# BACKEND_KEYPAIR_PATH=./backend-keypair.json

# ML Service
ML_SERVICE_URL=http://localhost:8000

# Audit
AUDIT_LOG_DIR=./logs
```

## Setup Steps

### 1. Start PostgreSQL
```bash
# Using Docker
docker run -d --name mitra-postgres \
  -p 5432:5432 \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=mitra \
  postgres:15

# Or use existing PostgreSQL and create database
createdb mitra
```

### 2. Build and Run Backend
```bash
cd backend

# If you have protoc installed:
cargo build

# If you don't have protoc (uses stub gRPC):
cargo build  # Will show warning but still builds

# Run the server
cargo run
```

### 3. (Optional) Start Local Solana Validator
```bash
cd solana
solana-test-validator

# In another terminal, deploy programs
anchor build
anchor deploy --provider.cluster localnet
```

### 4. (Optional) Start ML Service
```bash
cd math
poetry install
poetry run python main.py
```

## Testing the PoC

### Using grpcurl
```bash
# Install grpcurl
brew install grpcurl

# List services
grpcurl -plaintext localhost:50051 list

# Create a group (example)
grpcurl -plaintext -d '{
  "name": "Test Group",
  "admin_wallet": "wallet123",
  "solana_pubkey": "pubkey123",
  "signature": "sig123"
}' localhost:50051 mitra.MitraService/CreateFriendGroup
```

### Using WebSocket
```javascript
const ws = new WebSocket('ws://localhost:8080');

ws.onopen = () => {
  // Subscribe to event updates
  ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'event:some-event-id'
  }));
};

ws.onmessage = (event) => {
  console.log('Received:', JSON.parse(event.data));
};
```

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────┐
│                    CLIENT APPLICATIONS                       │
└────────────────────┬────────────────────────────────────────┘
                     │ gRPC (50051) / WebSocket (8080)
                     │
┌────────────────────▼────────────────────────────────────────┐
│              RUST BACKEND SERVICE                            │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  gRPC Service                                       │    │
│  │  - CreateGroup, CreateEvent, PlaceBet, etc.         │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  WebSocket Server                                   │    │
│  │  - Real-time price updates                          │    │
│  │  - Event notifications                              │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  Background Tasks                                   │    │
│  │  - Committer (merkle roots → Solana)                │    │
│  │  - ML Poller (price predictions)                    │    │
│  └─────────────────────────────────────────────────────┘    │
│  ┌─────────────────────────────────────────────────────┐    │
│  │  SolanaClient                                       │    │
│  │  - PDA derivation                                   │    │
│  │  - Transaction building                             │    │
│  │  - On-chain state queries                           │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
                     │
                     │ RPC / WebSocket
                     │
┌────────────────────▼────────────────────────────────────────┐
│                    SOLANA BLOCKCHAIN                         │
│  - events.so, friend_groups.so, treasury.so                 │
└─────────────────────────────────────────────────────────────┘
```

## Known Limitations (PoC)

1. **gRPC Stub Mode**: Without protoc, the gRPC server returns 501 Not Implemented. Install protoc for full gRPC support.

2. **Signature Verification**: Currently a placeholder. Full ed25519 verification would require proper message formatting.

3. **Solana Transactions**: Without a backend keypair, transactions are simulated. Configure `BACKEND_KEYPAIR_PATH` for real transactions.

4. **sqlx Query Verification**: The sqlx macros require a database connection at compile time. Run with a running database or use `cargo sqlx prepare`.

## Next Steps

1. Install protoc for full gRPC support
2. Set up a PostgreSQL database
3. Deploy Solana programs to localnet/devnet
4. Configure backend keypair for on-chain operations
5. Run end-to-end tests

