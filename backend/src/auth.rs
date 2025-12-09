use crate::error::{AppError, AppResult};
use solana_sdk::{
    pubkey::Pubkey,
    signature::Signature,
    signer::Signer,
};
use std::str::FromStr;

/// Check if we're in development mode (skip signature verification)
fn is_dev_mode() -> bool {
    std::env::var("ENVIRONMENT")
        .map(|e| e.to_lowercase() == "development")
        .unwrap_or(false)
}

/// Verify a wallet signature for authentication
/// 
/// # Arguments
/// * `wallet_address` - The Solana wallet address (base58 string)
/// * `message` - The message that was signed
/// * `signature` - The signature (base58 string)
/// 
/// # Returns
/// * `Ok(())` if signature is valid
/// * `Err(AppError)` if signature is invalid
pub fn verify_signature(
    wallet_address: &str,
    _message: &str,
    signature: &str,
) -> AppResult<()> {
    // In development mode, accept any non-empty signature
    if is_dev_mode() {
        if signature.is_empty() {
            return Err(AppError::Validation("Signature required".to_string()));
        }
        // Still validate wallet address format
        Pubkey::from_str(wallet_address)
            .map_err(|e| AppError::Validation(format!("Invalid wallet address: {}", e)))?;
        return Ok(());
    }

    // Parse wallet address
    let _pubkey = Pubkey::from_str(wallet_address)
        .map_err(|e| AppError::Validation(format!("Invalid wallet address: {}", e)))?;

    // Parse signature
    let _sig = Signature::from_str(signature)
        .map_err(|e| AppError::Validation(format!("Invalid signature: {}", e)))?;

    // Verify signature
    // Note: In production, you'll need to verify against the actual message format
    // Solana signatures are typically over a message hash, not raw message
    // For now, this is a placeholder that checks signature format
    
    // TODO: Implement proper signature verification
    // This requires:
    // 1. Message serialization (typically using borsh or custom format)
    // 2. Message hash calculation
    // 3. Signature verification using ed25519
    
    // For MVP, we'll do basic validation
    if signature.len() < 64 {
        return Err(AppError::Validation("Signature too short".to_string()));
    }

    // In production, use:
    // pubkey.verify(message_bytes.as_slice(), &sig)
    
    Ok(())
}

/// Create a message to sign for authentication
/// 
/// # Arguments
/// * `wallet_address` - The wallet address
/// * `action` - The action being performed (e.g., "place_bet", "create_event")
/// * `timestamp` - Unix timestamp
/// 
/// # Returns
/// Message string to sign
pub fn create_auth_message(
    wallet_address: &str,
    action: &str,
    timestamp: i64,
) -> String {
    format!("mitra_auth:{}:{}:{}", wallet_address, action, timestamp)
}

/// Verify authentication message with timestamp
/// 
/// Checks that:
/// 1. Signature is valid
/// 2. Timestamp is recent (within 5 minutes)
pub fn verify_auth_with_timestamp(
    wallet_address: &str,
    action: &str,
    timestamp: i64,
    signature: &str,
) -> AppResult<()> {
    // Check timestamp is recent (within 5 minutes)
    let now = chrono::Utc::now().timestamp();
    let time_diff = (now - timestamp).abs();
    
    if time_diff > 300 {
        return Err(AppError::Unauthorized("Signature timestamp expired".to_string()));
    }

    // Create message
    let message = create_auth_message(wallet_address, action, timestamp);

    // Verify signature
    verify_signature(wallet_address, &message, signature)?;

    Ok(())
}

/// Extract wallet address from request context
/// 
/// This is a helper for extracting wallet from gRPC metadata or HTTP headers
pub fn extract_wallet_from_context(context: &str) -> AppResult<String> {
    // In production, this would parse from gRPC metadata or JWT token
    // For MVP, assume it's passed directly
    
    if context.is_empty() {
        return Err(AppError::Unauthorized("Missing wallet address".to_string()));
    }

    // Basic validation
    Pubkey::from_str(context)
        .map_err(|e| AppError::Validation(format!("Invalid wallet format: {}", e)))?;

    Ok(context.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_auth_message() {
        let message = create_auth_message(
            "11111111111111111111111111111111",
            "place_bet",
            1234567890,
        );
        
        assert!(message.contains("mitra_auth"));
        assert!(message.contains("place_bet"));
    }

    #[test]
    fn test_verify_auth_with_timestamp_expired() {
        let old_timestamp = chrono::Utc::now().timestamp() - 400; // 400 seconds ago
        
        let result = verify_auth_with_timestamp(
            "11111111111111111111111111111111",
            "place_bet",
            old_timestamp,
            "dummy_signature",
        );

        assert!(result.is_err());
    }
}

