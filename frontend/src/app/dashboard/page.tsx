'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import {
  createGroup,
  getGroups,
  saveGroups,
  addGroupCreatorAsMember,
  getBalance,
  deposit,
  withdraw,
  formatUsdc,
  parseUsdc
} from '@/lib/api';
import { BRAND } from '@/lib/brand';
import type { FriendGroup, BalanceResponse } from '@/types';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import { SystemProgram, Transaction, LAMPORTS_PER_SOL, PublicKey } from '@solana/web3.js';
import { WalletMultiButton } from '@solana/wallet-adapter-react-ui';

export default function DashboardPage() {
  const router = useRouter();
  const { user, checkAuth, logout, isLoading, isInitialized } = useAuthStore();
  const [groups, setGroups] = useState<FriendGroup[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [newGroupName, setNewGroupName] = useState('');
  const [isCreating, setIsCreating] = useState(false);
  const [pageReady, setPageReady] = useState(false);

  // Wallet state
  const { connection } = useConnection();
  const { publicKey, sendTransaction, connected } = useWallet();
  const [balance, setBalance] = useState<BalanceResponse | null>(null);
  const [showDeposit, setShowDeposit] = useState(false);
  const [showWithdraw, setShowWithdraw] = useState(false);
  const [transactionAmount, setTransactionAmount] = useState('');
  const [transactionError, setTransactionError] = useState<string | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);

  // Check auth on mount if not initialized
  useEffect(() => {
    if (!isInitialized) {
      checkAuth();
    }
  }, [checkAuth, isInitialized]);

  // Redirect if not logged in (after initialization)
  useEffect(() => {
    if (isInitialized && !user.isLoggedIn) {
      router.push('/');
    } else if (isInitialized && user.isLoggedIn) {
      setPageReady(true);
    }
  }, [user.isLoggedIn, isInitialized, router]);

  // Load groups
  useEffect(() => {
    if (user.walletAddress) {
      getGroups(user.walletAddress).then(setGroups);
    }
  }, [user.walletAddress]);

  // Load wallet balance (using first group's treasury)
  useEffect(() => {
    if (user.walletAddress && groups.length > 0 && pageReady) {
      const firstGroup = groups[0];
      if (firstGroup.solanaPubkey && !firstGroup.solanaPubkey.includes('_')) {
        getBalance(firstGroup.groupId, user.walletAddress).then(setBalance);
      }
    }
  }, [user.walletAddress, groups, pageReady]);

  const handleDeposit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!user.walletAddress || !transactionAmount || groups.length === 0) return;

    setIsProcessing(true);
    setTransactionError(null);
    try {
      // Use first group as default logic for now
      const group = groups[0];
      const amount = parseFloat(transactionAmount);
      if (isNaN(amount) || amount <= 0) throw new Error("Invalid amount");

      let signature = 'dev';

      // If wallet connected, perform real transfer
      if (connected && publicKey) {
        try {
          // Check if group has valid pubkey (not mock)
          if (!group.solanaPubkey || group.solanaPubkey.startsWith('mock_')) {
            throw new Error("Group wallet not ready for real deposits (mock address)");
          }

          const transaction = new Transaction().add(
            SystemProgram.transfer({
              fromPubkey: publicKey,
              toPubkey: new PublicKey(group.solanaPubkey),
              lamports: amount * LAMPORTS_PER_SOL // Assuming amount is in SOL for this flow, or scale for USDC
            })
          );

          const { blockhash } = await connection.getLatestBlockhash();
          transaction.recentBlockhash = blockhash;
          transaction.feePayer = publicKey;

          signature = await sendTransaction(transaction, connection);
          await connection.confirmTransaction(signature, 'processed');
          console.log("Deposit Transaction confirmed:", signature);

        } catch (txError) {
          console.error("Wallet transaction failed:", txError);
          setTransactionError("Wallet transaction failed. Check console.");
          setIsProcessing(false);
          return;
        }
      }

      const success = await deposit(group.groupId, user.walletAddress, amount, signature, 'sol');

      if (success) {
        setShowDeposit(false);
        setTransactionAmount('');
        // Refresh balance
        const bal = await getBalance(group.groupId, user.walletAddress);
        setBalance(bal);
      } else {
        setTransactionError("Deposit failed backend verification");
      }
    } catch (e) {
      console.error(e);
      setTransactionError("Error processing deposit");
    } finally {
      setIsProcessing(false);
    }
  };

  const handleWithdraw = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!transactionAmount || !user.walletAddress || groups.length === 0) return;

    setIsProcessing(true);
    setTransactionError(null);

    try {
      const amountUsdc = parseUsdc(transactionAmount);
      const firstGroup = groups[0];

      if (!firstGroup.solanaPubkey || firstGroup.solanaPubkey.includes('_')) {
        throw new Error('Please create a group first to use wallet features');
      }

      const result = await withdraw(firstGroup.groupId, user.walletAddress, amountUsdc);

      if (result.success) {
        setBalance({
          balanceSol: result.newBalanceSol,
          balanceUsdc: result.newBalanceUsdc,
          fundsLocked: false,
        });
        setTransactionAmount('');
        setShowWithdraw(false);
      }
    } catch (error) {
      setTransactionError(error instanceof Error ? error.message : 'Withdrawal failed');
    } finally {
      setIsProcessing(false);
    }
  };

  const handleLogout = async () => {
    await logout();
    router.push('/');
  };

  const handleCreateGroup = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newGroupName.trim() || !user.walletAddress) return;

    setIsCreating(true);

    const newGroup = await createGroup(newGroupName, user.walletAddress);

    // Add creator as admin member
    addGroupCreatorAsMember(newGroup.groupId, user.walletAddress);

    const updatedGroups = [...groups, newGroup];
    setGroups(updatedGroups);
    saveGroups(updatedGroups);

    setNewGroupName('');
    setShowCreate(false);
    setIsCreating(false);
  };

  // Format wallet address for display
  const formatWallet = (address: string | null) => {
    if (!address) return '—';
    if (address.length <= 12) return address;
    return `${address.slice(0, 6)}...${address.slice(-4)}`;
  };

  // Show loading only during initial load
  if ((isLoading && !isInitialized) || !pageReady) {
    return (
      <main className="page-container">
        <p className="loading">loading...</p>
        <style jsx>{`
          .page-container {
            display: flex;
            flex-direction: column;
            justify-content: center;
            align-items: center;
            min-height: 100vh;
            min-height: 100dvh;
            padding: 5vw;
          }
          .loading {
            font-size: clamp(1.2rem, 5vw, 2rem);
            font-style: italic;
            opacity: 0.9;
          }
        `}</style>
      </main>
    );
  }

  return (
    <main className="page-container">
      {/* Sign out - fixed top left */}
      <button onClick={handleLogout} className="logout-btn">
        sign out →
      </button>

      <div className="content">
        {/* Header */}
        <header>
          <h1>{BRAND.name}</h1>
          <p className="tagline">{BRAND.tagline}</p>
        </header>

        {/* User Info */}
        <section className="user-section">
          <p className="label">signed in as</p>
          <p className="email">{user.email}</p>

          <div className="wallet-box">
            <p className="wallet-label">your solana wallet</p>
            <p className="wallet-address">
              {user.walletAddress === 'unavailable'
                ? 'wallet unavailable'
                : user.walletAddress
                  ? formatWallet(user.walletAddress)
                  : 'loading...'}
            </p>
            {user.walletAddress && user.walletAddress !== 'unavailable' && (
              <button
                onClick={() => navigator.clipboard.writeText(user.walletAddress!)}
                className="copy-btn"
              >
                copy full address
              </button>
            )}
          </div>
        </section>

        {/* Wallet Section */}
        <section className="wallet-section">
          <h2>your mitra wallet</h2>

          <div className="balance-display">
            <p className="balance-amount">
              ${balance ? formatUsdc(balance.balanceUsdc) : '0.00'} <span className="currency">USDC</span>
            </p>
            {balance?.fundsLocked && (
              <p className="funds-locked">⚠ some funds locked in active bets</p>
            )}
          </div>

          <div className="wallet-actions">
            <button
              onClick={() => { setShowDeposit(true); setShowWithdraw(false); setTransactionError(null); }}
              className="deposit-btn"
            >
              + deposit
            </button>
            <button
              onClick={() => { setShowWithdraw(true); setShowDeposit(false); setTransactionError(null); }}
              className="withdraw-btn"
            >
              − withdraw
            </button>
          </div>

          {/* Deposit Form */}
          {showDeposit && (
            <form onSubmit={handleDeposit} className="transaction-form">
              <input
                type="number"
                step="0.01"
                value={transactionAmount}
                onChange={(e) => setTransactionAmount(e.target.value)}
                placeholder="amount in USDC"
                disabled={isProcessing}
                className="amount-input"
                autoFocus
              />
              {transactionError && <p className="error-text">{transactionError}</p>}
              <div className="form-actions">
                <button type="submit" disabled={isProcessing || !transactionAmount} className="submit-btn">
                  {isProcessing ? 'processing...' : 'confirm deposit'}
                </button>
                <button type="button" onClick={() => { setShowDeposit(false); setTransactionAmount(''); setTransactionError(null); }} className="cancel-btn">
                  cancel
                </button>
              </div>
            </form>
          )}

          {/* Withdraw Form */}
          {showWithdraw && (
            <form onSubmit={handleWithdraw} className="transaction-form">
              <input
                type="number"
                step="0.01"
                value={transactionAmount}
                onChange={(e) => setTransactionAmount(e.target.value)}
                placeholder="amount in USDC"
                disabled={isProcessing}
                className="amount-input"
                autoFocus
              />
              {transactionError && <p className="error-text">{transactionError}</p>}
              <div className="form-actions">
                <button type="submit" disabled={isProcessing || !transactionAmount} className="submit-btn">
                  {isProcessing ? 'processing...' : 'confirm withdrawal'}
                </button>
                <button type="button" onClick={() => { setShowWithdraw(false); setTransactionAmount(''); setTransactionError(null); }} className="cancel-btn">
                  cancel
                </button>
              </div>
            </form>
          )}
        </section>

        {/* Groups Section */}
        <section className="groups-section">
          <div className="section-header">
            <h2>your groups</h2>
            {!showCreate && groups.length > 0 && (
              <button onClick={() => setShowCreate(true)} className="new-btn">
                + new
              </button>
            )}
          </div>

          {/* Create Group Form */}
          {showCreate && (
            <form onSubmit={handleCreateGroup} className="create-form">
              <input
                type="text"
                value={newGroupName}
                onChange={(e) => setNewGroupName(e.target.value)}
                placeholder="group name"
                autoFocus
              />
              <div className="form-actions">
                <button
                  type="submit"
                  disabled={!newGroupName.trim() || isCreating}
                  className="submit-btn"
                >
                  {isCreating ? 'creating...' : 'create →'}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowCreate(false);
                    setNewGroupName('');
                  }}
                  className="cancel-btn"
                >
                  cancel
                </button>
              </div>
            </form>
          )}

          {/* Groups List */}
          {groups.length > 0 ? (
            <ul className="groups-list">
              {groups.map((group) => (
                <li key={group.groupId}>
                  <button
                    onClick={() => router.push(`/group/${group.groupId}`)}
                    className="group-item"
                  >
                    {group.name}
                  </button>
                </li>
              ))}
            </ul>
          ) : !showCreate ? (
            <div className="empty-state">
              <p className="empty-text">no groups yet.</p>
              <p className="empty-hint">create a group to start betting with friends</p>
              <button onClick={() => setShowCreate(true)} className="create-first-btn">
                create your first group →
              </button>
            </div>
          ) : null}
        </section>
      </div>

      <style jsx>{`
        .page-container {
          display: flex;
          flex-direction: column;
          align-items: center;
          min-height: 100vh;
          min-height: 100dvh;
          padding: clamp(2rem, 5vw, 4rem);
          padding-top: clamp(3rem, 8vw, 6rem);
        }

        .content {
          max-width: 600px;
          width: 100%;
          text-align: center;
        }

        header {
          margin-bottom: clamp(3rem, 8vw, 5rem);
        }

        h1 {
          font-size: clamp(2rem, 8vw, 3rem);
          font-weight: normal;
          margin-bottom: clamp(0.5rem, 2vw, 1rem);
        }

        .tagline {
          font-size: clamp(1rem, 4vw, 1.3rem);
          font-style: italic;
          opacity: 0.7;
        }

        .user-section {
          padding: clamp(2rem, 5vw, 3rem) 0;
          border-top: 1px solid rgba(255, 255, 255, 0.15);
          border-bottom: 1px solid rgba(255, 255, 255, 0.15);
          margin-bottom: clamp(2rem, 5vw, 3rem);
        }

        .label {
          font-size: clamp(0.9rem, 3vw, 1.1rem);
          opacity: 0.6;
          margin-bottom: clamp(0.3rem, 1vw, 0.5rem);
        }

        .email {
          font-size: clamp(1.2rem, 5vw, 1.8rem);
          margin-bottom: clamp(1.5rem, 4vw, 2rem);
        }

        .wallet-box {
          display: inline-block;
          padding: clamp(1rem, 3vw, 1.5rem);
          background: rgba(255, 255, 255, 0.05);
          border-radius: 8px;
          margin-bottom: clamp(1.5rem, 4vw, 2rem);
        }

        .wallet-label {
          font-size: clamp(0.8rem, 2.5vw, 0.95rem);
          opacity: 0.6;
          margin-bottom: clamp(0.3rem, 1vw, 0.5rem);
        }

        .wallet-address {
          font-size: clamp(1rem, 3.5vw, 1.2rem);
          font-family: 'SF Mono', 'Monaco', monospace;
          letter-spacing: 0;
        }

        .copy-btn {
          display: block;
          margin-top: clamp(0.5rem, 1.5vw, 0.75rem);
          font-size: clamp(0.75rem, 2.5vw, 0.9rem);
          background: none;
          border: none;
          color: white;
          opacity: 0.5;
          cursor: pointer;
          transition: opacity 0.2s;
        }

        .copy-btn:hover {
          opacity: 0.8;
        }

        .wallet-section {
          padding: clamp(2rem, 5vw, 3rem) 0;
          border-bottom: 1px solid rgba(255, 255, 255, 0.15);
          margin-bottom: clamp(2rem, 5vw, 3rem);
        }

        .wallet-section h2 {
          font-size: clamp(1.2rem, 4vw, 1.5rem);
          font-weight: normal;
          margin-bottom: clamp(1.5rem, 4vw, 2rem);
        }

        .balance-display {
          margin-bottom: 2rem;
        }

        .balance-amount {
          font-size: clamp(2.5rem, 8vw, 3.5rem);
          font-weight: 300;
          margin-bottom: 0.5rem;
          letter-spacing: -0.02em;
        }

        .currency {
          font-size: clamp(1.5rem, 5vw, 2rem);
          opacity: 0.5;
        }

        .funds-locked {
          font-size: 0.9rem;
          color: #fbbf24;
          opacity: 0.9;
        }

        .wallet-actions {
          display: flex;
          gap: 1rem;
          justify-content: center;
          margin-bottom: 2rem;
        }

        .deposit-btn, .withdraw-btn {
          padding: 0.75rem 2rem;
          border-radius: 2rem;
          font-size: 1rem;
          cursor: pointer;
          transition: all 0.2s ease;
          border: none;
        }

        .deposit-btn {
          background: rgba(16, 185, 129, 0.2);
          color: #10b981;
        }

        .deposit-btn:hover {
          background: rgba(16, 185, 129, 0.3);
        }

        .withdraw-btn {
          background: rgba(255, 255, 255, 0.1);
          color: rgba(255, 255, 255, 0.7);
        }

        .withdraw-btn:hover {
          background: rgba(255, 255, 255, 0.2);
          color: rgba(255, 255, 255, 1);
        }

        .transaction-form {
          max-width: 350px;
          margin: 2rem auto 0;
          padding: 2rem;
          background: rgba(255, 255, 255, 0.03);
          border-radius: 1rem;
          border: 1px solid rgba(255, 255, 255, 0.1);
        }

        .amount-input {
          width: 100%;
          padding: 1rem;
          margin-bottom: 1rem;
          border-radius: 0.5rem;
          border: 1px solid rgba(255, 255, 255, 0.2);
          background: rgba(0, 0, 0, 0.3);
          color: white;
          font-size: 1.2rem;
          text-align: center;
        }

        .amount-input:focus {
          border-color: rgba(255, 255, 255, 0.4);
          outline: none;
        }

        .form-actions {
          display: flex;
          gap: 0.75rem;
          flex-direction: column;
        }

        .submit-btn {
          padding: 0.75rem 1.5rem;
          border-radius: 0.5rem;
          background: rgba(59, 130, 246, 0.2);
          color: #3b82f6;
          border: none;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .submit-btn:hover:not(:disabled) {
          background: rgba(59, 130, 246, 0.3);
        }

        .submit-btn:disabled {
          opacity: 0.5;
          cursor: not-allowed;
        }

        .cancel-btn {
          padding: 0.75rem 1.5rem;
          border-radius: 0.5rem;
          background: rgba(255, 255, 255, 0.05);
          color: rgba(255, 255, 255, 0.6);
          border: none;
          cursor: pointer;
          transition: all 0.2s ease;
        }

        .cancel-btn:hover {
          background: rgba(255, 255, 255, 0.1);
          color: rgba(255, 255, 255, 0.9);
        }

        .logout-btn {
          position: fixed;
          top: clamp(1rem, 3vw, 2rem);
          left: clamp(1rem, 3vw, 2rem);
          font-size: clamp(0.9rem, 3vw, 1.1rem);
          background: none;
          border: none;
          color: white;
          opacity: 0.5;
          cursor: pointer;
          transition: opacity 0.2s;
          z-index: 100;
        }

        .logout-btn:hover {
          opacity: 0.8;
        }

        .groups-section {
          text-align: center;
        }

        .section-header {
          display: flex;
          align-items: center;
          justify-content: center;
          gap: clamp(1rem, 3vw, 2rem);
          margin-bottom: clamp(1.5rem, 4vw, 2rem);
        }

        h2 {
          font-size: clamp(1.3rem, 5vw, 2rem);
          font-weight: normal;
        }

        .new-btn {
          font-size: clamp(0.9rem, 3vw, 1.1rem);
          background: none;
          border: none;
          color: white;
          opacity: 0.6;
          cursor: pointer;
          transition: opacity 0.2s;
        }

        .new-btn:hover {
          opacity: 1;
        }

        .create-form {
          margin-bottom: clamp(2rem, 5vw, 3rem);
        }

        .create-form input {
          font-size: clamp(1.1rem, 4vw, 1.5rem);
          padding: clamp(0.75rem, 2vw, 1rem);
          width: 100%;
          max-width: 350px;
          text-align: center;
          background: transparent;
          border: none;
          border-bottom: 1px solid rgba(255, 255, 255, 0.4);
          color: white;
          margin-bottom: clamp(1rem, 3vw, 1.5rem);
        }

        .create-form input:focus {
          outline: none;
          border-bottom-color: rgba(255, 255, 255, 0.8);
        }

        .create-form input::placeholder {
          color: rgba(255, 255, 255, 0.5);
          font-style: italic;
        }

        .form-actions {
          display: flex;
          justify-content: center;
          gap: clamp(1.5rem, 4vw, 2rem);
        }

        .submit-btn {
          font-size: clamp(1rem, 3.5vw, 1.2rem);
          background: none;
          border: none;
          color: white;
          opacity: 0.8;
          cursor: pointer;
          transition: opacity 0.2s;
        }

        .submit-btn:hover {
          opacity: 1;
        }

        .submit-btn:disabled {
          opacity: 0.3;
          cursor: not-allowed;
        }

        .cancel-btn {
          font-size: clamp(1rem, 3.5vw, 1.2rem);
          background: none;
          border: none;
          color: white;
          opacity: 0.5;
          cursor: pointer;
          transition: opacity 0.2s;
        }

        .cancel-btn:hover {
          opacity: 0.8;
        }

        .groups-list {
          list-style: none;
          padding: 0;
        }

        .groups-list li {
          margin-bottom: clamp(0.5rem, 1.5vw, 0.75rem);
        }

        .group-item {
          width: 100%;
          font-size: clamp(1.1rem, 4vw, 1.5rem);
          padding: clamp(1rem, 3vw, 1.5rem);
          background: none;
          border: 1px solid rgba(255, 255, 255, 0.15);
          border-radius: 8px;
          color: white;
          cursor: pointer;
          transition: all 0.2s;
        }

        .group-item:hover {
          border-color: rgba(255, 255, 255, 0.4);
          background: rgba(255, 255, 255, 0.03);
        }

        .empty-state {
          padding: clamp(2rem, 6vw, 4rem) 0;
        }

        .empty-text {
          font-size: clamp(1.2rem, 5vw, 1.8rem);
          font-style: italic;
          opacity: 0.6;
          margin-bottom: clamp(1rem, 3vw, 1.5rem);
        }

        .empty-hint {
          font-size: clamp(0.9rem, 3vw, 1.1rem);
          opacity: 0.5;
          margin-bottom: clamp(1.5rem, 4vw, 2rem);
        }

        .create-first-btn {
          font-size: clamp(1.1rem, 4vw, 1.4rem);
          font-style: italic;
          background: none;
          border: none;
          color: white;
          opacity: 0.8;
          cursor: pointer;
          transition: opacity 0.2s;
        }

        .create-first-btn:hover {
          opacity: 1;
        }

        @media (max-width: 767px) {
          .page-container {
            padding: 4vw;
            padding-top: 8vw;
          }
        }
      `}</style>
    </main>
  );
}
