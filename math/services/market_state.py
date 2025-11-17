"""Market state cache for storing historical market data"""

import logging
from datetime import datetime
from typing import Dict, List, Optional

logger = logging.getLogger(__name__)


class MarketState:
    """Represents the current state of a market"""

    def __init__(
        self,
        event_id: str,
        prices: Dict[str, float],
        total_volume: float,
        bet_count: int,
        created_at: Optional[str] = None,
    ):
        self.event_id = event_id
        self.prices = prices
        self.total_volume = total_volume
        self.bet_count = bet_count
        self.created_at = created_at
        self.timestamp = datetime.utcnow()

    def to_dict(self) -> Dict:
        """Convert to dictionary"""
        return {
            "event_id": self.event_id,
            "prices": self.prices,
            "total_volume": self.total_volume,
            "bet_count": self.bet_count,
            "created_at": self.created_at,
            "timestamp": self.timestamp.isoformat(),
        }


class MarketStateCache:
    """Cache for storing market state history"""

    def __init__(self, max_history_size: int = 1000):
        """
        Initialize cache
        
        Args:
            max_history_size: Maximum number of states to keep per event
        """
        self.cache: Dict[str, List[MarketState]] = {}
        self.max_history_size = max_history_size
        self.last_update_time: Optional[datetime] = None

    def update(self, event_id: str, state: MarketState):
        """Update cache with new market state"""
        if event_id not in self.cache:
            self.cache[event_id] = []

        self.cache[event_id].append(state)
        self.last_update_time = datetime.utcnow()

        # Trim history if too large
        if len(self.cache[event_id]) > self.max_history_size:
            self.cache[event_id] = self.cache[event_id][-self.max_history_size :]

    def get_latest(self, event_id: str) -> Optional[MarketState]:
        """Get latest market state for an event"""
        if event_id not in self.cache or not self.cache[event_id]:
            return None
        return self.cache[event_id][-1]

    def get_history(self, event_id: str, limit: Optional[int] = None) -> List[MarketState]:
        """
        Get historical market states for an event
        
        Args:
            event_id: Event ID
            limit: Maximum number of states to return (None for all)
        
        Returns:
            List of MarketState objects, most recent first
        """
        if event_id not in self.cache:
            return []

        history = self.cache[event_id]
        if limit:
            return history[-limit:]
        return history

    def size(self) -> int:
        """Get total number of cached states across all events"""
        return sum(len(states) for states in self.cache.values())

    def last_update(self) -> Optional[str]:
        """Get ISO format string of last update time"""
        if self.last_update_time:
            return self.last_update_time.isoformat()
        return None

    def clear(self, event_id: Optional[str] = None):
        """Clear cache for an event or all events"""
        if event_id:
            if event_id in self.cache:
                del self.cache[event_id]
        else:
            self.cache.clear()

