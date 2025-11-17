# Mitra Architecture: Backend ↔ Solana Connection

## Overview

The Mitra platform uses a **hybrid architecture** where:
- **Off-chain (Backend)**: Fast operations, complex logic, user-friendly APIs
- **On-chain (Solana)**: Trustless settlement, fund custody, immutable records

## Architecture Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    CLIENT APPLICATIONS                        │
│  (Web App, Mobile App)                                      │
└────────────────────┬────────────────────────────────────────┘
                     │ gRPC / WebSocket
                     │
┌────────────────────▼────────────────────────────────────────┐
│              RUST BACKEND SERVICE                            │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  PostgreSQL Database (Off-chain State)              │   │
│  │  - Users, Groups, Events, Bets                       │   │
│  │  - Fast queries, complex aggregations                │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────────────────────────────────────┐   │
│  │  Services:                                           │   │
│  │  - AMM (Price Calculation)                           │   │
│  │  - State Manager (Merkle Trees)                      │   │
│  │  - Settlement Service                                │   │
│  │  - ML Poller                                         │   │
│  └──────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌──────────────────────┐  ┌──────────────────────────┐  │
│  │  SolanaClient        │  │  Committer (Background)    │  │
│  │  (RPC Connection)    │  │  - Commits merkle roots   │  │
│  └──────────┬───────────┘  │  - Every 10 seconds        │  │
│             │               └──────────────────────────┘  │
└─────────────┼───────────────────────────────────────────────┘
              │ HTTP RPC / WebSocket
              │ (JSON-RPC Protocol)
              │
┌─────────────▼───────────────────────────────────────────────┐
│                    SOLANA BLOCKCHAIN                          │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  Anchor Programs (Smart Contracts)                  │    │
│  │  - events.so (Event management)                      │    │
│  │  - friend_groups.so (Group management)              │    │
│  │  - treasury.so (Fund custody)                       │    │
│  └──────────────────────────────────────────────────────┘    │
│                                                               │
│  ┌──────────────────────────────────────────────────────┐    │
│  │  On-chain Accounts (PDAs)                            │    │
│  │  - EventContract accounts                            │    │
│  │  - EventState accounts (merkle roots)                │    │
│  │  - FriendGroup accounts                              │    │
│  │  - Treasury accounts (SOL/USDC)                     │    │
│  └──────────────────────────────────────────────────────┘    │
└───────────────────────────────────────────────────────────────┘
```

## How the Connection Works

### 1. **RPC Connection (HTTP/WebSocket)**

The backend connects to Solana using **JSON-RPC** protocol:

```rust
// backend/src/solana-client/anchor_client.rs
pub struct SolanaClient {
    rpc_url: String,  // e.g., "https://api.devnet.solana.com"
    program_id: Option<Pubkey>,
}
```

**RPC Endpoints Used**:
- `getAccountInfo` - Read account data
- `sendTransaction` - Submit transactions
- `getSlot` - Get current blockchain slot
- `confirmTransaction` - Wait for confirmation

### 2. **Anchor Framework**

We use **Anchor** (Solana's framework) to interact with programs:

```rust
// Dependencies in Cargo.toml
solana-client = "1.18"      // Low-level RPC client
solana-sdk = "1.18"         // Solana SDK
anchor-client = "0.32.1"    // Anchor client (high-level)
```

**Anchor provides**:
- Type-safe program interactions
- Automatic account derivation (PDAs)
- IDL (Interface Definition Language) for type checking

## Key Connection Points

### Point 1: **Merkle Root Commitments** (Every 10 seconds)

**Flow**:
```
1. Backend generates merkle tree of all pending bets
2. Calculates merkle root hash
3. Calls Solana program: commit_state(merkle_root)
4. Program stores root in EventState account
```

**Code**:
```rust
// backend/src/committer.rs
pub async fn commit_pending_states(&self) {
    // 1. Generate merkle root
    let (merkle_root, _proofs) = self.state_manager
        .generate_merkle_root(event.id)
        .await?;
    
    // 2. Commit to Solana
    let tx_signature = self.solana_client
        .commit_merkle_root(event_pubkey, &merkle_root)
        .await?;
}
```

**On-chain**:
```rust
// solana/programs/events/src/lib.rs
pub fn commit_state(ctx: Context<CommitState>, merkle_root: [u8; 32]) -> Result<()> {
    let event_state = &mut ctx.accounts.event_state;
    event_state.last_merkle_root = merkle_root;
    event_state.last_commit_slot = Clock::get()?.slot;
    Ok(())
}
```

### Point 2: **Event Settlement**

**Flow**:
```
1. Admin/user calls backend: settle_event(event_id, outcome)
2. Backend verifies permissions
3. Backend calls Solana: settle_event(outcome)
4. Solana program distributes winnings
5. Backend updates database status
```

**Code**:
```rust
// backend/src/services/settlement.rs
pub async fn execute_settlement(&self, event: &Event, winning_outcome: &str) {
    // 1. Update database
    self.event_repo.update_status(event.id, EventStatus::Resolved).await?;
    
    // 2. Call Solana program
    let tx_signature = self.solana_client
        .settle_event(event_pubkey, winning_outcome)
        .await?;
    
    // 3. Broadcast notification
    self.ws_server.broadcast_event_settled(event.id, winning_outcome).await;
}
```

**On-chain**:
```rust
// solana/programs/events/src/lib.rs
pub fn settle_event(ctx: Context<SettleEvent>, winning_outcome: String) -> Result<()> {
    // Transfer winnings from losers to winners
    // Update event status
    // Emit events
}
```

### Point 3: **Reading On-chain State**

**Flow**:
```
1. Backend needs to verify on-chain state
2. Calls Solana RPC: getAccountInfo(event_state_pubkey)
3. Deserializes account data
4. Uses data for validation
```

**Example**: Emergency withdrawal verification
```rust
// backend/src/services/emergency_withdrawal.rs
pub async fn verify_proof_against_chain(&self, event_pubkey: &str, proof: &MerkleProof) {
    // Fetch last_merkle_root from Solana EventState account
    // Verify merkle proof against on-chain root
}
```

## Program Derived Addresses (PDAs)

**PDAs** are deterministic addresses derived from seeds. The backend calculates them the same way Solana does:

### Example: Event Account PDA

**On-chain (Solana)**:
```rust
// solana/programs/events/src/lib.rs
#[account(
    seeds = [
        b"event",
        group.key().as_ref(),
        &Keccak256::digest(title.as_bytes())[..]
    ],
    bump
)]
pub event_contract: Account<'info, EventContract>,
```

**Off-chain (Backend)**:
```rust
// Backend calculates the same PDA
use anchor_client::solana_sdk::pubkey::Pubkey;

let (event_pubkey, _bump) = Pubkey::find_program_address(
    &[
        b"event",
        group_pubkey.as_ref(),
        &sha3::Keccak256::digest(title.as_bytes())[..]
    ],
    &program_id
);
```

**Why PDAs?**:
- No keypair needed (can't sign)
- Deterministic (same inputs = same address)
- Program-controlled (only program can modify)

## Transaction Flow Example

### Complete Flow: Creating an Event

```
┌─────────────┐
│   Client    │
└──────┬──────┘
       │ 1. gRPC: CreateEvent(group_id, title, outcomes)
       │
┌──────▼──────────────────────────────────────────────┐
│  Backend Service                                    │
│  2. Validate request                                │
│  3. Store in PostgreSQL (off-chain)                │
│  4. Calculate PDA for event account                │
│  5. Build Anchor instruction                        │
│  6. Sign transaction (with backend keypair)        │
│  7. Send to Solana RPC                              │
└──────┬──────────────────────────────────────────────┘
       │ 8. HTTP POST to Solana RPC
       │    { "jsonrpc": "2.0", "method": "sendTransaction", ... }
       │
┌──────▼──────────────────────────────────────────────┐
│  Solana Network                                     │
│  9. Validator processes transaction                │
│  10. Creates EventContract account (PDA)            │
│  11. Creates EventState account (PDA)              │
│  12. Returns transaction signature                  │
└──────┬──────────────────────────────────────────────┘
       │ 13. Transaction signature
       │
┌──────▼──────────────────────────────────────────────┐
│  Backend Service                                    │
│  14. Wait for confirmation                          │
│  15. Update PostgreSQL with solana_pubkey           │
│  16. Return response to client                      │
└──────────────────────────────────────────────────────┘
```

## Key Concepts Explained

### 1. **RPC vs Direct Connection**

- **RPC (Remote Procedure Call)**: Backend calls Solana like a web API
- **No direct blockchain connection**: Uses HTTP/WebSocket to Solana RPC nodes
- **RPC Providers**: 
  - Public: `https://api.devnet.solana.com` (free, rate-limited)
  - Private: Helius, QuickNode, Alchemy (paid, faster)

### 2. **Anchor Client**

**What it does**:
- Loads program IDL (Interface Definition Language)
- Generates type-safe Rust bindings
- Handles account serialization/deserialization
- Manages transaction building

**Example**:
```rust
// Load program
let program = anchor_client::Program::new(
    program_id,
    rpc_url,
    keypair  // Backend's keypair for signing
);

// Call instruction
let tx = program
    .request()
    .accounts(accounts)
    .args(args)
    .send()?;
```

### 3. **Account Ownership**

**On-chain accounts**:
- Owned by programs (not users)
- Programs control modification
- Backend can read, but needs program's permission to write

**Backend's role**:
- Can read any account (public data)
- Can write only if program allows (via instructions)
- Uses backend keypair to sign transactions

### 4. **State Synchronization**

**Two sources of truth**:

1. **PostgreSQL (Fast, Off-chain)**:
   - All user data
   - Bet history
   - Real-time queries
   - Complex aggregations

2. **Solana (Trustless, On-chain)**:
   - Event metadata
   - Merkle roots (bet commitments)
   - Fund custody
   - Settlement records

**Synchronization**:
- Backend writes to both
- Solana is authoritative for funds
- PostgreSQL is authoritative for queries

## Implementation Details

### Current Implementation (Placeholder)

The current code has **placeholder implementations** that need to be completed:

```rust
// backend/src/solana-client/anchor_client.rs
pub async fn commit_merkle_root(&self, event_pubkey: &str, merkle_root: &[u8]) {
    // TODO: Implement actual Anchor client call
    // 1. Load Anchor IDL
    // 2. Create program client
    // 3. Build commit_state instruction
    // 4. Send transaction
    
    Ok("placeholder_tx_signature".to_string())
}
```

### What Needs to Be Implemented

1. **Load Anchor IDL**:
```rust
use anchor_client::Client;
use std::fs;

let idl = serde_json::from_str(&fs::read_to_string("target/idl/events.json")?)?;
let program_id = Pubkey::from_str("GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F")?;
```

2. **Create Program Client**:
```rust
let client = Client::new_with_options(
    Cluster::Devnet,
    keypair,  // Backend's keypair
    CommitmentConfig::confirmed(),
);
let program = client.program(program_id);
```

3. **Build and Send Transaction**:
```rust
let tx = program
    .request()
    .accounts(events::accounts::CommitState {
        event_state: event_state_pubkey,
        backend_authority: backend_authority_pda,
        system_program: System::id(),
    })
    .args(events::instruction::CommitState { merkle_root })
    .send()?;
```

## Environment Variables

```bash
# Solana RPC endpoint
SOLANA_RPC_URL=https://api.devnet.solana.com

# Backend's keypair (for signing transactions)
BACKEND_KEYPAIR_PATH=./backend-keypair.json

# Program IDs
EVENTS_PROGRAM_ID=GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F
FRIEND_GROUPS_PROGRAM_ID=...
TREASURY_PROGRAM_ID=...
```

## Summary

**How Backend Connects to Solana**:

1. **HTTP RPC**: Backend makes HTTP requests to Solana RPC nodes
2. **Anchor Client**: Uses Anchor framework for type-safe interactions
3. **Transactions**: Backend builds and signs transactions, sends via RPC
4. **Accounts**: Reads/writes on-chain accounts (PDAs) via program instructions
5. **Background Tasks**: Committer periodically commits merkle roots
6. **Settlement**: Settlement service calls Solana programs to distribute funds

**Key Takeaway**: The backend is a **middleware layer** that:
- Provides fast, user-friendly APIs (off-chain)
- Maintains trustless guarantees (on-chain)
- Synchronizes state between PostgreSQL and Solana

