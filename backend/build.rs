fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo to rerun this build script if the proto file changes
    let proto_file = "../shared/proto/mitra.proto";
    println!("cargo:rerun-if-changed={}", proto_file);

    // Tell Cargo to rerun if migrations directory changes
    println!("cargo:rerun-if-changed=migrations");

    // Get OUT_DIR for file descriptor
    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let descriptor_path = out_dir.join("mitra_descriptor.bin");

    // Build gRPC code from proto file with file descriptor for reflection
    match tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(&descriptor_path)
        .compile(&[proto_file], &["../shared/proto"])
    {
        Ok(_) => {
            println!("cargo:warning=Proto files compiled successfully");
        }
        Err(e) => {
            // If protoc is not installed, print a warning but don't fail
            // This allows development without protoc installed
            println!("cargo:warning=Failed to compile proto files: {}. Using fallback.", e);
            println!("cargo:warning=To enable gRPC, install protoc: brew install protobuf");
            
            // Generate a stub module that will be used when proto compilation fails
            let out_dir = std::env::var("OUT_DIR").unwrap();
            let stub_path = std::path::Path::new(&out_dir).join("mitra.rs");
            std::fs::write(&stub_path, generate_proto_stub())?;
        }
    }

    // Note: Database migrations are handled at runtime by sqlx::migrate
    // No compile-time code generation needed for migrations
    
    Ok(())
}

/// Generate a stub proto module for when protoc is not available
fn generate_proto_stub() -> String {
    r#"
// Auto-generated stub for gRPC messages when protoc is not available
// To enable full gRPC functionality, install protoc and rebuild.

// Request/Response types
#[derive(Clone, Debug, Default)]
pub struct CreateGroupRequest {
    pub name: String,
    pub admin_wallet: String,
    pub solana_pubkey: String,
    pub signature: String,
}

#[derive(Clone, Debug, Default)]
pub struct GroupResponse {
    pub group_id: String,
    pub solana_pubkey: String,
    pub name: String,
    pub admin_wallet: String,
    pub created_at: i64,
}

#[derive(Clone, Debug, Default)]
pub struct InviteMemberRequest {
    pub group_id: String,
    pub invited_wallet: String,
    pub inviter_wallet: String,
    pub signature: String,
}

#[derive(Clone, Debug, Default)]
pub struct MemberResponse {
    pub group_id: String,
    pub user_id: String,
    pub wallet_address: String,
    pub role: String,
    pub joined_at: i64,
}

#[derive(Clone, Debug, Default)]
pub struct CreateEventRequest {
    pub group_id: String,
    pub title: String,
    pub description: String,
    pub outcomes: Vec<String>,
    pub settlement_type: String,
    pub resolve_by: i64,
    pub creator_wallet: String,
    pub arbiter_wallet: String,
    pub signature: String,
}

#[derive(Clone, Debug, Default)]
pub struct EventResponse {
    pub event_id: String,
    pub group_id: String,
    pub solana_pubkey: String,
    pub title: String,
    pub description: String,
    pub outcomes: Vec<String>,
    pub settlement_type: String,
    pub status: String,
    pub arbiter_wallet: String,
    pub resolve_by: i64,
    pub created_at: i64,
}

#[derive(Clone, Debug, Default)]
pub struct PlaceBetRequest {
    pub event_id: String,
    pub user_wallet: String,
    pub outcome: String,
    pub amount_usdc: f64,
    pub signature: String,
}

#[derive(Clone, Debug, Default)]
pub struct BetResponse {
    pub bet_id: String,
    pub shares: f64,
    pub price: f64,
    pub updated_prices: Option<PricesResponse>,
}

#[derive(Clone, Debug, Default)]
pub struct GetPricesRequest {
    pub event_id: String,
}

#[derive(Clone, Debug, Default)]
pub struct PricesResponse {
    pub event_id: String,
    pub prices: std::collections::HashMap<String, f64>,
    pub total_volume: f64,
    pub timestamp: i64,
}

#[derive(Clone, Debug, Default)]
pub struct SettleEventRequest {
    pub event_id: String,
    pub winning_outcome: String,
    pub settler_wallet: String,
    pub signature: String,
}

#[derive(Clone, Debug, Default)]
pub struct DeleteEventRequest {
    pub event_id: String,
    pub deleter_wallet: String,
    pub signature: String,
}

#[derive(Clone, Debug, Default)]
pub struct DeleteEventResponse {
    pub success: bool,
    pub event_id: String,
}

#[derive(Clone, Debug, Default)]
pub struct SettleResponse {
    pub event_id: String,
    pub winning_outcome: String,
    pub settled_at: i64,
    pub solana_tx_signature: String,
}

/// Service trait for MitraService
#[tonic::async_trait]
pub trait MitraService: Send + Sync + 'static {
    async fn create_friend_group(&self, request: tonic::Request<CreateGroupRequest>) -> Result<tonic::Response<GroupResponse>, tonic::Status>;
    async fn invite_member(&self, request: tonic::Request<InviteMemberRequest>) -> Result<tonic::Response<MemberResponse>, tonic::Status>;
    async fn create_event(&self, request: tonic::Request<CreateEventRequest>) -> Result<tonic::Response<EventResponse>, tonic::Status>;
    async fn place_bet(&self, request: tonic::Request<PlaceBetRequest>) -> Result<tonic::Response<BetResponse>, tonic::Status>;
    async fn get_event_prices(&self, request: tonic::Request<GetPricesRequest>) -> Result<tonic::Response<PricesResponse>, tonic::Status>;
    async fn settle_event(&self, request: tonic::Request<SettleEventRequest>) -> Result<tonic::Response<SettleResponse>, tonic::Status>;
    async fn delete_event(&self, request: tonic::Request<DeleteEventRequest>) -> Result<tonic::Response<DeleteEventResponse>, tonic::Status>;
}

pub mod mitra_service_server {
    use super::*;
    
    #[derive(Clone)]
    pub struct MitraServiceServer<T> {
        inner: std::sync::Arc<T>,
    }
    
    impl<T: MitraService> MitraServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self { inner: std::sync::Arc::new(inner) }
        }
        
        pub fn from_arc(inner: std::sync::Arc<T>) -> Self {
            Self { inner }
        }
    }
    
    impl<T: MitraService> tonic::server::NamedService for MitraServiceServer<T> {
        const NAME: &'static str = "mitra.MitraService";
    }
    
    impl<T, B> tower_service::Service<http::Request<B>> for MitraServiceServer<T>
    where
        T: MitraService,
        B: http_body::Body + Send + 'static,
        B::Error: Into<Box<dyn std::error::Error + Send + Sync>> + Send,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;
        
        fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }
        
        fn call(&mut self, _req: http::Request<B>) -> Self::Future {
            Box::pin(async move {
                // Stub implementation - returns 501 Not Implemented
                // Install protoc and rebuild for full gRPC support
                let response = http::Response::builder()
                    .status(http::StatusCode::NOT_IMPLEMENTED)
                    .header("content-type", "application/grpc")
                    .header("grpc-status", "12") // UNIMPLEMENTED
                    .header("grpc-message", "gRPC not available - install protoc and rebuild")
                    .body(tonic::body::empty_body())
                    .unwrap();
                Ok(response)
            })
        }
    }
}
"#.to_string()
}
