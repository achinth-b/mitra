//! Mitra Backend Service
//!
//! Main entry point for the Mitra prediction market backend.
//! This service provides:
//! - gRPC API for client interactions
//! - WebSocket server for real-time updates
//! - Background tasks for merkle commitments and ML polling

// Use the library crate
use mitra_backend::config::AppConfig;
use mitra_backend::database::{create_pool, run_migrations};
use mitra_backend::error::{AppError, AppResult};
use mitra_backend::grpc_service::{self, MitraGrpcService};
use mitra_backend::repositories::*;
use mitra_backend::services::{AuditTrailService, EmergencyWithdrawalService, MlPoller, SettlementService};
use mitra_backend::solana_client::{SolanaClient, SolanaConfig};
use mitra_backend::state_manager::StateManager;
use mitra_backend::websocket::WebSocketServer;
use mitra_backend::committer::Committer;
use mitra_backend::AppState;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tonic::transport::Server;
use tracing::{error, info, warn};

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

    // Initialize Solana client first
    let solana_config = SolanaConfig::from_env();
    info!("Solana RPC: {}", solana_config.rpc_url);
    
    // Create Solana client and try to load keypair
    let solana_client = {
        let client = SolanaClient::with_config(solana_config);
        
        // Try to load keypair from environment or file
        let keypair_path = std::env::var("BACKEND_KEYPAIR_PATH").ok();
        if let Some(path) = keypair_path {
            info!("Loading backend keypair from: {}", path);
            match client.with_keypair_file(&path) {
                Ok(client_with_key) => {
                    info!("✓ Backend keypair loaded successfully");
                    client_with_key
                }
                Err(e) => {
                    warn!("Failed to load keypair from {}: {}", path, e);
                    SolanaClient::with_config(SolanaConfig::from_env())
                }
            }
        } else {
            client
        }
    };

    // Initialize application state with repositories and Solana client
    let app_state = Arc::new(AppState::new(pool.clone(), solana_client));
    info!("✓ Application state initialized with repositories");

    // Get a reference to the Solana client from app_state
    let solana_client = app_state.solana_client.clone();
    info!("✓ Solana client initialized (simulation mode for PoC)");

    // Initialize state manager
    let state_manager = Arc::new(StateManager::new(pool.clone()));
    info!("✓ State manager initialized");

    // Initialize WebSocket server
    let ws_server = Arc::new(WebSocketServer::new());
    info!("✓ WebSocket server initialized");



    // =========================================================================
    // BACKGROUND TASKS
    // =========================================================================
    info!("Starting background tasks...");

    // Initialize committer (background task for merkle root commitments)
    let committer = Committer::new(
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
    let settlement_service = Arc::new(SettlementService::new(
        app_state.event_repo.clone(),
        app_state.bet_repo.clone(),
        app_state.group_member_repo.clone(),
        app_state.balance_repo.clone(),
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

    // Use serve_with_shutdown for graceful shutdown support
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    
    // Load file descriptor for gRPC reflection
    let reflection_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(grpc_service::proto::FILE_DESCRIPTOR_SET)
        .build()
        .map_err(|e| AppError::Message(format!("Failed to create reflection service: {}", e)))?;
    
    // Initialize gRPC service with all dependencies
    let grpc_service = MitraGrpcService::new(
        app_state.clone(), 
        state_manager.clone(),
        settlement_service.clone()
    );
    info!("✓ gRPC service initialized");

    let grpc_server = Server::builder()
        .add_service(reflection_service)
        .add_service(grpc_service.into_server())
        .serve_with_shutdown(grpc_addr, async {
            shutdown_rx.await.ok();
        });

    let grpc_handle = tokio::spawn(async move {
        if let Err(e) = grpc_server.await {
            error!("gRPC server error: {}", e);
        }
    });

    // Give the server a moment to bind
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    
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
    
    // Wait for shutdown signal
    tokio::signal::ctrl_c().await.ok();
    info!("Shutdown signal received, shutting down gracefully...");
    
    // Signal gRPC server to shutdown
    let _ = shutdown_tx.send(());
    
    // Wait for gRPC server to finish
    let _ = grpc_handle.await;
    
    // Abort background tasks
    committer_handle.abort();
    ml_poller_handle.abort();
    
    // Abort WebSocket if running
    if let Some(handle) = ws_handle {
        handle.abort();
    }

    info!("Mitra backend service shutdown complete");
    Ok(())
}
