# Backend Testing Guide

## Overview

The backend uses a comprehensive testing strategy with unit tests, integration tests, and end-to-end tests.

## Test Structure

```
backend/tests/
├── helpers.rs              # Test utilities and fixtures
├── database_test.rs        # Database integration tests
├── unit_test.rs            # Unit tests for individual components
├── integration_test.rs     # Integration tests for services
└── e2e_test.rs             # End-to-end flow tests
```

## Running Tests

### All Tests
```bash
cd backend
cargo test
```

### Specific Test Suite
```bash
# Unit tests only
cargo test --test unit_test

# Integration tests only
cargo test --test integration_test

# E2E tests only
cargo test --test e2e_test

# Database tests only
cargo test --test database_test
```

### With Output
```bash
cargo test -- --nocapture
```

### Single Test
```bash
cargo test test_complete_e2e_flow
```

## Test Database Setup

Tests use `sqlx::test` which automatically:
- Creates a test database
- Runs migrations
- Cleans up after tests

Set `TEST_DATABASE_URL` environment variable:
```bash
export TEST_DATABASE_URL="postgresql://postgres:postgres@localhost/mitra_test"
```

## Test Categories

### Unit Tests (`unit_test.rs`)
- AMM price calculations
- Model conversions
- Error handling
- Utility functions

### Integration Tests (`integration_test.rs`)
- Repository operations
- Service interactions
- WebSocket subscriptions
- Settlement mechanisms
- Merkle proof generation

### E2E Tests (`e2e_test.rs`)
- Complete flows: Create group → Create event → Place bets → Settle
- Multiple events in same group
- User multiple bets
- Member removal
- Event cancellation

### Database Tests (`database_test.rs`)
- Connection pooling
- Migrations
- CRUD operations
- Transactions
- Error cases

## Writing Tests

### Example Unit Test
```rust
#[test]
fn test_amm_price_calculation() {
    let outcomes = vec!["YES".to_string(), "NO".to_string()];
    let amm = LmsrAmm::new(Decimal::new(100, 0), outcomes).unwrap();
    
    let prices = amm.get_prices().unwrap();
    assert_eq!(prices.len(), 2);
}
```

### Example Integration Test
```rust
#[sqlx::test]
async fn test_create_group(pool: PgPool) {
    let db = TestDatabase::from_pool(pool).await;
    db.cleanup().await;
    
    let group = db.friend_group_repo
        .create("pubkey", "Test Group", "admin")
        .await
        .expect("Failed to create group");
    
    assert_eq!(group.name, "Test Group");
}
```

## Test Fixtures

Use `TestFixtures` for common test data:
```rust
let fixtures = TestFixtures::create(&db).await;
// fixtures.user1, fixtures.user2, fixtures.friend_group, fixtures.event
```

## Best Practices

1. **Isolation**: Each test should be independent
2. **Cleanup**: Always clean up test data
3. **Fixtures**: Use helpers for reusable test data
4. **Assertions**: Use descriptive assertion messages
5. **Error Handling**: Test both success and error cases

## Coverage

Run with coverage:
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

