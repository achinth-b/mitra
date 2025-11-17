"""Backend client for fetching market state"""

import asyncio
import logging
from typing import Dict, List, Optional

import httpx

logger = logging.getLogger(__name__)


class BackendClient:
    """Client for communicating with the backend gRPC/HTTP service"""

    def __init__(self, backend_url: str):
        self.backend_url = backend_url.rstrip("/")
        self.client = httpx.AsyncClient(timeout=10.0)

    async def get_active_events(self) -> List[Dict]:
        """
        Get all active events from backend
        
        Returns list of event dictionaries with:
        - id: event UUID
        - group_id: group UUID
        - title: event title
        - outcomes: list of outcome strings
        - created_at: timestamp
        - bet_count: number of bets
        """
        try:
            # TODO: Replace with actual gRPC call when backend is ready
            # For now, return mock data
            response = await self.client.get(f"{self.backend_url}/api/events/active")
            if response.status_code == 200:
                return response.json()
            else:
                logger.warning(f"Backend returned status {response.status_code}")
                return []
        except Exception as e:
            logger.error(f"Error fetching active events: {e}")
            return []

    async def get_event_prices(self, event_id: str) -> Dict[str, float]:
        """
        Get current prices for an event
        
        Returns dictionary mapping outcome -> price
        """
        try:
            # TODO: Replace with actual gRPC call
            response = await self.client.get(
                f"{self.backend_url}/api/events/{event_id}/prices"
            )
            if response.status_code == 200:
                data = response.json()
                return data.get("prices", {})
            else:
                logger.warning(f"Backend returned status {response.status_code} for prices")
                return {}
        except Exception as e:
            logger.error(f"Error fetching event prices: {e}")
            return {}

    async def get_event_volume(self, event_id: str) -> float:
        """Get total volume for an event"""
        try:
            # TODO: Replace with actual gRPC call
            response = await self.client.get(
                f"{self.backend_url}/api/events/{event_id}/volume"
            )
            if response.status_code == 200:
                data = response.json()
                return data.get("total_volume", 0.0)
            else:
                return 0.0
        except Exception as e:
            logger.error(f"Error fetching event volume: {e}")
            return 0.0

    async def health_check(self) -> bool:
        """Check if backend is reachable"""
        try:
            response = await self.client.get(f"{self.backend_url}/health", timeout=2.0)
            return response.status_code == 200
        except Exception:
            return False

    async def close(self):
        """Close the HTTP client"""
        await self.client.aclose()

