"""Liquidity parameter optimization for AMM."""

import logging
from typing import Dict, Final, NamedTuple, Tuple

logger = logging.getLogger(__name__)

# Configuration constants
DEFAULT_MIN_LIQUIDITY: Final[float] = 50.0
DEFAULT_MAX_LIQUIDITY: Final[float] = 1000.0
BASE_LIQUIDITY: Final[float] = 100.0

# Thresholds for liquidity adjustments
HIGH_VOLATILITY_THRESHOLD: Final[float] = 0.2
LOW_VOLATILITY_THRESHOLD: Final[float] = 0.05
HIGH_VOLUME_THRESHOLD: Final[float] = 5000.0  # USDC
LOW_VOLUME_THRESHOLD: Final[float] = 100.0    # USDC
MINIMUM_ADJUSTMENT_PERCENT: Final[float] = 0.01  # 1%

# Adjustment factors
VOLATILITY_INCREASE_FACTOR: Final[float] = 0.15  # 15% increase for high volatility
VOLATILITY_DECREASE_FACTOR: Final[float] = 0.05  # 5% decrease for low volatility
VOLUME_ADJUSTMENT_FACTOR: Final[float] = 0.05    # 5% adjustment for volume
LOW_VOLUME_INCREASE_FACTOR: Final[float] = 0.10  # 10% increase for low volume


class LiquidityResult(NamedTuple):
    """Result from liquidity optimization."""
    recommended_liquidity: float
    adjustment_amount: float
    reason: str


class LiquidityOptimizer:
    """Optimizes AMM liquidity parameter (b) based on market conditions.
    
    Uses simple heuristics initially. Can be upgraded to reinforcement 
    learning for more sophisticated optimization.
    
    Attributes:
        min_liquidity: Floor for liquidity parameter.
        max_liquidity: Ceiling for liquidity parameter.
    """

    def __init__(
        self,
        min_liquidity: float = DEFAULT_MIN_LIQUIDITY,
        max_liquidity: float = DEFAULT_MAX_LIQUIDITY,
    ) -> None:
        """Initialize liquidity optimizer.
        
        Args:
            min_liquidity: Minimum liquidity parameter (b).
            max_liquidity: Maximum liquidity parameter (b).
        """
        self._min_liquidity = min_liquidity
        self._max_liquidity = max_liquidity

    @property
    def min_liquidity(self) -> float:
        """Get minimum liquidity bound."""
        return self._min_liquidity

    @property
    def max_liquidity(self) -> float:
        """Get maximum liquidity bound."""
        return self._max_liquidity

    def optimize(
        self,
        current_liquidity: float,
        current_prices: Dict[str, float],
        total_volume: float,
        price_volatility: float,
    ) -> LiquidityResult:
        """Optimize liquidity parameter based on market conditions.
        
        Applies heuristic adjustments based on:
        - Price volatility (higher volatility -> more liquidity for stability)
        - Trading volume (extreme volumes trigger adjustments)
        
        Args:
            current_liquidity: Current liquidity parameter (b).
            current_prices: Current prices per outcome (unused, for future use).
            total_volume: Total trading volume in USDC.
            price_volatility: Price volatility measure (0.0 to 1.0).
        
        Returns:
            LiquidityResult with recommended liquidity, adjustment, and reason.
        """
        recommended_liquidity = current_liquidity
        adjustment = 0.0
        reasons: list[str] = []

        # Apply volatility-based adjustment
        vol_adjustment, vol_reason = self._volatility_adjustment(
            current_liquidity, price_volatility
        )
        if vol_adjustment != 0.0:
            adjustment += vol_adjustment
            recommended_liquidity += vol_adjustment
            reasons.append(vol_reason)

        # Apply volume-based adjustment
        volume_adjustment, vol_reason = self._volume_adjustment(
            current_liquidity, total_volume
        )
        if volume_adjustment != 0.0:
            adjustment += volume_adjustment
            recommended_liquidity += volume_adjustment
            reasons.append(vol_reason)

        # Constrain to valid range
        recommended_liquidity = self._clamp(recommended_liquidity)
        adjustment = recommended_liquidity - current_liquidity

        # Skip tiny adjustments
        if abs(adjustment) < current_liquidity * MINIMUM_ADJUSTMENT_PERCENT:
            return LiquidityResult(
                recommended_liquidity=current_liquidity,
                adjustment_amount=0.0,
                reason="Adjustment too small, keeping current liquidity",
            )

        reason = "; ".join(reasons) if reasons else "No adjustment needed"
        return LiquidityResult(
            recommended_liquidity=recommended_liquidity,
            adjustment_amount=adjustment,
            reason=reason,
        )

    def _volatility_adjustment(
        self, current_liquidity: float, price_volatility: float
    ) -> Tuple[float, str]:
        """Calculate adjustment based on price volatility."""
        if price_volatility > HIGH_VOLATILITY_THRESHOLD:
            adjustment = current_liquidity * VOLATILITY_INCREASE_FACTOR
            return adjustment, f"High volatility ({price_volatility:.2f})"
        elif price_volatility < LOW_VOLATILITY_THRESHOLD:
            adjustment = -current_liquidity * VOLATILITY_DECREASE_FACTOR
            return adjustment, f"Low volatility ({price_volatility:.2f})"
        return 0.0, ""

    def _volume_adjustment(
        self, current_liquidity: float, total_volume: float
    ) -> Tuple[float, str]:
        """Calculate adjustment based on trading volume."""
        if total_volume > HIGH_VOLUME_THRESHOLD:
            adjustment = -current_liquidity * VOLUME_ADJUSTMENT_FACTOR
            return adjustment, f"High volume ({total_volume:.0f} USDC)"
        elif total_volume < LOW_VOLUME_THRESHOLD:
            adjustment = current_liquidity * LOW_VOLUME_INCREASE_FACTOR
            return adjustment, f"Low volume ({total_volume:.0f} USDC)"
        return 0.0, ""

    def _clamp(self, value: float) -> float:
        """Clamp value to valid liquidity range."""
        return max(self._min_liquidity, min(self._max_liquidity, value))

    def calculate_optimal_liquidity(
        self,
        total_volume: float,
        num_outcomes: int,
        target_price_stability: float = 0.1,
    ) -> float:
        """Calculate optimal liquidity based on market characteristics.
        
        Uses heuristics based on:
        - Base liquidity (100 USDC)
        - Volume scaling (+10 per $1000)
        - Outcome count scaling (+20 per additional outcome beyond 2)
        - Target stability multiplier
        
        Args:
            total_volume: Total trading volume in USDC.
            num_outcomes: Number of possible outcomes.
            target_price_stability: Target stability (lower = more stable, default 0.1).
        
        Returns:
            Recommended liquidity parameter within bounds.
        """
        # Volume scaling: +10 per $1000 volume
        volume_component = (total_volume / 1000.0) * 10.0
        
        # Outcome scaling: +20 per outcome beyond the base 2
        outcome_component = max(0, num_outcomes - 2) * 20.0
        
        optimal = BASE_LIQUIDITY + volume_component + outcome_component
        
        # Apply stability factor (baseline is 0.1)
        if target_price_stability > 0:
            stability_factor = target_price_stability / 0.1
            optimal *= stability_factor

        return self._clamp(optimal)

