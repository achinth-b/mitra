//! Mitra Backend Service
//!
//! Main entry point for the Mitra prediction market backend.
//! This service provides:
//! - gRPC API for client interactions
//! - WebSocket server for real-time updates
//! - Background tasks for merkle commitments and ML polling

mod amm;
mod auth;
mod committer;
mod config;
mod database;
mod error;
mod grpc_service;
mod models;
mod repositories;
mod services;
mod solana_client;
mod state_manager;
mod websocket;

use config::AppConfig;
use database::{create_pool, run_migrations, Database};
use error::{AppError, AppResult};
use grpc_service::MitraGrpcService;
use repositories::*;
use services::{AuditTrailService, EmergencyWithdrawalService, MlPoller, SettlementService};
use solana_client::{SolanaClient, SolanaConfig};
use state_manager::StateManager;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::transport::Server;
use tracing::{error, info, warn};

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
    let config = AppConfig::from_env().map_err(|e| {
        eprintln!("Configuration error: {}", e);
        AppError::Config(e)
    })?;

    // Initialize tracing/logging with config
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("mitra_backend={},sqlx=warn,tonic=info", config.log_level).into()
            }),
        )
        .init();

    info!("╔══════════════════════════════════════════════════════════╗");
    info!("║           Mitra Backend Service Starting                  ║");
    info!("╚══════════════════════════════════════════════════════════╝");
    info!("Environment: {}", config.environment);
    info!("Log level: {}", config.log_level);
    info!("gRPC port: {}", config.grpc_port);
    if let Some(http_port) = config.http_port {
        info!("HTTP/WebSocket port: {}", http_port);
    }

    // =========================================================================
    // DATABASE SETUP
    // =========================================================================
    info!("Connecting to database...");

    let pool = create_pool(&config.database).await.map_err(|e| {
        error!("Failed to create database pool: {}", e);
        AppError::Database(e)
    })?;

    info!("Database connection pool created successfully");
    info!("Max connections: {}", config.database.max_connections);

    // Run migrations
    info!("Running database migrations...");
    run_migrations(&pool, None).await.map_err(|e| {
        error!("Database migration failed: {}", e);
        AppError::Database(e)
    })?;

    info!("Database migrations completed successfully");

    // =========================================================================
    // CORE SERVICES INITIALIZATION
    // =========================================================================
    info!("Initializing core services...");

    // Initialize application state with repositories
    let app_state = Arc::new(AppState::new(pool.clone()));
    info!("✓ Application state initialized with repositories");

    // Initialize Solana client
    let solana_config = SolanaConfig::from_env();
    info!("Solana RPC: {}", solana_config.rpc_url);
    
    let solana_client = Arc::new(SolanaClient::with_config(solana_config));
    
    // Try to load keypair from environment or file
    let keypair_path = std::env::var("BACKEND_KEYPAIR_PATH").ok();
    if let Some(path) = &keypair_path {
        info!("Loading backend keypair from: {}", path);
        // Note: with_keypair_file consumes self, so we'd need to restructure
        // For PoC, we'll use simulation mode
    }
    info!("✓ Solana client initialized (simulation mode for PoC)");

    // Initialize state manager
    let state_manager = Arc::new(StateManager::new(pool.clone()));
    info!("✓ State manager initialized");

    // Initialize WebSocket server
    let ws_server = Arc::new(websocket::WebSocketServer::new());
    info!("✓ WebSocket server initialized");

    // Initialize gRPC service
    let grpc_service = MitraGrpcService::new(app_state.clone(), state_manager.clone());
    info!("✓ gRPC service initialized");

    // =========================================================================
    // BACKGROUND TASKS
    // =========================================================================
    info!("Starting background tasks...");

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
    info!("✓ Committer background task started (10s interval)");

    // Initialize ML poller (queries ML service and broadcasts price updates)
    let ml_service_url =
        std::env::var("ML_SERVICE_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());

    let ml_poller = MlPoller::new(
        ml_service_url.clone(),
        app_state.event_repo.clone(),
        app_state.bet_repo.clone(),
        ws_server.clone(),
    );

    // Start ML poller in background
    let ml_poller_handle = tokio::spawn(async move {
        ml_poller.start().await;
    });
    info!("✓ ML poller background task started (polling {})", ml_service_url);

    // Initialize settlement service
    let _settlement_service = Arc::new(SettlementService::new(
        app_state.event_repo.clone(),
        app_state.bet_repo.clone(),
        app_state.group_member_repo.clone(),
        solana_client.clone(),
        ws_server.clone(),
        pool.clone(),
    ));
    info!("✓ Settlement service initialized");

    // Initialize emergency withdrawal service
    let _emergency_withdrawal = Arc::new(EmergencyWithdrawalService::new(
        app_state.bet_repo.clone(),
        state_manager.clone(),
        solana_client.clone(),
    ));
    info!("✓ Emergency withdrawal service initialized");

    // Initialize audit trail service
    let audit_log_dir =
        std::path::PathBuf::from(std::env::var("AUDIT_LOG_DIR").unwrap_or_else(|_| "./logs".to_string()));
    
    // Create logs directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&audit_log_dir) {
        warn!("Could not create audit log directory: {}", e);
    }
    
    let _audit_trail = Arc::new(AuditTrailService::new(audit_log_dir).map_err(|e| {
        error!("Failed to initialize audit trail: {}", e);
        AppError::Message(format!("Audit trail initialization failed: {}", e))
    })?);
    info!("✓ Audit trail service initialized");

    // =========================================================================
    // START SERVERS
    // =========================================================================

    // Start gRPC server
    let grpc_addr: SocketAddr = format!("0.0.0.0:{}", config.grpc_port)
        .parse()
        .map_err(|e| AppError::Config(format!("Invalid gRPC address: {}", e)))?;

    info!("Starting gRPC server on {}...", grpc_addr);

    let grpc_server = Server::builder()
        .add_service(grpc_service.into_server())
        .serve(grpc_addr);

    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = grpc_server.await {
            error!("gRPC server error: {}", e);
        }
    });

    info!("✓ gRPC server started on {}", grpc_addr);

    // Start WebSocket server (if HTTP port is configured)
    let ws_handle = if let Some(http_port) = config.http_port {
        let ws_addr: SocketAddr = format!("0.0.0.0:{}", http_port)
            .parse()
            .map_err(|e| AppError::Config(format!("Invalid WebSocket address: {}", e)))?;

        info!("Starting WebSocket server on {}...", ws_addr);

        let ws_server_clone = ws_server.clone();
        let listener = TcpListener::bind(ws_addr).await.map_err(|e| {
            AppError::Message(format!("Failed to bind WebSocket server: {}", e))
        })?;

        let handle = tokio::spawn(async move {
            loop {
                match listener.accept().await {
                    Ok((stream, addr)) => {
                        info!("New WebSocket connection from {}", addr);
                        let ws = ws_server_clone.clone();
                        tokio::spawn(async move {
                            if let Err(e) = ws.handle_connection(stream).await {
                                error!("WebSocket connection error: {}", e);
                            }
                        });
                    }
                    Err(e) => {
                        error!("WebSocket accept error: {}", e);
                    }
                }
            }
        });

        info!("✓ WebSocket server started on {}", ws_addr);
        Some(handle)
    } else {
        warn!("HTTP_PORT not configured - WebSocket server not started");
        None
    };

    // =========================================================================
    // READY
    // =========================================================================
    info!("╔══════════════════════════════════════════════════════════╗");
    info!("║           Mitra Backend Service Ready!                    ║");
    info!("╠══════════════════════════════════════════════════════════╣");
    info!("║  gRPC API:     0.0.0.0:{}                              ║", config.grpc_port);
    if let Some(http_port) = config.http_port {
        info!("║  WebSocket:    0.0.0.0:{}                              ║", http_port);
    }
    info!("║  Environment:  {}                                    ║", config.environment);
    info!("╚══════════════════════════════════════════════════════════╝");
    info!("Press Ctrl+C to shutdown gracefully");

    // =========================================================================
    // SHUTDOWN HANDLING
    // =========================================================================
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received, shutting down gracefully...");
        }
        _ = grpc_handle => {
            error!("gRPC server exited unexpectedly");
        }
        _ = committer_handle => {
            error!("Committer task exited unexpectedly");
        }
        _ = ml_poller_handle => {
            error!("ML poller task exited unexpectedly");
        }
        _ = async {
            if let Some(handle) = ws_handle {
                handle.await.ok();
            } else {
                // Never completes if WebSocket is not running
                futures::future::pending::<()>().await;
            }
        } => {
            error!("WebSocket server exited unexpectedly");
        }
    }

    info!("Mitra backend service shutdown complete");
    Ok(())
}

// Re-export for use in tests
pub use models::*;
