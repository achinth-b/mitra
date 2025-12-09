'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/store/auth';

export default function HomePage() {
  const router = useRouter();
  const { user, checkAuth, isLoading, login, error } = useAuthStore();
  const [showLogin, setShowLogin] = useState(false);
  const [email, setEmail] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [loginStatus, setLoginStatus] = useState<string | null>(null);

  // Check if Magic.link is configured
  const isMockMode = !process.env.NEXT_PUBLIC_MAGIC_PUBLISHABLE_KEY || 
    process.env.NEXT_PUBLIC_MAGIC_PUBLISHABLE_KEY.includes('YOUR_KEY_HERE');

  useEffect(() => {
    checkAuth();
  }, [checkAuth]);

  useEffect(() => {
    if (user.isLoggedIn && !isLoading) {
      router.push('/dashboard');
    }
  }, [user.isLoggedIn, isLoading, router]);

  const handleEmailLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!email) return;
    
    setIsSubmitting(true);
    setLoginStatus(isMockMode ? 'logging in (dev mode)...' : 'sending magic link...');
    
    const success = await login(email);
    
    if (success) {
      setLoginStatus('success! redirecting...');
      // Small delay to show success message
      setTimeout(() => {
        router.push('/dashboard');
      }, 500);
    } else {
      setLoginStatus(null);
      setIsSubmitting(false);
    }
  };

  if (isLoading) {
    return (
      <main className="min-h-screen flex items-center justify-center">
        <p className="text-2xl text-white/40 italic">loading...</p>
      </main>
    );
  }

  return (
    <main className="min-h-screen flex flex-col items-center justify-center px-8">
      <div className="text-center w-full max-w-xl">
        
        {/* Landing */}
        {!showLogin && (
          <>
            <h1 className="text-4xl md:text-6xl lg:text-7xl font-normal tracking-wide mb-12 leading-tight">
              bet on (or against) your friends.
            </h1>
            
            <p className="text-2xl md:text-3xl lg:text-4xl mb-20">
              this <em>might</em> ruin your friendships.
            </p>
            
            <button
              onClick={() => setShowLogin(true)}
              className="text-2xl md:text-3xl italic text-white/60 hover:text-white transition-opacity duration-300"
            >
              enter →
            </button>
          </>
        )}

        {/* Login */}
        {showLogin && (
          <form onSubmit={handleEmailLogin} className="w-full">
            <h2 className="text-3xl md:text-4xl lg:text-5xl mb-16">sign in</h2>
            
            <input
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="your email"
              className="w-full text-center text-2xl md:text-3xl py-6 border-b border-white/30 focus:border-white/70 transition-colors duration-300 bg-transparent mb-10"
              autoFocus
              disabled={isSubmitting}
            />
            
            {error && (
              <p className="text-xl text-red-400/80 mb-8">{error}</p>
            )}
            
            {loginStatus && (
              <p className="text-xl text-white/60 mb-8 italic">{loginStatus}</p>
            )}
            
            {!loginStatus && (
              <button
                type="submit"
                disabled={!email || isSubmitting}
                className="text-2xl md:text-3xl italic text-white/60 hover:text-white transition-opacity duration-300 disabled:text-white/30 disabled:cursor-not-allowed"
              >
                continue →
              </button>
            )}
            
            <p className="mt-12 text-xl text-white/40">
              {isMockMode 
                ? 'dev mode: instant login, no email sent.'
                : 'we\'ll send you a magic link.'
              }
            </p>
            
            <p className="mt-4 text-lg text-white/30">
              a solana wallet will be created for you.
            </p>
            
            {!isSubmitting && (
              <button
                type="button"
                onClick={() => {
                  setShowLogin(false);
                  setLoginStatus(null);
                  setEmail('');
                }}
                className="mt-12 text-xl text-white/30 hover:text-white/60 transition-opacity"
              >
                ← back
              </button>
            )}
          </form>
        )}
      </div>
    </main>
  );
}
