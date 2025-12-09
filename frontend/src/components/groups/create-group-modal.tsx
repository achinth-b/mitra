'use client';

import { useState } from 'react';
import { Users } from 'lucide-react';
import { Modal } from '@/components/ui/modal';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { useAuthStore } from '@/store/auth';
import { useAppStore } from '@/store/app';
import { signMessage, getWalletAddress } from '@/lib/magic';
import { generatePubkey } from '@/lib/utils';

interface CreateGroupModalProps {
  isOpen: boolean;
  onClose: () => void;
  onCreated: (group: { groupId: string; name: string }) => void;
}

export function CreateGroupModal({ isOpen, onClose, onCreated }: CreateGroupModalProps) {
  const [name, setName] = useState('');
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const { user } = useAuthStore();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    setIsLoading(true);
    setError('');

    try {
      const walletAddress = user.walletAddress || await getWalletAddress();
      if (!walletAddress) {
        throw new Error('No wallet connected');
      }

      const signature = await signMessage(`Create group: ${name}`);
      const solanaPubkey = generatePubkey();

      // Call the backend API
      const response = await fetch('/api/grpc', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'CreateFriendGroup',
          data: {
            name,
            admin_wallet: walletAddress,
            solana_pubkey: solanaPubkey,
            signature,
          },
        }),
      });

      if (!response.ok) {
        throw new Error(await response.text());
      }

      const result = await response.json();
      onCreated({ groupId: result.groupId, name: result.name });
      setName('');
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create group');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Create New Group">
      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="flex items-center gap-3 p-4 rounded-xl bg-emerald-500/10 border border-emerald-500/20">
          <Users className="w-8 h-8 text-emerald-400" />
          <div>
            <p className="text-white font-medium">Private Prediction Group</p>
            <p className="text-sm text-slate-400">Invite friends to make predictions together</p>
          </div>
        </div>

        <Input
          label="Group Name"
          placeholder="e.g., Fantasy Football League 2024"
          value={name}
          onChange={(e) => setName(e.target.value)}
          error={error}
        />

        <div className="flex gap-3 pt-2">
          <Button
            type="button"
            variant="secondary"
            className="flex-1"
            onClick={onClose}
          >
            Cancel
          </Button>
          <Button
            type="submit"
            className="flex-1"
            isLoading={isLoading}
            disabled={!name.trim()}
          >
            Create Group
          </Button>
        </div>
      </form>
    </Modal>
  );
}

