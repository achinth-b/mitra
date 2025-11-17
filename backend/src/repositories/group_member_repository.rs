use crate::models::{GroupMember, MemberRole};
use sqlx::{PgPool, Result as SqlxResult};
use uuid::Uuid;

/// Repository for group member data access
pub struct GroupMemberRepository {
    pool: PgPool,
}

impl GroupMemberRepository {
    /// Create a new GroupMemberRepository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Add a member to a group
    pub async fn add_member(
        &self,
        group_id: Uuid,
        user_id: Uuid,
        role: MemberRole,
    ) -> SqlxResult<GroupMember> {
        let role_str = role.as_str();
        sqlx::query_as!(
            GroupMember,
            r#"
            INSERT INTO group_members (group_id, user_id, role)
            VALUES ($1, $2, $3)
            ON CONFLICT (group_id, user_id) DO UPDATE
            SET role = EXCLUDED.role
            RETURNING group_id, user_id, role, joined_at
            "#,
            group_id,
            user_id,
            role_str
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Remove a member from a group
    pub async fn remove_member(&self, group_id: Uuid, user_id: Uuid) -> SqlxResult<bool> {
        let rows_affected = sqlx::query!(
            r#"
            DELETE FROM group_members
            WHERE group_id = $1 AND user_id = $2
            "#,
            group_id,
            user_id
        )
        .execute(&self.pool)
        .await?
        .rows_affected();

        Ok(rows_affected > 0)
    }

    /// Find all members of a group
    pub async fn find_by_group(&self, group_id: Uuid) -> SqlxResult<Vec<GroupMember>> {
        sqlx::query_as!(
            GroupMember,
            r#"
            SELECT group_id, user_id, role, joined_at
            FROM group_members
            WHERE group_id = $1
            ORDER BY joined_at ASC
            "#,
            group_id
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Find all groups for a user
    pub async fn find_by_user(&self, user_id: Uuid) -> SqlxResult<Vec<GroupMember>> {
        sqlx::query_as!(
            GroupMember,
            r#"
            SELECT group_id, user_id, role, joined_at
            FROM group_members
            WHERE user_id = $1
            ORDER BY joined_at DESC
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await
    }

    /// Get the role of a member in a group
    pub async fn find_role(
        &self,
        group_id: Uuid,
        user_id: Uuid,
    ) -> SqlxResult<Option<MemberRole>> {
        let result = sqlx::query!(
            r#"
            SELECT role
            FROM group_members
            WHERE group_id = $1 AND user_id = $2
            "#,
            group_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.and_then(|r| MemberRole::from_str(&r.role).ok()))
    }

    /// Check if a user is a member of a group
    pub async fn is_member(&self, group_id: Uuid, user_id: Uuid) -> SqlxResult<bool> {
        let result = sqlx::query!(
            r#"
            SELECT 1
            FROM group_members
            WHERE group_id = $1 AND user_id = $2
            LIMIT 1
            "#,
            group_id,
            user_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.is_some())
    }

    /// Update a member's role
    pub async fn update_role(
        &self,
        group_id: Uuid,
        user_id: Uuid,
        role: MemberRole,
    ) -> SqlxResult<GroupMember> {
        let role_str = role.as_str();
        sqlx::query_as!(
            GroupMember,
            r#"
            UPDATE group_members
            SET role = $3
            WHERE group_id = $1 AND user_id = $2
            RETURNING group_id, user_id, role, joined_at
            "#,
            group_id,
            user_id,
            role_str
        )
        .fetch_one(&self.pool)
        .await
    }

    /// Get member count for a group
    pub async fn count_by_group(&self, group_id: Uuid) -> SqlxResult<i64> {
        let result = sqlx::query!(
            r#"
            SELECT COUNT(*) as count
            FROM group_members
            WHERE group_id = $1
            "#,
            group_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(result.count.unwrap_or(0))
    }
}

