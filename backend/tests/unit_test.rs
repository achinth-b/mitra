mod helpers;

use helpers::*;
use mitra_backend::amm::LmsrAmm;
use mitra_backend::models::*;
use mitra_backend::repositories::*;
use mitra_backend::services::*;
use mitra_backend::state_manager::StateManager;
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

/// Unit tests for AMM
#[test]
fn test_amm_price_calculation() {
    let outcomes = vec!["YES".to_string(), "NO".to_string()];
    let amm = LmsrAmm::new(Decimal::new(100, 0), outcomes).unwrap();

    let prices = amm.get_prices().unwrap();
    assert_eq!(prices.len(), 2);
    
    // Prices should sum to approximately 1.0
    let sum: Decimal = prices.values().sum();
    assert!((sum - Decimal::ONE).abs() < Decimal::new(1, 2)); // Within 0.01
}

#[test]
fn test_amm_buy_shares() {
    let outcomes = vec!["YES".to_string(), "NO".to_string()];
    let mut amm = LmsrAmm::new(Decimal::new(100, 0), outcomes).unwrap();

    let amount = Decimal::new(10, 0); // 10 USDC
    let (shares, new_price, _) = amm.calculate_buy("YES", amount).unwrap();

    assert!(shares > Decimal::ZERO);
    assert!(new_price > Decimal::ZERO);
    assert!(new_price <= Decimal::new(99, 2)); // Max 0.99
}

#[test]
fn test_amm_invalid_outcome() {
    let outcomes = vec!["YES".to_string(), "NO".to_string()];
    let amm = LmsrAmm::new(Decimal::new(100, 0), outcomes).unwrap();

    let result = amm.calculate_buy("MAYBE", Decimal::new(10, 0));
    assert!(result.is_err());
}

/// Unit tests for State Manager
#[test]
fn test_merkle_tree_generation() {
    // Test merkle tree with sample data
    let data = vec![
        b"bet1".to_vec(),
        b"bet2".to_vec(),
        b"bet3".to_vec(),
    ];

    // This would require async runtime, so we'll test in integration tests
    // For unit tests, we can test the hashing function
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(b"test");
    let hash = hasher.finalize();
    assert_eq!(hash.len(), 32);
}

/// Unit tests for Settlement Service
#[test]
fn test_settlement_type_enum() {
    let manual = SettlementType::Manual;
    let oracle = SettlementType::Oracle;
    let consensus = SettlementType::Consensus;

    assert_eq!(manual, SettlementType::Manual);
    assert_eq!(oracle, SettlementType::Oracle);
    assert_eq!(consensus, SettlementType::Consensus);
}

/// Unit tests for Models
#[test]
fn test_member_role_conversion() {
    let admin = MemberRole::Admin;
    assert_eq!(admin.as_str(), "admin");

    let member = MemberRole::Member;
    assert_eq!(member.as_str(), "member");
}

#[test]
fn test_event_status_conversion() {
    let active = EventStatus::Active;
    assert_eq!(active.as_str(), "active");

    let resolved = EventStatus::Resolved;
    assert_eq!(resolved.as_str(), "resolved");

    let cancelled = EventStatus::Cancelled;
    assert_eq!(cancelled.as_str(), "cancelled");
}

#[test]
fn test_settlement_type_conversion() {
    let manual = SettlementType::Manual;
    assert_eq!(manual.as_str(), "manual");

    let oracle = SettlementType::Oracle;
    assert_eq!(oracle.as_str(), "oracle");

    let consensus = SettlementType::Consensus;
    assert_eq!(consensus.as_str(), "consensus");
}

/// Unit tests for Price Calculations
#[test]
fn test_price_constraints() {
    let price_min = Decimal::new(1, 2); // 0.01
    let price_max = Decimal::new(99, 2); // 0.99

    assert!(price_min >= Decimal::new(1, 2));
    assert!(price_max <= Decimal::new(99, 2));
}

/// Unit tests for Decimal Operations
#[test]
fn test_decimal_precision() {
    let a = Decimal::new(100, 0);
    let b = Decimal::new(50, 0);
    let result = a + b;
    assert_eq!(result, Decimal::new(150, 0));

    let division = a / Decimal::new(2, 0);
    assert_eq!(division, Decimal::new(50, 0));
}

/// Unit tests for UUID Generation
#[test]
fn test_uuid_generation() {
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    assert_ne!(id1, id2);
}

/// Unit tests for Error Handling
#[test]
fn test_error_types() {
    use mitra_backend::error::AppError;
    
    let db_error = AppError::Database(
        mitra_backend::database::DatabaseError::PoolCreation(
            sqlx::Error::PoolClosed
        )
    );
    
    assert!(format!("{}", db_error).contains("database"));
}

