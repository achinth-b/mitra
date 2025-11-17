"""Integration tests for ML service"""

import pytest
import httpx
from fastapi.testclient import TestClient
from main import app


@pytest.fixture
def client():
    """Create test client"""
    return TestClient(app)


@pytest.fixture
def mock_event_state():
    """Mock event state request"""
    return {
        "event_id": "test-event-123",
        "current_prices": {"YES": 0.65, "NO": 0.35},
        "total_volume": 1000.0,
        "bet_count": 50,
        "time_since_creation": 24.0,
    }


@pytest.fixture
def mock_liquidity_request():
    """Mock liquidity adjustment request"""
    return {
        "event_id": "test-event-123",
        "current_liquidity": 100.0,
        "current_prices": {"YES": 0.65, "NO": 0.35},
        "total_volume": 1000.0,
        "price_volatility": 0.15,
    }


def test_health_check(client):
    """Test health check endpoint"""
    response = client.get("/health")
    assert response.status_code == 200
    data = response.json()
    assert "status" in data
    assert "backend_connected" in data
    assert "models_loaded" in data


def test_predict_prices_endpoint(client, mock_event_state):
    """Test price prediction endpoint"""
    response = client.post("/predict-prices", json=mock_event_state)
    assert response.status_code == 200
    
    data = response.json()
    assert "event_id" in data
    assert "recommended_prices" in data
    assert "confidence" in data
    assert "adjustment_reason" in data
    
    # Verify prices sum to approximately 1.0
    prices = data["recommended_prices"]
    total = sum(prices.values())
    assert abs(total - 1.0) < 0.1


def test_adjust_liquidity_endpoint(client, mock_liquidity_request):
    """Test liquidity adjustment endpoint"""
    response = client.post("/adjust-liquidity", json=mock_liquidity_request)
    assert response.status_code == 200
    
    data = response.json()
    assert "event_id" in data
    assert "recommended_liquidity" in data
    assert "adjustment_amount" in data
    assert "reason" in data
    
    # Verify liquidity is within bounds
    assert 50.0 <= data["recommended_liquidity"] <= 1000.0


def test_invalid_event_state(client):
    """Test with invalid event state"""
    invalid_request = {
        "event_id": "test-event-123",
        "current_prices": {},  # Empty prices
        "total_volume": -100.0,  # Negative volume
        "bet_count": 50,
        "time_since_creation": 24.0,
    }
    
    response = client.post("/predict-prices", json=invalid_request)
    # Should handle gracefully (either 200 with baseline or 400/500)
    assert response.status_code in [200, 400, 500]


def test_root_endpoint(client):
    """Test root endpoint"""
    response = client.get("/")
    assert response.status_code == 200
    data = response.json()
    assert "service" in data
    assert "version" in data
    assert "endpoints" in data


def test_concurrent_requests(client, mock_event_state):
    """Test handling of concurrent requests"""
    import concurrent.futures
    
    def make_request():
        return client.post("/predict-prices", json=mock_event_state)
    
    with concurrent.futures.ThreadPoolExecutor(max_workers=5) as executor:
        futures = [executor.submit(make_request) for _ in range(10)]
        results = [f.result() for f in concurrent.futures.as_completed(futures)]
    
    # All requests should succeed
    assert all(r.status_code == 200 for r in results)


def test_price_prediction_consistency(client, mock_event_state):
    """Test that predictions are consistent for same input"""
    response1 = client.post("/predict-prices", json=mock_event_state)
    response2 = client.post("/predict-prices", json=mock_event_state)
    
    assert response1.status_code == 200
    assert response2.status_code == 200
    
    data1 = response1.json()
    data2 = response2.json()
    
    # Prices should be similar (allowing for small variations)
    prices1 = data1["recommended_prices"]
    prices2 = data2["recommended_prices"]
    
    for outcome in prices1:
        assert abs(prices1[outcome] - prices2[outcome]) < 0.01


def test_liquidity_optimization_edge_cases(client):
    """Test liquidity optimization with edge cases"""
    # Very high volatility
    high_vol_request = {
        "event_id": "test-event-123",
        "current_liquidity": 100.0,
        "current_prices": {"YES": 0.65, "NO": 0.35},
        "total_volume": 1000.0,
        "price_volatility": 0.9,  # Very high
    }
    
    response = client.post("/adjust-liquidity", json=high_vol_request)
    assert response.status_code == 200
    data = response.json()
    assert data["recommended_liquidity"] >= 100.0  # Should increase


def test_missing_fields(client):
    """Test handling of missing required fields"""
    incomplete_request = {
        "event_id": "test-event-123",
        # Missing other required fields
    }
    
    response = client.post("/predict-prices", json=incomplete_request)
    assert response.status_code == 422  # Validation error

