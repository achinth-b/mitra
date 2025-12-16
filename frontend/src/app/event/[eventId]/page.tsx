'use client';

import { useEffect, useState, useCallback } from 'react';
import { useRouter, useParams } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import { getEvent, getEventPrices, placeBet, getUserBets, settleEvent, getPublicBets } from '@/lib/api';
import { signMessage } from '@/lib/magic';
import type { Event, Prices, Bet } from '@/types';

export default function EventPage() {
  const router = useRouter();
  const params = useParams();
  const eventId = params.eventId as string;

  const { user, checkAuth, isLoading: authLoading } = useAuthStore();
  const [event, setEvent] = useState<Event | null>(null);
  const [prices, setPrices] = useState<Prices | null>(null);
  const [userBets, setUserBets] = useState<Bet[]>([]);

  // Betting state
  const [selectedOutcome, setSelectedOutcome] = useState<string | null>(null);
  const [amount, setAmount] = useState('');
  const [isPublic, setIsPublic] = useState(false);
  const [isPlacing, setIsPlacing] = useState(false);
  const [lastBet, setLastBet] = useState<{ shares: number; price: number } | null>(null);
  const [publicBets, setPublicBets] = useState<Bet[]>([]);

  // Settlement state
  const [showSettle, setShowSettle] = useState(false);
  const [settleOutcome, setSettleOutcome] = useState<string | null>(null);
  const [isSettling, setIsSettling] = useState(false);

  useEffect(() => {
    checkAuth();
  }, [checkAuth]);

  useEffect(() => {
    if (!user.isLoggedIn && !authLoading) {
      router.push('/');
    }
  }, [user.isLoggedIn, authLoading, router]);

  // Load event
  useEffect(() => {
    getEvent(eventId).then(setEvent);
  }, [eventId]);

  // Load user's bets and public bets
  useEffect(() => {
    if (user.walletAddress) {
      setUserBets(getUserBets(eventId, user.walletAddress));
    }
    setPublicBets(getPublicBets(eventId));
  }, [eventId, user.walletAddress]);

  // Fetch prices
  const fetchPrices = useCallback(async () => {
    const p = await getEventPrices(eventId);
    setPrices(p);
  }, [eventId]);

  useEffect(() => {
    fetchPrices();
    const interval = setInterval(fetchPrices, 5000);
    return () => clearInterval(interval);
  }, [fetchPrices]);

  const handlePlaceBet = async () => {
    if (!selectedOutcome || !amount || !user.walletAddress || !event) return;

    const amountNum = parseFloat(amount);
    if (isNaN(amountNum) || amountNum <= 0) return;

    setIsPlacing(true);
    setLastBet(null);

    try {
      // Sign the bet
      const sig = await signMessage(`bet:${eventId}:${selectedOutcome}:${amountNum}`);

      const result = await placeBet(
        eventId,
        user.walletAddress,
        selectedOutcome,
        amountNum,
        sig,
        isPublic
      );

      setLastBet({ shares: result.shares, price: result.price });
      setPrices(result.updatedPrices);
      setUserBets(getUserBets(eventId, user.walletAddress));
      setPublicBets(getPublicBets(eventId));

      // Reset form
      setSelectedOutcome(null);
      setAmount('');
      setIsPublic(false);
    } catch (err) {
      console.error('Bet failed:', err);
    } finally {
      setIsPlacing(false);
    }
  };

  const handleSettle = async () => {
    if (!settleOutcome || !user.walletAddress || !event) return;

    setIsSettling(true);

    try {
      const sig = await signMessage(`settle:${eventId}:${settleOutcome}`);
      await settleEvent(eventId, settleOutcome, user.walletAddress, sig);

      // Refresh event
      const updated = await getEvent(eventId);
      if (updated) setEvent(updated);

      setShowSettle(false);
      setSettleOutcome(null);
    } catch (err) {
      console.error('Settlement failed:', err);
    } finally {
      setIsSettling(false);
    }
  };

  const formatPrice = (price: number) => `${Math.round(price * 100)}%`;
  const formatUsd = (val: number) => `$${val.toFixed(2)}`;

  // Calculate user's position
  const userPosition = userBets.reduce((acc, bet) => {
    acc[bet.outcome] = (acc[bet.outcome] || 0) + bet.shares;
    return acc;
  }, {} as Record<string, number>);

  const totalInvested = userBets.reduce((sum, b) => sum + b.amountUsdc, 0);

  if (authLoading || !event) {
    return (
      <main className="min-h-screen flex items-center justify-center">
        <p className="text-2xl text-white/40 italic">loading...</p>
      </main>
    );
  }

  const isActive = event.status === 'active';

  return (
    <main className="min-h-screen bg-[#080808] flex justify-center">
      <div className="w-full max-w-2xl px-6 py-16">

        {/* Back Button */}
        <button
          onClick={() => router.back()}
          className="text-sm text-white/40 hover:text-white/70 transition-colors mb-12 flex items-center gap-2 group"
        >
          <span className="group-hover:-translate-x-1 transition-transform">←</span>
          back
        </button>

        {/* Title Section - Centered */}
        <header className="text-center mb-16">
          <h1 className="text-3xl md:text-5xl font-light leading-tight mb-6 text-white text-center">
            {event.title}
          </h1>

          <div className="flex items-center justify-center gap-6 flex-wrap">
            {isActive && prices && (
              <div className="flex items-center gap-2">
                <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />
                <span className="text-sm text-emerald-400 font-medium uppercase tracking-wider">Live</span>
              </div>
            )}

            {isActive && prices && (
              <span className="text-sm text-white/50">
                <span className="text-white font-mono text-lg">{formatUsd(prices.totalVolume)}</span> volume
              </span>
            )}

            {event.status === 'resolved' && (
              <div className="flex items-center gap-2 px-4 py-2 rounded-full bg-blue-500/15 border border-blue-500/30">
                <span className="text-sm text-blue-400 uppercase tracking-wide">Winner:</span>
                <span className="text-lg text-blue-400 font-semibold">{event.winningOutcome}</span>
              </div>
            )}
          </div>
        </header>

        {/* Odds Section */}
        {isActive && prices && (
          <section className="mb-16">

            {/* Big Odds Display - Centered */}
            <div className="grid grid-cols-2 gap-6 mb-12">
              {event.outcomes.map((outcome, idx) => {
                const price = prices.prices[outcome] || 0.5;
                const isSelected = selectedOutcome === outcome;
                const isFirst = idx === 0;

                return (
                  <button
                    key={outcome}
                    onClick={() => setSelectedOutcome(isSelected ? null : outcome)}
                    className={`relative p-8 rounded-3xl border-2 transition-all duration-300 text-center overflow-hidden ${isSelected
                      ? isFirst
                        ? 'border-emerald-500 bg-emerald-500/10 shadow-lg shadow-emerald-500/20'
                        : 'border-rose-500 bg-rose-500/10 shadow-lg shadow-rose-500/20'
                      : 'border-white/10 bg-white/5 hover:border-white/20 hover:bg-white/8'
                      }`}
                  >
                    {/* Glow effect */}
                    <div className={`absolute inset-0 blur-2xl opacity-30 ${isFirst ? 'bg-emerald-500' : 'bg-rose-500'
                      } ${isSelected ? 'opacity-40' : 'opacity-0'} transition-opacity`} />

                    <div className="relative">
                      <p className={`text-5xl md:text-7xl font-bold tabular-nums mb-3 ${isFirst ? 'text-emerald-400' : 'text-rose-400'
                        }`}>
                        {formatPrice(price)}
                      </p>
                      <p className="text-xl text-white/60 capitalize font-light">{outcome}</p>

                      {isSelected && (
                        <div className="absolute -top-2 -right-2">
                          <span className="text-xs bg-white text-black px-2 py-1 rounded-full font-semibold uppercase">
                            Selected
                          </span>
                        </div>
                      )}
                    </div>
                  </button>
                );
              })}
            </div>

            {/* Probability Bar - Centered */}
            <div className="mb-12">
              <div className="flex items-center justify-center gap-8 mb-4">
                {event.outcomes.map((outcome, idx) => {
                  const price = prices.prices[outcome] || 0.5;
                  const isFirst = idx === 0;
                  return (
                    <div key={outcome} className="flex items-center gap-2">
                      <div className={`w-3 h-3 rounded-full ${isFirst ? 'bg-emerald-500' : 'bg-rose-500'}`} />
                      <span className="text-sm text-white/60 capitalize">{outcome}</span>
                      <span className={`text-sm font-bold tabular-nums ${isFirst ? 'text-emerald-400' : 'text-rose-400'}`}>
                        {formatPrice(price)}
                      </span>
                    </div>
                  );
                })}
              </div>

              {/* Bar */}
              <div className="h-4 rounded-full overflow-hidden bg-white/5 border border-white/10 flex">
                {event.outcomes.map((outcome, idx) => {
                  const price = prices.prices[outcome] || 0.5;
                  const isFirst = idx === 0;
                  return (
                    <div
                      key={outcome}
                      className={`h-full transition-all duration-700 ease-out ${isFirst
                        ? 'bg-gradient-to-r from-emerald-600 to-emerald-400'
                        : 'bg-gradient-to-r from-rose-400 to-rose-600'
                        }`}
                      style={{ width: `${price * 100}%` }}
                    />
                  );
                })}
              </div>
            </div>

            {/* Price History Chart */}
            <div className="p-6 rounded-2xl bg-white/[0.02] border border-white/10">
              <div className="flex items-center justify-between mb-4">
                <h3 className="text-sm font-medium text-white/40 uppercase tracking-wider">Price History</h3>
                <div className="flex items-center gap-4 text-xs">
                  <span className="flex items-center gap-1.5">
                    <div className="w-2 h-2 rounded-full bg-emerald-500" />
                    <span className="text-white/50 capitalize">{event.outcomes[0]}</span>
                  </span>
                  <span className="flex items-center gap-1.5">
                    <div className="w-2 h-2 rounded-full bg-rose-500" />
                    <span className="text-white/50 capitalize">{event.outcomes[1]}</span>
                  </span>
                </div>
              </div>

              {/* Chart Container */}
              <div className="relative">
                {/* Y-axis labels */}
                <div className="absolute left-0 top-0 bottom-6 w-10 flex flex-col justify-between text-right pr-2">
                  <span className="text-[10px] text-white/30 font-mono">100%</span>
                  <span className="text-[10px] text-white/30 font-mono">75%</span>
                  <span className="text-[10px] text-white/30 font-mono">50%</span>
                  <span className="text-[10px] text-white/30 font-mono">25%</span>
                  <span className="text-[10px] text-white/30 font-mono">0%</span>
                </div>

                {/* Chart Area */}
                <div className="ml-12 h-40 relative">
                  {/* Horizontal grid lines */}
                  <div className="absolute inset-0 flex flex-col justify-between pointer-events-none">
                    {[0, 1, 2, 3, 4].map((i) => (
                      <div key={i} className="h-px bg-white/5 w-full" />
                    ))}
                  </div>

                  {/* SVG Chart */}
                  <svg className="absolute inset-0 w-full h-full" preserveAspectRatio="none" viewBox="0 0 100 100">
                    {/* Yes line */}
                    <polyline
                      fill="none"
                      stroke="rgb(52, 211, 153)"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      points={`0,50 12,48 25,52 37,45 50,47 62,42 75,48 87,52 100,${100 - (prices.prices[event.outcomes[0]] || 0.5) * 100}`}
                    />
                    {/* No line */}
                    <polyline
                      fill="none"
                      stroke="rgb(251, 113, 133)"
                      strokeWidth="2"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                      points={`0,50 12,52 25,48 37,55 50,53 62,58 75,52 87,48 100,${100 - (prices.prices[event.outcomes[1]] || 0.5) * 100}`}
                    />
                  </svg>

                  {/* Current price indicators on right edge */}
                  <div className="absolute right-0 top-1/2 -translate-y-1/2 translate-x-2 flex flex-col gap-1">
                    <span className="text-[10px] font-mono text-emerald-400 bg-[#0a0a0a] px-1 rounded">
                      {Math.round((prices.prices[event.outcomes[0]] || 0.5) * 100)}%
                    </span>
                    <span className="text-[10px] font-mono text-rose-400 bg-[#0a0a0a] px-1 rounded">
                      {Math.round((prices.prices[event.outcomes[1]] || 0.5) * 100)}%
                    </span>
                  </div>
                </div>

                {/* X-axis date labels */}
                <div className="ml-12 flex justify-between mt-2 text-[10px] text-white/30 font-mono">
                  <span>{new Date(event.createdAt * 1000).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}</span>
                  <span>{new Date((event.createdAt + (Date.now() / 1000 - event.createdAt) * 0.25) * 1000).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}</span>
                  <span>{new Date((event.createdAt + (Date.now() / 1000 - event.createdAt) * 0.5) * 1000).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}</span>
                  <span>{new Date((event.createdAt + (Date.now() / 1000 - event.createdAt) * 0.75) * 1000).toLocaleDateString('en-US', { month: 'short', day: 'numeric' })}</span>
                  <span>Now</span>
                </div>
              </div>
            </div>
          </section>
        )}

        {/* Place Bet */}
        {isActive && selectedOutcome && (
          <section className="mb-16 pb-16 border-b border-white/10">
            <h2 className="text-xl text-white/40 mb-6">
              bet on <span className="text-white">{selectedOutcome}</span>
            </h2>

            <div className="mb-6">
              <input
                type="number"
                value={amount}
                onChange={(e) => setAmount(e.target.value)}
                placeholder="amount (USDC)"
                className="w-full text-3xl py-4 border-b border-white/30 focus:border-white/70 transition-colors bg-transparent"
                autoFocus
              />
            </div>

            {/* Quick amounts */}
            <div className="flex gap-6 mb-8">
              {[10, 25, 50, 100].map((quick) => (
                <button
                  key={quick}
                  onClick={() => setAmount(quick.toString())}
                  className="text-lg text-white/40 hover:text-white transition-opacity"
                >
                  ${quick}
                </button>
              ))}
            </div>

            {/* Potential payout */}
            {amount && parseFloat(amount) > 0 && prices && (
              <p className="text-lg text-white/60 mb-8">
                if {selectedOutcome} wins, you get{' '}
                <span className="text-white text-xl">
                  {formatUsd(parseFloat(amount) / (prices.prices[selectedOutcome] || 0.5))}
                </span>
              </p>
            )}

            {/* Visibility toggle */}
            <label className="flex items-center gap-3 mb-8 cursor-pointer">
              <input
                type="checkbox"
                checked={isPublic}
                onChange={(e) => setIsPublic(e.target.checked)}
                className="w-5 h-5 accent-white"
              />
              <span className="text-lg text-white/60">
                share this bet with my group
              </span>
            </label>

            <button
              onClick={handlePlaceBet}
              disabled={!amount || parseFloat(amount) <= 0 || isPlacing}
              className="text-2xl italic text-white/60 hover:text-white transition-opacity disabled:text-white/30"
            >
              {isPlacing ? 'placing bet...' : 'place bet →'}
            </button>
          </section>
        )}

        {/* Last Bet Confirmation */}
        {lastBet && (
          <section className="mb-16 p-6 border border-white/20">
            <p className="text-xl text-white/60 mb-2">bet placed!</p>
            <p className="text-lg">
              {lastBet.shares.toFixed(2)} shares at {formatPrice(lastBet.price)}
            </p>
          </section>
        )}

        {/* Your Position */}
        {userBets.length > 0 && (
          <section className="mb-16">
            <h2 className="text-xl text-white/40 mb-6">your position</h2>

            <div className="space-y-4 mb-6">
              {Object.entries(userPosition).map(([outcome, shares]) => (
                <div key={outcome} className="flex justify-between text-lg">
                  <span>{outcome}</span>
                  <span>{shares.toFixed(2)} shares</span>
                </div>
              ))}
            </div>

            <p className="text-white/40">
              total invested: {formatUsd(totalInvested)}
            </p>

            {event.status === 'resolved' && event.winningOutcome && (
              <div className="mt-6 p-4 border border-white/20">
                <p className="text-lg">
                  {userPosition[event.winningOutcome] > 0 ? (
                    <>
                      you won{' '}
                      <span className="text-white text-xl">
                        {formatUsd(userPosition[event.winningOutcome])}
                      </span>
                    </>
                  ) : (
                    'you did not win this market'
                  )}
                </p>
              </div>
            )}
          </section>
        )}

        {/* Settlement (Admin only - simplified for demo) */}
        {isActive && (
          <section className="pt-16 border-t border-white/10">
            {!showSettle ? (
              <button
                onClick={() => setShowSettle(true)}
                className="text-lg text-white/30 hover:text-white/60 transition-opacity"
              >
                settle this market →
              </button>
            ) : (
              <div>
                <h2 className="text-xl mb-6">settle market</h2>
                <p className="text-white/40 mb-6">select the winning outcome:</p>

                <div className="flex gap-4 mb-8">
                  {event.outcomes.map((outcome) => (
                    <button
                      key={outcome}
                      onClick={() => setSettleOutcome(outcome)}
                      className={`px-6 py-3 border transition-colors ${settleOutcome === outcome
                        ? 'border-white text-white'
                        : 'border-white/20 text-white/60 hover:border-white/40'
                        }`}
                    >
                      {outcome}
                    </button>
                  ))}
                </div>

                <div className="flex gap-6">
                  <button
                    onClick={handleSettle}
                    disabled={!settleOutcome || isSettling}
                    className="text-xl text-white/60 hover:text-white transition-opacity disabled:text-white/30"
                  >
                    {isSettling ? 'settling...' : 'confirm settlement →'}
                  </button>
                  <button
                    onClick={() => {
                      setShowSettle(false);
                      setSettleOutcome(null);
                    }}
                    className="text-xl text-white/30 hover:text-white/60 transition-opacity"
                  >
                    cancel
                  </button>
                </div>
              </div>
            )}
          </section>
        )}

        {/* Public Bets from Group */}
        {publicBets.length > 0 && (
          <section className="mb-16 pt-16 border-t border-white/10">
            <h2 className="text-xl text-white/40 mb-6">group activity</h2>
            <ul className="space-y-3">
              {publicBets
                .filter(b => b.userId !== user.walletAddress) // Don't show own bets
                .slice(0, 10) // Limit to 10 most recent
                .map((bet) => (
                  <li
                    key={bet.betId}
                    className="flex items-center justify-between py-3 px-4 border border-white/10 rounded-lg"
                  >
                    <div>
                      <span className="text-white/60 font-mono text-sm">
                        {bet.userId.slice(0, 6)}...{bet.userId.slice(-4)}
                      </span>
                      <span className="text-white/40 mx-2">bet on</span>
                      <span className="text-white">{bet.outcome}</span>
                    </div>
                    <span className="text-white/60">
                      {bet.shares.toFixed(1)} shares
                    </span>
                  </li>
                ))}
            </ul>
          </section>
        )}
      </div>
    </main>
  );
}
