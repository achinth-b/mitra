# Solana Client Implementation Guide

## Current Status

The `SolanaClient` currently has **placeholder implementations** that need to be completed with actual Anchor client code.

## What Needs to Be Done

### 1. Complete `commit_merkle_root` Implementation

**Current** (Placeholder):
```rust
pub async fn commit_merkle_root(&self, event_pubkey: &str, merkle_root: &[u8]) -> AppResult<String> {
    Ok("placeholder_tx_signature".to_string())
}
```

**Should Be**:
```rust
use anchor_client::{Client, Program, Cluster};
use anchor_client::solana_sdk::{
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signer},
};
use std::fs;

pub async fn commit_merkle_root(&self, event_pubkey: &str, merkle_root: &[u8]) -> AppResult<String> {
    // 1. Load backend keypair
    let keypair = self.load_backend_keypair()?;
    
    // 2. Create Anchor client
    let client = Client::new_with_options(
        Cluster::Devnet,  // or Mainnet
        keypair,
        CommitmentConfig::confirmed(),
    );
    
    // 3. Load program IDL
    let program_id = self.program_id.ok_or_else(|| {
        AppError::Validation("Program ID not set".to_string())
    })?;
    
    let idl = self.load_idl()?;
    let program = client.program(program_id);
    
    // 4. Derive PDAs
    let event_pubkey = Pubkey::from_str(event_pubkey)?;
    let (event_state_pda, _bump) = Pubkey::find_program_address(
        &[b"event_state", event_pubkey.as_ref()],
        &program_id,
    );
    
    let (backend_authority_pda, _bump) = Pubkey::find_program_address(
        &[b"backend_authority"],
        &program_id,
    );
    
    // 5. Build and send transaction
    let tx = program
        .request()
        .accounts(events::accounts::CommitState {
            event_state: event_state_pda,
            backend_authority: backend_authority_pda,
            system_program: anchor_client::solana_sdk::system_program::ID,
        })
        .args(events::instruction::CommitState {
            merkle_root: *merkle_root,
        })
        .send()?;
    
    Ok(tx.to_string())
}
```

### 2. Complete `settle_event` Implementation

Similar pattern - load program, build instruction, send transaction.

### 3. Complete `get_current_slot` Implementation

```rust
use solana_client::rpc_client::RpcClient;

pub async fn get_current_slot(&self) -> AppResult<u64> {
    let client = RpcClient::new(self.rpc_url.clone());
    let slot = client.get_slot()?;
    Ok(slot)
}
```

## Required Setup

### 1. Generate Backend Keypair

```bash
solana-keygen new -o backend-keypair.json
```

### 2. Fund Backend Account (Devnet)

```bash
solana airdrop 2 $(solana-keygen pubkey backend-keypair.json) --url devnet
```

### 3. Load IDL Files

After building Anchor programs, IDL files are in `solana/target/idl/`:
- `events.json`
- `friend_groups.json`
- `treasury.json`

Copy these to `backend/idl/` or load from build directory.

### 4. Environment Variables

```bash
SOLANA_RPC_URL=https://api.devnet.solana.com
BACKEND_KEYPAIR_PATH=./backend-keypair.json
EVENTS_PROGRAM_ID=GHzeKGDCAsPzt2BMkXrS8y8azC4jDYec2SNuwd4tmZ9F
```

## Testing

### Local Testing

1. Start local validator:
```bash
solana-test-validator
```

2. Deploy programs:
```bash
cd solana
anchor build
anchor deploy
```

3. Update RPC URL:
```bash
export SOLANA_RPC_URL=http://localhost:8899
```

### Integration Testing

Create integration tests that:
1. Deploy programs to local validator
2. Test actual transactions
3. Verify account state changes

## Resources

- [Anchor Documentation](https://www.anchor-lang.com/)
- [Solana Cookbook](https://solanacookbook.com/)
- [Anchor Client Examples](https://github.com/coral-xyz/anchor/tree/master/client)

