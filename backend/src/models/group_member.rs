use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Member role in a friend group
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemberRole {
    Admin,
    Member,
}

impl MemberRole {
    /// Convert from database string
    pub fn from_str(s: &str) -> Result<Self, String> {
        match s.to_lowercase().as_str() {
            "admin" => Ok(MemberRole::Admin),
            "member" => Ok(MemberRole::Member),
            _ => Err(format!("Invalid role: {}", s)),
        }
    }

    /// Convert to database string
    pub fn as_str(&self) -> &'static str {
        match self {
            MemberRole::Admin => "admin",
            MemberRole::Member => "member",
        }
    }
}

impl From<String> for MemberRole {
    fn from(s: String) -> Self {
        Self::from_str(&s).unwrap_or(MemberRole::Member)
    }
}

impl From<MemberRole> for String {
    fn from(role: MemberRole) -> Self {
        role.as_str().to_string()
    }
}

/// Group Member model representing a user's membership in a friend group
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct GroupMember {
    pub group_id: Uuid,
    pub user_id: Uuid,
    pub role: String, // Stored as TEXT in DB, use MemberRole enum for type safety
    pub joined_at: NaiveDateTime,
}

impl GroupMember {
    /// Create a new GroupMember
    pub fn new(group_id: Uuid, user_id: Uuid, role: MemberRole) -> Self {
        Self {
            group_id,
            user_id,
            role: role.as_str().to_string(),
            joined_at: chrono::Utc::now().naive_utc(),
        }
    }

    /// Get the role as an enum
    pub fn role_enum(&self) -> MemberRole {
        MemberRole::from_str(&self.role).unwrap_or(MemberRole::Member)
    }

    /// Check if member is an admin
    pub fn is_admin(&self) -> bool {
        self.role_enum() == MemberRole::Admin
    }
}