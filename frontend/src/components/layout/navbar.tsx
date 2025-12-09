'use client';

import Link from 'next/link';
import { motion } from 'framer-motion';
import { Sparkles, LogOut, User, Wallet } from 'lucide-react';
import { useAuthStore } from '@/store/auth';
import { Button } from '@/components/ui/button';
import { shortenAddress } from '@/lib/utils';

export function Navbar() {
  const { user, logout, isLoading } = useAuthStore();

  return (
    <nav className="fixed top-0 left-0 right-0 z-30 bg-slate-900/80 backdrop-blur-lg border-b border-slate-800">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
        <div className="flex items-center justify-between h-16">
          {/* Logo */}
          <Link href="/dashboard" className="flex items-center gap-2">
            <motion.div
              whileHover={{ rotate: 15 }}
              className="w-10 h-10 rounded-xl bg-gradient-to-br from-emerald-500 to-teal-500 flex items-center justify-center"
            >
              <Sparkles className="w-5 h-5 text-white" />
            </motion.div>
            <span className="text-xl font-bold bg-gradient-to-r from-emerald-400 to-teal-400 bg-clip-text text-transparent">
              Mitra
            </span>
          </Link>

          {/* Right side */}
          {user.isLoggedIn && (
            <div className="flex items-center gap-4">
              {/* Wallet indicator */}
              <div className="hidden sm:flex items-center gap-2 px-3 py-1.5 rounded-lg bg-slate-800/50 border border-slate-700/50">
                <Wallet className="w-4 h-4 text-emerald-400" />
                <span className="text-sm text-slate-300">
                  {user.walletAddress ? shortenAddress(user.walletAddress) : 'No wallet'}
                </span>
              </div>
              
              {/* User menu */}
              <div className="flex items-center gap-2 px-3 py-1.5 rounded-lg bg-slate-800/50 border border-slate-700/50">
                <User className="w-4 h-4 text-slate-400" />
                <span className="text-sm text-slate-300 hidden sm:inline">
                  {user.email}
                </span>
              </div>

              <Button
                variant="ghost"
                size="sm"
                onClick={logout}
                disabled={isLoading}
              >
                <LogOut className="w-4 h-4" />
              </Button>
            </div>
          )}
        </div>
      </div>
    </nav>
  );
}

