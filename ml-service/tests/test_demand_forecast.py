"""Unit tests for demand forecaster"""

import pytest
from ml_models.demand_forecast import DemandForecaster


@pytest.fixture
def demand_forecaster():
    """Create a demand forecaster instance"""
    return DemandForecaster(window_size=10)


@pytest.fixture
def mock_historical_volumes():
    """Mock historical volumes"""
    return [100.0, 150.0, 200.0, 250.0, 300.0]


@pytest.fixture
def mock_historical_prices():
    """Mock historical prices"""
    return [
        {"YES": 0.50, "NO": 0.50},
        {"YES": 0.55, "NO": 0.45},
        {"YES": 0.60, "NO": 0.40},
        {"YES": 0.65, "NO": 0.35},
    ]


def test_forecast_with_sufficient_data(
    demand_forecaster, mock_historical_volumes, mock_historical_prices
):
    """Test demand forecasting with sufficient historical data"""
    forecast = demand_forecaster.forecast(
        historical_volumes=mock_historical_volumes,
        historical_prices=mock_historical_prices,
        time_horizon=1.0,
    )

    assert "YES" in forecast
    assert "NO" in forecast
    # Forecast should sum to approximately 1.0
    assert abs(sum(forecast.values()) - 1.0) < 0.1


def test_forecast_insufficient_data(demand_forecaster):
    """Test demand forecasting with insufficient data"""
    forecast = demand_forecaster.forecast(
        historical_volumes=[100.0],
        historical_prices=[{"YES": 0.50, "NO": 0.50}],
        time_horizon=1.0,
    )

    # Should return equal distribution
    assert "YES" in forecast
    assert "NO" in forecast
    assert abs(forecast["YES"] - forecast["NO"]) < 0.1


def test_volume_trend_calculation(demand_forecaster, mock_historical_volumes):
    """Test volume trend calculation"""
    trend = demand_forecaster.predict_volume_trend(mock_historical_volumes)

    # Trend should be positive (increasing volume)
    assert trend > 0


def test_volume_trend_decreasing(demand_forecaster):
    """Test volume trend with decreasing volumes"""
    decreasing_volumes = [300.0, 250.0, 200.0, 150.0, 100.0]
    trend = demand_forecaster.predict_volume_trend(decreasing_volumes)

    # Trend should be negative (decreasing volume)
    assert trend < 0


def test_forecast_price_trends(
    demand_forecaster, mock_historical_volumes, mock_historical_prices
):
    """Test that price trends influence demand forecast"""
    # Prices trending toward YES
    forecast = demand_forecaster.forecast(
        historical_volumes=mock_historical_volumes,
        historical_prices=mock_historical_prices,
        time_horizon=1.0,
    )

    # YES should have higher demand than NO (prices increasing)
    assert forecast["YES"] >= forecast["NO"]


def test_empty_historical_data(demand_forecaster):
    """Test handling of empty historical data"""
    forecast = demand_forecaster.forecast(
        historical_volumes=[],
        historical_prices=[],
        time_horizon=1.0,
    )

    # Should return default equal distribution
    assert len(forecast) > 0


def test_window_size_effect(demand_forecaster):
    """Test that window size affects forecast"""
    long_history = [100.0] * 20
    short_history = [100.0] * 5

    forecast_long = demand_forecaster.forecast(
        historical_volumes=long_history,
        historical_prices=[{"YES": 0.50, "NO": 0.50}] * 20,
        time_horizon=1.0,
    )

    forecast_short = demand_forecaster.forecast(
        historical_volumes=short_history,
        historical_prices=[{"YES": 0.50, "NO": 0.50}] * 5,
        time_horizon=1.0,
    )

    # Both should return valid forecasts
    assert "YES" in forecast_long
    assert "YES" in forecast_short

