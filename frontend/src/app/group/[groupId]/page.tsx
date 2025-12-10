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
  
  const { user, checkAuth, isLoading: authLoading, isInitialized } = useAuthStore();
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
  const [pageReady, setPageReady] = useState(false);
  
  // Form state
  const [title, setTitle] = useState('');
  const [outcome1, setOutcome1] = useState('yes');
  const [outcome2, setOutcome2] = useState('no');

  useEffect(() => {
    if (!isInitialized) {
      checkAuth();
    }
  }, [checkAuth, isInitialized]);

  useEffect(() => {
    if (isInitialized && !user.isLoggedIn) {
      router.push('/');
    } else if (isInitialized && user.isLoggedIn) {
      setPageReady(true);
    }
  }, [user.isLoggedIn, isInitialized, router]);

  // Load group and events
  useEffect(() => {
    if (user.walletAddress && pageReady) {
      getGroups(user.walletAddress).then(groups => {
        const found = groups.find(g => g.groupId === groupId);
        if (found) setGroup(found);
      });
      
      getEvents(groupId).then(setEvents);
      getBalance(groupId, user.walletAddress).then(setBalance);
    }
  }, [groupId, user.walletAddress, pageReady]);

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

  if ((authLoading && !isInitialized) || !pageReady || !group) {
    return (
      <main className="min-h-screen flex items-center justify-center">
        <p className="text-3xl text-white/70 italic">loading...</p>
      </main>
    );
  }

  return (
    <main className="min-h-screen px-8 py-16 md:py-24">
      <div className="max-w-3xl mx-auto">
        {/* Header */}
        <header className="mb-16 text-center">
          <button
            onClick={() => router.push('/dashboard')}
            className="text-lg text-white/50 hover:text-white/80 transition-opacity mb-10 block mx-auto"
          >
            ← back to groups
          </button>
          <h1 className="text-4xl md:text-5xl text-white">{group.name}</h1>
        </header>

        {/* Balance Section */}
        <section className="mb-16 pb-16 border-b border-white/20 text-center">
          <div className="flex items-center justify-center gap-8 mb-6">
            <h2 className="text-2xl md:text-3xl text-white">your balance</h2>
            <div className="flex gap-4">
              <button
                onClick={() => { setShowDeposit(true); setShowWithdraw(false); setTransactionError(null); }}
                className="text-lg text-white/60 hover:text-white transition-opacity"
              >
                + deposit
              </button>
              <button
                onClick={() => { setShowWithdraw(true); setShowDeposit(false); setTransactionError(null); }}
                className="text-lg text-white/60 hover:text-white transition-opacity"
              >
                − withdraw
              </button>
            </div>
          </div>

          <p className="text-4xl md:text-5xl mb-2 text-white">
            ${balance ? formatUsdc(balance.balanceUsdc) : '0.00'} <span className="text-white/60 text-2xl">USDC</span>
          </p>
          
          {balance?.fundsLocked && (
            <p className="text-lg text-white/60 italic">some funds locked in active bets</p>
          )}

          {/* Deposit Form */}
          {showDeposit && (
            <form onSubmit={handleDeposit} className="mt-8 p-6 border border-white/30 rounded-lg max-w-md mx-auto text-left">
              <p className="text-lg text-white/80 mb-4">deposit USDC to bet in this group</p>
              <input
                type="number"
                value={transactionAmount}
                onChange={(e) => setTransactionAmount(e.target.value)}
                placeholder="amount in USDC"
                step="0.01"
                min="0.01"
                className="w-full text-xl py-4 border-b-2 border-white/40 focus:border-white transition-colors bg-transparent mb-4 text-white placeholder:text-white/50"
                autoFocus
              />
              {transactionError && (
                <p className="text-red-400 mb-4">{transactionError}</p>
              )}
              <div className="flex gap-8">
                <button
                  type="submit"
                  disabled={!transactionAmount || isTransacting}
                  className="text-xl text-white/80 hover:text-white transition-opacity disabled:text-white/40"
                >
                  {isTransacting ? 'depositing...' : 'deposit →'}
                </button>
                <button
                  type="button"
                  onClick={() => { setShowDeposit(false); setTransactionAmount(''); setTransactionError(null); }}
                  className="text-xl text-white/50 hover:text-white/80 transition-opacity"
                >
                  cancel
                </button>
              </div>
            </form>
          )}

          {/* Withdraw Form */}
          {showWithdraw && (
            <form onSubmit={handleWithdraw} className="mt-8 p-6 border border-white/30 rounded-lg max-w-md mx-auto text-left">
              <p className="text-lg text-white/80 mb-4">withdraw USDC from this group</p>
              <input
                type="number"
                value={transactionAmount}
                onChange={(e) => setTransactionAmount(e.target.value)}
                placeholder="amount in USDC"
                step="0.01"
                min="0.01"
                max={balance ? parseFloat(formatUsdc(balance.balanceUsdc)) : undefined}
                className="w-full text-xl py-4 border-b-2 border-white/40 focus:border-white transition-colors bg-transparent mb-4 text-white placeholder:text-white/50"
                autoFocus
              />
              {transactionError && (
                <p className="text-red-400 mb-4">{transactionError}</p>
              )}
              <div className="flex gap-8">
                <button
                  type="submit"
                  disabled={!transactionAmount || isTransacting}
                  className="text-xl text-white/80 hover:text-white transition-opacity disabled:text-white/40"
                >
                  {isTransacting ? 'withdrawing...' : 'withdraw →'}
                </button>
                <button
                  type="button"
                  onClick={() => { setShowWithdraw(false); setTransactionAmount(''); setTransactionError(null); }}
                  className="text-xl text-white/50 hover:text-white/80 transition-opacity"
                >
                  cancel
                </button>
              </div>
            </form>
          )}
        </section>

        {/* Markets Section */}
        <section className="text-center">
          <div className="flex items-center justify-center gap-8 mb-12">
            <h2 className="text-2xl md:text-3xl text-white">markets</h2>
            {!showCreate && (
              <button
                onClick={() => setShowCreate(true)}
                className="text-lg text-white/60 hover:text-white transition-opacity"
              >
                + new market
              </button>
            )}
          </div>

          {/* Create Market Form */}
          {showCreate && (
            <form onSubmit={handleCreateMarket} className="mb-12 pb-12 border-b border-white/20 max-w-md mx-auto text-left">
              <div className="mb-8">
                <label className="block text-lg text-white/60 mb-3">question</label>
                <input
                  type="text"
                  value={title}
                  onChange={(e) => setTitle(e.target.value)}
                  placeholder="will [something] happen?"
                  className="w-full text-xl py-4 border-b-2 border-white/40 focus:border-white transition-colors bg-transparent text-white placeholder:text-white/50"
                  autoFocus
                />
              </div>
              
              <div className="mb-8">
                <label className="block text-lg text-white/60 mb-3">outcomes</label>
                <div className="flex gap-4 items-end">
                  <input
                    type="text"
                    value={outcome1}
                    onChange={(e) => setOutcome1(e.target.value)}
                    placeholder="yes"
                    className="flex-1 text-lg py-3 border-b-2 border-white/40 focus:border-white transition-colors bg-transparent text-white placeholder:text-white/50"
                  />
                  <span className="text-white/50 py-3">vs</span>
                  <input
                    type="text"
                    value={outcome2}
                    onChange={(e) => setOutcome2(e.target.value)}
                    placeholder="no"
                    className="flex-1 text-lg py-3 border-b-2 border-white/40 focus:border-white transition-colors bg-transparent text-white placeholder:text-white/50"
                  />
                </div>
              </div>
              
              <div className="flex gap-8">
                <button
                  type="submit"
                  disabled={!title.trim() || isCreating}
                  className="text-xl text-white/80 hover:text-white transition-opacity disabled:text-white/40"
                >
                  {isCreating ? 'creating...' : 'create market →'}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowCreate(false);
                    setTitle('');
                  }}
                  className="text-xl text-white/50 hover:text-white/80 transition-opacity"
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
                      className="w-full text-left py-8 px-6 border border-white/20 hover:border-white/40 rounded-lg transition-all hover:bg-white/5"
                    >
                      <p className="text-2xl md:text-3xl mb-4 text-white">{event.title}</p>
                      
                      {isActive && eventPrices && (
                        <div className="flex gap-10 text-lg">
                          {event.outcomes.map((outcome) => (
                            <span key={outcome} className="text-white/70">
                              <span className="text-white font-medium">{formatPrice(eventPrices.prices[outcome] || 0.5)}</span>
                              {' '}{outcome}
                            </span>
                          ))}
                          {eventPrices.totalVolume > 0 && (
                            <span className="text-white/50">
                              {formatVolume(eventPrices.totalVolume)} volume
                            </span>
                          )}
                        </div>
                      )}
                      
                      {event.status === 'resolved' && (
                        <p className="text-lg text-white/60">
                          resolved: <span className="text-white/80 italic">{event.winningOutcome}</span>
                        </p>
                      )}
                    </button>
                  </li>
                );
              })}
            </ul>
          ) : !showCreate ? (
            <div className="py-16">
              <p className="text-2xl text-white/60 italic mb-10">
                no markets yet.
              </p>
              <button
                onClick={() => setShowCreate(true)}
                className="text-2xl text-white/80 hover:text-white transition-opacity"
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
