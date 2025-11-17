mod amm;
mod auth;
mod committer;
mod config;
mod database;
mod error;
mod grpc_service;
mod models;
mod repositories;
mod solana_client;
mod state_manager;
mod websocket;

use config::AppConfig;
use database::{create_pool, run_migrations, Database};
use error::{AppError, AppResult};
use repositories::*;
use solana_client::SolanaClient;
use state_manager::StateManager;
use std::sync::Arc;
use tracing::{info, error, warn};

/// Application state containing all repositories and services
pub struct AppState {
    pub database: Database,
    pub friend_group_repo: Arc<FriendGroupRepository>,
    pub user_repo: Arc<UserRepository>,
    pub group_member_repo: Arc<GroupMemberRepository>,
    pub event_repo: Arc<EventRepository>,
    pub bet_repo: Arc<BetRepository>,
}

impl AppState {
    /// Create a new AppState with initialized repositories
    pub fn new(pool: sqlx::PgPool) -> Self {
        let database = Database::new(pool.clone());
        
        Self {
            database: database.clone(),
            friend_group_repo: Arc::new(FriendGroupRepository::new(pool.clone())),
            user_repo: Arc::new(UserRepository::new(pool.clone())),
            group_member_repo: Arc::new(GroupMemberRepository::new(pool.clone())),
            event_repo: Arc::new(EventRepository::new(pool.clone())),
            bet_repo: Arc::new(BetRepository::new(pool)),
        }
    }
}

#[tokio::main]
async fn main() -> AppResult<()> {
    // Load environment variables first
    dotenv::dotenv().ok();

    // Load configuration
    let config = AppConfig::from_env()
        .map_err(|e| {
            error!("Configuration error: {}", e);
            AppError::Config(e)
        })?;

    // Initialize tracing/logging with config
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("mitra_backend={},sqlx=warn", config.log_level).into())
        )
        .init();

    info!("Starting Mitra backend service...");
    info!("Environment: {}", config.environment);
    info!("Log level: {}", config.log_level);
    info!("gRPC port: {}", config.grpc_port);
    if let Some(http_port) = config.http_port {
        info!("HTTP port: {}", http_port);
    }

    info!("Connecting to database...");
    
    // Create connection pool using config
    let pool = create_pool(&config.database).await
        .map_err(|e| {
            error!("Failed to create database pool: {}", e);
            AppError::Database(e)
        })?;
    
    info!("Database connection pool created successfully");
    info!("Max connections: {}", config.database.max_connections);

    // Run migrations
    info!("Running database migrations...");
    run_migrations(&pool, None).await
        .map_err(|e| {
            error!("Database migration failed: {}", e);
            AppError::Database(e)
        })?;
    
    info!("Database migrations completed successfully");

    // Initialize application state with repositories
    let app_state = Arc::new(AppState::new(pool.clone()));
    info!("Application state initialized with repositories");

    // Initialize Solana client
    let solana_rpc_url = std::env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.devnet.solana.com".to_string());
    let solana_client = Arc::new(SolanaClient::new(solana_rpc_url));
    info!("Solana client initialized");

    // Initialize state manager
    let state_manager = Arc::new(StateManager::new(pool.clone()));
    info!("State manager initialized");

    // Initialize WebSocket server
    let ws_server = Arc::new(websocket::WebSocketServer::new());
    info!("WebSocket server initialized");

    // Initialize gRPC service
    let grpc_service = Arc::new(grpc_service::MitraService::new(
        app_state.clone(),
        state_manager.clone(),
    ));
    info!("gRPC service initialized");

    // Initialize committer (background task for merkle root commitments)
    let committer = committer::Committer::new(
        state_manager.clone(),
        app_state.event_repo.clone(),
        solana_client.clone(),
        pool.clone(),
    );
    
    // Start committer in background
    let committer_handle = tokio::spawn(async move {
        committer.start().await;
    });
    info!("Committer background task started");

    // TODO: Start gRPC server
    // TODO: Start WebSocket server on HTTP port
    // For MVP, these are placeholders - full implementation requires:
    // 1. Generated gRPC code from proto files
    // 2. HTTP server setup for WebSocket upgrade
    
    warn!("gRPC and WebSocket servers not yet fully implemented - see TODOs");
    warn!("Service is running but not accepting connections yet");
    
    info!("Mitra backend service is ready!");
    info!("Press Ctrl+C to shutdown gracefully");

    // Wait for shutdown signal
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received, shutting down gracefully...");
        }
        _ = committer_handle => {
            error!("Committer task exited unexpectedly");
        }
    }

    Ok(())
}