'use client';

import { useEffect, useState, useCallback } from 'react';
import { useRouter, useParams } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import {
  getGroups, getEvents, saveEvents, createEvent, getEventPrices,
  getBalance, deposit, withdraw, formatUsdc, parseUsdc,
  getGroupMembers, generateInviteLink, isGroupAdmin, addGroupCreatorAsMember
} from '@/lib/api';
import type { FriendGroup, Event, Prices, BalanceResponse, GroupMember } from '@/types';

export default function GroupPage() {
  const router = useRouter();
  const params = useParams();
  const groupId = params.groupId as string;

  const { user, checkAuth, isLoading: authLoading, isInitialized } = useAuthStore();
  const [group, setGroup] = useState<FriendGroup | null>(null);
  const [events, setEvents] = useState<Event[]>([]);
  const [prices, setPrices] = useState<Record<string, Prices>>({});
  const [balance, setBalance] = useState<BalanceResponse | null>(null);
  const [members, setMembers] = useState<GroupMember[]>([]);
  const [isAdmin, setIsAdmin] = useState(false);
  const [inviteLink, setInviteLink] = useState<string | null>(null);
  const [showCreate, setShowCreate] = useState(false);
  const [showDeposit, setShowDeposit] = useState(false);
  const [showWithdraw, setShowWithdraw] = useState(false);
  const [showMembers, setShowMembers] = useState(false);
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
        if (found) {
          setGroup(found);
          // Ensure creator is added as admin (for existing groups)
          addGroupCreatorAsMember(groupId, found.adminWallet);
        }
      });

      getEvents(groupId).then(setEvents);
      getBalance(groupId, user.walletAddress).then(setBalance);

      // Load members and check admin status
      const groupMembers = getGroupMembers(groupId);
      setMembers(groupMembers);
      setIsAdmin(isGroupAdmin(groupId, user.walletAddress));
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
    <main className="min-h-screen px-6 py-12 md:py-20" style={{
      background: 'linear-gradient(135deg, #0a0a0a 0%, #1a1a2e 50%, #0a0a0a 100%)',
    }}>
      <div className="max-w-4xl mx-auto">
        {/* Header */}
        <header className="mb-12 text-center">
          <button
            onClick={() => router.push('/dashboard')}
            className="text-base text-white/40 hover:text-white/70 transition-all mb-8 block mx-auto group"
          >
            <span className="group-hover:-translate-x-1 inline-block transition-transform">←</span> back to groups
          </button>
          <h1 className="text-5xl md:text-6xl font-light text-white tracking-tight">{group.name}</h1>
        </header>

        {/* Members Section */}
        <section className="mb-8 p-6 rounded-2xl" style={{
          background: 'rgba(255, 255, 255, 0.03)',
          backdropFilter: 'blur(10px)',
          border: '1px solid rgba(255, 255, 255, 0.08)',
        }}>
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-xl font-medium text-white/90">members</h2>
            <div className="flex gap-4">
              <button
                onClick={() => setShowMembers(!showMembers)}
                className="text-sm text-white/50 hover:text-white/80 transition-colors"
              >
                {showMembers ? 'hide' : `show (${members.length || 1})`}
              </button>
              {isAdmin && (
                <button
                  onClick={() => {
                    if (!inviteLink) {
                      const link = generateInviteLink(groupId, user.walletAddress!);
                      setInviteLink(link);
                    }
                  }}
                  className="text-sm px-3 py-1 rounded-full bg-white/10 text-white/70 hover:bg-white/20 hover:text-white transition-all"
                >
                  + invite
                </button>
              )}
            </div>
          </div>

          {/* Invite Link */}
          {inviteLink && (
            <div className="mb-4 p-4 rounded-xl" style={{
              background: 'rgba(255, 255, 255, 0.05)',
              border: '1px solid rgba(255, 255, 255, 0.1)',
            }}>
              <p className="text-xs text-white/50 mb-2">share this link to invite members:</p>
              <div className="flex items-center gap-2">
                <code className="flex-1 text-xs text-white/70 bg-black/30 px-3 py-2 rounded-lg overflow-x-auto">
                  {inviteLink}
                </code>
                <button
                  onClick={() => {
                    navigator.clipboard.writeText(inviteLink);
                  }}
                  className="text-xs text-white/60 hover:text-white transition-colors px-3 py-2 bg-white/10 hover:bg-white/20 rounded-lg"
                >
                  copy
                </button>
              </div>
            </div>
          )}

          {/* Members List */}
          {showMembers && (
            <ul className="space-y-2">
              {members.length > 0 ? (
                members.map((member) => (
                  <li
                    key={member.walletAddress}
                    className="flex items-center justify-between py-3 px-4 rounded-xl transition-colors hover:bg-white/5"
                    style={{ background: 'rgba(255, 255, 255, 0.02)' }}
                  >
                    <span className="text-white/70 font-mono text-sm">
                      {member.walletAddress.slice(0, 8)}...{member.walletAddress.slice(-4)}
                    </span>
                    {member.role === 'admin' && (
                      <span className="text-xs text-emerald-400/80 bg-emerald-400/10 px-2 py-1 rounded-full">
                        admin
                      </span>
                    )}
                  </li>
                ))
              ) : (
                <li className="text-center text-white/40 py-4 text-sm">
                  just you for now — invite some friends!
                </li>
              )}
            </ul>
          )}
        </section>

        {/* Balance Section */}
        <section className="mb-8 p-6 rounded-2xl text-center" style={{
          background: 'linear-gradient(135deg, rgba(16, 185, 129, 0.08) 0%, rgba(59, 130, 246, 0.08) 100%)',
          backdropFilter: 'blur(10px)',
          border: '1px solid rgba(255, 255, 255, 0.08)',
        }}>
          <div className="flex items-center justify-between mb-6">
            <h2 className="text-xl font-medium text-white/90">your balance</h2>
            <div className="flex gap-3">
              <button
                onClick={() => { setShowDeposit(true); setShowWithdraw(false); setTransactionError(null); }}
                className="text-sm px-4 py-2 rounded-full bg-emerald-500/20 text-emerald-400 hover:bg-emerald-500/30 transition-all"
              >
                + deposit
              </button>
              <button
                onClick={() => { setShowWithdraw(true); setShowDeposit(false); setTransactionError(null); }}
                className="text-sm px-4 py-2 rounded-full bg-white/10 text-white/70 hover:bg-white/20 hover:text-white transition-all"
              >
                − withdraw
              </button>
            </div>
          </div>

          <div className="text-center py-4">
            <p className="text-5xl md:text-6xl font-light mb-2 text-white tracking-tight">
              ${balance ? formatUsdc(balance.balanceUsdc) : '0.00'} <span className="text-white/40 text-3xl">USDC</span>
            </p>
          </div>

          {balance?.fundsLocked && (
            <p className="text-sm text-amber-400/80 mt-2">⚠ some funds locked in active bets</p>
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
        <section className="p-6 rounded-2xl" style={{
          background: 'rgba(255, 255, 255, 0.03)',
          backdropFilter: 'blur(10px)',
          border: '1px solid rgba(255, 255, 255, 0.08)',
        }}>
          <div className="flex items-center justify-between mb-8">
            <h2 className="text-xl font-medium text-white/90">markets</h2>
            {!showCreate && (
              <button
                onClick={() => setShowCreate(true)}
                className="text-sm px-4 py-2 rounded-full bg-blue-500/20 text-blue-400 hover:bg-blue-500/30 transition-all"
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
