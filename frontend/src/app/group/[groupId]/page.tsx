'use client';

import { useEffect, useState, useCallback } from 'react';
import { useRouter, useParams } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import {
  getGroups, getEvents, saveEvents, createEvent,
  getEventPrices,
  getBalance, deposit, withdraw, requestFaucet, formatUsdc, parseUsdc,
  deleteEvent, deleteGroup,
  getGroupMembers, generateInviteLink, isGroupAdmin, addGroupCreatorAsMember
} from '@/lib/api';
import { BRAND } from '@/lib/brand';
import type { FriendGroup, Event, Prices, BalanceResponse, GroupMember } from '@/types';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { SystemProgram, Transaction, LAMPORTS_PER_SOL, PublicKey } from '@solana/web3.js';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';

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
  const [settlementType, setSettlementType] = useState<'manual' | 'oracle' | 'consensus'>('manual');
  const [arbiterWallet, setArbiterWallet] = useState('');

  // Wallet
  const { connection } = useConnection();
  const { publicKey, sendTransaction, connected } = useWallet();

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
    const walletAddress = user.walletAddress;
    if (walletAddress && pageReady) {
      getGroups(walletAddress).then(groups => {
        const found = groups.find(g => g.groupId === groupId);
        if (found) {
          setGroup(found);
          // Ensure creator is added as admin (for existing groups)
          addGroupCreatorAsMember(groupId, found.adminWallet);

          // Only fetch balance if we have a valid Sol pubkey (no mock/underscores)
          if (found.solanaPubkey && !found.solanaPubkey.includes('_')) {
            getBalance(found.groupId, walletAddress).then(setBalance);
          }
        }
      });

      getEvents(groupId).then(setEvents);

      // Load members and check admin status
      const groupMembers = getGroupMembers(groupId);
      setMembers(groupMembers);
      setIsAdmin(isGroupAdmin(groupId, walletAddress));
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
    if (!title.trim() || !user.walletAddress || !group) return;

    setIsCreating(true);
    try {
      const outcomes = [outcome1, outcome2];
      // Use group.groupId (cleaned UUID) instead of URL params which may have legacy format
      const newEvent = await createEvent(
        group.groupId,
        title,
        '', // description
        outcomes,
        settlementType,
        null, // resolveBy
        user.walletAddress,
        arbiterWallet
      );

      if (newEvent) {
        setEvents([newEvent, ...events]);
        // Init prices
        setPrices(prev => ({
          ...prev,
          [newEvent.eventId]: {
            eventId: newEvent.eventId,
            prices: { [outcome1]: 0.5, [outcome2]: 0.5 },
            totalVolume: 0,
            timestamp: Date.now() / 1000
          }
        }));
        setShowCreate(false);
        setTitle('');
        setOutcome1('yes');
        setOutcome2('no');
      }
    } finally {
      setIsCreating(false);
    }
  };

  const handleDeleteEvent = async (eventId: string, e: React.MouseEvent) => {
    e.stopPropagation(); // Prevent navigation
    if (!user.walletAddress || !confirm('Are you sure you want to delete this market?')) return;

    const success = await deleteEvent(eventId, user.walletAddress);
    if (success) {
      setEvents(events.filter(ev => ev.eventId !== eventId));
    } else {
      alert('Failed to delete event');
    }
  };

  const handleDeleteGroup = async () => {
    if (!group || !user.walletAddress) return;

    if (confirm("⚠️ ARE YOU SURE? ⚠️\n\nThis will permanently delete the group and all its history.\nThis action cannot be undone.")) {
      try {
        const success = await deleteGroup(groupId as string, user.walletAddress);
        if (success) {
          router.push('/dashboard');
        } else {
          alert("Failed to delete group. Please try again.");
        }
      } catch (e) {
        console.error("Delete group error:", e);
        alert("An error occurred.");
      }
    }
  };

  async function handleFaucet() {
    if (!user || !user.walletAddress || !group) return;

    setIsTransacting(true);
    setTransactionError(null);
    try {
      const sig = await requestFaucet(user.walletAddress);
      console.log('Faucet success:', sig);

      // Wait for confirmation then refresh balance
      setTimeout(async () => {
        if (user.walletAddress && group) { // Re-check existence
          const bal = await getBalance(group.groupId, user.walletAddress);
          setBalance(bal);
        }
        setIsTransacting(false);
        alert('Received 1000 Test USDC!');
      }, 2000);
    } catch (e: any) {
      console.error(e);
      setTransactionError(e.message || 'Faucet failed');
      setIsTransacting(false);
    }
  }

  const handleDeposit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!user.walletAddress || !transactionAmount) return;

    setIsTransacting(true);
    setTransactionError(null);
    try {
      const amount = parseFloat(transactionAmount);
      if (isNaN(amount) || amount <= 0) throw new Error("Invalid amount");

      let signature = 'dev';

      // Real Wallet Transfer
      if (connected && publicKey) {
        try {
          if (!group?.solanaPubkey || group.solanaPubkey.startsWith('mock_')) {
            throw new Error("Group wallet not ready for real deposits");
          }

          const transaction = new Transaction().add(
            SystemProgram.transfer({
              fromPubkey: publicKey,
              toPubkey: new PublicKey(group.solanaPubkey),
              lamports: amount * LAMPORTS_PER_SOL
            })
          );

          const { blockhash } = await connection.getLatestBlockhash();
          transaction.recentBlockhash = blockhash;
          transaction.feePayer = publicKey;

          signature = await sendTransaction(transaction, connection);
          await connection.confirmTransaction(signature, 'processed');
        } catch (txError) {
          console.error("Wallet transaction failed:", txError);
          setTransactionError("Wallet transaction failed. Check console.");
          setIsTransacting(false);
          return;
        }
      }

      const success = await deposit(group!.groupId!, user.walletAddress, amount, signature, 'sol');

      if (success) {
        setShowDeposit(false);
        setTransactionAmount('');
        // Refresh balance
        if (group?.solanaPubkey) {
          const bal = await getBalance(group.groupId, user.walletAddress);
          setBalance(bal);
        }
      } else {
        setTransactionError("Deposit failed backend verification");
      }
    } catch (e) {
      console.error(e);
      setTransactionError(e instanceof Error ? e.message : "Error processing deposit");
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
      if (!group?.solanaPubkey || group.solanaPubkey.includes('_')) {
        throw new Error('This group is not on-chain (mock). Please create a new group.');
      }

      const result = await withdraw(group.groupId, user.walletAddress, amountUsdc);

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
    <div style={{
      minHeight: '100vh',
      width: '100%',
      overflow: 'auto',
      position: 'relative'
    }}>
      <main style={{
        minHeight: '100vh',
        paddingLeft: '24px',
        paddingRight: '24px',
        paddingTop: '48px',
        paddingBottom: '80px',
        background: 'linear-gradient(135deg, #0a0a0a 0%, #1a1a2e 50%, #0a0a0a 100%)',
      }}>
        <div style={{
          maxWidth: '1400px',
          margin: '0 auto',
          display: 'flex',
          flexDirection: 'column',
          gap: '24px',
          position: 'relative',
          zIndex: 1
        }}>
          {/* Header */}
          <header style={{
            marginBottom: '48px',
            textAlign: 'center'
          }}>
            <button
              onClick={() => router.push('/dashboard')}
              style={{
                fontSize: '16px',
                color: 'rgba(255, 255, 255, 0.4)',
                marginBottom: '32px',
                display: 'block',
                margin: '0 auto 32px auto',
                background: 'none',
                border: 'none',
                cursor: 'pointer',
                transition: 'color 0.2s'
              }}
              onMouseEnter={(e) => e.currentTarget.style.color = 'rgba(255, 255, 255, 0.7)'}
              onMouseLeave={(e) => e.currentTarget.style.color = 'rgba(255, 255, 255, 0.4)'}
            >
              <span style={{ display: 'inline-block', transition: 'transform 0.2s' }}>←</span> back to home
            </button>
            <h1 style={{
              fontSize: 'clamp(3rem, 5vw, 4rem)',
              fontWeight: '300',
              color: '#ffffff',
              letterSpacing: '-0.02em',
              margin: 0
            }}>{group.name}</h1>
          </header>


          {/* Members Section */}
          <section style={{
            marginBottom: '48px',
            padding: '32px',
            borderRadius: '24px',
            background: 'linear-gradient(145deg, #0a0a0a 0%, #050505 100%)',
            border: '1px solid #222',
            boxShadow: '0 8px 32px rgba(0,0,0,0.2)'
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '32px' }}>
              <h2 style={{ fontSize: '14px', fontWeight: '600', color: '#666', letterSpacing: '0.1em', textTransform: 'uppercase' }}>Group Members</h2>
              <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
                {isAdmin && (
                  <button
                    onClick={() => {
                      if (!inviteLink) {
                        const link = generateInviteLink(groupId, user.walletAddress!);
                        setInviteLink(link);
                      }
                    }}
                    className="text-xs font-medium px-4 py-2 rounded-full bg-white text-black hover:bg-gray-200 transition-all flex items-center gap-2"
                  >
                    <span>+</span> Invite Friend
                  </button>
                )}
                <button
                  onClick={() => setShowMembers(!showMembers)}
                  className="text-xs text-white/40 hover:text-white transition-colors"
                >
                  {showMembers ? 'Hide' : `Show All (${members.length})`}
                </button>
              </div>
            </div>

            {/* Invite Link */}
            {inviteLink && (
              <div className="mb-8 p-6 rounded-2xl relative overflow-hidden group" style={{
                background: 'rgba(255, 255, 255, 0.03)',
                border: '1px dashed rgba(255, 255, 255, 0.2)',
              }}>
                <div className="absolute inset-0 bg-gradient-to-r from-blue-500/10 to-purple-500/10 opacity-0 group-hover:opacity-100 transition-opacity" />
                <p className="text-xs text-blue-400 mb-3 font-medium uppercase tracking-wider relative z-10">Invite Link Generated</p>
                <div className="flex items-center gap-4 relative z-10">
                  <div className="flex-1 bg-black/50 border border-white/10 rounded-xl px-4 py-3 font-mono text-xs text-white/70 overflow-hidden text-ellipsis whitespace-nowrap">
                    {inviteLink}
                  </div>
                  <button
                    onClick={() => {
                      navigator.clipboard.writeText(inviteLink);
                      alert('Copied to clipboard!');
                    }}
                    className="text-xs font-medium px-4 py-3 bg-white/10 hover:bg-white/20 text-white rounded-xl transition-all whitespace-nowrap"
                  >
                    Copy Link
                  </button>
                </div>
              </div>
            )}

            {/* Members Grid */}
            {showMembers && (
              <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                {members.map((member) => (
                  <div
                    key={member.walletAddress}
                    className="flex items-center gap-4 p-4 rounded-xl border border-white/5 hover:border-white/10 transition-all bg-white/[0.02]"
                  >
                    <div className="w-10 h-10 rounded-full bg-gradient-to-br from-gray-800 to-black flex items-center justify-center border border-white/10 text-xs font-mono text-white/50">
                      {member.walletAddress.slice(0, 2)}
                    </div>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2">
                        <span className="text-sm text-white/90 font-medium truncate font-mono">
                          {member.walletAddress.slice(0, 4)}...{member.walletAddress.slice(-4)}
                        </span>
                        {member.role === 'admin' && (
                          <span className="text-[10px] font-bold text-amber-500/90 bg-amber-500/10 px-2 py-0.5 rounded uppercase tracking-wide">
                            Admin
                          </span>
                        )}
                      </div>
                      <p className="text-xs text-white/30 truncate">
                        Joined {new Date(Number(member.joinedAt) * 1000).toLocaleDateString()}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            )}

            {/* Danger Zone */}
            {isAdmin && group && showMembers && (
              <div className="mt-8 pt-8 border-t border-white/5 text-center">
                <button
                  onClick={handleDeleteGroup}
                  className="text-xs text-red-500/50 hover:text-red-500 hover:underline transition-all"
                >
                  Delete Group Permanently
                </button>
              </div>
            )}
          </section>

          {/* Balance Section */}
          <section style={{
            width: '100%',
            padding: '48px',
            borderRadius: '24px',
            textAlign: 'center',
            background: 'radial-gradient(circle at center, #1a1a1a 0%, #000 100%)',
            border: '1px solid #222',
            boxSizing: 'border-box',
            position: 'relative',
            overflow: 'hidden'
          }}>
            <div className="absolute top-0 left-0 w-full h-[1px] bg-gradient-to-r from-transparent via-white/20 to-transparent" />

            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'center', gap: '32px', marginBottom: '40px' }}>
              <button
                onClick={() => { setShowDeposit(true); setShowWithdraw(false); setTransactionError(null); }}
                className="text-sm font-medium px-6 py-2 rounded-full border border-white/10 hover:bg-white/5 hover:border-white/30 text-white/70 hover:text-white transition-all"
              >
                Deposit
              </button>
              <h2 className="text-xs font-bold tracking-[0.2em] text-white/30 uppercase">Your Balance</h2>
              <button
                onClick={() => { setShowWithdraw(true); setShowDeposit(false); setTransactionError(null); }}
                className="text-sm font-medium px-6 py-2 rounded-full border border-white/10 hover:bg-white/5 hover:border-white/30 text-white/70 hover:text-white transition-all"
              >
                Withdraw
              </button>
            </div>

            <div className="text-center py-8 relative">
              <p className="text-6xl md:text-8xl font-light mb-4 text-white tracking-tighter" style={{ textShadow: '0 0 40px rgba(255,255,255,0.1)' }}>
                ${balance ? formatUsdc(balance.balanceUsdc) : '0.00'}
              </p>
              <p className="text-sm text-white/30 font-mono">USDC / SOLANA</p>
            </div>

            {balance?.fundsLocked && (
              <div className="inline-flex items-center gap-2 px-4 py-2 rounded-full bg-amber-500/10 border border-amber-500/20 text-amber-500/80 text-xs mt-4">
                <span className="w-1.5 h-1.5 rounded-full bg-amber-500 animate-pulse" />
                Funds locked in active bets
              </div>
            )}

            {/* Transaction Forms - Keeping functional logic but improving style */}
            {(showDeposit || showWithdraw) && (
              <div className="mt-8 animate-in fade-in slide-in-from-bottom-4 duration-300">
                <form
                  onSubmit={showDeposit ? handleDeposit : handleWithdraw}
                  className="max-w-md mx-auto p-8 rounded-2xl border border-white/10 bg-black/50 backdrop-blur-xl"
                >
                  <h3 className="text-lg text-white mb-6 font-light flex justify-between items-center">
                    <span>{showDeposit ? 'Deposit Funds' : 'Withdraw Funds'}</span>
                    {showDeposit && (
                      <button
                        type="button"
                        onClick={handleFaucet}
                        className="text-xs px-3 py-1 bg-blue-600/20 text-blue-400 hover:bg-blue-600/30 rounded-full transition-colors"
                      >
                        + Get Test USDC
                      </button>
                    )}
                  </h3>

                  <div className="relative mb-8">
                    <span className="absolute left-4 top-1/2 -translate-y-1/2 text-white/30 text-xl">$</span>
                    <input
                      type="number"
                      value={transactionAmount}
                      onChange={(e) => setTransactionAmount(e.target.value)}
                      placeholder="0.00"
                      step="0.01"
                      min="0.01"
                      className="w-full bg-transparent border-b border-white/20 py-4 pl-10 pr-4 text-3xl text-white placeholder-white/10 focus:outline-none focus:border-white/50 transition-colors"
                      autoFocus
                    />
                  </div>

                  {transactionError && (
                    <p className="text-red-400 text-sm mb-6 bg-red-500/10 py-2 px-3 rounded text-center">{transactionError}</p>
                  )}

                  <div className="flex gap-4">
                    <button
                      type="submit"
                      disabled={!transactionAmount || isTransacting}
                      className="flex-1 py-3 px-6 rounded-xl bg-white text-black font-medium hover:bg-gray-200 disabled:opacity-50 disabled:cursor-not-allowed transition-all"
                    >
                      {isTransacting ? 'Processing...' : (showDeposit ? 'Confirm Deposit' : 'Confirm Withdraw')}
                    </button>
                    <button
                      type="button"
                      onClick={() => { setShowDeposit(false); setShowWithdraw(false); setTransactionAmount(''); setTransactionError(null); }}
                      className="px-6 py-3 rounded-xl border border-white/10 text-white/60 hover:text-white hover:bg-white/5 transition-all"
                    >
                      Cancel
                    </button>
                  </div>
                </form>
              </div>
            )}
          </section>

          {/* Markets Section */}
          <section style={{
            width: '100%',
            padding: '32px',
            borderRadius: '24px',
            background: '#080808', // Darker background
            border: '1px solid #1a1a1a', // Subtle border
            boxSizing: 'border-box',
            marginTop: '48px'
          }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '32px' }}>
              <div className="flex items-center gap-3">
                <h2 style={{ fontSize: '24px', fontWeight: '500', color: '#fff', margin: 0 }}>Markets</h2>
                <span className="px-2 py-0.5 rounded bg-white/10 text-white/50 text-xs font-mono">{events.length}</span>
              </div>
              {!showCreate && (
                <button
                  onClick={() => setShowCreate(true)}
                  className="text-sm px-6 py-2.5 rounded-full bg-indigo-600 hover:bg-indigo-500 text-white transition-all shadow-lg shadow-indigo-900/20 font-medium"
                >
                  + New Market
                </button>
              )}
            </div>

            {/* Create Market Modal */}
            {showCreate && (
              <div className="fixed inset-0 z-50 flex items-center justify-center p-4">
                <div
                  className="absolute inset-0 bg-black/80 backdrop-blur-sm"
                  onClick={() => setShowCreate(false)}
                />
                <div className="relative w-full max-w-lg bg-[#0a0a0a] border border-[#333] rounded-3xl p-8 shadow-2xl animate-in zoom-in-95 duration-200">
                  <h2 className="text-2xl font-light text-white mb-8 text-center">Create New Market</h2>

                  <form onSubmit={handleCreateMarket}>
                    <div className="mb-6">
                      <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-2">Question</label>
                      <input
                        type="text"
                        value={title}
                        onChange={(e) => setTitle(e.target.value)}
                        placeholder="e.g. Will SOL hit $250 by Friday?"
                        className="w-full bg-[#111] border border-[#222] rounded-xl p-4 text-white text-lg focus:outline-none focus:border-indigo-500/50 transition-colors"
                        autoFocus
                      />
                    </div>

                    <div className="mb-6">
                      <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-2">Outcomes</label>
                      <div className="flex gap-3">
                        <input
                          type="text"
                          value={outcome1}
                          onChange={(e) => setOutcome1(e.target.value)}
                          className="flex-1 bg-[#111] border border-[#222] rounded-xl p-3 text-white text-center focus:outline-none focus:border-indigo-500/50"
                        />
                        <input
                          type="text"
                          value={outcome2}
                          onChange={(e) => setOutcome2(e.target.value)}
                          className="flex-1 bg-[#111] border border-[#222] rounded-xl p-3 text-white text-center focus:outline-none focus:border-indigo-500/50"
                        />
                      </div>
                    </div>

                    <div className="mb-8">
                      <label className="block text-xs font-bold text-gray-500 uppercase tracking-wider mb-2">Settlement</label>
                      <select
                        value={settlementType}
                        onChange={(e) => setSettlementType(e.target.value as any)}
                        className="w-full bg-[#111] border border-[#222] rounded-xl p-4 text-white/80 focus:outline-none appearance-none"
                      >
                        <option value="manual">Manual (Arbiter Settles)</option>
                        <option value="oracle">Oracle (Automated)</option>
                        <option value="consensus">Group Consensus</option>
                      </select>
                    </div>

                    <div className="flex gap-4">
                      <button
                        type="button"
                        onClick={() => setShowCreate(false)}
                        className="flex-1 py-4 rounded-xl border border-[#333] text-gray-400 hover:text-white hover:bg-[#111] transition-all"
                      >
                        Cancel
                      </button>
                      <button
                        type="submit"
                        disabled={!title.trim() || isCreating}
                        className="flex-1 py-4 rounded-xl bg-indigo-600 hover:bg-indigo-500 text-white font-medium disabled:opacity-50 disabled:cursor-not-allowed transition-all"
                      >
                        {isCreating ? 'Creating...' : 'Launch Market'}
                      </button>
                    </div>
                  </form>
                </div>
              </div>
            )}

            {/* Markets List */}
            {events.length > 0 ? (
              <ul className="flex flex-col gap-4">
                {events.map((event) => {
                  const eventPrices = prices[event.eventId];
                  const isActive = event.status === 'active';
                  const outcomes = event.outcomes || ['yes', 'no'];

                  return (
                    <li key={event.eventId}>
                      <button
                        onClick={() => router.push(`/event/${event.eventId}`)}
                        className="w-full p-6 md:p-8 rounded-2xl bg-[#0a0a0a] border border-white/10 hover:border-indigo-500/30 hover:bg-[#0d0d0d] transition-all duration-200 text-left group"
                      >
                        {/* Title Row */}
                        <div className="flex items-start justify-between gap-4 mb-6">
                          <h3 className="text-xl md:text-2xl text-white font-normal leading-snug group-hover:text-indigo-200 transition-colors">
                            {event.title}
                          </h3>

                          {/* Status Badge */}
                          <div className={`shrink-0 px-4 py-1.5 rounded-full text-xs font-semibold uppercase tracking-wider flex items-center gap-2 ${isActive
                              ? 'bg-emerald-500/20 text-emerald-400 border border-emerald-500/30'
                              : event.status === 'resolved'
                                ? 'bg-blue-500/20 text-blue-400 border border-blue-500/30'
                                : 'bg-gray-500/20 text-gray-400 border border-gray-500/30'
                            }`}>
                            {isActive && <span className="w-2 h-2 rounded-full bg-emerald-400 animate-pulse" />}
                            {isActive ? 'Live' : event.status}
                          </div>
                        </div>

                        {/* Odds Display - Large and Clear */}
                        {isActive && eventPrices && (
                          <div className="flex flex-wrap items-center gap-4 mb-6">
                            {outcomes.map((outcome, idx) => {
                              const price = eventPrices.prices[outcome] || 0.5;
                              const isYes = idx === 0;
                              return (
                                <div
                                  key={outcome}
                                  className={`flex items-center gap-3 px-5 py-3 rounded-xl border ${isYes
                                      ? 'bg-emerald-500/10 border-emerald-500/20'
                                      : 'bg-rose-500/10 border-rose-500/20'
                                    }`}
                                >
                                  <span className={`text-sm font-medium uppercase tracking-wide ${isYes ? 'text-emerald-400/80' : 'text-rose-400/80'}`}>
                                    {outcome}
                                  </span>
                                  <span className={`text-2xl md:text-3xl font-bold tabular-nums ${isYes ? 'text-emerald-400' : 'text-rose-400'}`}>
                                    {Math.round(price * 100)}%
                                  </span>
                                </div>
                              );
                            })}
                          </div>
                        )}

                        {/* Resolved State */}
                        {event.status === 'resolved' && event.winningOutcome && (
                          <div className="flex items-center gap-3 px-5 py-3 rounded-xl bg-blue-500/10 border border-blue-500/20 mb-6 w-fit">
                            <span className="text-sm font-medium uppercase tracking-wide text-blue-400/80">Winner</span>
                            <span className="text-2xl md:text-3xl font-bold text-blue-400">{event.winningOutcome}</span>
                          </div>
                        )}

                        {/* Meta Row - Clear Labels */}
                        <div className="flex items-center gap-8 pt-4 border-t border-white/5">
                          <div className="flex items-center gap-3">
                            <span className="text-sm text-white/40">Volume</span>
                            <span className="text-base text-white/80 font-mono font-medium">
                              {eventPrices ? formatVolume(eventPrices.totalVolume) : '$0'}
                            </span>
                          </div>
                          <div className="w-px h-4 bg-white/10" />
                          <div className="flex items-center gap-3">
                            <span className="text-sm text-white/40">Expires</span>
                            <span className="text-base text-white/80 font-mono font-medium">
                              {event.resolveBy ? new Date(event.resolveBy * 1000).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' }) : 'Never'}
                            </span>
                          </div>
                        </div>
                      </button>
                    </li>
                  );
                })}
              </ul>
            ) : (
              /* Empty State */
              <div className="py-20 text-center rounded-2xl bg-[#0a0a0a] border border-dashed border-[#222]">
                <div className="w-16 h-16 rounded-full bg-[#111] flex items-center justify-center mx-auto mb-6 text-2xl">
                  🎲
                </div>
                <h3 className="text-lg text-white mb-2">No active markets</h3>
                <p className="text-gray-500 text-sm mb-8">Be the first to create a prediction market in this group.</p>
                <button
                  onClick={() => setShowCreate(true)}
                  className="text-sm px-6 py-3 rounded-xl bg-white/5 hover:bg-white/10 text-white transition-all border border-white/10 hover:border-white/20"
                >
                  Create Market
                </button>
              </div>
            )}

          </section >
        </div >
      </main >
    </div >
  );
}
