//! Mitra Backend Library
//!
//! This module exposes the backend components for use by tests and other consumers.

pub mod amm;
pub mod auth;
pub mod committer;
pub mod config;
pub mod database;
pub mod error;
pub mod grpc_service;
pub mod models;
pub mod repositories;
pub mod services;
pub mod solana_client;
pub mod state_manager;
pub mod websocket;

// Re-export commonly used types
pub use config::AppConfig;
pub use error::{AppError, AppResult};

use database::Database;
use repositories::*;
use solana_client::SolanaClient;
use std::sync::Arc;

/// Application state containing all repositories and services
pub struct AppState {
    pub database: Database,
    pub friend_group_repo: Arc<FriendGroupRepository>,
    pub user_repo: Arc<UserRepository>,
    pub group_member_repo: Arc<GroupMemberRepository>,
    pub event_repo: Arc<EventRepository>,
    pub bet_repo: Arc<BetRepository>,
    pub balance_repo: Arc<BalanceRepository>,
    pub solana_client: Arc<SolanaClient>,
}

impl AppState {
    /// Create a new AppState with initialized repositories
    pub fn new(pool: sqlx::PgPool, solana_client: SolanaClient) -> Self {
        let database = Database::new(pool.clone());

        Self {
            database: database.clone(),
            friend_group_repo: Arc::new(FriendGroupRepository::new(pool.clone())),
            user_repo: Arc::new(UserRepository::new(pool.clone())),
            group_member_repo: Arc::new(GroupMemberRepository::new(pool.clone())),
            event_repo: Arc::new(EventRepository::new(pool.clone())),
            bet_repo: Arc::new(BetRepository::new(pool.clone())),
            balance_repo: Arc::new(BalanceRepository::new(pool)),
            solana_client: Arc::new(solana_client),
        }
    }
}

