use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo to rerun this build script if the proto file changes
    let proto_file = "shared/proto/mitra.proto";
    println!("cargo:rerun-if-changed={}", proto_file);

    // Tell Cargo to rerun if migrations directory changes
    println!("cargo:rerun-if-changed=migrations");

    // Build gRPC code from proto file
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .out_dir(out_dir.join("proto"))
        .compile(&[proto_file], &["shared/proto"])?;

    // Note: Database migrations are handled at runtime by sqlx::migrate
    // No compile-time code generation needed for migrations
    
    Ok(())
}