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
            'w-full px-4 py-3 rounded-none bg-transparent border-0 border-b border-white/20',
            'text-white placeholder-white/30 text-lg',
            'focus:outline-none focus:border-white/80 focus:ring-0',
            'transition-all duration-300',
            error && 'border-red-500 focus:border-red-400',
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

