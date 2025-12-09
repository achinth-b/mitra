'use client';

import { create } from 'zustand';
import { persist } from 'zustand/middleware';
import { getUser, loginWithEmail, logout as magicLogout, getWalletAddress } from '@/lib/magic';

interface UserState {
  email: string | null;
  walletAddress: string | null;
  isLoggedIn: boolean;
}

interface AuthState {
  user: UserState;
  isLoading: boolean;
  error: string | null;
  
  // Actions
  login: (email: string) => Promise<boolean>;
  logout: () => Promise<void>;
  checkAuth: () => Promise<void>;
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
      isLoading: false,
      error: null,

      login: async (email: string) => {
        set({ isLoading: true, error: null });
        
        try {
          const didToken = await loginWithEmail(email);
          
          if (!didToken) {
            set({ error: 'Login failed. Please try again.', isLoading: false });
            return false;
          }

          const walletAddress = await getWalletAddress();
          
          set({
            user: {
              email,
              walletAddress,
              isLoggedIn: true,
            },
            isLoading: false,
          });
          
          return true;
        } catch (error) {
          set({ 
            error: error instanceof Error ? error.message : 'Login failed',
            isLoading: false 
          });
          return false;
        }
      },

      logout: async () => {
        set({ isLoading: true });
        
        try {
          await magicLogout();
          set({
            user: {
              email: null,
              walletAddress: null,
              isLoggedIn: false,
            },
            isLoading: false,
          });
        } catch (error) {
          console.error('Logout error:', error);
          set({ isLoading: false });
        }
      },

      checkAuth: async () => {
        set({ isLoading: true });
        
        try {
          // Add timeout to prevent hanging
          const timeoutPromise = new Promise((_, reject) => 
            setTimeout(() => reject(new Error('Auth check timeout')), 3000)
          );
          
          const userData = await Promise.race([
            getUser(),
            timeoutPromise,
          ]).catch(() => null);
          
          if (userData && typeof userData === 'object' && 'email' in userData) {
            set({
              user: {
                email: (userData as any).email || null,
                walletAddress: (userData as any).publicAddress || null,
                isLoggedIn: true,
              },
              isLoading: false,
            });
          } else {
            set({
              user: {
                email: null,
                walletAddress: null,
                isLoggedIn: false,
              },
              isLoading: false,
            });
          }
        } catch (error) {
          console.error('Auth check error:', error);
          set({
            user: {
              email: null,
              walletAddress: null,
              isLoggedIn: false,
            },
            isLoading: false,
          });
        }
      },

      setUser: (user: UserState) => set({ user, isLoading: false }),
      
      clearError: () => set({ error: null }),
    }),
    {
      name: 'mitra-auth',
      partialize: (state) => ({ user: state.user }),
    }
  )
);

