"""Liquidity parameter optimization"""

import logging
from typing import Dict, Tuple

import numpy as np

logger = logging.getLogger(__name__)


class LiquidityOptimizer:
    """
    Optimizes AMM liquidity parameter (b) based on market conditions
    
    Uses simple heuristics initially
    Can be upgraded to reinforcement learning later
    """

    def __init__(self, min_liquidity: float = 50.0, max_liquidity: float = 1000.0):
        """
        Initialize liquidity optimizer
        
        Args:
            min_liquidity: Minimum liquidity parameter (b)
            max_liquidity: Maximum liquidity parameter (b)
        """
        self.min_liquidity = min_liquidity
        self.max_liquidity = max_liquidity

    def optimize(
        self,
        current_liquidity: float,
        current_prices: Dict[str, float],
        total_volume: float,
        price_volatility: float,
    ) -> Tuple[float, float, str]:
        """
        Optimize liquidity parameter
        
        Args:
            current_liquidity: Current liquidity parameter (b)
            current_prices: Current prices per outcome
            total_volume: Total volume in USDC
            price_volatility: Price volatility measure (0-1)
        
        Returns:
            Tuple of (recommended_liquidity, adjustment_amount, reason)
        """
        # Baseline: Keep current liquidity
        # Adjustments based on simple heuristics:
        # - High volatility -> increase liquidity (more stable prices)
        # - High volume -> can decrease liquidity (more efficient)
        # - Low volume -> increase liquidity (better price discovery)

        recommended_liquidity = current_liquidity
        adjustment = 0.0
        reason = "No adjustment needed"

        # Volatility-based adjustment
        if price_volatility > 0.2:  # High volatility
            # Increase liquidity by 10-20%
            adjustment = current_liquidity * 0.15
            recommended_liquidity = current_liquidity + adjustment
            reason = f"High volatility ({price_volatility:.2f}), increasing liquidity for stability"
        elif price_volatility < 0.05:  # Low volatility
            # Can decrease liquidity slightly
            adjustment = -current_liquidity * 0.05
            recommended_liquidity = current_liquidity + adjustment
            reason = f"Low volatility ({price_volatility:.2f}), optimizing liquidity"

        # Volume-based adjustment
        volume_threshold = 5000.0  # $5000 USDC
        if total_volume > volume_threshold:
            # High volume: can reduce liquidity slightly (more efficient)
            volume_adjustment = -current_liquidity * 0.05
            recommended_liquidity += volume_adjustment
            adjustment += volume_adjustment
            if "volume" not in reason.lower():
                reason += f"; High volume ({total_volume:.0f} USDC)"
        elif total_volume < 100.0:
            # Low volume: increase liquidity for better price discovery
            volume_adjustment = current_liquidity * 0.1
            recommended_liquidity += volume_adjustment
            adjustment += volume_adjustment
            if "volume" not in reason.lower():
                reason += f"; Low volume ({total_volume:.0f} USDC), increasing liquidity"

        # Constrain to valid range
        recommended_liquidity = max(self.min_liquidity, min(self.max_liquidity, recommended_liquidity))
        
        # Recalculate adjustment after constraints
        adjustment = recommended_liquidity - current_liquidity

        # If adjustment is very small, don't recommend change
        if abs(adjustment) < current_liquidity * 0.01:  # Less than 1% change
            recommended_liquidity = current_liquidity
            adjustment = 0.0
            reason = "Adjustment too small, keeping current liquidity"

        return recommended_liquidity, adjustment, reason

    def calculate_optimal_liquidity(
        self,
        total_volume: float,
        num_outcomes: int,
        target_price_stability: float = 0.1,
    ) -> float:
        """
        Calculate optimal liquidity based on market characteristics
        
        Args:
            total_volume: Total volume in USDC
            num_outcomes: Number of outcomes
            target_price_stability: Target price stability (lower = more stable)
        
        Returns:
            Recommended liquidity parameter
        """
        # Simple heuristic: liquidity should scale with volume
        # Base liquidity: 100
        # Scale with volume: +10 per $1000 volume
        # Scale with outcomes: +20 per additional outcome
        
        base_liquidity = 100.0
        volume_component = (total_volume / 1000.0) * 10.0
        outcome_component = (num_outcomes - 2) * 20.0
        
        optimal = base_liquidity + volume_component + outcome_component
        
        # Adjust for target stability
        stability_factor = target_price_stability / 0.1
        optimal = optimal * stability_factor

        return max(self.min_liquidity, min(self.max_liquidity, optimal))

