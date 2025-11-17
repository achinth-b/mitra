use crate::models::{Event, EventStatus};
use sqlx::{PgPool, Result as SqlxResult};
use uuid::Uuid;

/// Repository for event data access
pub struct EventRepository {
    pool: PgPool,
}

impl EventRepository {
    /// Create a new EventRepository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Insert a new event
    pub async fn create(
        &self,
        group_id: Uuid,
        title: &str,
        description: Option<&str>,
        outcomes: &serde_json::Value,
        settlement_type: &str,
        resolve_by: Option<chrono::NaiveDateTime>,
    ) -> SqlxResult<Event> {
        sqlx::query_as!(
            Event,
            r#"
            INSERT INTO events (group_id, title, description, outcomes, settlement_type, resolve_by)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING 
                id, 
                group_id, 
                solana_pubkey, 
                title, 
                description, 
                outcomes as "outcomes: serde_json::Value",
                settlement_type, 
                status, 
                resolve_by, 
                created_at
            "#,
            group_id,
            title,
            description,
            outcomes,
            settlement_type,
            resolve_by
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Find an event by UUID
    pub async fn find_by_id(&self, id: Uuid) -> SqlxResult<Option<Event>> {
        sqlx::query_as!(
            Event,
            r#"
            SELECT 
                id, 
                group_id, 
                solana_pubkey, 
                title, 
                description, 
                outcomes as "outcomes: serde_json::Value",
                settlement_type, 
                status, 
                resolve_by, 
                created_at
            FROM events
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Find an event by Solana pubkey
    pub async fn find_by_solana_pubkey(&self, pubkey: &str) -> SqlxResult<Option<Event>> {
        sqlx::query_as!(
            Event,
            r#"
            SELECT 
                id, 
                group_id, 
                solana_pubkey, 
                title, 
                description, 
                outcomes as "outcomes: serde_json::Value",
                settlement_type, 
                status, 
                resolve_by, 
                created_at
            FROM events
            WHERE solana_pubkey = $1
            "#,
            pubkey
        )
        .fetch_optional(&self.pool)
        .await
    }

    /// Find all events for a group
    pub async fn find_by_group(&self, group_id: Uuid) -> SqlxResult<Vec<Event>> {
        sqlx::query_as!(
            Event,
            r#"
            SELECT 
                id, 
                group_id, 
                solana_pubkey, 
                title, 
                description, 
                outcomes as "outcomes: serde_json::Value",
                settlement_type, 
                status, 
                resolve_by, 
                created_at
            FROM events
            WHERE group_id = $1
            ORDER BY created_at DESC
            "#,
            group_id
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Update event status
    pub async fn update_status(
        &self,
        id: Uuid,
        status: EventStatus,
    ) -> SqlxResult<Event> {
        let status_str = status.as_str();
        sqlx::query_as!(
            Event,
            r#"
            UPDATE events
            SET status = $2
            WHERE id = $1
            RETURNING 
                id, 
                group_id, 
                solana_pubkey, 
                title, 
                description, 
                outcomes as "outcomes: serde_json::Value",
                settlement_type, 
                status, 
                resolve_by, 
                created_at
            "#,
            id,
            status_str
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Update Solana pubkey after on-chain creation
    pub async fn update_solana_pubkey(
        &self,
        id: Uuid,
        solana_pubkey: &str,
    ) -> SqlxResult<Event> {
        sqlx::query_as!(
            Event,
            r#"
            UPDATE events
            SET solana_pubkey = $2
            WHERE id = $1
            RETURNING 
                id, 
                group_id, 
                solana_pubkey, 
                title, 
                description, 
                outcomes as "outcomes: serde_json::Value",
                settlement_type, 
                status, 
                resolve_by, 
                created_at
            "#,
            id,
            solana_pubkey
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Find all active events
    pub async fn find_active_events(&self) -> SqlxResult<Vec<Event>> {
        sqlx::query_as!(
            Event,
            r#"
            SELECT 
                id, 
                group_id, 
                solana_pubkey, 
                title, 
                description, 
                outcomes as "outcomes: serde_json::Value",
                settlement_type, 
                status, 
                resolve_by, 
                created_at
            FROM events
            WHERE status = 'active'
            ORDER BY created_at DESC
            "#
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Find active events for a group
    pub async fn find_active_events_by_group(&self, group_id: Uuid) -> SqlxResult<Vec<Event>> {
        sqlx::query_as!(
            Event,
            r#"
            SELECT 
                id, 
                group_id, 
                solana_pubkey, 
                title, 
                description, 
                outcomes as "outcomes: serde_json::Value",
                settlement_type, 
                status, 
                resolve_by, 
                created_at
            FROM events
            WHERE group_id = $1 AND status = 'active'
            ORDER BY created_at DESC
            "#,
            group_id
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Find events that need resolution (past resolve_by deadline)
    pub async fn find_events_past_deadline(&self) -> SqlxResult<Vec<Event>> {
        sqlx::query_as!(
            Event,
            r#"
            SELECT 
                id, 
                group_id, 
                solana_pubkey, 
                title, 
                description, 
                outcomes as "outcomes: serde_json::Value",
                settlement_type, 
                status, 
                resolve_by, 
                created_at
            FROM events
            WHERE status = 'active' 
                AND resolve_by IS NOT NULL 
                AND resolve_by < NOW()
            ORDER BY resolve_by ASC
            "#
        )
        .fetch_all(&self.pool)
        .await
    }
}

