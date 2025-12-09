'use client';

import { useState } from 'react';
import { Zap } from 'lucide-react';
import { Modal } from '@/components/ui/modal';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { useAuthStore } from '@/store/auth';
import { signMessage } from '@/lib/magic';

interface CreateEventModalProps {
  isOpen: boolean;
  onClose: () => void;
  groupId: string;
  onCreated: (event: { eventId: string; title: string }) => void;
}

export function CreateEventModal({ isOpen, onClose, groupId, onCreated }: CreateEventModalProps) {
  const [title, setTitle] = useState('');
  const [description, setDescription] = useState('');
  const [outcomes, setOutcomes] = useState(['YES', 'NO']);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const { user } = useAuthStore();

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!title.trim()) return;

    setIsLoading(true);
    setError('');

    try {
      const walletAddress = user.walletAddress;
      if (!walletAddress) {
        throw new Error('No wallet connected');
      }

      const signature = await signMessage(`Create event: ${title}`);

      const response = await fetch('/api/grpc', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          method: 'CreateEvent',
          data: {
            group_id: groupId,
            title,
            description,
            outcomes,
            settlement_type: 'manual',
            resolve_by: Math.floor(Date.now() / 1000) + 86400 * 30, // 30 days
            creator_wallet: walletAddress,
            signature,
          },
        }),
      });

      if (!response.ok) {
        throw new Error(await response.text());
      }

      const result = await response.json();
      onCreated({ eventId: result.eventId, title: result.title });
      setTitle('');
      setDescription('');
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to create event');
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <Modal isOpen={isOpen} onClose={onClose} title="Create Prediction Market">
      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="flex items-center gap-3 p-4 rounded-xl bg-amber-500/10 border border-amber-500/20">
          <Zap className="w-8 h-8 text-amber-400" />
          <div>
            <p className="text-white font-medium">New Prediction</p>
            <p className="text-sm text-slate-400">Create a market for your group to bet on</p>
          </div>
        </div>

        <Input
          label="Question"
          placeholder="e.g., Will the Lakers win the championship?"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          error={error}
        />

        <Input
          label="Description (optional)"
          placeholder="Add more context..."
          value={description}
          onChange={(e) => setDescription(e.target.value)}
        />

        <div>
          <label className="block text-sm font-medium text-slate-300 mb-2">
            Outcomes
          </label>
          <div className="grid grid-cols-2 gap-2">
            {outcomes.map((outcome, index) => (
              <Input
                key={index}
                value={outcome}
                onChange={(e) => {
                  const newOutcomes = [...outcomes];
                  newOutcomes[index] = e.target.value;
                  setOutcomes(newOutcomes);
                }}
                placeholder={`Outcome ${index + 1}`}
              />
            ))}
          </div>
        </div>

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
            disabled={!title.trim()}
          >
            Create Market
          </Button>
        </div>
      </form>
    </Modal>
  );
}

