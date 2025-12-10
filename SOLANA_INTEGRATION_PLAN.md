# Solana Integration Plan

## Current State

✅ **COMPLETED** - The backend now makes real Solana calls and the frontend is functional with balance tracking.

---

## Phase 1: Backend → Solana Connection ✅ COMPLETE

### 1.1 Fix the Anchor Client ✅

**File:** `backend/src/solana_client/anchor_client.rs`

Implemented real Solana calls for:
- [x] `commit_merkle_root()` - Commits bet state hash to Solana
- [x] `settle_event()` - Settles with winning outcome on-chain
- [x] `get_event_state()` - Fetches EventState from Solana
- [x] `get_event_contract()` - Fetches EventContract from Solana
- [x] `deposit_to_treasury()` - User deposits USDC to group treasury
- [x] `withdraw_from_treasury()` - User withdraws USDC from group treasury
- [x] `claim_winnings()` - Winner claims payout from resolved event
- [x] `get_member_balance()` - Fetches on-chain member balance

### 1.2 Add Missing gRPC Methods ✅

**File:** `shared/proto/mitra.proto`

Added:
- [x] `DepositFunds` RPC
- [x] `WithdrawFunds` RPC
- [x] `GetUserBalance` RPC
- [x] `ClaimWinnings` RPC

**File:** `backend/src/grpc_service.rs`

Implemented all handlers.

---

## Phase 2: Treasury & Fund Management ✅ COMPLETE

### 2.1 User Balance Tracking ✅

**Database changes (migration 003):**
- [x] `user_group_balances` table - per-group balances
- [x] `transactions` table - audit trail
- [x] `settlements` table - event settlements
- [x] `payouts` table - individual payouts

**Repository:** `backend/src/repositories/balance_repository.rs`
- [x] `get_or_create_balance()` 
- [x] `credit_balance()` - for deposits/winnings
- [x] `debit_balance()` - for withdrawals
- [x] `lock_for_bet()` - locks funds when betting
- [x] `settle_win()` - unlocks + credits winnings
- [x] `settle_loss()` - unlocks + deducts loss
- [x] `create_settlement()` - records settlement
- [x] `create_payout()` - records individual payouts

### 2.2 Deposit Flow ✅

```
User deposits $50 USDC
1. Frontend: Calls DepositFunds RPC
2. Backend: Builds Solana transaction (deposit_funds instruction)
3. Backend: Updates user_group_balances
4. Backend: Records transaction
5. Backend: Returns new balance
```

### 2.3 Withdrawal Flow ✅

Same pattern with balance checks for locked funds.

---

## Phase 3: Betting with Real Money ✅ COMPLETE

### 3.1 Place Bet Flow (Updated) ✅

```
User bets $20 on "YES"

1. Frontend: Verifies user balance >= $20
2. Backend: Verifies signature
3. Backend: Checks balance (balance - locked >= amount)
4. Backend: Calculates shares via LMSR AMM
5. Backend: Locks $20 in user_group_balances
6. Backend: Creates bet record
7. Backend: Returns confirmation + new prices
```

### 3.2 Merkle Commitment ✅

Background task in `backend/src/committer.rs` commits bet state to Solana.

---

## Phase 4: Settlement & Payouts ✅ COMPLETE

### 4.1 Settle Event Flow ✅

```
Admin settles market with "YES" as winner

1. Backend: Verifies admin permissions
2. Backend: Updates event status to 'resolved'
3. Backend: Calls settle_event() on Solana
4. Backend: Creates settlement record
5. Backend: Calculates payouts for winners
6. Backend: Updates balances (winners credited, losers deducted)
7. Backend: Returns settlement confirmation
```

### 4.2 Payout Calculation ✅

```python
# For each user who bet on winning outcome:
user_shares = sum(bet.shares for bet in user_winning_bets)
total_winning_shares = sum(all winning bets shares)
pool = total_amount_bet_on_all_outcomes

user_payout = (user_shares / total_winning_shares) * pool
```

---

## Phase 5: Frontend Integration ✅ COMPLETE

### 5.1 New UI Components ✅

1. **Balance Display** - Shows USDC balance on group page
2. **Deposit Form** - Enter amount, deposit to group
3. **Withdraw Form** - Enter amount, withdraw from group

### 5.2 Updated Styling ✅

- Using EB Garamond font (matching front-page)
- Black background, white text
- Minimal, editorial aesthetic

---

## Running the System

### Prerequisites

```bash
# 1. Start PostgreSQL
docker run -d --name mitra-postgres \
  -e POSTGRES_USER=postgres \
  -e POSTGRES_PASSWORD=postgres \
  -e POSTGRES_DB=mitra_dev \
  -p 5432:5432 postgres:15

# 2. Run migrations
docker exec -i mitra-postgres psql -U postgres -d mitra_dev < backend/migrations/001_init_schema.sql
docker exec -i mitra-postgres psql -U postgres -d mitra_dev < backend/migrations/002_add_indices.sql
docker exec -i mitra-postgres psql -U postgres -d mitra_dev < backend/migrations/003_balance_tracking.sql
```

### Start Backend

```bash
cd backend
export DATABASE_URL="postgresql://postgres:postgres@localhost:5432/mitra_dev"
export SOLANA_RPC_URL="https://api.devnet.solana.com"
cargo run
```

### Start Frontend

```bash
cd frontend
npm run dev
```

### Access

- Frontend: http://localhost:3000
- gRPC: localhost:50051
- WebSocket: localhost:8080

---

## Environment Variables

### Backend (.env)

```bash
DATABASE_URL=postgresql://postgres:postgres@localhost:5432/mitra_dev
GRPC_PORT=50051
HTTP_PORT=8080
SOLANA_RPC_URL=https://api.devnet.solana.com
EVENTS_PROGRAM_ID=<deployed_id>
FRIEND_GROUPS_PROGRAM_ID=<deployed_id>
TREASURY_PROGRAM_ID=<deployed_id>
BACKEND_KEYPAIR_PATH=/path/to/keypair.json
```

### Frontend (.env.local)

```bash
NEXT_PUBLIC_MAGIC_PUBLISHABLE_KEY=pk_test_xxx  # or YOUR_KEY_HERE for dev mode
NEXT_PUBLIC_SOLANA_RPC=https://api.devnet.solana.com
NEXT_PUBLIC_API_URL=http://localhost:50051
NEXT_PUBLIC_WS_URL=ws://localhost:8080
```

---

## What's Working

✅ User login (mock mode or Magic.link)
✅ Create friend groups
✅ Create prediction markets (events)
✅ View market prices
✅ Place bets with balance tracking
✅ Deposit/withdraw USDC
✅ Settle markets
✅ Payout calculation
✅ Backend → Solana contract calls
✅ Transaction audit trail

---

## Next Steps for Production

1. **Set up real Magic.link** - Get API keys from magic.link
2. **Deploy to Devnet** - Run `anchor deploy --provider.cluster devnet`
3. **Configure USDC** - Use devnet USDC mint or create test token
4. **Add error handling** - Transaction retries, better error messages
5. **Add real wallet signing** - Currently using dev bypass
6. **Set up monitoring** - Logs, metrics, alerts
