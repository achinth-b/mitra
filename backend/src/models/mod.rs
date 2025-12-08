//! Domain models for the Mitra backend.
//!
//! This module contains all database-backed models representing
//! the core entities of the prediction market platform.

pub mod bet;
pub mod event;
pub mod friend_group;
pub mod group_member;
pub mod price_snapshot;
pub mod user;

// Re-export all models for convenient access
pub use bet::Bet;
pub use event::{Event, EventStatus, SettlementType};
pub use friend_group::FriendGroup;
pub use group_member::{GroupMember, MemberRole};
pub use user::User;

// Note: PriceSnapshot is deferred for MVP - uncomment when implementing:
// pub use price_snapshot::PriceSnapshot;
