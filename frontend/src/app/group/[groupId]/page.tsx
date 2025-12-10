'use client';

import { useEffect, useState, useCallback } from 'react';
import { useRouter, useParams } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import { 
  getGroups, getEvents, saveEvents, createEvent, getEventPrices,
  getBalance, deposit, withdraw, formatUsdc, parseUsdc
} from '@/lib/api';
import type { FriendGroup, Event, Prices, BalanceResponse } from '@/types';

export default function GroupPage() {
  const router = useRouter();
  const params = useParams();
  const groupId = params.groupId as string;
  
  const { user, checkAuth, isLoading: authLoading } = useAuthStore();
  const [group, setGroup] = useState<FriendGroup | null>(null);
  const [events, setEvents] = useState<Event[]>([]);
  const [prices, setPrices] = useState<Record<string, Prices>>({});
  const [balance, setBalance] = useState<BalanceResponse | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [showDeposit, setShowDeposit] = useState(false);
  const [showWithdraw, setShowWithdraw] = useState(false);
  const [isCreating, setIsCreating] = useState(false);
  const [isTransacting, setIsTransacting] = useState(false);
  const [transactionAmount, setTransactionAmount] = useState('');
  const [transactionError, setTransactionError] = useState<string | null>(null);
  
  // Form state
  const [title, setTitle] = useState('');
  const [outcome1, setOutcome1] = useState('yes');
  const [outcome2, setOutcome2] = useState('no');

  useEffect(() => {
    checkAuth();
  }, [checkAuth]);

  useEffect(() => {
    if (!user.isLoggedIn && !authLoading) {
      router.push('/');
    }
  }, [user.isLoggedIn, authLoading, router]);

  // Load group and events
  useEffect(() => {
    if (user.walletAddress) {
      getGroups(user.walletAddress).then(groups => {
        const found = groups.find(g => g.groupId === groupId);
        if (found) setGroup(found);
      });
      
      getEvents(groupId).then(setEvents);
      
      // Load balance
      getBalance(groupId, user.walletAddress).then(setBalance);
    }
  }, [groupId, user.walletAddress]);

  // Fetch prices for active events
  const fetchPrices = useCallback(async () => {
    for (const event of events) {
      if (event.status === 'active') {
        const p = await getEventPrices(event.eventId);
        setPrices(prev => ({ ...prev, [event.eventId]: p }));
      }
    }
  }, [events]);

  useEffect(() => {
    if (events.length > 0) {
      fetchPrices();
      const interval = setInterval(fetchPrices, 10000);
      return () => clearInterval(interval);
    }
  }, [events, fetchPrices]);

  const handleCreateMarket = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim() || !user.walletAddress) return;

    setIsCreating(true);
    
    const outcomes = [outcome1.trim(), outcome2.trim()].filter(Boolean);
    if (outcomes.length < 2) {
      setIsCreating(false);
      return;
    }

    const newEvent = await createEvent(
      groupId,
      title,
      '',
      outcomes,
      'manual',
      null,
      user.walletAddress
    );
    
    const updatedEvents = [...events, newEvent];
    setEvents(updatedEvents);
    saveEvents(groupId, updatedEvents);
    
    // Reset form
    setTitle('');
    setOutcome1('yes');
    setOutcome2('no');
    setShowCreate(false);
    setIsCreating(false);
  };

  const handleDeposit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!user.walletAddress || !transactionAmount) return;
    
    setIsTransacting(true);
    setTransactionError(null);
    
    try {
      const amountUsdc = parseUsdc(transactionAmount);
      const result = await deposit(groupId, user.walletAddress, amountUsdc);
      
      if (result.success) {
        setBalance({
          balanceSol: result.newBalanceSol,
          balanceUsdc: result.newBalanceUsdc,
          fundsLocked: balance?.fundsLocked || false,
        });
        setShowDeposit(false);
        setTransactionAmount('');
      }
    } catch (err) {
      setTransactionError(err instanceof Error ? err.message : 'Deposit failed');
    } finally {
      setIsTransacting(false);
    }
  };

  const handleWithdraw = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!user.walletAddress || !transactionAmount) return;
    
    setIsTransacting(true);
    setTransactionError(null);
    
    try {
      const amountUsdc = parseUsdc(transactionAmount);
      const result = await withdraw(groupId, user.walletAddress, amountUsdc);
      
      if (result.success) {
        setBalance({
          balanceSol: result.newBalanceSol,
          balanceUsdc: result.newBalanceUsdc,
          fundsLocked: balance?.fundsLocked || false,
        });
        setShowWithdraw(false);
        setTransactionAmount('');
      }
    } catch (err) {
      setTransactionError(err instanceof Error ? err.message : 'Withdrawal failed');
    } finally {
      setIsTransacting(false);
    }
  };

  const formatPrice = (price: number) => `${Math.round(price * 100)}%`;
  const formatVolume = (vol: number) => `$${vol.toFixed(0)}`;

  if (authLoading || !group) {
    return (
      <main className="min-h-screen flex items-center justify-center">
        <p className="text-2xl text-white/40 italic">loading...</p>
      </main>
    );
  }

  return (
    <main className="min-h-screen px-8 py-16 md:py-24">
      <div className="max-w-3xl mx-auto">
        {/* Header */}
        <header className="mb-16">
          <button
            onClick={() => router.push('/dashboard')}
            className="text-xl text-white/40 hover:text-white transition-opacity mb-10 block"
          >
            ← back to groups
          </button>
          <h1 className="text-4xl md:text-5xl">{group.name}</h1>
        </header>

        {/* Balance Section */}
        <section className="mb-16 pb-16 border-b border-white/10">
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-2xl md:text-3xl">your balance</h2>
            <div className="flex gap-4">
              <button
                onClick={() => { setShowDeposit(true); setShowWithdraw(false); setTransactionError(null); }}
                className="text-lg text-white/40 hover:text-white transition-opacity"
              >
                + deposit
              </button>
              <button
                onClick={() => { setShowWithdraw(true); setShowDeposit(false); setTransactionError(null); }}
                className="text-lg text-white/40 hover:text-white transition-opacity"
              >
                − withdraw
              </button>
            </div>
          </div>

          <p className="text-4xl md:text-5xl mb-2">
            ${balance ? formatUsdc(balance.balanceUsdc) : '0.00'} <span className="text-white/40 text-2xl">USDC</span>
          </p>
          
          {balance?.fundsLocked && (
            <p className="text-lg text-white/40 italic">some funds locked in active bets</p>
          )}

          {/* Deposit Form */}
          {showDeposit && (
            <form onSubmit={handleDeposit} className="mt-8 p-6 border border-white/20 rounded">
              <p className="text-lg text-white/60 mb-4">deposit USDC to bet in this group</p>
              <input
                type="number"
                value={transactionAmount}
                onChange={(e) => setTransactionAmount(e.target.value)}
                placeholder="amount in USDC"
                step="0.01"
                min="0.01"
                className="w-full text-2xl py-4 border-b border-white/30 focus:border-white/70 transition-colors bg-transparent mb-4"
                autoFocus
              />
              {transactionError && (
                <p className="text-red-400/80 mb-4">{transactionError}</p>
              )}
              <div className="flex gap-8">
                <button
                  type="submit"
                  disabled={!transactionAmount || isTransacting}
                  className="text-xl text-white/60 hover:text-white transition-opacity disabled:text-white/30"
                >
                  {isTransacting ? 'depositing...' : 'deposit →'}
                </button>
                <button
                  type="button"
                  onClick={() => { setShowDeposit(false); setTransactionAmount(''); setTransactionError(null); }}
                  className="text-xl text-white/30 hover:text-white/60 transition-opacity"
                >
                  cancel
                </button>
              </div>
            </form>
          )}

          {/* Withdraw Form */}
          {showWithdraw && (
            <form onSubmit={handleWithdraw} className="mt-8 p-6 border border-white/20 rounded">
              <p className="text-lg text-white/60 mb-4">withdraw USDC from this group</p>
              <input
                type="number"
                value={transactionAmount}
                onChange={(e) => setTransactionAmount(e.target.value)}
                placeholder="amount in USDC"
                step="0.01"
                min="0.01"
                max={balance ? parseFloat(formatUsdc(balance.balanceUsdc)) : undefined}
                className="w-full text-2xl py-4 border-b border-white/30 focus:border-white/70 transition-colors bg-transparent mb-4"
                autoFocus
              />
              {transactionError && (
                <p className="text-red-400/80 mb-4">{transactionError}</p>
              )}
              <div className="flex gap-8">
                <button
                  type="submit"
                  disabled={!transactionAmount || isTransacting}
                  className="text-xl text-white/60 hover:text-white transition-opacity disabled:text-white/30"
                >
                  {isTransacting ? 'withdrawing...' : 'withdraw →'}
                </button>
                <button
                  type="button"
                  onClick={() => { setShowWithdraw(false); setTransactionAmount(''); setTransactionError(null); }}
                  className="text-xl text-white/30 hover:text-white/60 transition-opacity"
                >
                  cancel
                </button>
              </div>
            </form>
          )}
        </section>

        {/* Markets Section */}
        <section>
          <div className="flex items-center justify-between mb-12">
            <h2 className="text-2xl md:text-3xl">markets</h2>
            {!showCreate && (
              <button
                onClick={() => setShowCreate(true)}
                className="text-xl text-white/40 hover:text-white transition-opacity"
              >
                + new market
              </button>
            )}
          </div>

          {/* Create Market Form */}
          {showCreate && (
            <form onSubmit={handleCreateMarket} className="mb-12 pb-12 border-b border-white/10">
              <div className="mb-8">
                <label className="block text-lg text-white/40 mb-3">question</label>
                <input
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="will [something] happen?"
                  className="w-full text-2xl py-4 border-b border-white/30 focus:border-white/70 transition-colors bg-transparent"
                  autoFocus
                />
              </div>
              
              <div className="mb-8">
                <label className="block text-lg text-white/40 mb-3">outcomes</label>
                <div className="flex gap-4">
                  <input
                    type="text"
                    value={outcome1}
                    onChange={(e) => setOutcome1(e.target.value)}
                    placeholder="yes"
                    className="flex-1 text-xl py-3 border-b border-white/30 focus:border-white/70 transition-colors bg-transparent"
                  />
                  <span className="text-white/30 self-end py-3">vs</span>
                  <input
                    type="text"
                    value={outcome2}
                    onChange={(e) => setOutcome2(e.target.value)}
                    placeholder="no"
                    className="flex-1 text-xl py-3 border-b border-white/30 focus:border-white/70 transition-colors bg-transparent"
                  />
                </div>
              </div>
              
              <div className="flex gap-8">
                <button
                  type="submit"
                  disabled={!title.trim() || isCreating}
                  className="text-xl text-white/60 hover:text-white transition-opacity disabled:text-white/30"
                >
                  {isCreating ? 'creating...' : 'create market →'}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowCreate(false);
                    setTitle('');
                  }}
                  className="text-xl text-white/30 hover:text-white/60 transition-opacity"
                >
                  cancel
                </button>
              </div>
            </form>
          )}

          {/* Markets List */}
          {events.length > 0 ? (
            <ul className="space-y-4">
              {events.map((event) => {
                const eventPrices = prices[event.eventId];
                const isActive = event.status === 'active';
                
                return (
                  <li key={event.eventId}>
                    <button
                      onClick={() => router.push(`/event/${event.eventId}`)}
                      className="w-full text-left py-8 border-b border-white/10 hover:border-white/30 transition-colors"
                    >
                      <p className="text-2xl md:text-3xl mb-4">{event.title}</p>
                      
                      {isActive && eventPrices && (
                        <div className="flex gap-10 text-lg">
                          {event.outcomes.map((outcome) => (
                            <span key={outcome} className="text-white/60">
                              <span className="text-white">{formatPrice(eventPrices.prices[outcome] || 0.5)}</span>
                              {' '}{outcome}
                            </span>
                          ))}
                          {eventPrices.totalVolume > 0 && (
                            <span className="text-white/40">
                              {formatVolume(eventPrices.totalVolume)} volume
                            </span>
                          )}
                        </div>
                      )}
                      
                      {event.status === 'resolved' && (
                        <p className="text-lg text-white/40">
                          resolved: <span className="text-white/60 italic">{event.winningOutcome}</span>
                        </p>
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
          ) : !showCreate ? (
            <div className="text-center py-20">
              <p className="text-2xl text-white/40 italic mb-10">
                no markets yet.
              </p>
              <button
                onClick={() => setShowCreate(true)}
                className="text-2xl text-white/60 hover:text-white transition-opacity"
              >
                create the first market →
              </button>
            </div>
          ) : null}
        </section>
      </div>
    </main>
  );
}
