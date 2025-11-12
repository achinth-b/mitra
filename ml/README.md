# Mitra ML Components

Machine learning components for social prediction markets.

## Overview

This directory contains ML implementations for:

1. **AMM (Automated Market Maker)**: LMSR-based market maker with dynamic liquidity
2. **Recommender**: Collaborative filtering for market recommendations
3. **Graph**: Social graph analysis for user similarity
4. **Analytics**: User stats and market metrics

## Structure

```
ml/
├── amm/
│   ├── lmsr.py              # LMSR market maker implementation
│   └── dynamic_liquidity.py # ML-enhanced liquidity adjustment
├── recommender/
│   ├── collaborative_filtering.py  # User-item CF
│   └── market_ranker.py     # Market ranking for recommendations
├── graph/
│   ├── social_graph.py      # NetworkX graph construction
│   └── user_similarity.py   # User similarity metrics
└── analytics/
    ├── user_stats.py        # User accuracy and insights
    └── market_metrics.py    # Market engagement metrics
```

## Installation

```bash
cd ml
pip install -r requirements.txt
```

## Usage

ML components are imported and used by the backend services:

```python
from ml.amm.lmsr import LMSRMarketMaker
from ml.recommender.collaborative_filtering import MarketRecommender
from ml.graph.social_graph import SocialGraph
```

## Development

1. **AMM (Week 2)**: Implement LMSR for binary and multi-outcome markets
2. **Betting Integration (Week 2-3)**: Connect AMM to betting service
3. **Recommendations (Week 5)**: Build collaborative filtering
4. **Analytics (Week 5)**: Generate user insights and leaderboards

## Testing

```bash
pytest ml/tests/
```

## Phase 1 Scope

For MVP, keep ML simple:
- **LMSR with fixed liquidity parameter** (no ML adjustment initially)
- **Basic collaborative filtering** (k-NN on user-item matrix)
- **Simple graph metrics** (centrality, clustering)
- **Statistical user stats** (win rate, accuracy)

Advanced ML (graph neural networks, deep RL) → Phase 2

## Notes

- ML components should be stateless and deterministic where possible
- Cache expensive computations (graph analysis, recommendations)
- Log all predictions for model improvement
- Start with simple models, layer complexity as data grows

