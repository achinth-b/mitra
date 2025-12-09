'use client';

import { useState, useEffect } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { TrendingUp, TrendingDown, DollarSign, Zap } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Card, CardContent } from '@/components/ui/card';
import { useAuthStore } from '@/store/auth';
import { signMessage } from '@/lib/magic';
import { formatPrice, formatUsd } from '@/lib/utils';
import type { Event, Prices } from '@/types';

interface BetInterfaceProps {
  event: Event;
  prices: Prices;
  onBetPlaced: () => void;
}

export function BetInterface({ event, prices, onBetPlaced }: BetInterfaceProps) {
  const [selectedOutcome, setSelectedOutcome] = useState<string | null>(null);
  const [amount, setAmount] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [estimatedShares, setEstimatedShares] = useState(0);
  const { user } = useAuthStore();

  // Calculate estimated shares when amount changes
  useEffect(() => {
    if (selectedOutcome && amount) {
      const amountNum = parseFloat(amount);
      const price = prices.prices[selectedOutcome] || 0.5;
      // Simple estimation: shares = amount / price
      setEstimatedShares(amountNum / price);
    } else {
      setEstimatedShares(0);
    }
  }, [selectedOutcome, amount, prices]);

  const handlePlaceBet = async () => {
    if (!selectedOutcome || !amount || !user.walletAddress) return;

    setIsLoading(true);
    setError('');

    try {
      const signature = await signMessage(
        `Place bet: ${amount} USDC on ${selectedOutcome}`
      );

      const response = await fetch('/api/grpc', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'PlaceBet',
          data: {
            event_id: event.eventId,
            user_wallet: user.walletAddress,
            outcome: selectedOutcome,
            amount_usdc: parseFloat(amount),
            signature,
          },
        }),
      });

      if (!response.ok) {
        throw new Error(await response.text());
      }

      const result = await response.json();
      console.log('Bet placed:', result);
      
      // Reset form
      setSelectedOutcome(null);
      setAmount('');
      onBetPlaced();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to place bet');
    } finally {
      setIsLoading(false);
    }
  };

  const quickAmounts = [10, 25, 50, 100];

  return (
    <Card className="overflow-hidden">
      <CardContent className="p-6">
        <h3 className="text-lg font-semibold text-white mb-4 flex items-center gap-2">
          <Zap className="w-5 h-5 text-amber-400" />
          Place Your Bet
        </h3>

        {/* Outcome Selection */}
        <div className="grid grid-cols-2 gap-3 mb-6">
          {event.outcomes.map((outcome) => {
            const price = prices.prices[outcome] || 0.5;
            const isSelected = selectedOutcome === outcome;
            const isYes = outcome.toUpperCase() === 'YES' || price > 0.5;

            return (
              <motion.button
                key={outcome}
                whileHover={{ scale: 1.02 }}
                whileTap={{ scale: 0.98 }}
                onClick={() => setSelectedOutcome(outcome)}
                className={`p-4 rounded-xl border-2 transition-all duration-200 ${
                  isSelected
                    ? isYes
                      ? 'bg-emerald-500/20 border-emerald-500'
                      : 'bg-rose-500/20 border-rose-500'
                    : 'bg-slate-800/50 border-slate-700 hover:border-slate-600'
                }`}
              >
                <div className="flex items-center justify-between mb-2">
                  <span className="font-medium text-white">{outcome}</span>
                  {isYes ? (
                    <TrendingUp className={`w-5 h-5 ${isSelected ? 'text-emerald-400' : 'text-slate-500'}`} />
                  ) : (
                    <TrendingDown className={`w-5 h-5 ${isSelected ? 'text-rose-400' : 'text-slate-500'}`} />
                  )}
                </div>
                <p className={`text-2xl font-bold ${
                  isSelected
                    ? isYes ? 'text-emerald-400' : 'text-rose-400'
                    : 'text-slate-300'
                }`}>
                  {formatPrice(price)}
                </p>
              </motion.button>
            );
          })}
        </div>

        {/* Amount Input */}
        <AnimatePresence>
          {selectedOutcome && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: 'auto' }}
              exit={{ opacity: 0, height: 0 }}
              className="space-y-4"
            >
              <div>
                <label className="block text-sm font-medium text-slate-300 mb-2">
                  Amount (USDC)
                </label>
                <div className="relative">
                  <DollarSign className="absolute left-3 top-1/2 -translate-y-1/2 w-5 h-5 text-slate-500" />
                  <input
                    type="number"
                    value={amount}
                    onChange={(e) => setAmount(e.target.value)}
                    placeholder="0.00"
                    className="w-full pl-10 pr-4 py-3 rounded-xl bg-slate-800/50 border border-slate-700/50 text-white placeholder-slate-500 focus:outline-none focus:ring-2 focus:ring-emerald-500/50 text-lg"
                  />
                </div>
              </div>

              {/* Quick amounts */}
              <div className="flex gap-2">
                {quickAmounts.map((quickAmount) => (
                  <Button
                    key={quickAmount}
                    variant="secondary"
                    size="sm"
                    onClick={() => setAmount(quickAmount.toString())}
                    className="flex-1"
                  >
                    ${quickAmount}
                  </Button>
                ))}
              </div>

              {/* Estimated return */}
              {amount && parseFloat(amount) > 0 && (
                <div className="p-4 rounded-xl bg-slate-800/30 border border-slate-700/30">
                  <div className="flex justify-between text-sm text-slate-400 mb-2">
                    <span>Estimated Shares</span>
                    <span className="text-white font-medium">
                      {estimatedShares.toFixed(2)}
                    </span>
                  </div>
                  <div className="flex justify-between text-sm text-slate-400">
                    <span>Potential Payout (if wins)</span>
                    <span className="text-emerald-400 font-medium">
                      {formatUsd(estimatedShares)}
                    </span>
                  </div>
                </div>
              )}

              {error && (
                <p className="text-sm text-rose-400">{error}</p>
              )}

              <Button
                className="w-full"
                size="lg"
                onClick={handlePlaceBet}
                isLoading={isLoading}
                disabled={!amount || parseFloat(amount) <= 0}
              >
                Place Bet on {selectedOutcome}
              </Button>
            </motion.div>
          )}
        </AnimatePresence>

        {!selectedOutcome && (
          <p className="text-center text-slate-500 py-4">
            Select an outcome above to place a bet
          </p>
        )}
      </CardContent>
    </Card>
  );
}

