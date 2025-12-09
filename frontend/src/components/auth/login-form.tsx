'use client';

import { useState } from 'react';
import { motion } from 'framer-motion';
import { Mail, Sparkles } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { useAuthStore } from '@/store/auth';

export function LoginForm() {
  const [email, setEmail] = useState('');
  const { login, isLoading, error, clearError } = useAuthStore();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!email) return;
    
    const success = await login(email);
    if (success) {
      // Redirect handled by parent
    }
  };

  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      className="w-full max-w-md mx-auto"
    >
      <div className="text-center mb-8">
        <motion.div
          initial={{ scale: 0 }}
          animate={{ scale: 1 }}
          transition={{ delay: 0.2, type: 'spring' }}
          className="inline-flex items-center justify-center w-16 h-16 rounded-2xl bg-gradient-to-br from-emerald-500 to-teal-500 mb-4"
        >
          <Sparkles className="w-8 h-8 text-white" />
        </motion.div>
        <h1 className="text-3xl font-bold text-white mb-2">Welcome to Mitra</h1>
        <p className="text-slate-400">Prediction markets for your inner circle</p>
      </div>

      <form onSubmit={handleSubmit} className="space-y-4">
        <Input
          type="email"
          placeholder="Enter your email"
          value={email}
          onChange={(e) => {
            setEmail(e.target.value);
            clearError();
          }}
          error={error || undefined}
          className="text-center"
        />
        
        <Button
          type="submit"
          className="w-full"
          size="lg"
          isLoading={isLoading}
          disabled={!email}
        >
          <Mail className="w-5 h-5 mr-2" />
          Continue with Email
        </Button>
      </form>

      <p className="mt-6 text-center text-sm text-slate-500">
        We&apos;ll send you a magic link to sign in.
        <br />
        No password needed!
      </p>
    </motion.div>
  );
}

