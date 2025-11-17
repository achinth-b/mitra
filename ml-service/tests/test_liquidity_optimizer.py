"""Unit tests for liquidity optimizer"""

import pytest
from ml_models.liquidity_optimizer import LiquidityOptimizer


@pytest.fixture
def liquidity_optimizer():
    """Create a liquidity optimizer instance"""
    return LiquidityOptimizer(min_liquidity=50.0, max_liquidity=1000.0)


@pytest.fixture
def mock_current_prices():
    """Mock current prices"""
    return {"YES": 0.65, "NO": 0.35}


def test_high_volatility_adjustment(liquidity_optimizer, mock_current_prices):
    """Test liquidity adjustment for high volatility"""
    recommended, adjustment, reason = liquidity_optimizer.optimize(
        current_liquidity=100.0,
        current_prices=mock_current_prices,
        total_volume=1000.0,
        price_volatility=0.25,  # High volatility
    )

    assert recommended > 100.0  # Should increase liquidity
    assert adjustment > 0
    assert "volatility" in reason.lower()


def test_low_volatility_adjustment(liquidity_optimizer, mock_current_prices):
    """Test liquidity adjustment for low volatility"""
    recommended, adjustment, reason = liquidity_optimizer.optimize(
        current_liquidity=100.0,
        current_prices=mock_current_prices,
        total_volume=1000.0,
        price_volatility=0.03,  # Low volatility
    )

    # May decrease slightly or stay same
    assert recommended >= 50.0  # Within bounds
    assert recommended <= 1000.0


def test_high_volume_optimization(liquidity_optimizer, mock_current_prices):
    """Test liquidity optimization for high volume"""
    recommended, adjustment, reason = liquidity_optimizer.optimize(
        current_liquidity=100.0,
        current_prices=mock_current_prices,
        total_volume=10000.0,  # High volume
        price_volatility=0.1,
    )

    assert recommended >= 50.0
    assert recommended <= 1000.0
    assert "volume" in reason.lower() or abs(adjustment) < 1.0


def test_low_volume_optimization(liquidity_optimizer, mock_current_prices):
    """Test liquidity optimization for low volume"""
    recommended, adjustment, reason = liquidity_optimizer.optimize(
        current_liquidity=100.0,
        current_prices=mock_current_prices,
        total_volume=50.0,  # Low volume
        price_volatility=0.1,
    )

    assert recommended >= 100.0  # Should increase for better price discovery
    assert "volume" in reason.lower()


def test_liquidity_bounds(liquidity_optimizer, mock_current_prices):
    """Test that liquidity stays within bounds"""
    # Test minimum bound
    recommended_min, _, _ = liquidity_optimizer.optimize(
        current_liquidity=10.0,  # Below minimum
        current_prices=mock_current_prices,
        total_volume=1000.0,
        price_volatility=0.1,
    )
    assert recommended_min >= 50.0

    # Test maximum bound
    recommended_max, _, _ = liquidity_optimizer.optimize(
        current_liquidity=2000.0,  # Above maximum
        current_prices=mock_current_prices,
        total_volume=1000.0,
        price_volatility=0.1,
    )
    assert recommended_max <= 1000.0


def test_optimal_liquidity_calculation(liquidity_optimizer):
    """Test optimal liquidity calculation"""
    optimal = liquidity_optimizer.calculate_optimal_liquidity(
        total_volume=5000.0,
        num_outcomes=2,
        target_price_stability=0.1,
    )

    assert 50.0 <= optimal <= 1000.0


def test_small_adjustment_threshold(liquidity_optimizer, mock_current_prices):
    """Test that very small adjustments are not recommended"""
    recommended, adjustment, reason = liquidity_optimizer.optimize(
        current_liquidity=100.0,
        current_prices=mock_current_prices,
        total_volume=1000.0,
        price_volatility=0.05,  # Moderate volatility
    )

    # If adjustment is very small, should keep current liquidity
    if abs(adjustment) < 1.0:
        assert "keeping current" in reason.lower() or recommended == 100.0


def test_multiple_outcomes(liquidity_optimizer):
    """Test optimal liquidity with multiple outcomes"""
    optimal_2 = liquidity_optimizer.calculate_optimal_liquidity(
        total_volume=1000.0,
        num_outcomes=2,
        target_price_stability=0.1,
    )

    optimal_3 = liquidity_optimizer.calculate_optimal_liquidity(
        total_volume=1000.0,
        num_outcomes=3,
        target_price_stability=0.1,
    )

    # More outcomes should require more liquidity
    assert optimal_3 >= optimal_2

