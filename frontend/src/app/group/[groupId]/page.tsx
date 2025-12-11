'use client';

import { useEffect, useState, useCallback } from 'react';
import { useRouter, useParams } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import {
  getGroups, getEvents, saveEvents, createEvent,
  getEventPrices,
  getBalance, deposit, withdraw, formatUsdc, parseUsdc,
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
            getBalance(found.solanaPubkey, walletAddress).then(setBalance);
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
      const newEvent = await createEvent(
        groupId as string,
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

      const success = await deposit(group!.solanaPubkey!, user.walletAddress, amount, signature, 'sol');

      if (success) {
        setShowDeposit(false);
        setTransactionAmount('');
        // Refresh balance
        if (group?.solanaPubkey) {
          const bal = await getBalance(group.solanaPubkey, user.walletAddress);
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

      const result = await withdraw(group.solanaPubkey, user.walletAddress, amountUsdc);

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
            padding: '24px',
            borderRadius: '12px',
            background: '#050505',
            border: '1px solid #222',
          }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '24px' }}>
              <h2 style={{ fontSize: '16px', fontWeight: '500', color: '#888', letterSpacing: '0.05em', textTransform: 'lowercase' }}>members</h2>
              <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
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

            {/* Danger Zone (Admin Only) */}
            {isAdmin && group && (
              <div style={{ marginTop: '64px', borderTop: '1px solid #222', paddingTop: '32px', textAlign: 'center' }}>
                <button
                  onClick={handleDeleteGroup}
                  style={{
                    background: 'transparent',
                    border: '1px solid #450a0a',
                    color: '#ef4444',
                    padding: '12px 24px',
                    borderRadius: '8px',
                    fontSize: '14px',
                    cursor: 'pointer',
                    opacity: 0.7,
                    transition: 'all 0.2s'
                  }}
                  onMouseEnter={(e) => { e.currentTarget.style.background = '#450a0a'; e.currentTarget.style.opacity = '1'; }}
                  onMouseLeave={(e) => { e.currentTarget.style.background = 'transparent'; e.currentTarget.style.opacity = '0.7'; }}
                >
                  Delete Group
                </button>
              </div>
            )}
          </section>

          {/* Balance Section */}
          <section style={{
            width: '100%',
            padding: '32px',
            borderRadius: '16px',
            textAlign: 'center',
            background: '#000',
            border: '1px solid #222',
            boxSizing: 'border-box'
          }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '24px' }}>
              <h2 style={{ fontSize: '18px', fontWeight: '500', color: '#fff', margin: 0 }}>your balance</h2>
              <div style={{ display: 'flex', gap: '12px', alignItems: 'center' }}>
                <button
                  onClick={() => { setShowDeposit(true); setShowWithdraw(false); setTransactionError(null); }}
                  style={{ fontSize: '14px', padding: '8px 16px', borderRadius: '8px', background: '#111', color: '#fff', border: '1px solid #333', cursor: 'pointer' }}
                >
                  + deposit
                </button>
                <button
                  onClick={() => { setShowWithdraw(true); setShowDeposit(false); setTransactionError(null); }}
                  style={{ fontSize: '14px', padding: '8px 16px', borderRadius: '8px', background: 'transparent', color: '#888', border: 'none', cursor: 'pointer' }}
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
              <form onSubmit={handleDeposit} style={{
                marginTop: '32px',
                padding: '32px',
                border: '1px solid rgba(255, 255, 255, 0.3)',
                borderRadius: '16px',
                maxWidth: '600px',
                margin: '32px auto 0',
                textAlign: 'left'
              }}>
                <p style={{ fontSize: '18px', color: 'rgba(255, 255, 255, 0.8)', marginBottom: '16px' }}>deposit USDC to bet in this group</p>
                <input
                  type="number"
                  value={transactionAmount}
                  onChange={(e) => setTransactionAmount(e.target.value)}
                  placeholder="amount in USDC"
                  step="0.01"
                  min="0.01"
                  style={{
                    width: '100%',
                    fontSize: '20px',
                    padding: '16px 0',
                    border: 'none',
                    borderBottom: '2px solid rgba(255, 255, 255, 0.4)',
                    background: 'transparent',
                    marginBottom: '16px',
                    color: 'white',
                    outline: 'none'
                  }}
                  autoFocus
                />
                {transactionError && (
                  <p style={{ color: '#ef4444', marginBottom: '16px' }}>{transactionError}</p>
                )}
                <div style={{ display: 'flex', gap: '16px', alignItems: 'center' }}>
                  <WalletMultiButton style={{ background: '#333', height: '40px', fontSize: '14px' }} />
                  <button
                    onClick={() => router.push('/dashboard')}
                    style={{ padding: '8px 16px', borderRadius: '8px', background: 'rgba(255, 255, 255, 0.1)', color: 'white', border: 'none', cursor: 'pointer' }}
                  >
                    back to home
                  </button>
                  <button
                    type="submit"
                    disabled={!transactionAmount || isTransacting}
                    style={{
                      fontSize: '20px',
                      color: !transactionAmount || isTransacting ? 'rgba(255, 255, 255, 0.4)' : 'rgba(255, 255, 255, 0.8)',
                      background: 'none',
                      border: 'none',
                      cursor: !transactionAmount || isTransacting ? 'not-allowed' : 'pointer',
                      transition: 'color 0.2s'
                    }}
                  >
                    {isTransacting ? 'depositing...' : 'deposit →'}
                  </button>
                  <button
                    type="button"
                    onClick={() => { setShowDeposit(false); setTransactionAmount(''); setTransactionError(null); }}
                    style={{ fontSize: '20px', color: 'rgba(255, 255, 255, 0.5)', background: 'none', border: 'none', cursor: 'pointer' }}
                  >
                    cancel
                  </button>
                </div>
              </form>
            )}

            {/* Withdraw Form */}
            {showWithdraw && (
              <form onSubmit={handleWithdraw} style={{
                marginTop: '32px',
                padding: '32px',
                border: '1px solid rgba(255, 255, 255, 0.3)',
                borderRadius: '16px',
                maxWidth: '600px',
                margin: '32px auto 0',
                textAlign: 'left'
              }}>
                <p style={{ fontSize: '18px', color: 'rgba(255, 255, 255, 0.8)', marginBottom: '16px' }}>withdraw USDC from this group</p>
                <input
                  type="number"
                  value={transactionAmount}
                  onChange={(e) => setTransactionAmount(e.target.value)}
                  placeholder="amount in USDC"
                  step="0.01"
                  min="0.01"
                  style={{
                    width: '100%',
                    fontSize: '20px',
                    padding: '16px 0',
                    border: 'none',
                    borderBottom: '2px solid rgba(255, 255, 255, 0.4)',
                    background: 'transparent',
                    marginBottom: '16px',
                    color: 'white',
                    outline: 'none'
                  }}
                  autoFocus
                />
                {transactionError && (
                  <p style={{ color: '#ef4444', marginBottom: '16px' }}>{transactionError}</p>
                )}
                <div style={{ display: 'flex', gap: '32px' }}>
                  <button
                    type="submit"
                    disabled={!transactionAmount || isTransacting}
                    style={{
                      fontSize: '20px',
                      color: !transactionAmount || isTransacting ? 'rgba(255, 255, 255, 0.4)' : 'rgba(255, 255, 255, 0.8)',
                      background: 'none',
                      border: 'none',
                      cursor: !transactionAmount || isTransacting ? 'not-allowed' : 'pointer',
                      transition: 'color 0.2s'
                    }}
                  >
                    {isTransacting ? 'withdrawing...' : 'withdraw →'}
                  </button>
                  <button
                    type="button"
                    onClick={() => { setShowWithdraw(false); setTransactionAmount(''); setTransactionError(null); }}
                    style={{ fontSize: '20px', color: 'rgba(255, 255, 255, 0.5)', background: 'none', border: 'none', cursor: 'pointer' }}
                  >
                    cancel
                  </button>
                </div>
              </form>
            )}
          </section>

          {/* Markets Section */}
          <section style={{
            width: '100%',
            padding: '32px',
            borderRadius: '16px',
            background: 'rgba(255, 255, 255, 0.03)',
            backdropFilter: 'blur(10px)',
            border: '1px solid rgba(255, 255, 255, 0.08)',
            boxSizing: 'border-box'
          }}>
            <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '32px' }}>
              <h2 style={{ fontSize: '20px', fontWeight: '500', color: 'rgba(255, 255, 255, 0.9)', margin: 0 }}>new bets</h2>
              {!showCreate && (
                <button
                  onClick={() => setShowCreate(true)}
                  className="text-sm px-4 py-2 rounded-full bg-blue-500/20 text-blue-400 hover:bg-blue-500/30 transition-all"
                >
                  + new market
                </button>
              )}
            </div>

            {/* Create Market Modal */}
            {showCreate && (
              <div style={{
                position: 'fixed',
                top: 0,
                left: 0,
                width: '100vw',
                height: '100vh',
                background: 'rgba(0, 0, 0, 0.6)',
                backdropFilter: 'blur(12px)',
                zIndex: 1000,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                padding: '20px'
              }}>
                <div style={{
                  background: '#090909',
                  border: '1px solid #222',
                  borderRadius: '24px',
                  padding: '40px',
                  width: '100%',
                  maxWidth: '550px',
                  boxShadow: '0 50px 100px -20px rgba(0,0,0,0.9)'
                }}>
                  <h2 style={{ fontSize: '20px', marginBottom: '32px', textAlign: 'center', color: '#fff' }}>New Market</h2>

                  <form onSubmit={handleCreateMarket}>
                    <div style={{ marginBottom: '24px' }}>
                      <label style={{ display: 'block', fontSize: '12px', color: '#666', marginBottom: '8px', textTransform: 'uppercase', letterSpacing: '0.05em' }}>Question</label>
                      <input
                        type="text"
                        value={title}
                        onChange={(e) => setTitle(e.target.value)}
                        placeholder="e.g. Will SOL hit $200?"
                        style={{ width: '100%', padding: '16px', background: '#111', border: '1px solid #333', borderRadius: '8px', color: 'white', fontSize: '16px', outline: 'none' }}
                        autoFocus
                      />
                    </div>

                    <div style={{ marginBottom: '24px' }}>
                      <label style={{ display: 'block', fontSize: '14px', color: '#888', marginBottom: '8px' }}>Outcomes</label>
                      <div style={{ display: 'flex', gap: '12px' }}>
                        <input
                          type="text"
                          value={outcome1}
                          onChange={(e) => setOutcome1(e.target.value)}
                          placeholder="Yes"
                          style={{ flex: 1, padding: '16px', background: 'rgba(255, 255, 255, 0.05)', border: '1px solid rgba(255, 255, 255, 0.1)', borderRadius: '12px', color: 'white', outline: 'none' }}
                        />
                        <input
                          type="text"
                          value={outcome2}
                          onChange={(e) => setOutcome2(e.target.value)}
                          placeholder="No"
                          style={{ flex: 1, padding: '16px', background: 'rgba(255, 255, 255, 0.05)', border: '1px solid rgba(255, 255, 255, 0.1)', borderRadius: '12px', color: 'white', outline: 'none' }}
                        />
                      </div>
                    </div>

                    <div style={{ marginBottom: '24px' }}>
                      <label style={{ display: 'block', fontSize: '14px', color: '#888', marginBottom: '8px' }}>Settlement Mode</label>
                      <select
                        value={settlementType}
                        onChange={(e) => setSettlementType(e.target.value as any)}
                        style={{ width: '100%', padding: '16px', background: 'rgba(255, 255, 255, 0.05)', border: '1px solid rgba(255, 255, 255, 0.1)', borderRadius: '12px', color: 'white', outline: 'none', appearance: 'none' }}
                      >
                        <option value="manual">Manual / Arbiter</option>
                        <option value="oracle">Oracle (Automated)</option>
                        <option value="consensus">Group Consensus</option>
                      </select>
                    </div>

                    {settlementType === 'manual' && (
                      <div style={{ marginBottom: '32px' }}>
                        <label style={{ display: 'block', fontSize: '14px', color: '#888', marginBottom: '8px' }}>Arbiter</label>
                        <select
                          value={arbiterWallet}
                          onChange={(e) => setArbiterWallet(e.target.value)}
                          style={{ width: '100%', padding: '16px', background: 'rgba(255, 255, 255, 0.05)', border: '1px solid rgba(255, 255, 255, 0.1)', borderRadius: '12px', color: 'white', outline: 'none', appearance: 'none' }}
                        >
                          <option value="">Select arbiter (defaults to you)</option>
                          {members.map(member => (
                            <option key={member.userId} value={member.walletAddress}>
                              {member.walletAddress.slice(0, 6)}...{member.walletAddress.slice(-4)}
                              {member.role === 'admin' ? ' (Admin)' : ''}
                            </option>
                          ))}
                        </select>
                      </div>
                    )}

                    <div style={{ display: 'flex', gap: '16px' }}>
                      <button
                        type="button"
                        onClick={() => setShowCreate(false)}
                        style={{ flex: 1, padding: '16px', borderRadius: '12px', background: 'transparent', border: '1px solid rgba(255, 255, 255, 0.2)', color: 'white', cursor: 'pointer' }}
                      >
                        Cancel
                      </button>
                      <button
                        type="submit"
                        disabled={!title.trim() || isCreating}
                        style={{ flex: 1, padding: '16px', borderRadius: '12px', background: 'linear-gradient(135deg, #10B981 0%, #059669 100%)', border: 'none', color: 'white', fontWeight: '500', cursor: 'pointer', opacity: (!title.trim() || isCreating) ? 0.5 : 1 }}
                      >
                        {isCreating ? 'Creating...' : 'Create Market'}
                      </button>
                    </div>
                  </form>
                </div>
              </div>
            )}

            {/* Markets List */}
            {events.length > 0 ? (
              <ul style={{ display: 'flex', flexDirection: 'column', gap: '16px', listStyle: 'none', padding: 0, margin: 0 }}>
                {events.map((event) => {
                  const eventPrices = prices[event.eventId];
                  const isActive = event.status === 'active';

                  return (
                    <li key={event.eventId}>
                      <button
                        onClick={() => router.push(`/event/${event.eventId}`)}
                        style={{
                          width: '100%',
                          textAlign: 'left',
                          padding: '32px 24px',
                          border: '1px solid rgba(255, 255, 255, 0.2)',
                          borderRadius: '8px',
                          background: 'transparent',
                          transition: 'all 0.2s',
                          cursor: 'pointer',
                          display: 'block',
                          position: 'relative'
                        }}
                        onMouseEnter={(e) => {
                          e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.4)';
                          e.currentTarget.style.background = 'rgba(255, 255, 255, 0.05)';
                        }}
                        onMouseLeave={(e) => {
                          e.currentTarget.style.borderColor = 'rgba(255, 255, 255, 0.2)';
                          e.currentTarget.style.background = 'transparent';
                        }}
                      >
                        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start' }}>
                          <p style={{ fontSize: '24px', marginBottom: '16px', color: 'white', margin: '0 0 16px 0', flex: 1 }}>{event.title}</p>

                          {group?.adminWallet === user.walletAddress && (
                            <div
                              role="button"
                              onClick={(e) => handleDeleteEvent(event.eventId, e)}
                              style={{
                                color: 'rgba(255, 50, 50, 0.6)',
                                fontSize: '14px',
                                padding: '4px 8px',
                                border: '1px solid rgba(255, 50, 50, 0.3)',
                                borderRadius: '4px',
                                marginLeft: '16px',
                                transition: 'all 0.2s',
                                zIndex: 10
                              }}
                              onMouseEnter={(e) => {
                                e.currentTarget.style.color = 'rgba(255, 50, 50, 1)';
                                e.currentTarget.style.borderColor = 'rgba(255, 50, 50, 0.8)';
                                e.currentTarget.style.background = 'rgba(255, 50, 50, 0.1)';
                              }}
                              onMouseLeave={(e) => {
                                e.currentTarget.style.color = 'rgba(255, 50, 50, 0.6)';
                                e.currentTarget.style.borderColor = 'rgba(255, 50, 50, 0.3)';
                                e.currentTarget.style.background = 'transparent';
                              }}
                            >
                              delete
                            </div>
                          )}
                        </div>

                        {isActive && eventPrices && (
                          <div style={{ display: 'flex', gap: '40px', fontSize: '18px' }}>
                            {event.outcomes.map((outcome) => (
                              <span key={outcome} style={{ color: 'rgba(255, 255, 255, 0.7)' }}>
                                <span style={{ color: 'white', fontWeight: '500' }}>{formatPrice(eventPrices.prices[outcome] || 0.5)}</span>
                                {' '}{outcome}
                              </span>
                            ))}
                            {eventPrices.totalVolume > 0 && (
                              <span style={{ color: 'rgba(255, 255, 255, 0.5)' }}>
                                {formatUsdc(eventPrices.totalVolume)} vol
                              </span>
                            )}
                          </div>
                        )}

                        {event.status === 'resolved' && (
                          <p style={{ fontSize: '18px', color: 'rgba(255, 255, 255, 0.6)', margin: 0 }}>
                            resolved: <span style={{ color: 'rgba(255, 255, 255, 0.8)', fontStyle: 'italic' }}>{event.winningOutcome}</span>
                          </p>
                        )}
                      </button>
                    </li>
                  );
                })}
              </ul>
            ) : !showCreate ? (
              <div style={{ padding: '64px 0', textAlign: 'center' }}>
                <p style={{ fontSize: '24px', color: 'rgba(255, 255, 255, 0.6)', fontStyle: 'italic', marginBottom: '40px' }}>
                  no markets yet.
                </p>
                <button
                  onClick={() => setShowCreate(true)}
                  style={{
                    fontSize: '24px',
                    color: 'rgba(255, 255, 255, 0.8)',
                    background: 'none',
                    border: 'none',
                    cursor: 'pointer',
                    transition: 'color 0.2s'
                  }}
                  onMouseEnter={(e) => e.currentTarget.style.color = 'white'}
                  onMouseLeave={(e) => e.currentTarget.style.color = 'rgba(255, 255, 255, 0.8)'}
                >
                  create the first market →
                </button>
              </div>
            ) : null}
          </section>
        </div >
      </main >
    </div >
  );
}
