'use client';

import { motion } from 'framer-motion';
import { Users, ChevronRight, Calendar } from 'lucide-react';
import { Card } from '@/components/ui/card';
import { formatDate, shortenAddress } from '@/lib/utils';
import type { FriendGroup } from '@/types';

interface GroupCardProps {
  group: FriendGroup;
  onClick: () => void;
  delay?: number;
}

export function GroupCard({ group, onClick, delay = 0 }: GroupCardProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay }}
    >
      <Card hover onClick={onClick} className="overflow-hidden">
        <div className="p-5">
          <div className="flex items-start justify-between">
            <div className="flex items-center gap-3">
              <div className="w-12 h-12 rounded-xl bg-gradient-to-br from-violet-500 to-purple-600 flex items-center justify-center">
                <Users className="w-6 h-6 text-white" />
              </div>
              <div>
                <h3 className="text-lg font-semibold text-white">{group.name}</h3>
                <p className="text-sm text-slate-400">
                  Admin: {shortenAddress(group.adminWallet)}
                </p>
              </div>
            </div>
            <ChevronRight className="w-5 h-5 text-slate-500" />
          </div>
          
          <div className="mt-4 pt-4 border-t border-slate-700/50 flex items-center justify-between text-sm">
            <div className="flex items-center gap-1 text-slate-400">
              <Calendar className="w-4 h-4" />
              <span>Created {formatDate(group.createdAt)}</span>
            </div>
            {group.memberCount && (
              <span className="text-slate-400">
                {group.memberCount} member{group.memberCount !== 1 ? 's' : ''}
              </span>
            )}
          </div>
        </div>
      </Card>
    </motion.div>
  );
}

