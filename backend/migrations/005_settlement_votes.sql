-- Settlement Votes Table
-- Persists consensus voting to eliminate in-memory race conditions
CREATE TABLE IF NOT EXISTS settlement_votes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    voter_wallet TEXT NOT NULL,
    winning_outcome TEXT NOT NULL,
    created_at TIMESTAMP NOT NULL DEFAULT now(),
    
    -- Each user can only vote once per event
    UNIQUE(event_id, voter_wallet)
);

-- Index for efficient vote counting and lookup
CREATE INDEX IF NOT EXISTS idx_settlement_votes_event_id ON settlement_votes(event_id);
