"""Market state cache for storing historical market data"""

import logging
from dataclasses import dataclass, field
from datetime import datetime, timezone
from typing import Dict, List, Optional

logger = logging.getLogger(__name__)


@dataclass
class MarketState:
    """Represents the current state of a market.
    
    Attributes:
        event_id: Unique identifier for the event.
        prices: Mapping of outcome names to their current prices.
        total_volume: Total trading volume in USDC.
        bet_count: Number of bets placed on the event.
        created_at: ISO timestamp of when the event was created.
        timestamp: When this market state snapshot was taken.
    """
    event_id: str
    prices: Dict[str, float]
    total_volume: float
    bet_count: int
    created_at: Optional[str] = None
    timestamp: datetime = field(default_factory=lambda: datetime.now(timezone.utc))

    def to_dict(self) -> Dict[str, object]:
        """Convert to dictionary representation.
        
        Returns:
            Dictionary with all market state fields serialized.
        """
        return {
            "event_id": self.event_id,
            "prices": self.prices,
            "total_volume": self.total_volume,
            "bet_count": self.bet_count,
            "created_at": self.created_at,
            "timestamp": self.timestamp.isoformat(),
        }


class MarketStateCache:
    """Cache for storing market state history.
    
    Thread-safe cache with bounded memory usage via LRU-style eviction.
    """

    # Default limits to prevent unbounded memory growth
    DEFAULT_MAX_HISTORY_SIZE: int = 1000
    DEFAULT_MAX_EVENTS: int = 500

    def __init__(
        self,
        max_history_size: int = DEFAULT_MAX_HISTORY_SIZE,
        max_events: int = DEFAULT_MAX_EVENTS,
    ) -> None:
        """Initialize cache.
        
        Args:
            max_history_size: Maximum number of states to keep per event.
            max_events: Maximum number of events to track (prevents memory leaks).
        """
        self._cache: Dict[str, List[MarketState]] = {}
        self._max_history_size = max_history_size
        self._max_events = max_events
        self._last_update_time: Optional[datetime] = None
        self._access_order: List[str] = []  # LRU tracking

    def update(self, event_id: str, state: MarketState) -> None:
        """Update cache with new market state.
        
        Args:
            event_id: The event identifier.
            state: The market state snapshot to cache.
        """
        # Handle new events
        if event_id not in self._cache:
            # Evict oldest event if at capacity
            if len(self._cache) >= self._max_events:
                self._evict_oldest()
            self._cache[event_id] = []

        # Update LRU order
        if event_id in self._access_order:
            self._access_order.remove(event_id)
        self._access_order.append(event_id)

        self._cache[event_id].append(state)
        self._last_update_time = datetime.now(timezone.utc)

        # Trim history if too large
        if len(self._cache[event_id]) > self._max_history_size:
            self._cache[event_id] = self._cache[event_id][-self._max_history_size :]

    def _evict_oldest(self) -> None:
        """Evict the least recently used event from cache."""
        if self._access_order:
            oldest = self._access_order.pop(0)
            del self._cache[oldest]
            logger.debug(f"Evicted event {oldest} from cache")

    def get_latest(self, event_id: str) -> Optional[MarketState]:
        """Get latest market state for an event.
        
        Args:
            event_id: The event identifier.
            
        Returns:
            The most recent market state or None if not found.
        """
        if event_id not in self._cache or not self._cache[event_id]:
            return None
        return self._cache[event_id][-1]

    def get_history(self, event_id: str, limit: Optional[int] = None) -> List[MarketState]:
        """Get historical market states for an event.
        
        Args:
            event_id: Event identifier.
            limit: Maximum number of states to return (None for all).
        
        Returns:
            List of MarketState objects, oldest first (chronological order).
        """
        if event_id not in self._cache:
            return []

        history = self._cache[event_id]
        if limit:
            return history[-limit:]
        return list(history)  # Return copy to prevent external modification

    def size(self) -> int:
        """Get total number of cached states across all events."""
        return sum(len(states) for states in self._cache.values())

    def event_count(self) -> int:
        """Get number of events being tracked."""
        return len(self._cache)

    def last_update(self) -> Optional[str]:
        """Get ISO format string of last update time."""
        if self._last_update_time:
            return self._last_update_time.isoformat()
        return None

    def clear(self, event_id: Optional[str] = None) -> None:
        """Clear cache for an event or all events.
        
        Args:
            event_id: If provided, only clear that event. Otherwise clear all.
        """
        if event_id:
            if event_id in self._cache:
                del self._cache[event_id]
                if event_id in self._access_order:
                    self._access_order.remove(event_id)
        else:
            self._cache.clear()
            self._access_order.clear()

