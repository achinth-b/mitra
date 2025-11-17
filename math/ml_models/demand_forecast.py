"""Demand forecasting model"""

import logging
from typing import Dict, List, Optional

import numpy as np
from sklearn.linear_model import LinearRegression

logger = logging.getLogger(__name__)


class DemandForecaster:
    """
    Forecasts buy/sell pressure using moving averages and simple regression
    
    Initially uses simple moving averages
    Can be upgraded to LSTM/Transformer models later
    """

    def __init__(self, window_size: int = 10):
        """
        Initialize demand forecaster
        
        Args:
            window_size: Size of moving average window
        """
        self.window_size = window_size
        self.model = LinearRegression()

    def forecast(
        self,
        historical_volumes: List[float],
        historical_prices: List[Dict[str, float]],
        time_horizon: float = 1.0,  # Hours ahead
    ) -> Dict[str, float]:
        """
        Forecast demand for each outcome
        
        Args:
            historical_volumes: List of historical volumes (most recent last)
            historical_prices: List of historical price dictionaries
            time_horizon: Hours ahead to forecast
        
        Returns:
            Dictionary mapping outcome -> expected demand (normalized)
        """
        if len(historical_volumes) < 2:
            # Not enough data, return equal distribution
            if historical_prices:
                outcomes = list(historical_prices[-1].keys())
            else:
                outcomes = ["YES", "NO"]  # Default
            
            return {outcome: 1.0 / len(outcomes) for outcome in outcomes}

        # Calculate moving average of volume
        recent_volumes = historical_volumes[-self.window_size :]
        avg_volume = np.mean(recent_volumes)

        # Calculate price trends
        if len(historical_prices) >= 2:
            recent_prices = historical_prices[-1]
            prev_prices = historical_prices[-2]
            
            # Price changes indicate demand
            demand_forecast = {}
            for outcome in recent_prices.keys():
                price_change = recent_prices.get(outcome, 0) - prev_prices.get(outcome, 0)
                # Positive price change = increased demand
                demand_forecast[outcome] = max(0.0, 0.5 + price_change * 10)
        else:
            # Equal demand if no price history
            outcomes = list(historical_prices[-1].keys()) if historical_prices else ["YES", "NO"]
            demand_forecast = {outcome: 1.0 / len(outcomes) for outcome in outcomes}

        # Normalize to sum to 1.0
        total = sum(demand_forecast.values())
        if total > 0:
            demand_forecast = {k: v / total for k, v in demand_forecast.items()}

        return demand_forecast

    def predict_volume_trend(self, historical_volumes: List[float]) -> float:
        """
        Predict volume trend
        
        Returns:
            Expected volume change (positive = increasing, negative = decreasing)
        """
        if len(historical_volumes) < 2:
            return 0.0

        # Simple linear trend
        recent = historical_volumes[-self.window_size :]
        if len(recent) >= 2:
            trend = (recent[-1] - recent[0]) / len(recent)
            return trend
        
        return 0.0

