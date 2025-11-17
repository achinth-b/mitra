-- Migration: Add Indexes
-- Description: Creates indexes for common query patterns to optimize performance

-- Indexes on bets table (most frequently queried)
CREATE INDEX idx_bets_event_id ON bets(event_id);
CREATE INDEX idx_bets_user_id ON bets(user_id);
CREATE INDEX idx_bets_timestamp ON bets(timestamp DESC); -- DESC for recent-first queries
CREATE INDEX idx_bets_event_user ON bets(event_id, user_id); -- Composite for user bets per event

-- Indexes on group_members table
CREATE INDEX idx_group_members_user_id ON group_members(user_id);
CREATE INDEX idx_group_members_group_id ON group_members(group_id);

-- Indexes on events table
CREATE INDEX idx_events_group_id ON events(group_id);
CREATE INDEX idx_events_status ON events(status) WHERE status = 'active'; -- Partial index for active events
CREATE INDEX idx_events_resolve_by ON events(resolve_by) WHERE status = 'active'; -- For deadline queries

-- Indexes on friend_groups table
-- solana_pubkey already has UNIQUE constraint (creates index automatically)
-- Adding explicit index for clarity and potential composite queries
CREATE INDEX idx_friend_groups_admin_wallet ON friend_groups(admin_wallet);

-- Indexes on users table
-- wallet_address already has UNIQUE constraint (creates index automatically)

-- GIN index for JSONB outcomes queries (if needed for searching)
-- CREATE INDEX idx_events_outcomes_gin ON events USING GIN (outcomes); -- Uncomment if needed

-- Comments for documentation
COMMENT ON INDEX idx_bets_event_id IS 'Fast lookup of all bets for an event';
COMMENT ON INDEX idx_bets_user_id IS 'Fast lookup of all bets by a user';
COMMENT ON INDEX idx_bets_timestamp IS 'Fast chronological ordering of bets';
COMMENT ON INDEX idx_group_members_user_id IS 'Fast lookup of all groups a user belongs to';
COMMENT ON INDEX idx_events_status IS 'Partial index for active events only';