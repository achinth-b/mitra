'use client';

import { forwardRef } from 'react';
import { cn } from '@/lib/utils';

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(
  ({ className, label, error, ...props }, ref) => {
    return (
      <div className="w-full">
        {label && (
          <label className="block text-sm font-medium text-slate-300 mb-2">
            {label}
          </label>
        )}
        <input
          ref={ref}
          className={cn(
            'w-full px-4 py-3 rounded-xl bg-slate-800/50 border border-slate-700/50',
            'text-white placeholder-slate-500',
            'focus:outline-none focus:ring-2 focus:ring-emerald-500/50 focus:border-emerald-500/50',
            'transition-all duration-200',
            error && 'border-rose-500 focus:ring-rose-500/50',
            className
          )}
          {...props}
        />
        {error && (
          <p className="mt-2 text-sm text-rose-400">{error}</p>
        )}
      </div>
    );
  }
);

Input.displayName = 'Input';

