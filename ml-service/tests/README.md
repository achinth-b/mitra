# ML Service Testing Guide

## Overview

The ML service uses pytest for comprehensive testing with unit tests, integration tests, and mock data.

## Test Structure

```
ml-service/tests/
├── __init__.py
├── test_price_predictor.py    # Price predictor unit tests
├── test_demand_forecast.py    # Demand forecaster unit tests
├── test_liquidity_optimizer.py # Liquidity optimizer unit tests
└── test_integration.py        # Integration tests with FastAPI
```

## Running Tests

### All Tests
```bash
cd ml-service
poetry run pytest
```

### Specific Test File
```bash
poetry run pytest tests/test_price_predictor.py
```

### With Coverage
```bash
poetry run pytest --cov=ml_models --cov=services
```

### Verbose Output
```bash
poetry run pytest -v
```

### Single Test
```bash
poetry run pytest tests/test_price_predictor.py::test_baseline_prediction
```

## Test Categories

### Unit Tests
- `test_price_predictor.py`: Price prediction logic
- `test_demand_forecast.py`: Demand forecasting
- `test_liquidity_optimizer.py`: Liquidity optimization

### Integration Tests
- `test_integration.py`: FastAPI endpoint tests
- Health checks
- Endpoint validation
- Concurrent request handling

## Writing Tests

### Example Unit Test
```python
def test_baseline_prediction(price_predictor, mock_current_prices):
    """Test baseline prediction (pure AMM)"""
    recommended, confidence, reason = price_predictor.predict(
        current_prices=mock_current_prices,
        total_volume=1000.0,
        bet_count=50,
        time_since_creation=24.0,
        historical_data=None,
    )
    
    assert recommended == mock_current_prices
    assert confidence == 0.5
```

### Example Integration Test
```python
def test_predict_prices_endpoint(client, mock_event_state):
    """Test price prediction endpoint"""
    response = client.post("/predict-prices", json=mock_event_state)
    assert response.status_code == 200
    
    data = response.json()
    assert "recommended_prices" in data
```

## Fixtures

Common fixtures are defined in test files:
- `price_predictor`: Price predictor instance
- `demand_forecaster`: Demand forecaster instance
- `liquidity_optimizer`: Liquidity optimizer instance
- `mock_current_prices`: Sample price data
- `mock_historical_data`: Historical market data
- `client`: FastAPI test client

## Mock Data

Tests use mock data to avoid external dependencies:
- Mock prices
- Mock volumes
- Mock historical data
- Mock event states

## Best Practices

1. **Isolation**: Each test should be independent
2. **Fixtures**: Use pytest fixtures for reusable data
3. **Descriptive Names**: Use clear test function names
4. **Docstrings**: Document what each test verifies
5. **Assertions**: Use specific assertions with messages

## Coverage

Run with coverage report:
```bash
poetry run pytest --cov=ml_models --cov=services --cov-report=html
```

Open `htmlcov/index.html` to view coverage report.

## Continuous Integration

Tests should run in CI/CD pipeline:
```yaml
- name: Run tests
  run: |
    cd ml-service
    poetry install
    poetry run pytest
```

