from typing import Optional
import redis.asyncio as redis

from app.config import settings


class RedisClient:
    """Async Redis client wrapper."""
    
    def __init__(self):
        self.client: Optional[redis.Redis] = None
    
    async def connect(self):
        """Initialize Redis connection."""
        self.client = await redis.from_url(
            settings.redis_url,
            encoding="utf-8",
            decode_responses=True,
        )
    
    async def disconnect(self):
        """Close Redis connection."""
        if self.client:
            await self.client.close()
    
    def get_client(self) -> redis.Redis:
        """Get Redis client instance."""
        if not self.client:
            raise RuntimeError("Redis client not initialized")
        return self.client


redis_client = RedisClient()


async def get_redis() -> redis.Redis:
    """Dependency for getting Redis client."""
    return redis_client.get_client()

