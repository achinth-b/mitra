"""Demand forecasting model for predicting buy/sell pressure."""

import logging
from typing import Dict, Final, List

import numpy as np
from sklearn.linear_model import LinearRegression

logger = logging.getLogger(__name__)

# Configuration constants
DEFAULT_WINDOW_SIZE: Final[int] = 10
DEFAULT_OUTCOMES: Final[List[str]] = ["YES", "NO"]
PRICE_CHANGE_SENSITIVITY: Final[float] = 10.0  # Multiplier for price change to demand


class DemandForecaster:
    """Forecasts buy/sell pressure using moving averages and simple regression.
    
    Uses simple moving averages for initial implementation.
    Can be upgraded to LSTM/Transformer models later for better predictions.
    
    Attributes:
        window_size: Size of the moving average window.
    """

    def __init__(self, window_size: int = DEFAULT_WINDOW_SIZE) -> None:
        """Initialize demand forecaster.
        
        Args:
            window_size: Size of moving average window for trend analysis.
        """
        self._window_size = window_size
        self._model = LinearRegression()

    @property
    def window_size(self) -> int:
        """Get the window size."""
        return self._window_size

    def forecast(
        self,
        historical_volumes: List[float],
        historical_prices: List[Dict[str, float]],
        time_horizon: float = 1.0,
    ) -> Dict[str, float]:
        """Forecast demand for each outcome.
        
        Args:
            historical_volumes: List of historical volumes (oldest first).
            historical_prices: List of historical price dictionaries (oldest first).
            time_horizon: Hours ahead to forecast (currently unused, reserved for future).
        
        Returns:
            Dictionary mapping outcome to expected demand (normalized to sum to 1.0).
        """
        # Determine outcomes from data or use defaults
        outcomes = self._extract_outcomes(historical_prices)
        
        if len(historical_volumes) < 2:
            # Not enough data, return equal distribution
            return self._equal_distribution(outcomes)

        # Calculate demand based on price trends
        if len(historical_prices) >= 2:
            demand_forecast = self._calculate_price_based_demand(
                historical_prices[-1],
                historical_prices[-2],
            )
        else:
            demand_forecast = self._equal_distribution(outcomes)

        return self._normalize(demand_forecast)

    def _extract_outcomes(self, historical_prices: List[Dict[str, float]]) -> List[str]:
        """Extract outcome names from historical data."""
        if historical_prices and historical_prices[-1]:
            return list(historical_prices[-1].keys())
        return list(DEFAULT_OUTCOMES)

    def _equal_distribution(self, outcomes: List[str]) -> Dict[str, float]:
        """Create equal probability distribution across outcomes."""
        n = len(outcomes)
        return {outcome: 1.0 / n for outcome in outcomes}

    def _calculate_price_based_demand(
        self,
        recent_prices: Dict[str, float],
        prev_prices: Dict[str, float],
    ) -> Dict[str, float]:
        """Calculate demand based on price changes."""
        demand_forecast: Dict[str, float] = {}
        for outcome in recent_prices:
            price_change = recent_prices.get(outcome, 0.0) - prev_prices.get(outcome, 0.0)
            # Positive price change = increased demand (base 0.5 + scaled change)
            demand_forecast[outcome] = max(0.0, 0.5 + price_change * PRICE_CHANGE_SENSITIVITY)
        return demand_forecast

    @staticmethod
    def _normalize(demand: Dict[str, float]) -> Dict[str, float]:
        """Normalize demand values to sum to 1.0."""
        total = sum(demand.values())
        if total > 0:
            return {k: v / total for k, v in demand.items()}
        return demand

    def predict_volume_trend(self, historical_volumes: List[float]) -> float:
        """Predict volume trend direction and magnitude.
        
        Args:
            historical_volumes: List of historical volumes (oldest first).
        
        Returns:
            Expected volume change rate per period:
            - Positive = increasing volume
            - Negative = decreasing volume
            - Zero = stable or insufficient data
        """
        if len(historical_volumes) < 2:
            return 0.0

        recent = historical_volumes[-self._window_size:]
        if len(recent) < 2:
            return 0.0
        
        # Simple linear trend: (end - start) / periods
        return (recent[-1] - recent[0]) / len(recent)

