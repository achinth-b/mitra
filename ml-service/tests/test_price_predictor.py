"""Unit tests for price predictor"""

import pytest
from ml_models.price_predictor import PricePredictor


@pytest.fixture
def price_predictor():
    """Create a price predictor instance"""
    return PricePredictor()


@pytest.fixture
def mock_current_prices():
    """Mock current prices"""
    return {"YES": 0.65, "NO": 0.35}


@pytest.fixture
def mock_historical_data():
    """Mock historical market data"""
    return [
        {
            "prices": {"YES": 0.60, "NO": 0.40},
            "total_volume": 500.0,
            "bet_count": 10,
        },
        {
            "prices": {"YES": 0.65, "NO": 0.35},
            "total_volume": 1000.0,
            "bet_count": 20,
        },
    ]


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
    assert "baseline" in reason.lower() or "amm" in reason.lower()


def test_price_smoothing(price_predictor, mock_current_prices):
    """Test price smoothing for low volume"""
    recommended, _, _ = price_predictor.predict(
        current_prices=mock_current_prices,
        total_volume=50.0,  # Low volume
        bet_count=5,
        time_since_creation=1.0,
        historical_data=None,
    )

    # Prices should be smoothed toward equal distribution
    assert "YES" in recommended
    assert "NO" in recommended
    # Prices should still sum to approximately 1.0
    assert abs(sum(recommended.values()) - 1.0) < 0.1


def test_feature_extraction(price_predictor, mock_current_prices, mock_historical_data):
    """Test feature extraction"""
    features = price_predictor._extract_features(
        current_prices=mock_current_prices,
        total_volume=1000.0,
        bet_count=50,
        time_since_creation=24.0,
        historical_data=mock_historical_data,
    )

    assert len(features) > 0
    assert features.shape == (1, -1)  # Single sample


def test_invalid_inputs(price_predictor):
    """Test handling of invalid inputs"""
    # Empty prices
    with pytest.raises((KeyError, AttributeError)):
        price_predictor.predict(
            current_prices={},
            total_volume=1000.0,
            bet_count=50,
            time_since_creation=24.0,
            historical_data=None,
        )


def test_price_normalization(price_predictor):
    """Test that prices are normalized"""
    current_prices = {"YES": 0.7, "NO": 0.4}  # Sum > 1.0

    recommended, _, _ = price_predictor.predict(
        current_prices=current_prices,
        total_volume=1000.0,
        bet_count=50,
        time_since_creation=24.0,
        historical_data=None,
    )

    # Prices should sum to approximately 1.0
    total = sum(recommended.values())
    assert abs(total - 1.0) < 0.01


def test_price_constraints(price_predictor, mock_current_prices):
    """Test that prices are constrained between 0.01 and 0.99"""
    recommended, _, _ = price_predictor.predict(
        current_prices=mock_current_prices,
        total_volume=1000.0,
        bet_count=50,
        time_since_creation=24.0,
        historical_data=None,
    )

    for price in recommended.values():
        assert 0.01 <= price <= 0.99

