"""
ML Service for Mitra Prediction Market Platform

FastAPI service that:
- Fetches current market state from backend every 1-5 seconds
- Runs ML models for probability calibration, demand forecasting, liquidity optimization
- Returns recommended price adjustments or liquidity parameters
"""

import asyncio
import logging
import os
from contextlib import asynccontextmanager
from typing import Dict, List, Optional

import httpx
import numpy as np
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel, Field

from ml_models.demand_forecast import DemandForecaster
from ml_models.liquidity_optimizer import LiquidityOptimizer
from ml_models.price_predictor import PricePredictor
from services.backend_client import BackendClient
from services.market_state import MarketState, MarketStateCache

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format="%(asctime)s - %(name)s - %(levelname)s - %(message)s"
)
logger = logging.getLogger(__name__)

# Global state
backend_client: Optional[BackendClient] = None
market_state_cache: Optional[MarketStateCache] = None
price_predictor: Optional[PricePredictor] = None
demand_forecaster: Optional[DemandForecaster] = None
liquidity_optimizer: Optional[LiquidityOptimizer] = None


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Startup and shutdown logic"""
    global backend_client, market_state_cache
    global price_predictor, demand_forecaster, liquidity_optimizer

    logger.info("Starting ML Service...")

    # Initialize backend client
    backend_url = os.getenv("BACKEND_URL", "http://localhost:50051")
    backend_client = BackendClient(backend_url)
    logger.info(f"Backend client initialized: {backend_url}")

    # Initialize market state cache
    market_state_cache = MarketStateCache()

    # Initialize ML models
    price_predictor = PricePredictor()
    demand_forecaster = DemandForecaster()
    liquidity_optimizer = LiquidityOptimizer()
    logger.info("ML models initialized")

    # Start background task to fetch market state
    fetch_interval = int(os.getenv("FETCH_INTERVAL_SECONDS", "3"))
    asyncio.create_task(fetch_market_state_periodically(fetch_interval))

    logger.info("ML Service started successfully")
    yield

    logger.info("Shutting down ML Service...")


app = FastAPI(
    title="Mitra ML Service",
    description="ML service for prediction market price optimization",
    version="0.1.0",
    lifespan=lifespan,
)


# Request/Response Models
class EventStateRequest(BaseModel):
    """Request for price prediction"""
    event_id: str
    current_prices: Dict[str, float] = Field(..., description="Current prices per outcome")
    total_volume: float = Field(..., description="Total volume in USDC")
    bet_count: int = Field(..., description="Number of bets placed")
    time_since_creation: float = Field(..., description="Hours since event creation")


class PricePredictionResponse(BaseModel):
    """Response with predicted optimal prices"""
    event_id: str
    recommended_prices: Dict[str, float] = Field(..., description="Recommended prices per outcome")
    confidence: float = Field(..., ge=0.0, le=1.0, description="Model confidence")
    adjustment_reason: str = Field(..., description="Reason for price adjustment")


class LiquidityAdjustmentRequest(BaseModel):
    """Request for liquidity optimization"""
    event_id: str
    current_liquidity: float = Field(..., description="Current liquidity parameter (b)")
    current_prices: Dict[str, float]
    total_volume: float
    price_volatility: float = Field(..., description="Price volatility measure")


class LiquidityAdjustmentResponse(BaseModel):
    """Response with recommended liquidity parameter"""
    event_id: str
    recommended_liquidity: float = Field(..., description="Recommended liquidity parameter (b)")
    adjustment_amount: float = Field(..., description="Change from current liquidity")
    reason: str = Field(..., description="Reason for adjustment")


class HealthResponse(BaseModel):
    """Health check response"""
    status: str
    backend_connected: bool
    models_loaded: bool
    cache_size: int
    last_update: Optional[str] = None


# Background task to fetch market state
async def fetch_market_state_periodically(interval_seconds: int):
    """Periodically fetch market state from backend"""
    global backend_client, market_state_cache

    while True:
        try:
            if backend_client and market_state_cache:
                # Fetch active events and their states
                events = await backend_client.get_active_events()
                
                for event in events:
                    # Get current prices and volume
                    prices = await backend_client.get_event_prices(event["id"])
                    volume = await backend_client.get_event_volume(event["id"])
                    
                    # Update cache
                    market_state = MarketState(
                        event_id=event["id"],
                        prices=prices,
                        total_volume=volume,
                        bet_count=event.get("bet_count", 0),
                        created_at=event.get("created_at"),
                    )
                    market_state_cache.update(event["id"], market_state)

                logger.debug(f"Updated market state for {len(events)} events")
        except Exception as e:
            logger.error(f"Error fetching market state: {e}")

        await asyncio.sleep(interval_seconds)


# API Endpoints
@app.post("/predict-prices", response_model=PricePredictionResponse)
async def predict_prices(request: EventStateRequest):
    """
    Predict optimal prices for an event based on ML models
    
    Uses:
    - Probability calibration from historical data
    - Demand forecasting
    - Current market state
    """
    global price_predictor, market_state_cache

    if not price_predictor:
        raise HTTPException(status_code=503, detail="Price predictor not initialized")

    try:
        # Get historical context from cache if available
        historical_data = None
        if market_state_cache:
            historical_data = market_state_cache.get_history(request.event_id)

        # Predict optimal prices
        recommended_prices, confidence, reason = price_predictor.predict(
            current_prices=request.current_prices,
            total_volume=request.total_volume,
            bet_count=request.bet_count,
            time_since_creation=request.time_since_creation,
            historical_data=historical_data,
        )

        return PricePredictionResponse(
            event_id=request.event_id,
            recommended_prices=recommended_prices,
            confidence=confidence,
            adjustment_reason=reason,
        )
    except Exception as e:
        logger.error(f"Error predicting prices: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@app.post("/adjust-liquidity", response_model=LiquidityAdjustmentResponse)
async def adjust_liquidity(request: LiquidityAdjustmentRequest):
    """
    Recommend liquidity parameter adjustments
    
    Analyzes:
    - Price volatility
    - Trading volume
    - Market depth
    """
    global liquidity_optimizer

    if not liquidity_optimizer:
        raise HTTPException(status_code=503, detail="Liquidity optimizer not initialized")

    try:
        recommended_liquidity, adjustment, reason = liquidity_optimizer.optimize(
            current_liquidity=request.current_liquidity,
            current_prices=request.current_prices,
            total_volume=request.total_volume,
            price_volatility=request.price_volatility,
        )

        return LiquidityAdjustmentResponse(
            event_id=request.event_id,
            recommended_liquidity=recommended_liquidity,
            adjustment_amount=adjustment,
            reason=reason,
        )
    except Exception as e:
        logger.error(f"Error optimizing liquidity: {e}")
        raise HTTPException(status_code=500, detail=str(e))


@app.get("/health", response_model=HealthResponse)
async def health_check():
    """Health check endpoint"""
    global backend_client, market_state_cache, price_predictor

    backend_connected = False
    if backend_client:
        try:
            backend_connected = await backend_client.health_check()
        except Exception:
            pass

    models_loaded = (
        price_predictor is not None
        and demand_forecaster is not None
        and liquidity_optimizer is not None
    )

    cache_size = 0
    last_update = None
    if market_state_cache:
        cache_size = market_state_cache.size()
        last_update = market_state_cache.last_update()

    return HealthResponse(
        status="healthy" if (backend_connected and models_loaded) else "degraded",
        backend_connected=backend_connected,
        models_loaded=models_loaded,
        cache_size=cache_size,
        last_update=last_update,
    )


@app.get("/")
async def root():
    """Root endpoint"""
    return {
        "service": "Mitra ML Service",
        "version": "0.1.0",
        "endpoints": {
            "predict_prices": "/predict-prices",
            "adjust_liquidity": "/adjust-liquidity",
            "health": "/health",
        },
    }


if __name__ == "__main__":
    import os
    import uvicorn

    port = int(os.getenv("PORT", "8000"))
    uvicorn.run(app, host="0.0.0.0", port=port)

