// Core Types - Maps directly to backend proto definitions

export interface User {
  id: string;
  walletAddress: string;
  email?: string;
  createdAt: number;
}

export interface UserBalance {
  balanceSol: number; // in lamports
  balanceUsdc: number; // in smallest units (6 decimals)
  fundsLocked: boolean;
}

export interface FriendGroup {
  groupId: string;
  solanaPubkey: string;
  name: string;
  adminWallet: string;
  createdAt: number;
  memberCount?: number;
  balance?: UserBalance;
}

export interface GroupMember {
  groupId: string;
  userId: string;
  walletAddress: string;
  role: 'admin' | 'member';
  joinedAt: number;
}

export interface Event {
  eventId: string;
  groupId: string;
  solanaPubkey?: string;
  title: string;
  description?: string;
  outcomes: string[];
  settlementType: 'manual' | 'oracle' | 'consensus';
  status: 'active' | 'resolved' | 'cancelled';
  resolveBy?: number;
  createdAt: number;
  winningOutcome?: string;
}

export interface Bet {
  betId: string;
  eventId: string;
  userId: string;
  outcome: string;
  shares: number;
  price: number;
  amountUsdc: number;
  createdAt: number;
  isPublic: boolean; // If true, visible to other group members
}

export interface Prices {
  eventId: string;
  prices: Record<string, number>; // { "YES": 0.65, "NO": 0.35 }
  totalVolume: number;
  timestamp: number;
}

// API Request/Response types
export interface CreateGroupRequest {
  name: string;
  adminWallet: string;
  solanaPubkey: string;
  signature: string;
}

export interface CreateEventRequest {
  groupId: string;
  title: string;
  description?: string;
  outcomes: string[];
  settlementType: 'manual' | 'oracle' | 'consensus';
  resolveBy?: number;
  creatorWallet: string;
  signature: string;
}

export interface PlaceBetRequest {
  eventId: string;
  userWallet: string;
  outcome: string;
  amountUsdc: number;
  signature: string;
}

export interface BetResponse {
  betId: string;
  shares: number;
  price: number;
  updatedPrices: Prices;
}

// WebSocket message types
export interface WsMessage {
  type: 'price_update' | 'bet_placed' | 'event_settled' | 'connected' | 'subscribed';
  channel?: string;
  eventId?: string;
  data?: unknown;
}

// Balance/Treasury types
export interface DepositRequest {
  groupId: string;
  userWallet: string;
  amountSol: number;
  amountUsdc: number;
  userUsdcAccount: string;
  signature: string;
}

export interface WithdrawRequest {
  groupId: string;
  userWallet: string;
  amountSol: number;
  amountUsdc: number;
  userUsdcAccount: string;
  signature: string;
}

export interface BalanceResponse {
  balanceSol: number;
  balanceUsdc: number;
  fundsLocked: boolean;
}

export interface TransactionResponse {
  success: boolean;
  solanaTxSignature: string;
  newBalanceSol: number;
  newBalanceUsdc: number;
}

