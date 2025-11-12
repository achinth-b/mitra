from datetime import datetime
from typing import Optional
from uuid import uuid4

from sqlalchemy import String, Integer, DateTime, ForeignKey
from sqlalchemy.orm import Mapped, mapped_column, relationship

from app.database import Base


class Transaction(Base):
    __tablename__ = "transactions"
    
    id: Mapped[str] = mapped_column(String(36), primary_key=True, default=lambda: str(uuid4()))
    user_id: Mapped[str] = mapped_column(String(36), ForeignKey("users.id"), index=True)
    type: Mapped[str] = mapped_column(String(50))  # bet_placed, bet_won, market_created, invite_bonus, stake_returned
    amount: Mapped[int] = mapped_column(Integer)  # Can be positive or negative
    market_id: Mapped[Optional[str]] = mapped_column(String(36), ForeignKey("markets.id"), nullable=True, index=True)
    reference_id: Mapped[Optional[str]] = mapped_column(String(36), nullable=True)  # FK to bets or other entities
    description: Mapped[str] = mapped_column(String(200))
    created_at: Mapped[datetime] = mapped_column(DateTime, default=datetime.utcnow, index=True)
    
    # Relationships
    user: Mapped["User"] = relationship(back_populates="transactions")
    market: Mapped[Optional["Market"]] = relationship()

