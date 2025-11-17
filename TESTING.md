# Comprehensive Testing Strategy

## Overview

This document outlines the comprehensive testing strategy for the Mitra prediction market platform, covering all components from Solana programs to ML services.

## Test Coverage

### 1. Solana Programs (Anchor + Bankrun)

**Location**: `solana/tests/`

**Test Files**:
- `events.ts` - Event program unit tests
- `friend-groups.ts` - Friend group program unit tests
- `treasury.ts` - Treasury program unit tests
- `integration.spec.ts` - E2E integration tests

**Running Tests**:
```bash
cd solana
anchor test
```

**Key Test Scenarios**:
- ✅ Create friend group
- ✅ Invite and accept members
- ✅ Create events
- ✅ Place bets
- ✅ Settle events
- ✅ Error cases (unauthorized access, invalid inputs)
- ✅ Edge cases (concurrent operations, large amounts)

### 2. Backend (Rust Unit + Integration Tests)

**Location**: `backend/tests/`

**Test Files**:
- `unit_test.rs` - Unit tests for AMM, models, utilities
- `integration_test.rs` - Service integration tests
- `database_test.rs` - Database layer tests
- `e2e_test.rs` - End-to-end flow tests

**Running Tests**:
```bash
cd backend
cargo test

# Specific suites
cargo test --test unit_test
cargo test --test integration_test
cargo test --test e2e_test
```

**Key Test Scenarios**:
- ✅ AMM price calculations
- ✅ Database CRUD operations
- ✅ Repository operations
- ✅ WebSocket subscriptions
- ✅ Settlement mechanisms
- ✅ Merkle proof generation
- ✅ Complete E2E flows

**Test Database Setup**:
```bash
export TEST_DATABASE_URL="postgresql://postgres:postgres@localhost/mitra_test"
```

### 3. ML Service (Python pytest with Mock Data)

**Location**: `ml-service/tests/`

**Test Files**:
- `test_price_predictor.py` - Price prediction unit tests
- `test_demand_forecast.py` - Demand forecasting tests
- `test_liquidity_optimizer.py` - Liquidity optimization tests
- `test_integration.py` - FastAPI endpoint integration tests

**Running Tests**:
```bash
cd ml-service
poetry run pytest

# With coverage
poetry run pytest --cov=ml_models --cov=services
```

**Key Test Scenarios**:
- ✅ Baseline price predictions (pure AMM)
- ✅ Price smoothing for low volume
- ✅ Demand forecasting with historical data
- ✅ Liquidity optimization
- ✅ API endpoint validation
- ✅ Concurrent request handling
- ✅ Error handling

### 4. E2E Automated Flow Testing

**Complete Flow**: Create group → Create event → Place bets → Settle

**Backend E2E** (`backend/tests/e2e_test.rs`):
```rust
#[sqlx::test]
async fn test_complete_e2e_flow(pool: PgPool) {
    // Step 1: Create users
    // Step 2: Create friend group
    // Step 3: Add members
    // Step 4: Create event
    // Step 5: Place bets
    // Step 6: Settle event
    // Step 7: Verify results
}
```

**Solana E2E** (`solana/tests/integration.spec.ts`):
```typescript
describe("E2E Flow: Create Group → Create Event → Place Bets → Settle", () => {
    it("Step 1: Create friend group", async () => { /* ... */ });
    it("Step 2: Invite members", async () => { /* ... */ });
    it("Step 3: Accept invites", async () => { /* ... */ });
    it("Step 4: Create event", async () => { /* ... */ });
    it("Step 5: Place bets", async () => { /* ... */ });
    it("Step 6: Settle event", async () => { /* ... */ });
});
```

## Test Execution Strategy

### Local Development

1. **Unit Tests First**: Run unit tests during development
   ```bash
   # Backend
   cargo test --lib
   
   # ML Service
   poetry run pytest tests/test_price_predictor.py
   ```

2. **Integration Tests**: Run after unit tests pass
   ```bash
   # Backend
   cargo test --test integration_test
   
   # ML Service
   poetry run pytest tests/test_integration.py
   ```

3. **E2E Tests**: Run before committing
   ```bash
   # Backend
   cargo test --test e2e_test
   
   # Solana
   anchor test
   ```

### Continuous Integration

**GitHub Actions / CI Pipeline**:
```yaml
test:
  runs-on: ubuntu-latest
  steps:
    - name: Test Solana Programs
      run: |
        cd solana
        anchor test
    
    - name: Test Backend
      run: |
        cd backend
        cargo test
    
    - name: Test ML Service
      run: |
        cd ml-service
        poetry install
        poetry run pytest
```

## Test Data Management

### Mock Data
- **ML Service**: Uses pytest fixtures for mock prices, volumes, historical data
- **Backend**: Uses `TestFixtures` helper for reusable test data
- **Solana**: Uses test harness for account setup

### Test Database
- Separate test database (`mitra_test`)
- Automatic cleanup after tests
- Migrations run automatically

## Coverage Goals

- **Unit Tests**: >80% code coverage
- **Integration Tests**: All critical paths covered
- **E2E Tests**: Complete user flows covered

## Best Practices

1. **Test Isolation**: Each test should be independent
2. **Fast Execution**: Unit tests should run quickly
3. **Clear Assertions**: Use descriptive assertion messages
4. **Error Testing**: Test both success and error cases
5. **Documentation**: Document test scenarios and setup

## Running All Tests

**Complete Test Suite**:
```bash
# 1. Solana Programs
cd solana && anchor test && cd ..

# 2. Backend
cd backend && cargo test && cd ..

# 3. ML Service
cd ml-service && poetry run pytest && cd ..
```

## Troubleshooting

### Backend Tests
- Ensure PostgreSQL is running
- Set `TEST_DATABASE_URL` environment variable
- Check database migrations are up to date

### Solana Tests
- Ensure Anchor is installed
- Local validator should be running (or use `--skip-local-validator`)
- Check program IDs match

### ML Service Tests
- Ensure Poetry is installed
- Install dependencies: `poetry install`
- Check Python version (3.11+)

## Future Enhancements

- [ ] Performance/load testing
- [ ] Security testing (fuzzing)
- [ ] Chaos engineering tests
- [ ] Visual regression testing (for frontend)
- [ ] Contract testing between services

