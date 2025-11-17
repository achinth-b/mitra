-- Migration: Initial Schema
-- Description: Creates core tables for Mitra prediction market platform
-- MVP-focused: Deferred price_snapshots, committed_slot, merkle_proof

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users table
-- Stores user information indexed by wallet address
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    wallet_address TEXT UNIQUE NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Friend Groups table
-- Stores friend group information with Solana on-chain pubkey
CREATE TABLE friend_groups (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    solana_pubkey TEXT UNIQUE NOT NULL,
    name TEXT NOT NULL CHECK (LENGTH(name) > 0 AND LENGTH(name) <= 50),
    admin_wallet TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Group Members table
-- Junction table linking users to friend groups with roles
CREATE TABLE group_members (
    group_id UUID NOT NULL REFERENCES friend_groups(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('admin', 'member')),
    joined_at TIMESTAMP NOT NULL DEFAULT NOW(),
    PRIMARY KEY (group_id, user_id)
);

-- Events table
-- Stores prediction market events
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    group_id UUID NOT NULL REFERENCES friend_groups(id) ON DELETE CASCADE,
    solana_pubkey TEXT UNIQUE, -- Nullable until on-chain creation
    title TEXT NOT NULL CHECK (LENGTH(title) > 0 AND LENGTH(title) <= 100),
    description TEXT CHECK (LENGTH(description) <= 500), -- Nullable for MVP
    outcomes JSONB NOT NULL CHECK (jsonb_typeof(outcomes) = 'array' AND jsonb_array_length(outcomes) >= 2),
    settlement_type TEXT NOT NULL CHECK (settlement_type IN ('manual', 'oracle', 'consensus')),
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'resolved', 'cancelled')),
    resolve_by TIMESTAMP, -- Nullable, deadline for resolution
    created_at TIMESTAMP NOT NULL DEFAULT NOW()
);

-- Bets table
-- Stores individual bets placed on events
-- Note: committed_slot and merkle_proof deferred for Phase 7 (merkle commitments)
CREATE TABLE bets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    outcome TEXT NOT NULL,
    shares DECIMAL(20, 8) NOT NULL CHECK (shares > 0),
    price DECIMAL(5, 4) NOT NULL CHECK (price >= 0.01 AND price <= 0.99),
    amount_usdc DECIMAL(20, 8) NOT NULL CHECK (amount_usdc > 0),
    timestamp TIMESTAMP NOT NULL DEFAULT NOW()
    -- committed_slot BIGINT, -- To be added in Phase 7 migration
    -- merkle_proof JSONB -- To be added in Phase 7 migration
);

-- Price Snapshots table
-- Deferred for MVP - will be added later for historical price tracking
-- CREATE TABLE price_snapshots (
--     id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
--     event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
--     outcome TEXT NOT NULL,
--     price DECIMAL(5, 4) NOT NULL CHECK (price >= 0.01 AND price <= 0.99),
--     liquidity DECIMAL(20, 8) NOT NULL CHECK (liquidity >= 0),
--     timestamp TIMESTAMP NOT NULL DEFAULT NOW()
-- );

-- Add comments for documentation
COMMENT ON TABLE users IS 'Stores user accounts indexed by Solana wallet address';
COMMENT ON TABLE friend_groups IS 'Stores friend group information with on-chain Solana pubkey';
COMMENT ON TABLE group_members IS 'Junction table for user-group relationships with roles';
COMMENT ON TABLE events IS 'Stores prediction market events with outcomes and settlement type';
COMMENT ON TABLE bets IS 'Stores individual bets placed on events (merkle commitment fields deferred)';

COMMENT ON COLUMN events.solana_pubkey IS 'On-chain event contract pubkey, nullable until created on-chain';
COMMENT ON COLUMN events.outcomes IS 'JSONB array of outcome strings (e.g., ["Yes", "No"])';
COMMENT ON COLUMN bets.shares IS 'Number of shares purchased (DECIMAL for precision)';
COMMENT ON COLUMN bets.price IS 'Price per share at time of bet (0.01 to 0.99)';
COMMENT ON COLUMN bets.amount_usdc IS 'Total USDC amount spent on bet';