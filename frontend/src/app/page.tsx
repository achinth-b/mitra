'use client';

import { useEffect, useState, useCallback } from 'react';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import { BRAND } from '@/lib/brand';
import { motion, AnimatePresence } from 'framer-motion';

export default function HomePage() {
  const router = useRouter();
  const { user, checkAuth, silentCheckAuth, isLoading, isInitialized, login, error } = useAuthStore();
  const [showLogin, setShowLogin] = useState(false);
  const [email, setEmail] = useState('');
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [loginStatus, setLoginStatus] = useState<string | null>(null);

  const isMockMode = typeof process !== 'undefined' && (!process.env.NEXT_PUBLIC_MAGIC_PUBLISHABLE_KEY ||
    process.env.NEXT_PUBLIC_MAGIC_PUBLISHABLE_KEY.includes('YOUR_KEY_HERE'));

  useEffect(() => {
    if (!isInitialized) {
      checkAuth();
    }
  }, [checkAuth, isInitialized]);

  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        silentCheckAuth();
      }
    };
    const handleFocus = () => silentCheckAuth();

    document.addEventListener('visibilitychange', handleVisibilityChange);
    window.addEventListener('focus', handleFocus);

    return () => {
      document.removeEventListener('visibilitychange', handleVisibilityChange);
      window.removeEventListener('focus', handleFocus);
    };
  }, [silentCheckAuth]);

  useEffect(() => {
    if (user.isLoggedIn && isInitialized) {
      router.push('/dashboard');
    }
  }, [user.isLoggedIn, isInitialized, router]);

  const pollForAuth = useCallback(async () => {
    if (isMockMode) return;
    const interval = setInterval(() => silentCheckAuth(), 2000);
    setTimeout(() => {
      clearInterval(interval);
      if (!user.isLoggedIn) {
        setLoginStatus(null);
        setIsSubmitting(false);
      }
    }, 5 * 60 * 1000);
    return () => clearInterval(interval);
  }, [silentCheckAuth, isMockMode, user.isLoggedIn]);

  const handleEmailLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!email) return;

    setIsSubmitting(true);

    if (isMockMode) {
      setLoginStatus('logging in...');
      const success = await login(email);
      if (success) {
        setLoginStatus('success!');
        setTimeout(() => router.push('/dashboard'), 500);
      } else {
        setLoginStatus(null);
        setIsSubmitting(false);
      }
    } else {
      setLoginStatus('sending magic link...');
      pollForAuth();
      const success = await login(email);
      if (success) {
        setLoginStatus('success!');
        setTimeout(() => router.push('/dashboard'), 500);
      } else if (document.visibilityState === 'visible') {
        setLoginStatus('check your email');
      }
    }
  };

  if (isLoading && !isInitialized) {
    return (
      <div className="min-h-screen bg-black text-white flex flex-col justify-center items-center p-8">
        <motion.p
          className="text-2xl font-light italic opacity-90"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
        >
          loading...
        </motion.p>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-black text-white flex flex-col justify-center items-center p-8 overflow-hidden fixed inset-0">
      <div className="w-full max-w-2xl mx-auto flex flex-col items-center">
        <AnimatePresence mode="wait">
          {!showLogin ? (
            <motion.div
              key="landing"
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
              transition={{ duration: 0.5 }}
              className="text-center"
            >
              <h1 className="text-4xl md:text-6xl lg:text-7xl font-light mb-8 md:mb-12 leading-tight tracking-tight">
                {BRAND.tagline}
              </h1>
              <h2 className="text-2xl md:text-3xl lg:text-4xl font-light mb-12 md:mb-16 leading-tight opacity-90">
                this <em className="font-serif italic font-light opacity-80">might</em> ruin your friendships.
              </h2>
              <motion.p
                className="text-2xl md:text-4xl italic font-light cursor-pointer hover:opacity-100 opacity-90 transition-opacity mt-8"
                onClick={() => setShowLogin(true)}
                whileHover={{ scale: 1.05 }}
                whileTap={{ scale: 0.95 }}
              >
                enter →
              </motion.p>
            </motion.div>
          ) : (
            <motion.form
              key="login"
              onSubmit={handleEmailLogin}
              className="w-full flex flex-col items-center"
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              exit={{ opacity: 0, y: -20 }}
              transition={{ duration: 0.5 }}
            >
              <h1 className="text-3xl md:text-5xl font-light mb-12">sign in</h1>

              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                placeholder="your email"
                className="w-full max-w-md bg-transparent border-0 border-b border-white/30 text-white text-center text-xl md:text-2xl py-3 focus:outline-none focus:border-white/60 transition-colors placeholder:text-white/40 placeholder:italic mb-8"
                autoFocus
                disabled={isSubmitting}
              />

              {error && <p className="text-red-400 mt-4 text-sm md:text-base">{error}</p>}

              {loginStatus ? (
                <p className="text-2xl italic mt-6 opacity-90">{loginStatus}</p>
              ) : (
                <motion.button
                  type="submit"
                  disabled={!email || isSubmitting}
                  className="bg-transparent border-0 text-white text-2xl md:text-3xl italic cursor-pointer disabled:opacity-40 disabled:cursor-not-allowed mt-8 hover:scale-105 transition-transform"
                  whileHover={{ scale: 1.05 }}
                  whileTap={{ scale: 0.95 }}
                >
                  continue →
                </motion.button>
              )}

              <p className="mt-8 text-base md:text-lg opacity-60">
                {isMockMode
                  ? 'dev mode — instant login'
                  : isSubmitting
                    ? 'click the link in your email'
                    : 'we\'ll send you a magic link'
                }
              </p>

              <p className="mt-2 text-sm opacity-40">a solana wallet will be created for you</p>

              {!isSubmitting ? (
                <button
                  type="button"
                  onClick={() => { setShowLogin(false); setLoginStatus(null); setEmail(''); }}
                  className="mt-12 bg-transparent border-0 text-white opacity-40 hover:opacity-70 transition-opacity cursor-pointer text-base md:text-lg"
                >
                  ← back
                </button>
              ) : !isMockMode && (
                <button
                  type="button"
                  onClick={() => { setIsSubmitting(false); setLoginStatus(null); }}
                  className="mt-12 bg-transparent border-0 text-white opacity-40 hover:opacity-70 transition-opacity cursor-pointer text-base md:text-lg"
                >
                  cancel
                </button>
              )}
            </motion.form>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
