'use client';

import { Magic } from 'magic-sdk';
import { SolanaExtension } from '@magic-ext/solana';

// Use 'any' for the magic instance to avoid complex typing issues with extensions
// eslint-disable-next-line @typescript-eslint/no-explicit-any
let magic: any = null;

/**
 * Get the Magic SDK instance (client-side only)
 * 
 * Magic.link handles:
 * - Passwordless email authentication
 * - Automatic Solana wallet creation per user
 * - Wallet signing for transactions
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function getMagic(): any {
  if (typeof window === 'undefined') {
    throw new Error('Magic can only be used on the client');
  }

  if (!magic) {
    const publishableKey = process.env.NEXT_PUBLIC_MAGIC_PUBLISHABLE_KEY;
    
    if (!publishableKey || publishableKey.includes('YOUR_KEY_HERE')) {
      console.warn('Magic.link key not configured - using mock for development');
      magic = createMockMagic();
      return magic;
    }

    if (!publishableKey.startsWith('pk_')) {
      console.error('Invalid Magic key. Use your publishable key (pk_test_xxx or pk_live_xxx)');
      magic = createMockMagic();
      return magic;
    }

    magic = new Magic(publishableKey, {
      extensions: [
        new SolanaExtension({
          rpcUrl: process.env.NEXT_PUBLIC_SOLANA_RPC || 'https://api.devnet.solana.com',
        }),
      ],
    });
  }

  return magic;
}

// ===========================================
// Authentication
// ===========================================

/**
 * Login with email - sends a magic link
 * On success, a Solana wallet is automatically created/retrieved
 */
export async function loginWithEmail(email: string): Promise<string | null> {
  try {
    const m = getMagic();
    const didToken = await m.auth.loginWithMagicLink({ email });
    return didToken;
  } catch (error) {
    console.error('Login error:', error);
    return null;
  }
}

/**
 * Restore a mock session directly (for dev mode session restoration)
 * This creates the mock_user without requiring email verification
 */
export function restoreMockSession(email: string): { walletAddress: string } {
  console.log('[MAGIC] restoreMockSession called for:', email);
  
  // Generate FULLY deterministic wallet from email (same logic as mock login)
  let hash = 0;
  for (let i = 0; i < email.length; i++) {
    const char = email.charCodeAt(i);
    hash = ((hash << 5) - hash) + char;
    hash = hash & hash; // Convert to 32bit integer
  }
  const hashStr = Math.abs(hash).toString(36);
  const mockWallet = `Dev${hashStr.padStart(8, '0')}`;
  
  const userData = {
    email,
    publicAddress: mockWallet,
    issuer: `did:ethr:${mockWallet}`,
  };
  
  localStorage.setItem('mock_user', JSON.stringify(userData));
  console.log('[MAGIC] Session restored:', email, '→', mockWallet);
  
  return { walletAddress: mockWallet };
}

/**
 * Logout and clear session
 */
export async function logout(): Promise<void> {
  try {
    const m = getMagic();
    await m.user.logout();
    localStorage.removeItem('mock_user');
  } catch (error) {
    console.error('Logout error:', error);
  }
}

/**
 * Get current user info from Magic
 */
export async function getUser() {
  try {
    const m = getMagic();
    
    // Check if user is logged in
    const isLoggedIn = await m.user.isLoggedIn();
    if (!isLoggedIn) {
      return null;
    }
    
    // Get user info (email)
    let email: string | null = null;
    
    // Try getMetadata first
    try {
      const metadata = await m.user.getMetadata();
      email = metadata?.email || null;
    } catch {
      // getMetadata not available in this SDK version
    }
    
    // If no email, try getInfo as fallback
    if (!email) {
      try {
        const info = await m.user.getInfo();
        email = info?.email || null;
      } catch {
        // getInfo also failed
      }
    }
    
    if (!email) {
      return null;
    }
    
    // Get Solana wallet address - magic-sdk v31+ uses solana.getPublicAddress()
    let solanaAddress: string | null = null;
    try {
      const solana = (m as any).solana;
      if (solana?.getPublicAddress) {
        const address = await solana.getPublicAddress();
        solanaAddress = address || null;
      }
    } catch {
      // Solana extension not available or error
    }
    
    return {
      email,
      publicAddress: solanaAddress,
    };
  } catch (error) {
    console.error('Magic getUser error:', error);
    return null;
  }
}

/**
 * Get user's Solana wallet address
 */
export async function getWalletAddress(): Promise<string | null> {
  try {
    const user = await getUser();
    return user?.publicAddress || null;
  } catch (error) {
    console.error('Get wallet error:', error);
    return null;
  }
}

// ===========================================
// Solana Wallet Operations
// ===========================================

/**
 * Sign a message with the user's Solana wallet
 * Used for authenticating bets and off-chain operations
 */
export async function signMessage(message: string): Promise<string> {
  try {
    const m = getMagic();
    const solana = (m as any).solana;
    
    if (!solana) {
      console.warn('Solana extension not available');
      return 'dev_sig_' + Date.now();
    }

    const encoded = new TextEncoder().encode(message);
    const signed = await solana.signMessage(encoded);
    return Buffer.from(signed).toString('base64');
  } catch (error) {
    console.error('Sign error:', error);
    return 'dev_sig_' + Date.now();
  }
}

/**
 * Sign a Solana transaction
 */
export async function signTransaction(transaction: unknown): Promise<unknown> {
  try {
    const m = getMagic();
    const solana = (m as any).solana;
    
    if (!solana) {
      throw new Error('Solana extension not available');
    }

    return await solana.signTransaction(transaction);
  } catch (error) {
    console.error('Sign transaction error:', error);
    throw error;
  }
}

/**
 * Sign and send a Solana transaction
 */
export async function signAndSendTransaction(transaction: unknown): Promise<string | null> {
  try {
    const m = getMagic();
    const solana = (m as any).solana;
    
    if (!solana) {
      throw new Error('Solana extension not available');
    }

    return await solana.signAndSendTransaction(transaction);
  } catch (error) {
    console.error('Send transaction error:', error);
    return null;
  }
}

// ===========================================
// Development Mock
// ===========================================

// eslint-disable-next-line @typescript-eslint/no-explicit-any
function createMockMagic(): any {
  console.log('🔧 Using mock Magic.link for development');
  
  const mockMagic = {
    user: {
      isLoggedIn: async (): Promise<boolean> => {
        return !!localStorage.getItem('mock_user');
      },
      getMetadata: async () => {
        const stored = localStorage.getItem('mock_user');
        return stored ? JSON.parse(stored) : null;
      },
      logout: async (): Promise<boolean> => {
        localStorage.removeItem('mock_user');
        return true;
      },
      getIdToken: async () => 'mock_id_token',
    },
    auth: {
      loginWithMagicLink: async ({ email }: { email: string }): Promise<string | null> => {
        // Simulate login delay
        await new Promise(resolve => setTimeout(resolve, 500));
        
        // Generate FULLY deterministic wallet from email (no random!)
        // This ensures the same email always gets the same wallet
        let hash = 0;
        for (let i = 0; i < email.length; i++) {
          const char = email.charCodeAt(i);
          hash = ((hash << 5) - hash) + char;
          hash = hash & hash; // Convert to 32bit integer
        }
        const hashStr = Math.abs(hash).toString(36);
        const mockWallet = `Dev${hashStr.padStart(8, '0')}`;
        
        const userData = {
          email,
          publicAddress: mockWallet,
          issuer: `did:ethr:${mockWallet}`,
        };
        
        localStorage.setItem('mock_user', JSON.stringify(userData));
        console.log('✅ Mock login:', email, '→', mockWallet);
        
        return 'mock_did_token';
      },
    },
    solana: {
      signMessage: async () => new Uint8Array(64).fill(1),
      signTransaction: async (tx: unknown) => tx,
      signAndSendTransaction: async () => 'mock_tx_' + Date.now(),
    },
  };
  
  return mockMagic as unknown as Magic;
}
