from datetime import datetime
from typing import Optional
from uuid import uuid4

from sqlalchemy import String, Integer, Boolean, DateTime, ForeignKey, Float
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.database import Base


class Market(Base):
    __tablename__ = "markets"
    
    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=lambda: str(uuid4()))
    group_id: Mapped[str] = mapped_column(String(36), ForeignKey("groups.id"), index=True)
    creator_id: Mapped[str] = mapped_column(String(36), ForeignKey("users.id"), index=True)
    title: Mapped[str] = mapped_column(String(200))
    description: Mapped[Optional[str]] = mapped_column(String(1000), nullable=True)
    market_type: Mapped[str] = mapped_column(String(20))  # binary, multi_outcome
    outcomes: Mapped[str] = mapped_column(String)  # JSON array
    stake_amount: Mapped[int] = mapped_column(Integer)  # Creator's stake in credits
    resolution_method: Mapped[str] = mapped_column(String(20))  # manual, democratic, both
    end_date: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)
    resolved: Mapped[bool] = mapped_column(Boolean, default=False, index=True)
    resolved_outcome: Mapped[Optional[str]] = mapped_column(String(100), nullable=True)
    resolved_at: Mapped[Optional[datetime]] = mapped_column(DateTime, nullable=True)
    created_at: Mapped[datetime] = mapped_column(DateTime, default=datetime.utcnow, index=True)
    
    # Relationships
    group: Mapped["Group"] = relationship(back_populates="markets")
    creator: Mapped["User"] = relationship(back_populates="created_markets", foreign_keys=[creator_id])
    bets: Mapped[list["Bet"]] = relationship(back_populates="market", cascade="all, delete-orphan")
    state: Mapped[Optional["MarketState"]] = relationship(back_populates="market", uselist=False, cascade="all, delete-orphan")
    resolutions: Mapped[list["Resolution"]] = relationship(back_populates="market", cascade="all, delete-orphan")


class MarketState(Base):
    __tablename__ = "market_state"
    
    market_id: Mapped[str] = mapped_column(String(36), ForeignKey("markets.id"), primary_key=True)
    current_odds: Mapped[str] = mapped_column(String)  # JSON object
    total_volume: Mapped[int] = mapped_column(Integer, default=0)
    liquidity_param: Mapped[float] = mapped_column(Float)
    share_counts: Mapped[str] = mapped_column(String)  # JSON object
    unique_bettors: Mapped[int] = mapped_column(Integer, default=0)
    updated_at: Mapped[datetime] = mapped_column(DateTime, default=datetime.utcnow, onupdate=datetime.utcnow)
    
    # Relationships
    market: Mapped["Market"] = relationship(back_populates="state")


class Bet(Base):
    __tablename__ = "bets"
    
    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=lambda: str(uuid4()))
    market_id: Mapped[str] = mapped_column(String(36), ForeignKey("markets.id"), index=True)
    user_id: Mapped[str] = mapped_column(String(36), ForeignKey("users.id"), index=True)
    outcome: Mapped[str] = mapped_column(String(100))
    shares: Mapped[float] = mapped_column(Float)
    amount_paid: Mapped[int] = mapped_column(Integer)  # Credits in cents
    odds_at_purchase: Mapped[float] = mapped_column(Float)
    created_at: Mapped[datetime] = mapped_column(DateTime, default=datetime.utcnow, index=True)
    
    # Relationships
    market: Mapped["Market"] = relationship(back_populates="bets")
    user: Mapped["User"] = relationship(back_populates="bets")


class Resolution(Base):
    __tablename__ = "resolutions"
    
    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=lambda: str(uuid4()))
    market_id: Mapped[str] = mapped_column(String(36), ForeignKey("markets.id"), index=True)
    proposed_by: Mapped[str] = mapped_column(String(36), ForeignKey("users.id"))
    proposed_outcome: Mapped[str] = mapped_column(String(100))
    status: Mapped[str] = mapped_column(String(20))  # pending, voting, accepted, rejected
    votes: Mapped[Optional[str]] = mapped_column(String, nullable=True)  # JSON object
    created_at: Mapped[datetime] = mapped_column(DateTime, default=datetime.utcnow)
    
    # Relationships
    market: Mapped["Market"] = relationship(back_populates="resolutions")
    proposer: Mapped["User"] = relationship()

