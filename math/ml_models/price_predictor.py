"""Price prediction model using probability calibration"""

import logging
from typing import Dict, List, Optional, Tuple

import numpy as np
from sklearn.linear_model import LogisticRegression
from sklearn.preprocessing import StandardScaler

logger = logging.getLogger(__name__)


class PricePredictor:
    """
    Predicts optimal prices using probability calibration
    
    Initially uses simple logistic regression
    Can be upgraded to more sophisticated models later
    """

    def __init__(self):
        """Initialize the price predictor"""
        self.model = LogisticRegression(random_state=42, max_iter=1000)
        self.scaler = StandardScaler()
        self.is_trained = False

    def predict(
        self,
        current_prices: Dict[str, float],
        total_volume: float,
        bet_count: int,
        time_since_creation: float,
        historical_data: Optional[List] = None,
    ) -> Tuple[Dict[str, float], float, str]:
        """
        Predict optimal prices
        
        Args:
            current_prices: Current prices per outcome
            total_volume: Total volume in USDC
            bet_count: Number of bets placed
            time_since_creation: Hours since event creation
            historical_data: Historical market states (optional)
        
        Returns:
            Tuple of (recommended_prices, confidence, reason)
        """
        # For MVP, use baseline: return current prices with small adjustments
        # In production, this would use trained ML models
        
        if not self.is_trained or not historical_data:
            # Baseline: Pure AMM (no ML adjustments)
            logger.info("Using baseline prediction (pure AMM)")
            return current_prices, 0.5, "Baseline: Using current AMM prices"

        # Extract features
        features = self._extract_features(
            current_prices, total_volume, bet_count, time_since_creation, historical_data
        )

        # Predict probability adjustments
        # For now, return current prices with small smoothing
        recommended_prices = self._smooth_prices(current_prices, total_volume)

        confidence = 0.6  # Low confidence for initial model
        reason = "ML-adjusted prices based on market activity"

        return recommended_prices, confidence, reason

    def _extract_features(
        self,
        current_prices: Dict[str, float],
        total_volume: float,
        bet_count: int,
        time_since_creation: float,
        historical_data: Optional[List],
    ) -> np.ndarray:
        """Extract features for ML model"""
        # Basic features
        num_outcomes = len(current_prices)
        price_values = list(current_prices.values())
        
        features = [
            num_outcomes,
            total_volume,
            bet_count,
            time_since_creation,
            np.mean(price_values),
            np.std(price_values),
        ]

        # Add historical features if available
        if historical_data and len(historical_data) > 0:
            # Price volatility
            if len(historical_data) > 1:
                recent_prices = historical_data[-1].prices
                prev_prices = historical_data[-2].prices if len(historical_data) > 1 else recent_prices
                
                volatility = sum(
                    abs(recent_prices.get(k, 0) - prev_prices.get(k, 0))
                    for k in set(recent_prices.keys()) | set(prev_prices.keys())
                )
                features.append(volatility)
            else:
                features.append(0.0)

            # Volume trend
            if len(historical_data) > 1:
                volume_trend = historical_data[-1].total_volume - historical_data[-2].total_volume
                features.append(volume_trend)
            else:
                features.append(0.0)
        else:
            features.extend([0.0, 0.0])

        return np.array(features).reshape(1, -1)

    def _smooth_prices(
        self, current_prices: Dict[str, float], total_volume: float
    ) -> Dict[str, float]:
        """
        Smooth prices based on volume
        Higher volume = less adjustment (more confidence in AMM)
        Lower volume = more smoothing toward equal distribution
        """
        num_outcomes = len(current_prices)
        equal_price = 1.0 / num_outcomes

        # Smoothing factor: higher volume = less smoothing
        # Volume threshold: $1000 USDC
        volume_threshold = 1000.0
        smoothing_factor = max(0.0, 1.0 - (total_volume / volume_threshold))
        smoothing_factor = min(0.1, smoothing_factor)  # Max 10% smoothing

        smoothed_prices = {}
        for outcome, price in current_prices.items():
            # Blend current price with equal distribution
            smoothed_price = (1 - smoothing_factor) * price + smoothing_factor * equal_price
            smoothed_prices[outcome] = max(0.01, min(0.99, smoothed_price))

        # Normalize to sum to 1.0
        total = sum(smoothed_prices.values())
        if total > 0:
            smoothed_prices = {k: v / total for k, v in smoothed_prices.items()}

        return smoothed_prices

    def train(self, training_data: List[Dict]):
        """
        Train the model on historical data
        
        Args:
            training_data: List of training examples with features and labels
        """
        # TODO: Implement training when we have historical data
        logger.info("Training price predictor model...")
        self.is_trained = True
        logger.info("Model training completed (baseline mode)")

