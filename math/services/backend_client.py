"""Backend client for fetching market state."""

import logging
from contextlib import asynccontextmanager
from typing import Any, AsyncIterator, Dict, List, Optional

import httpx

logger = logging.getLogger(__name__)

# Configuration constants
DEFAULT_TIMEOUT_SECONDS: float = 10.0
HEALTH_CHECK_TIMEOUT_SECONDS: float = 2.0


class BackendClient:
    """Client for communicating with the backend gRPC/HTTP service.
    
    This client provides async methods for fetching market state data.
    It should be used as an async context manager to ensure proper cleanup:
    
        async with BackendClient(url) as client:
            events = await client.get_active_events()
    """

    def __init__(
        self,
        backend_url: str,
        timeout: float = DEFAULT_TIMEOUT_SECONDS,
    ) -> None:
        """Initialize backend client.
        
        Args:
            backend_url: Base URL of the backend service.
            timeout: Request timeout in seconds.
        """
        self._backend_url = backend_url.rstrip("/")
        self._timeout = timeout
        self._client: Optional[httpx.AsyncClient] = None

    async def __aenter__(self) -> "BackendClient":
        """Async context manager entry."""
        self._client = httpx.AsyncClient(timeout=self._timeout)
        return self

    async def __aexit__(self, exc_type: Any, exc_val: Any, exc_tb: Any) -> None:
        """Async context manager exit with cleanup."""
        await self.close()

    @property
    def backend_url(self) -> str:
        """Get the backend URL."""
        return self._backend_url

    def _get_client(self) -> httpx.AsyncClient:
        """Get the HTTP client, creating if needed."""
        if self._client is None:
            self._client = httpx.AsyncClient(timeout=self._timeout)
        return self._client

    async def get_active_events(self) -> List[Dict[str, Any]]:
        """Get all active events from backend.
        
        Returns:
            List of event dictionaries with:
                - id: event UUID
                - group_id: group UUID
                - title: event title
                - outcomes: list of outcome strings
                - created_at: timestamp
                - bet_count: number of bets
        """
        client = self._get_client()
        try:
            response = await client.get(f"{self._backend_url}/api/events/active")
            response.raise_for_status()
            return response.json()
        except httpx.HTTPStatusError as e:
            logger.warning(f"Backend returned status {e.response.status_code}")
            return []
        except httpx.RequestError as e:
            logger.error(f"Error fetching active events: {e}")
            return []

    async def get_event_prices(self, event_id: str) -> Dict[str, float]:
        """Get current prices for an event.
        
        Args:
            event_id: The event identifier.
            
        Returns:
            Dictionary mapping outcome name to price (0.0-1.0).
        """
        client = self._get_client()
        try:
            response = await client.get(
                f"{self._backend_url}/api/events/{event_id}/prices"
            )
            response.raise_for_status()
            data = response.json()
            return data.get("prices", {})
        except httpx.HTTPStatusError as e:
            logger.warning(f"Backend returned status {e.response.status_code} for prices")
            return {}
        except httpx.RequestError as e:
            logger.error(f"Error fetching event prices: {e}")
            return {}

    async def get_event_volume(self, event_id: str) -> float:
        """Get total volume for an event.
        
        Args:
            event_id: The event identifier.
            
        Returns:
            Total trading volume in USDC.
        """
        client = self._get_client()
        try:
            response = await client.get(
                f"{self._backend_url}/api/events/{event_id}/volume"
            )
            response.raise_for_status()
            data = response.json()
            return float(data.get("total_volume", 0.0))
        except httpx.HTTPStatusError:
            return 0.0
        except httpx.RequestError as e:
            logger.error(f"Error fetching event volume: {e}")
            return 0.0

    async def health_check(self) -> bool:
        """Check if backend is reachable.
        
        Returns:
            True if backend is healthy, False otherwise.
        """
        client = self._get_client()
        try:
            response = await client.get(
                f"{self._backend_url}/health",
                timeout=HEALTH_CHECK_TIMEOUT_SECONDS,
            )
            return response.status_code == 200
        except (httpx.RequestError, httpx.HTTPStatusError):
            return False

    async def close(self) -> None:
        """Close the HTTP client and release resources."""
        if self._client is not None:
            await self._client.aclose()
            self._client = None

