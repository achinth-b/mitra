'use client';

import { cn } from '@/lib/utils';
import { motion } from 'framer-motion';

interface CardProps {
  children: React.ReactNode;
  className?: string;
  hover?: boolean;
  onClick?: () => void;
}

export function Card({ children, className, hover = false, onClick }: CardProps) {
  const Component = hover ? motion.div : 'div';

  return (
    <Component
      {...(hover && {
        whileHover: { scale: 1.02, y: -4 },
        transition: { duration: 0.2 },
      })}
      onClick={onClick}
      className={cn(
        'rounded-3xl bg-black/40 backdrop-blur-md',
        'border border-white/10',
        'shadow-[0_8px_32px_rgba(0,0,0,0.5)]',
        hover && 'cursor-pointer',
        className
      )}
    >
      {children}
    </Component>
  );
}

export function CardHeader({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={cn('px-6 py-4 border-b border-slate-700/50', className)}>
      {children}
    </div>
  );
}

export function CardContent({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={cn('px-6 py-4', className)}>
      {children}
    </div>
  );
}

export function CardFooter({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={cn('px-6 py-4 border-t border-slate-700/50', className)}>
      {children}
    </div>
  );
}

