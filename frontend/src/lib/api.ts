/**
 * API Client for Mitra Backend
 * 
 * Maps to gRPC service methods:
 * - CreateFriendGroup
 * - InviteMember  
 * - CreateEvent
 * - PlaceBet
 * - GetEventPrices
 * - SettleEvent
 */

import type {
  FriendGroup,
  Event,
  Bet,
  Prices,
  BetResponse,
  GroupMember,
} from '@/types';

const API_BASE = '/api/grpc';

// Check if backend is available
let backendAvailable: boolean | null = null;

async function checkBackend(): Promise<boolean> {
  if (backendAvailable !== null) return backendAvailable;
  
  try {
    const res = await fetch(API_BASE, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ method: 'ping' }),
    });
    backendAvailable = res.ok;
  } catch {
    backendAvailable = false;
  }
  
  console.log(backendAvailable ? '✅ Backend connected' : '⚠️ Backend offline - using mock data');
  return backendAvailable;
}

// ===========================================
// Friend Groups
// ===========================================

export async function createGroup(
  name: string,
  adminWallet: string,
  signature: string = 'dev'
): Promise<FriendGroup> {
  const isOnline = await checkBackend();
  
  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'CreateFriendGroup',
          data: {
            name,
            admin_wallet: adminWallet,
            solana_pubkey: 'grp_' + Math.random().toString(36).substring(2, 10),
            signature,
          },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        return {
          groupId: data.groupId || data.group_id,
          name: data.name,
          solanaPubkey: data.solanaPubkey || data.solana_pubkey,
          adminWallet: data.adminWallet || data.admin_wallet,
          createdAt: data.createdAt || data.created_at || Date.now() / 1000,
        };
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
  }
  
  // Mock fallback
  return {
    groupId: 'grp_' + Math.random().toString(36).substring(2, 15),
    name,
    solanaPubkey: 'mock_' + Math.random().toString(36).substring(2, 10),
    adminWallet,
    createdAt: Math.floor(Date.now() / 1000),
  };
}

export async function getGroups(walletAddress: string): Promise<FriendGroup[]> {
  // For now, groups are stored in localStorage
  // In production, this would query the backend
  const stored = localStorage.getItem('mitra_groups');
  if (stored) {
    return JSON.parse(stored);
  }
  return [];
}

export function saveGroups(groups: FriendGroup[]): void {
  localStorage.setItem('mitra_groups', JSON.stringify(groups));
}

// ===========================================
// Events (Markets)
// ===========================================

export async function createEvent(
  groupId: string,
  title: string,
  description: string,
  outcomes: string[],
  settlementType: 'manual' | 'oracle' | 'consensus',
  resolveBy: number | null,
  creatorWallet: string,
  signature: string = 'dev'
): Promise<Event> {
  const isOnline = await checkBackend();
  
  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'CreateEvent',
          data: {
            group_id: groupId,
            title,
            description,
            outcomes,
            settlement_type: settlementType,
            resolve_by: resolveBy || 0,
            creator_wallet: creatorWallet,
            signature,
          },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        return {
          eventId: data.eventId || data.event_id,
          groupId: data.groupId || data.group_id,
          solanaPubkey: data.solanaPubkey || data.solana_pubkey,
          title: data.title,
          description: data.description,
          outcomes: data.outcomes,
          settlementType: data.settlementType || data.settlement_type || 'manual',
          status: data.status || 'active',
          resolveBy: data.resolveBy || data.resolve_by,
          createdAt: data.createdAt || data.created_at || Date.now() / 1000,
        };
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
  }
  
  // Mock fallback
  return {
    eventId: 'evt_' + Math.random().toString(36).substring(2, 15),
    groupId,
    title,
    description,
    outcomes,
    settlementType,
    status: 'active',
    resolveBy: resolveBy || undefined,
    createdAt: Math.floor(Date.now() / 1000),
  };
}

export async function getEvents(groupId: string): Promise<Event[]> {
  const stored = localStorage.getItem(`mitra_events_${groupId}`);
  if (stored) {
    return JSON.parse(stored);
  }
  return [];
}

export function saveEvents(groupId: string, events: Event[]): void {
  localStorage.setItem(`mitra_events_${groupId}`, JSON.stringify(events));
}

export async function getEvent(eventId: string): Promise<Event | null> {
  // Search all groups for this event
  const allKeys = Object.keys(localStorage).filter(k => k.startsWith('mitra_events_'));
  for (const key of allKeys) {
    const events = JSON.parse(localStorage.getItem(key) || '[]') as Event[];
    const found = events.find(e => e.eventId === eventId);
    if (found) return found;
  }
  return null;
}

// ===========================================
// Prices & AMM
// ===========================================

export async function getEventPrices(eventId: string): Promise<Prices> {
  const isOnline = await checkBackend();
  
  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'GetEventPrices',
          data: { event_id: eventId },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        return {
          eventId: data.eventId || data.event_id,
          prices: data.prices || {},
          totalVolume: data.totalVolume || data.total_volume || 0,
          timestamp: data.timestamp || Date.now() / 1000,
        };
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
  }
  
  // Mock prices - start at 50/50
  const event = await getEvent(eventId);
  const outcomes = event?.outcomes || ['yes', 'no'];
  const prices: Record<string, number> = {};
  outcomes.forEach(o => {
    prices[o] = 1 / outcomes.length;
  });
  
  // Adjust based on stored bets
  const bets = getBets(eventId);
  let totalVolume = 0;
  const volumes: Record<string, number> = {};
  outcomes.forEach(o => { volumes[o] = 0; });
  
  bets.forEach(b => {
    volumes[b.outcome] = (volumes[b.outcome] || 0) + b.amountUsdc;
    totalVolume += b.amountUsdc;
  });
  
  if (totalVolume > 0) {
    outcomes.forEach(o => {
      // Simple volume-weighted pricing
      prices[o] = 0.1 + 0.8 * (volumes[o] / totalVolume);
    });
    // Normalize to sum to 1
    const sum = Object.values(prices).reduce((a, b) => a + b, 0);
    outcomes.forEach(o => {
      prices[o] = prices[o] / sum;
    });
  }
  
  return {
    eventId,
    prices,
    totalVolume,
    timestamp: Date.now() / 1000,
  };
}

// ===========================================
// Bets
// ===========================================

export async function placeBet(
  eventId: string,
  userWallet: string,
  outcome: string,
  amountUsdc: number,
  signature: string = 'dev',
  isPublic: boolean = false
): Promise<BetResponse> {
  const isOnline = await checkBackend();
  
  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'PlaceBet',
          data: {
            event_id: eventId,
            user_wallet: userWallet,
            outcome,
            amount_usdc: amountUsdc,
            signature,
            is_public: isPublic,
          },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        
        // Save bet locally too
        const bet: Bet = {
          betId: data.betId || data.bet_id,
          eventId,
          userId: userWallet,
          outcome,
          shares: data.shares,
          price: data.price,
          amountUsdc,
          createdAt: Date.now() / 1000,
          isPublic,
        };
        saveBet(eventId, bet);
        
        return {
          betId: bet.betId,
          shares: data.shares,
          price: data.price,
          updatedPrices: data.updatedPrices || data.updated_prices,
        };
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
  }
  
  // Mock bet
  const prices = await getEventPrices(eventId);
  const price = prices.prices[outcome] || 0.5;
  const shares = amountUsdc / price;
  
  const bet: Bet = {
    betId: 'bet_' + Math.random().toString(36).substring(2, 15),
    eventId,
    userId: userWallet,
    outcome,
    shares,
    price,
    amountUsdc,
    createdAt: Date.now() / 1000,
    isPublic,
  };
  
  saveBet(eventId, bet);
  
  // Recalculate prices after bet
  const newPrices = await getEventPrices(eventId);
  
  return {
    betId: bet.betId,
    shares,
    price,
    updatedPrices: newPrices,
  };
}

export function getBets(eventId: string): Bet[] {
  const stored = localStorage.getItem(`mitra_bets_${eventId}`);
  if (stored) {
    return JSON.parse(stored);
  }
  return [];
}

export function getUserBets(eventId: string, userWallet: string): Bet[] {
  return getBets(eventId).filter(b => b.userId === userWallet);
}

function saveBet(eventId: string, bet: Bet): void {
  const bets = getBets(eventId);
  bets.push(bet);
  localStorage.setItem(`mitra_bets_${eventId}`, JSON.stringify(bets));
}

/**
 * Get all public bets for an event (visible to all group members)
 */
export function getPublicBets(eventId: string): Bet[] {
  return getBets(eventId).filter(b => b.isPublic);
}

// ===========================================
// Settlement
// ===========================================

export async function settleEvent(
  eventId: string,
  winningOutcome: string,
  settlerWallet: string,
  signature: string = 'dev'
): Promise<{ success: boolean; txSignature?: string }> {
  const isOnline = await checkBackend();
  
  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'SettleEvent',
          data: {
            event_id: eventId,
            winning_outcome: winningOutcome,
            settler_wallet: settlerWallet,
            signature,
          },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        
        // Update local event status
        updateEventStatus(eventId, 'resolved', winningOutcome);
        
        return {
          success: true,
          txSignature: data.solanaTxSignature || data.solana_tx_signature,
        };
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
  }
  
  // Mock settlement
  updateEventStatus(eventId, 'resolved', winningOutcome);
  
  return {
    success: true,
    txSignature: 'mock_tx_' + Date.now(),
  };
}

function updateEventStatus(eventId: string, status: string, winningOutcome?: string): void {
  const allKeys = Object.keys(localStorage).filter(k => k.startsWith('mitra_events_'));
  for (const key of allKeys) {
    const events = JSON.parse(localStorage.getItem(key) || '[]') as Event[];
    const idx = events.findIndex(e => e.eventId === eventId);
    if (idx >= 0) {
      events[idx].status = status as Event['status'];
      if (winningOutcome) {
        events[idx].winningOutcome = winningOutcome;
      }
      localStorage.setItem(key, JSON.stringify(events));
      break;
    }
  }
}

// ===========================================
// Balance / Treasury Operations
// ===========================================

import type { BalanceResponse, TransactionResponse } from '@/types';

export async function getBalance(
  groupId: string,
  userWallet: string
): Promise<BalanceResponse> {
  const isOnline = await checkBackend();
  
  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'GetUserBalance',
          data: {
            group_id: groupId,
            user_wallet: userWallet,
          },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        return {
          balanceSol: data.balanceSol || data.balance_sol || 0,
          balanceUsdc: data.balanceUsdc || data.balance_usdc || 0,
          fundsLocked: data.fundsLocked || data.funds_locked || false,
        };
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
  }
  
  // Mock balance from localStorage
  const stored = localStorage.getItem(`mitra_balance_${groupId}_${userWallet}`);
  if (stored) {
    return JSON.parse(stored);
  }
  return { balanceSol: 0, balanceUsdc: 0, fundsLocked: false };
}

export async function deposit(
  groupId: string,
  userWallet: string,
  amountUsdc: number,
  signature: string = 'dev'
): Promise<TransactionResponse> {
  const isOnline = await checkBackend();
  
  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'DepositFunds',
          data: {
            group_id: groupId,
            user_wallet: userWallet,
            amount_sol: 0,
            amount_usdc: amountUsdc,
            user_usdc_account: userWallet, // Placeholder - would need real USDC ATA
            signature,
          },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        return {
          success: data.success,
          solanaTxSignature: data.solanaTxSignature || data.solana_tx_signature,
          newBalanceSol: data.newBalanceSol || data.new_balance_sol || 0,
          newBalanceUsdc: data.newBalanceUsdc || data.new_balance_usdc || 0,
        };
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
  }
  
  // Mock deposit
  const current = await getBalance(groupId, userWallet);
  const newBalance: BalanceResponse = {
    ...current,
    balanceUsdc: current.balanceUsdc + amountUsdc,
  };
  localStorage.setItem(`mitra_balance_${groupId}_${userWallet}`, JSON.stringify(newBalance));
  
  return {
    success: true,
    solanaTxSignature: 'mock_deposit_' + Date.now(),
    newBalanceSol: newBalance.balanceSol,
    newBalanceUsdc: newBalance.balanceUsdc,
  };
}

export async function withdraw(
  groupId: string,
  userWallet: string,
  amountUsdc: number,
  signature: string = 'dev'
): Promise<TransactionResponse> {
  const isOnline = await checkBackend();
  
  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'WithdrawFunds',
          data: {
            group_id: groupId,
            user_wallet: userWallet,
            amount_sol: 0,
            amount_usdc: amountUsdc,
            user_usdc_account: userWallet, // Placeholder
            signature,
          },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        return {
          success: data.success,
          solanaTxSignature: data.solanaTxSignature || data.solana_tx_signature,
          newBalanceSol: data.newBalanceSol || data.new_balance_sol || 0,
          newBalanceUsdc: data.newBalanceUsdc || data.new_balance_usdc || 0,
        };
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
  }
  
  // Mock withdraw
  const current = await getBalance(groupId, userWallet);
  if (current.balanceUsdc < amountUsdc) {
    throw new Error('Insufficient balance');
  }
  
  const newBalance: BalanceResponse = {
    ...current,
    balanceUsdc: current.balanceUsdc - amountUsdc,
  };
  localStorage.setItem(`mitra_balance_${groupId}_${userWallet}`, JSON.stringify(newBalance));
  
  return {
    success: true,
    solanaTxSignature: 'mock_withdraw_' + Date.now(),
    newBalanceSol: newBalance.balanceSol,
    newBalanceUsdc: newBalance.balanceUsdc,
  };
}

// Format USDC for display (6 decimal places on chain -> display)
export function formatUsdc(amount: number): string {
  return (amount / 1_000_000).toFixed(2);
}

// Parse USDC from display (display -> 6 decimal places)
export function parseUsdc(display: string): number {
  return Math.floor(parseFloat(display) * 1_000_000);
}

// ===========================================
// Invite System & Members
// ===========================================

interface InviteData {
  groupId: string;
  code: string;
  createdBy: string;
  createdAt: number;
}

/**
 * Generate a unique invite link for a group
 */
export function generateInviteLink(groupId: string, creatorWallet: string): string {
  const code = `inv_${groupId.slice(0, 8)}_${Math.random().toString(36).substring(2, 8)}`;
  
  // Store invite in localStorage
  const invites = getStoredInvites();
  invites.push({
    groupId,
    code,
    createdBy: creatorWallet,
    createdAt: Date.now(),
  });
  localStorage.setItem('mitra_invites', JSON.stringify(invites));
  
  // Return full URL
  return `${window.location.origin}/invite/${code}`;
}

function getStoredInvites(): InviteData[] {
  const stored = localStorage.getItem('mitra_invites');
  return stored ? JSON.parse(stored) : [];
}

/**
 * Get invite data by code
 */
export function getInviteByCode(code: string): InviteData | null {
  const invites = getStoredInvites();
  return invites.find(i => i.code === code) || null;
}

/**
 * Join a group using an invite code
 */
export async function joinGroupByInvite(
  code: string,
  walletAddress: string,
  email: string
): Promise<{ success: boolean; groupId?: string; error?: string }> {
  const invite = getInviteByCode(code);
  if (!invite) {
    return { success: false, error: 'Invalid invite code' };
  }
  
  // Check if already a member
  const members = getGroupMembers(invite.groupId);
  if (members.some(m => m.walletAddress === walletAddress)) {
    return { success: true, groupId: invite.groupId }; // Already a member
  }
  
  // Add as member
  const newMember: GroupMember = {
    groupId: invite.groupId,
    userId: walletAddress,
    walletAddress,
    role: 'member',
    joinedAt: Date.now(),
  };
  
  const allMembers = getAllMembers();
  allMembers.push(newMember);
  localStorage.setItem('mitra_members', JSON.stringify(allMembers));
  
  // Also add group to user's groups list if not there
  const groups = await getGroups(walletAddress);
  const group = groups.find(g => g.groupId === invite.groupId);
  if (!group) {
    // Need to fetch group info from all groups
    const allGroups = getAllGroupsFromStorage();
    const targetGroup = allGroups.find(g => g.groupId === invite.groupId);
    if (targetGroup) {
      groups.push(targetGroup);
      saveGroups(groups);
    }
  }
  
  return { success: true, groupId: invite.groupId };
}

function getAllGroupsFromStorage(): FriendGroup[] {
  const stored = localStorage.getItem('mitra_groups');
  return stored ? JSON.parse(stored) : [];
}

function getAllMembers(): GroupMember[] {
  const stored = localStorage.getItem('mitra_members');
  return stored ? JSON.parse(stored) : [];
}

/**
 * Get all members of a group
 */
export function getGroupMembers(groupId: string): GroupMember[] {
  const allMembers = getAllMembers();
  return allMembers.filter(m => m.groupId === groupId);
}

/**
 * Add creator as admin when group is created
 */
export function addGroupCreatorAsMember(groupId: string, walletAddress: string): void {
  const members = getAllMembers();
  
  // Check if already exists
  if (members.some(m => m.groupId === groupId && m.walletAddress === walletAddress)) {
    return;
  }
  
  members.push({
    groupId,
    userId: walletAddress,
    walletAddress,
    role: 'admin',
    joinedAt: Date.now(),
  });
  
  localStorage.setItem('mitra_members', JSON.stringify(members));
}

/**
 * Promote a member to admin
 */
export function promoteToAdmin(groupId: string, walletAddress: string): boolean {
  const members = getAllMembers();
  const idx = members.findIndex(m => m.groupId === groupId && m.walletAddress === walletAddress);
  
  if (idx === -1) return false;
  
  members[idx].role = 'admin';
  localStorage.setItem('mitra_members', JSON.stringify(members));
  return true;
}

/**
 * Check if user is admin of a group
 */
export function isGroupAdmin(groupId: string, walletAddress: string): boolean {
  const members = getGroupMembers(groupId);
  const member = members.find(m => m.walletAddress === walletAddress);
  return member?.role === 'admin';
}
