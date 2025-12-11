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
      console.warn('Magic.link key not configured!');
      return null;
    }

    if (!publishableKey.startsWith('pk_')) {
      console.error('Invalid Magic key.');
      return null;
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

