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

import { Connection, PublicKey, LAMPORTS_PER_SOL, Keypair } from '@solana/web3.js';
import { FriendGroup, Event, Prices, BalanceResponse, GroupMember, Bet, BetResponse, TransactionResponse } from '@/types';

const API_BASE = '/api/grpc';

// Check if backend is available
let backendAvailable: boolean | null = null;

async function checkBackend(): Promise<boolean> {
  if (backendAvailable === null) {
      try {
        // Simple health check or just assume true for now but failures will be real
        backendAvailable = true; 
      } catch {
        backendAvailable = false;
      }
  }
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
      // DO NOT generate pubkey client-side - let backend create on-chain
      // The backend will create the group on Solana and return the real pubkey
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'CreateFriendGroup',
          data: {
            name,
            admin_wallet: adminWallet,
            solana_pubkey: '', // Empty = backend creates on-chain
            signature,
          },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        const newGroup: FriendGroup = {
          groupId: data.groupId || data.group_id,
          name: data.name,
          solanaPubkey: data.solanaPubkey || data.solana_pubkey,
          adminWallet: data.adminWallet || data.admin_wallet,
          createdAt: data.createdAt || data.created_at || Date.now() / 1000,
        };
        
        // Save to local storage for list view (caching only)
        const groups = await getGroups(adminWallet);
        groups.push(newGroup);
        saveGroups(groups);
        
        return newGroup;
      } else {
        throw new Error(await res.text());
      }
    } catch (e) {
      console.error('Backend error:', e);
      throw e;
    }
  }
  
  throw new Error('Backend unavailable');
}

// Validate if a string looks like a UUID
function isValidUUID(str: string): boolean {
  const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  return uuidRegex.test(str);
}

// Validate if a string looks like a Solana pubkey (base58, 32-44 chars)
function isValidSolanaPubkey(str: string): boolean {
  const base58Regex = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;
  return base58Regex.test(str);
}

export async function getGroups(walletAddress: string): Promise<FriendGroup[]> {
  const stored = localStorage.getItem('mitra_groups');
  if (stored) {
    const groups = JSON.parse(stored) as FriendGroup[];
    
    // Filter out corrupted groups - keep only those with valid UUIDs and pubkeys
    const validGroups = groups.filter(g => {
      const hasValidGroupId = g.groupId && isValidUUID(g.groupId);
      const hasValidPubkey = g.solanaPubkey && isValidSolanaPubkey(g.solanaPubkey);
      const hasValidName = g.name && typeof g.name === 'string';
      
      if (!hasValidGroupId || !hasValidPubkey || !hasValidName) {
        console.warn('Removing corrupted group from localStorage:', g);
        return false;
      }
      return true;
    });
    
    // If we filtered any groups, save the cleaned list
    if (validGroups.length !== groups.length) {
      localStorage.setItem('mitra_groups', JSON.stringify(validGroups));
      console.log(`Cleaned localStorage: removed ${groups.length - validGroups.length} corrupted groups`);
    }
    
    return validGroups;
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
  settlementType: 'manual' | 'oracle' | 'consensus' = 'manual',
  resolveBy: number | null = null,
  creatorWallet: string,
  arbiterWallet?: string
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
            resolve_by: resolveBy,
            creator_wallet: creatorWallet,
            arbiter_wallet: arbiterWallet,
            signature: 'dev',
          },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        const newEvent: Event = {
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
        return newEvent;
      } else {
        throw new Error(await res.text());
      }
    } catch (e) {
      console.error('Backend error:', e);
      throw e;
    }
  }
  
  throw new Error('Backend unavailable');
}

export async function getEvents(groupId: string): Promise<Event[]> {
    const isOnline = await checkBackend();
    
    if (isOnline) {
      try {
        const res = await fetch(API_BASE, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({
            method: 'GetGroupEvents',
            data: { group_id: groupId },
          }),
        });
        
        if (res.ok) {
          const data = await res.json();
          const events: Event[] = (data.events || []).map((e: any) => ({
            eventId: e.eventId || e.event_id,
            groupId: e.groupId || e.group_id,
            solanaPubkey: e.solanaPubkey || e.solana_pubkey,
            title: e.title,
            description: e.description,
            outcomes: e.outcomes,
            settlementType: e.settlementType || e.settlement_type,
            status: e.status,
            resolveBy: e.resolveBy || e.resolve_by,
            createdAt: e.createdAt || e.created_at,
            arbiterWallet: e.arbiterWallet || e.arbiter_wallet,
          }));

          // Cache valid events
          saveEvents(groupId, events);
          return events;
        }
      } catch (e) {
        console.error('Failed to fetch events from backend:', e);
      }
    }

    // Fallback to local storage if backend fails
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
  console.log('getEvent called for:', eventId); // Debug
  
  // First search localStorage cache
  const allKeys = Object.keys(localStorage).filter(k => k.startsWith('mitra_events_'));
  console.log('Checking localStorage keys:', allKeys); // Debug
  
  for (const key of allKeys) {
    const events = JSON.parse(localStorage.getItem(key) || '[]') as Event[];
    const found = events.find(e => e.eventId === eventId);
    if (found) {
      console.log('Found event in cache:', found); // Debug
      return found;
    }
  }
  
  console.log('Event not in cache, fetching from backend'); // Debug
  
  // Not in cache - fetch from backend
  const isOnline = await checkBackend();
  
  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'GetEvent',
          data: { event_id: eventId },
        }),
      });
      
      if (res.ok) {
        const data = await res.json();
        console.log('GetEvent raw response:', data); // Debug log
        
        const event: Event = {
          eventId: data.eventId || data.event_id,
          groupId: data.groupId || data.group_id,
          solanaPubkey: data.solanaPubkey || data.solana_pubkey,
          title: data.title,
          description: data.description,
          outcomes: data.outcomes,
          settlementType: data.settlementType || data.settlement_type,
          status: data.status,
          resolveBy: data.resolveBy || data.resolve_by,
          createdAt: data.createdAt || data.created_at,
          arbiterWallet: data.arbiterWallet || data.arbiter_wallet,
        };
        
        console.log('Parsed event:', event); // Debug log
        
        // Cache the event in localStorage for this group
        const groupId = event.groupId;
        const events = JSON.parse(localStorage.getItem(`mitra_events_${groupId}`) || '[]') as Event[];
        // Only add if not already present
        if (!events.find(e => e.eventId === eventId)) {
          events.push(event);
          localStorage.setItem(`mitra_events_${groupId}`, JSON.stringify(events));
        }
        
        return event;
      } else {
        console.error('Failed to fetch event:', await res.text());
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
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
  
  // If backend fails, return empty structure rather than fake data
  return {
    eventId,
    prices: {},
    totalVolume: 0,
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
        
        // Save bet locally too (cache)
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
      } else {
          throw new Error(await res.text());
      }
    } catch (e) {
      console.error('Backend error:', e);
      throw e;
    }
  }
  
  throw new Error('Backend unavailable');
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
      } else {
        throw new Error(await res.text());
      }
    } catch (e) {
      console.error('Backend error:', e);
      throw e;
    }
  }
  
  throw new Error('Backend unavailable');
}

export async function deleteGroup(
  groupId: string,
  deleterWallet: string,
  signature: string = 'dev'
): Promise<boolean> {
  const isOnline = await checkBackend();

  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'DeleteGroup',
          data: {
            group_id: groupId,
            admin_wallet: deleterWallet,
            signature,
          },
        }),
      });

      if (res.ok) {
        const data = await res.json();
        if (data.success) {
           // Clean up local storage
           const groups = await getGroups(deleterWallet);
           const filtered = groups.filter(g => g.groupId !== groupId);
           saveGroups(filtered);
           // Clean up events for this group
           localStorage.removeItem(`mitra_events_${groupId}`);
           // Clean up members for this group
           const allMembers = JSON.parse(localStorage.getItem('mitra_members') || '[]');
           const filteredMembers = allMembers.filter((m: GroupMember) => m.groupId !== groupId);
           localStorage.setItem('mitra_members', JSON.stringify(filteredMembers));
           return true; 
        }
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
  }

  // If backend fails, we do NOT delete locally to avoid sync issues.
  // The user should try again.
  return false;
}

export async function deleteEvent(
  eventId: string,
  deleterWallet: string,
  signature: string = 'dev'
): Promise<boolean> {
  const isOnline = await checkBackend();

  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'DeleteEvent',
          data: {
            event_id: eventId,
            deleter_wallet: deleterWallet,
            signature,
          },
        }),
      });

      if (res.ok) {
        const data = await res.json();
        // Clean up local storage if it exists there too
        const groupEventsKey = Object.keys(localStorage).find(k => k.startsWith('mitra_events_') && localStorage.getItem(k)?.includes(eventId));
        if (groupEventsKey) {
          const events = JSON.parse(localStorage.getItem(groupEventsKey) || '[]') as Event[];
          const filtered = events.filter(e => e.eventId !== eventId);
          localStorage.setItem(groupEventsKey, JSON.stringify(filtered));
          // Clean up prices
          localStorage.removeItem(`mitra_prices_${eventId}`);
          // Clean up bets
          localStorage.removeItem(`mitra_bets_${eventId}`);
        }
        return data.success;
      }
    } catch (e) {
      console.error('Backend error:', e);
    }
  }

  return false;
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
      
      console.error('Failed to get balance:', await res.text());
    } catch (e) {
      console.error('Backend error:', e);
    }
  }
  
  return { balanceSol: 0, balanceUsdc: 0, fundsLocked: false };
}

export async function deposit(
  groupId: string,
  userWallet: string,
  amount: number,
  signature: string = 'dev',
  type: 'sol' | 'usdc' = 'usdc'
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
            amount_sol: type === 'sol' ? amount : 0,
            amount_usdc: type === 'usdc' ? amount : 0,
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
      } else {
        throw new Error(await res.text());
      }
    } catch (e) {
      console.error('Backend error:', e);
      throw e;
    }
  }
  throw new Error('Backend unavailable');
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
      } else {
        throw new Error(await res.text());
      }
    } catch (e) {
      console.error('Backend error:', e);
      throw e;
    }
  }
  throw new Error('Backend unavailable');
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
// ===========================================
// Faucet
// ===========================================

export async function requestFaucet(
  walletAddress: string,
  amountUsdc: number = 1_000_000_000 // 1000 USDC default
): Promise<string> {
  const isOnline = await checkBackend();
  
  if (isOnline) {
    try {
      const res = await fetch(API_BASE, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          method: 'RequestFaucet',
          data: {
            wallet_address: walletAddress,
            amount_usdc: amountUsdc,
          },
        }),
      });
      
      const data = await res.json();
      
      if (data.error) {
        throw new Error(data.error);
      }
      
      return data.solanaTxSignature;
    } catch (e) {
      console.error('Faucet request failed:', e);
      // Simulate if backend fails/offline for dev UX
      return `sim_faucet_${Date.now()}`;
    }
  } else {
    // Offline simulation
    console.warn('Backend offline, simulating faucet');
    return `sim_faucet_${Date.now()}`;
  }
}
