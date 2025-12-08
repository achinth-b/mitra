# Mitra Platform Roadmap

This document outlines future functionality that can be built on top of the existing foundation.

## Current State Summary

The platform currently has:
- **Backend (Rust)**: gRPC service skeleton, LMSR AMM, PostgreSQL repositories, state management with Merkle trees
- **ML Service (Python)**: FastAPI endpoints for price prediction, demand forecasting, liquidity optimization
- **Solana Programs**: Friend groups, events, treasury management with basic CPI flows

---

## Phase 1: Core Trading Features (High Priority)

### 1.1 Complete gRPC Implementation
**Status**: Skeleton exists, needs proto files and server
**Effort**: 2-3 days

```protobuf
// proto/mitra.proto
service MitraService {
  rpc CreateGroup(CreateGroupRequest) returns (CreateGroupResponse);
  rpc PlaceBet(PlaceBetRequest) returns (PlaceBetResponse);
  rpc GetEventPrices(GetPricesRequest) returns (GetPricesResponse);
  rpc SubscribeToEvents(SubscribeRequest) returns (stream EventUpdate);
}
```

**Tasks**:
- [ ] Define proto files for all RPCs
- [ ] Generate Rust code with tonic-build
- [ ] Implement server handlers
- [ ] Add authentication middleware

### 1.2 Real-time WebSocket Server
**Status**: Module exists, not integrated
**Effort**: 1-2 days

**Tasks**:
- [ ] Implement WebSocket upgrade handler
- [ ] Add subscription management for events
- [ ] Broadcast price updates from ML poller
- [ ] Add heartbeat/reconnection logic

### 1.3 Actual Solana Integration
**Status**: Placeholders exist
**Effort**: 3-5 days

**Tasks**:
- [ ] Load Anchor IDL in backend
- [ ] Implement `commit_merkle_root` with real transactions
- [ ] Implement `settle_event` with fund distribution
- [ ] Add transaction confirmation handling
- [ ] Implement retry logic for failed transactions

---

## Phase 2: ML Model Improvements (Medium Priority)

### 2.1 Advanced Price Prediction
**Current**: Simple logistic regression baseline
**Target**: Time-series models for better predictions

**Tasks**:
- [ ] Collect training data from live markets
- [ ] Implement LSTM-based price predictor
- [ ] Add feature engineering (time-of-day, market sentiment)
- [ ] A/B test against baseline

### 2.2 Demand Forecasting Improvements
**Current**: Moving average heuristics
**Target**: Prophet/ARIMA for volume prediction

**Tasks**:
- [ ] Integrate Facebook Prophet for demand forecasting
- [ ] Add external data sources (social media sentiment)
- [ ] Implement confidence intervals

### 2.3 Dynamic Liquidity (Reinforcement Learning)
**Current**: Heuristic-based adjustments
**Target**: RL agent that learns optimal liquidity

**Tasks**:
- [ ] Define reward function (minimize slippage + maximize fees)
- [ ] Implement PPO/DQN agent
- [ ] Train in simulation environment
- [ ] Gradual rollout with fallback to heuristics

---

## Phase 3: User Experience Features

### 3.1 Price History & Charting
**Status**: Model defined, not persisted
**Effort**: 2-3 days

**Tasks**:
- [ ] Implement `PriceSnapshotRepository`
- [ ] Add migration for `price_snapshots` table
- [ ] Create background job to capture snapshots
- [ ] Expose charting API endpoint

### 3.2 User Portfolio Tracking
**Tasks**:
- [ ] Track user positions across events
- [ ] Calculate unrealized P&L
- [ ] Add portfolio summary endpoint
- [ ] Implement position history

### 3.3 Event Discovery
**Tasks**:
- [ ] Add event categories/tags
- [ ] Implement search functionality
- [ ] Add trending events algorithm
- [ ] User-based recommendations

---

## Phase 4: Advanced Trading Features

### 4.1 Limit Orders
**Current**: Market orders only
**Target**: Order book with limit orders

**Tasks**:
- [ ] Design order matching engine
- [ ] Implement order persistence
- [ ] Add order expiration handling
- [ ] Partial fills support

### 4.2 Multi-outcome Events
**Current**: Binary YES/NO events
**Target**: Support for 3+ outcomes

**Tasks**:
- [ ] Update AMM for N outcomes
- [ ] Adjust price normalization
- [ ] Update UI components

### 4.3 Event Combinations
**Tasks**:
- [ ] Allow betting on multiple correlated events
- [ ] Implement parlay calculations
- [ ] Add risk limits

---

## Phase 5: Security & Reliability

### 5.1 Security Audit
**Tasks**:
- [ ] Audit Solana programs (external firm)
- [ ] Penetration testing on backend
- [ ] Rate limiting implementation
- [ ] Input validation hardening

### 5.2 Monitoring & Observability
**Tasks**:
- [ ] Add Prometheus metrics
- [ ] Implement distributed tracing
- [ ] Create Grafana dashboards
- [ ] Set up alerting

### 5.3 Disaster Recovery
**Tasks**:
- [ ] Database backup strategy
- [ ] State reconstruction from on-chain data
- [ ] Failover procedures

---

## Phase 6: Scaling

### 6.1 Horizontal Scaling
**Tasks**:
- [ ] Stateless backend design
- [ ] Redis for session/cache
- [ ] Load balancer setup
- [ ] Database read replicas

### 6.2 Performance Optimization
**Tasks**:
- [ ] Profile hot paths
- [ ] Optimize database queries
- [ ] Implement caching layer
- [ ] Benchmark under load

---

## Technical Debt Items

### Immediate
- [ ] Add comprehensive unit tests for backend repositories
- [ ] Implement integration tests for Solana programs
- [ ] Add end-to-end tests for complete flows
- [ ] Set up CI/CD pipeline

### Short-term
- [ ] Refactor error handling to be more consistent
- [ ] Add request validation middleware
- [ ] Implement proper logging with correlation IDs
- [ ] Add API versioning

### Long-term
- [ ] Consider GraphQL for flexible queries
- [ ] Evaluate move to microservices if needed
- [ ] Database sharding strategy

---

## Feature Ideas (Future Consideration)

1. **Social Features**: Follow traders, copy trading
2. **Governance**: Token-based voting on platform parameters
3. **Mobile App**: React Native or Flutter
4. **Oracle Integration**: Chainlink, Pyth for external data
5. **Cross-chain**: Support for other L1/L2 chains
6. **NFT Rewards**: Achievement badges, loyalty rewards
7. **API Marketplace**: Third-party integrations

---

## Contributing

When implementing new features:
1. Start with an issue describing the feature
2. Write tests first (TDD when possible)
3. Follow existing code patterns
4. Update documentation
5. Request review before merging

## Questions?

Open an issue or reach out to the team.
