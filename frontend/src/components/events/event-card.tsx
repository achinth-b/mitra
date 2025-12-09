'use client';

import { motion } from 'framer-motion';
import { TrendingUp, Clock, CheckCircle, XCircle } from 'lucide-react';
import { Card } from '@/components/ui/card';
import { formatPrice, formatDate, formatUsd } from '@/lib/utils';
import type { Event, Prices } from '@/types';

interface EventCardProps {
  event: Event;
  prices?: Prices;
  onClick: () => void;
  delay?: number;
}

export function EventCard({ event, prices, onClick, delay = 0 }: EventCardProps) {
  const getStatusBadge = () => {
    switch (event.status) {
      case 'active':
        return (
          <span className="flex items-center gap-1 px-2 py-1 rounded-full bg-emerald-500/20 text-emerald-400 text-xs">
            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400 animate-pulse" />
            Live
          </span>
        );
      case 'resolved':
        return (
          <span className="flex items-center gap-1 px-2 py-1 rounded-full bg-blue-500/20 text-blue-400 text-xs">
            <CheckCircle className="w-3 h-3" />
            Resolved
          </span>
        );
      case 'cancelled':
        return (
          <span className="flex items-center gap-1 px-2 py-1 rounded-full bg-rose-500/20 text-rose-400 text-xs">
            <XCircle className="w-3 h-3" />
            Cancelled
          </span>
        );
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay }}
    >
      <Card hover onClick={onClick} className="overflow-hidden">
        <div className="p-5">
          {/* Header */}
          <div className="flex items-start justify-between mb-4">
            <div className="flex-1">
              <h3 className="text-lg font-semibold text-white mb-1 line-clamp-2">
                {event.title}
              </h3>
              {event.description && (
                <p className="text-sm text-slate-400 line-clamp-1">
                  {event.description}
                </p>
              )}
            </div>
            {getStatusBadge()}
          </div>

          {/* Prices */}
          {prices && event.status === 'active' && (
            <div className="grid grid-cols-2 gap-2 mb-4">
              {event.outcomes.map((outcome) => {
                const price = prices.prices[outcome] || 0.5;
                const isHigh = price > 0.5;
                return (
                  <div
                    key={outcome}
                    className={`p-3 rounded-xl border ${
                      isHigh
                        ? 'bg-emerald-500/10 border-emerald-500/30'
                        : 'bg-rose-500/10 border-rose-500/30'
                    }`}
                  >
                    <p className="text-xs text-slate-400 mb-1">{outcome}</p>
                    <p className={`text-2xl font-bold ${isHigh ? 'text-emerald-400' : 'text-rose-400'}`}>
                      {formatPrice(price)}
                    </p>
                  </div>
                );
              })}
            </div>
          )}

          {/* Resolved outcome */}
          {event.status === 'resolved' && event.winningOutcome && (
            <div className="p-3 rounded-xl bg-blue-500/10 border border-blue-500/30 mb-4">
              <p className="text-xs text-slate-400 mb-1">Winner</p>
              <p className="text-lg font-bold text-blue-400">{event.winningOutcome}</p>
            </div>
          )}

          {/* Footer */}
          <div className="flex items-center justify-between text-sm text-slate-400">
            <div className="flex items-center gap-1">
              <TrendingUp className="w-4 h-4" />
              <span>{prices ? formatUsd(prices.totalVolume) : '$0'} volume</span>
            </div>
            {event.resolveBy && (
              <div className="flex items-center gap-1">
                <Clock className="w-4 h-4" />
                <span>Resolves {formatDate(event.resolveBy)}</span>
              </div>
            )}
          </div>
        </div>
      </Card>
    </motion.div>
  );
}

