# Mitra ML Service

ML service for the Mitra prediction market platform that provides price predictions and liquidity optimization recommendations.

## Features

- **Price Prediction**: ML-based price adjustments using probability calibration
- **Demand Forecasting**: Predicts buy/sell pressure using moving averages
- **Liquidity Optimization**: Recommends optimal AMM liquidity parameters
- **Real-time Updates**: Fetches market state from backend every 1-5 seconds

## Prerequisites

- Python 3.11+
- Poetry (for dependency management)

## Setup

### Install Poetry

```bash
# macOS/Linux
curl -sSL https://install.python-poetry.org | python3 -

# Or via pip
pip install poetry
```

### Install Dependencies

```bash
# Install dependencies
poetry install

# Install with PyTorch support (optional, for future models)
poetry install --with torch
```

### Environment Variables

Create a `.env` file:

```bash
# Backend service URL
BACKEND_URL=http://localhost:50051

# Fetch interval (seconds)
FETCH_INTERVAL_SECONDS=3

# Service port
PORT=8000
```

## Running the Service

```bash
# Activate Poetry shell
poetry shell

# Run the service
poetry run python main.py

# Or with uvicorn directly
poetry run uvicorn main:app --host 0.0.0.0 --port 8000
```

## API Endpoints

### POST `/predict-prices`

Predict optimal prices for an event.

**Request:**
```json
{
  "event_id": "uuid",
  "current_prices": {"YES": 0.65, "NO": 0.35},
  "total_volume": 1000.0,
  "bet_count": 50,
  "time_since_creation": 24.5
}
```

**Response:**
```json
{
  "event_id": "uuid",
  "recommended_prices": {"YES": 0.67, "NO": 0.33},
  "confidence": 0.6,
  "adjustment_reason": "ML-adjusted prices based on market activity"
}
```

### POST `/adjust-liquidity`

Recommend liquidity parameter adjustments.

**Request:**
```json
{
  "event_id": "uuid",
  "current_liquidity": 100.0,
  "current_prices": {"YES": 0.65, "NO": 0.35},
  "total_volume": 1000.0,
  "price_volatility": 0.15
}
```

**Response:**
```json
{
  "event_id": "uuid",
  "recommended_liquidity": 115.0,
  "adjustment_amount": 15.0,
  "reason": "High volatility (0.15), increasing liquidity for stability"
}
```

### GET `/health`

Health check endpoint.

**Response:**
```json
{
  "status": "healthy",
  "backend_connected": true,
  "models_loaded": true,
  "cache_size": 42,
  "last_update": "2024-01-01T12:00:00"
}
```

## Development

### Code Formatting

```bash
poetry run black .
poetry run ruff check .
```

### Type Checking

```bash
poetry run mypy .
```

### Running Tests

```bash
poetry run pytest
```

## Project Structure

```
ml-service/
├── main.py                 # FastAPI application
├── pyproject.toml         # Poetry configuration
├── README.md              # This file
├── ml_models/             # ML model implementations
│   ├── __init__.py
│   ├── price_predictor.py
│   ├── demand_forecast.py
│   └── liquidity_optimizer.py
└── services/              # Service modules
    ├── __init__.py
    ├── backend_client.py
    └── market_state.py
```

## ML Models

### Current Models (Baseline)

1. **Price Predictor**: Pure AMM baseline (no ML adjustments)
2. **Demand Forecaster**: Simple moving averages
3. **Liquidity Optimizer**: Heuristic-based adjustments

### Future Models

- PyTorch-based neural networks
- LSTM for time series forecasting
- Transformer models for sequence prediction
- Reinforcement learning for liquidity optimization

## Notes

- Initially uses simple models (logistic regression, moving averages)
- Baseline mode returns current AMM prices without adjustments
- Models can be trained as historical data accumulates
- PyTorch support is optional and can be enabled later

