'use client';

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { getUser, loginWithEmail, logout as magicLogout, getWalletAddress, restoreMockSession } from '@/lib/magic';

interface UserState {
  email: string | null;
  walletAddress: string | null;
  isLoggedIn: boolean;
}

interface AuthState {
  user: UserState;
  isLoading: boolean;
  isInitialized: boolean; // Track if we've done the initial auth check
  error: string | null;
  
  // Actions
  login: (email: string) => Promise<boolean>;
  logout: () => Promise<void>;
  checkAuth: () => Promise<void>;
  silentCheckAuth: () => Promise<void>; // Check without showing loading
  setUser: (user: UserState) => void;
  clearError: () => void;
}

export const useAuthStore = create<AuthState>()(
  persist(
    (set, get) => ({
      user: {
        email: null,
        walletAddress: null,
        isLoggedIn: false,
      },
      isLoading: true, // Start as loading
      isInitialized: false,
      error: null,

      login: async (email: string) => {
        set({ error: null });
        
        try {
          // loginWithMagicLink is a blocking call that waits for email verification
          const didToken = await loginWithEmail(email);
          
          if (!didToken) {
            return false;
          }

          // Successfully authenticated - get wallet address
          const walletAddress = await getWalletAddress();
          
          set({
            user: {
              email,
              walletAddress,
              isLoggedIn: true,
            },
            isInitialized: true,
          });
          
          return true;
        } catch (error) {
          console.error('Login error:', error);
          const errorMessage = error instanceof Error ? error.message : 'Login failed';
          if (!errorMessage.includes('cancelled') && !errorMessage.includes('closed')) {
            set({ error: errorMessage });
          }
          return false;
        }
      },

      logout: async () => {
        try {
          await magicLogout();
        } catch (error) {
          console.error('Logout error:', error);
        }
        
        // Always clear state
        localStorage.removeItem('mitra-auth');
        localStorage.removeItem('mock_user');
        
        set({
          user: {
            email: null,
            walletAddress: null,
            isLoggedIn: false,
          },
          isInitialized: true,
        });
      },

      // Initial auth check - shows loading state
      checkAuth: async () => {
        const state = get();
        
        // If already initialized and logged in with wallet, skip but ensure loading is false
        if (state.isInitialized && state.user.isLoggedIn && state.user.walletAddress) {
          if (state.isLoading) {
            set({ isLoading: false });
          }
          return;
        }
        
        // Show loading only if not logged in
        if (!state.user.isLoggedIn) {
          set({ isLoading: true });
        }
        
        try {
          const timeoutPromise = new Promise((_, reject) => 
            setTimeout(() => reject(new Error('timeout')), 3000)
          );
          
          const userData = await Promise.race([
            getUser(),
            timeoutPromise,
          ]).catch(() => null);
          
          if (userData && typeof userData === 'object' && 'email' in userData) {
            const email = (userData as Record<string, unknown>).email as string | null;
            const publicAddress = (userData as Record<string, unknown>).publicAddress as string | null;
            
            set({
              user: {
                email: email || null,
                walletAddress: publicAddress || null,
                isLoggedIn: true,
              },
              isLoading: false,
              isInitialized: true,
            });
          } else {
            // No user data from Magic - check if we have persisted state
            const persistedEmail = state.user.email;
            
            if (persistedEmail) {
              // Keep session with unavailable wallet (graceful degradation)
              set({
                user: {
                  email: persistedEmail,
                  walletAddress: 'unavailable',
                  isLoggedIn: true,
                },
                isLoading: false,
                isInitialized: true,
              });
            } else {
              // No persisted email - user is logged out
              set({
                user: {
                  email: null,
                  walletAddress: null,
                  isLoggedIn: false,
                },
                isLoading: false,
                isInitialized: true,
              });
            }
          }
        } catch (error) {
          console.error('Auth check error:', error);
          set({ isLoading: false, isInitialized: true });
        }
      },

      // Silent auth check - doesn't show loading (for background refreshes)
      silentCheckAuth: async () => {
        try {
          const userData = await Promise.race([
            getUser(),
            new Promise((_, reject) => setTimeout(() => reject(new Error('timeout')), 3000)),
          ]).catch(() => null);
          
          if (userData && typeof userData === 'object' && 'email' in userData) {
            const email = (userData as Record<string, unknown>).email as string | null;
            const publicAddress = (userData as Record<string, unknown>).publicAddress as string | null;
            
            set({
              user: {
                email: email || null,
                walletAddress: publicAddress || null,
                isLoggedIn: true,
              },
              isInitialized: true,
            });
          }
        } catch {
          // Silent fail for background checks
        }
      },

      setUser: (user: UserState) => set({ user, isLoading: false, isInitialized: true }),
      
      clearError: () => set({ error: null }),
    }),
    {
      name: 'mitra-auth',
      partialize: (state) => ({ 
        user: state.user,
        isInitialized: state.isInitialized,
      }),
    }
  )
);
