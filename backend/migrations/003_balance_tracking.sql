-- Migration: Balance Tracking
-- Description: Adds user balance tracking and transaction history for real money flow

-- Add balance columns to users table
ALTER TABLE users ADD COLUMN IF NOT EXISTS balance_usdc DECIMAL(20, 8) NOT NULL DEFAULT 0;
ALTER TABLE users ADD COLUMN IF NOT EXISTS locked_usdc DECIMAL(20, 8) NOT NULL DEFAULT 0;

-- Add balance columns per group (user can have different balances in different groups)
CREATE TABLE IF NOT EXISTS user_group_balances (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    group_id UUID NOT NULL REFERENCES friend_groups(id) ON DELETE CASCADE,
    balance_usdc DECIMAL(20, 8) NOT NULL DEFAULT 0,
    locked_usdc DECIMAL(20, 8) NOT NULL DEFAULT 0, -- Funds locked in active bets
    updated_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, group_id)
);

-- Transaction history for deposits, withdrawals, and bet outcomes
CREATE TABLE IF NOT EXISTS transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    group_id UUID REFERENCES friend_groups(id) ON DELETE SET NULL,
    event_id UUID REFERENCES events(id) ON DELETE SET NULL,
    transaction_type TEXT NOT NULL CHECK (
        transaction_type IN ('deposit', 'withdrawal', 'bet_placed', 'bet_won', 'bet_lost', 'refund')
    ),
    amount_usdc DECIMAL(20, 8) NOT NULL,
    balance_before DECIMAL(20, 8) NOT NULL,
    balance_after DECIMAL(20, 8) NOT NULL,
    solana_tx_signature TEXT,
    status TEXT NOT NULL DEFAULT 'confirmed' CHECK (
        status IN ('pending', 'confirmed', 'failed')
    ),
    description TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Settlement records for tracking payouts
CREATE TABLE IF NOT EXISTS settlements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    winning_outcome TEXT NOT NULL,
    total_pool DECIMAL(20, 8) NOT NULL,
    total_winning_shares DECIMAL(20, 8) NOT NULL,
    settled_by_wallet TEXT NOT NULL,
    solana_tx_signature TEXT,
    settled_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Individual payout records
CREATE TABLE IF NOT EXISTS payouts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    settlement_id UUID NOT NULL REFERENCES settlements(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    shares DECIMAL(20, 8) NOT NULL,
    payout_amount DECIMAL(20, 8) NOT NULL,
    claimed BOOLEAN NOT NULL DEFAULT FALSE,
    claimed_at TIMESTAMP,
    solana_tx_signature TEXT,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Indices for performance
CREATE INDEX IF NOT EXISTS idx_transactions_user_id ON transactions(user_id);
CREATE INDEX IF NOT EXISTS idx_transactions_group_id ON transactions(group_id);
CREATE INDEX IF NOT EXISTS idx_transactions_created_at ON transactions(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_user_group_balances_user_id ON user_group_balances(user_id);
CREATE INDEX IF NOT EXISTS idx_settlements_event_id ON settlements(event_id);
CREATE INDEX IF NOT EXISTS idx_payouts_user_id ON payouts(user_id);
CREATE INDEX IF NOT EXISTS idx_payouts_unclaimed ON payouts(user_id) WHERE claimed = FALSE;

-- Add winning_outcome to events table for tracking
ALTER TABLE events ADD COLUMN IF NOT EXISTS winning_outcome TEXT;

-- Comments
COMMENT ON TABLE user_group_balances IS 'Tracks user balances per friend group for treasury management';
COMMENT ON TABLE transactions IS 'Transaction history for all fund movements';
COMMENT ON TABLE settlements IS 'Records of event settlements with winning outcomes';
COMMENT ON TABLE payouts IS 'Individual user payouts from settlements';
COMMENT ON COLUMN users.balance_usdc IS 'Global USDC balance across all groups';
COMMENT ON COLUMN users.locked_usdc IS 'Total USDC locked in active bets';
COMMENT ON COLUMN user_group_balances.locked_usdc IS 'USDC locked in active bets within this group';

