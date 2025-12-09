'use client';

import { useEffect, useState } from 'react';
import { useRouter } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import { createGroup, getGroups, saveGroups } from '@/lib/api';
import type { FriendGroup } from '@/types';

export default function DashboardPage() {
  const router = useRouter();
  const { user, checkAuth, logout, isLoading: authLoading } = useAuthStore();
  const [groups, setGroups] = useState<FriendGroup[]>([]);
  const [showCreate, setShowCreate] = useState(false);
  const [newGroupName, setNewGroupName] = useState('');
  const [isCreating, setIsCreating] = useState(false);

  useEffect(() => {
    checkAuth();
  }, [checkAuth]);

  useEffect(() => {
    if (!user.isLoggedIn && !authLoading) {
      router.push('/');
    }
  }, [user.isLoggedIn, authLoading, router]);

  useEffect(() => {
    if (user.walletAddress) {
      getGroups(user.walletAddress).then(setGroups);
    }
  }, [user.walletAddress]);

  const handleLogout = async () => {
    await logout();
    router.push('/');
  };

  const handleCreateGroup = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newGroupName.trim() || !user.walletAddress) return;

    setIsCreating(true);
    
    const newGroup = await createGroup(newGroupName, user.walletAddress);
    const updatedGroups = [...groups, newGroup];
    setGroups(updatedGroups);
    saveGroups(updatedGroups);
    
    setNewGroupName('');
    setShowCreate(false);
    setIsCreating(false);
  };

  if (authLoading) {
    return (
      <main className="min-h-screen flex items-center justify-center">
        <p className="text-2xl text-white/40 italic">loading...</p>
      </main>
    );
  }

  if (!user.isLoggedIn) {
    return null;
  }

  return (
    <main className="min-h-screen px-8 py-16 md:py-24">
      <div className="max-w-3xl mx-auto">
        {/* Header */}
        <header className="flex items-start justify-between mb-20">
          <div>
            <h1 className="text-4xl md:text-5xl mb-4">mitra</h1>
            <p className="text-xl text-white/40">bet on your friends</p>
          </div>
          <button
            onClick={handleLogout}
            className="text-xl text-white/40 hover:text-white transition-opacity"
          >
            sign out
          </button>
        </header>

        {/* User Info */}
        <section className="mb-16 pb-16 border-b border-white/10">
          <p className="text-white/40 text-xl mb-2">signed in as</p>
          <p className="text-2xl md:text-3xl mb-6">{user.email}</p>
          
          <p className="text-white/40 text-lg mb-2">your solana wallet</p>
          <p className="text-lg font-mono text-white/60 break-all">
            {user.walletAddress}
          </p>
        </section>

        {/* Groups Section */}
        <section>
          <div className="flex items-center justify-between mb-12">
            <h2 className="text-3xl md:text-4xl">your groups</h2>
            {!showCreate && (
              <button
                onClick={() => setShowCreate(true)}
                className="text-xl text-white/40 hover:text-white transition-opacity"
              >
                + new group
              </button>
            )}
          </div>

          {/* Create Group Form */}
          {showCreate && (
            <form onSubmit={handleCreateGroup} className="mb-12 pb-12 border-b border-white/10">
              <input
                type="text"
                value={newGroupName}
                onChange={(e) => setNewGroupName(e.target.value)}
                placeholder="group name"
                className="w-full text-2xl md:text-3xl py-4 border-b border-white/30 focus:border-white/70 transition-colors mb-8 bg-transparent"
                autoFocus
              />
              <div className="flex gap-8">
                <button
                  type="submit"
                  disabled={!newGroupName.trim() || isCreating}
                  className="text-xl text-white/60 hover:text-white transition-opacity disabled:text-white/30"
                >
                  {isCreating ? 'creating...' : 'create →'}
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setShowCreate(false);
                    setNewGroupName('');
                  }}
                  className="text-xl text-white/30 hover:text-white/60 transition-opacity"
                >
                  cancel
                </button>
              </div>
            </form>
          )}

          {/* Groups List */}
          {groups.length > 0 ? (
            <ul className="space-y-2">
              {groups.map((group) => (
                <li key={group.groupId}>
                  <button
                    onClick={() => router.push(`/group/${group.groupId}`)}
                    className="w-full text-left py-8 border-b border-white/10 hover:border-white/30 transition-colors group flex items-center justify-between"
                  >
                    <span className="text-2xl md:text-3xl group-hover:text-white/80 transition-opacity">
                      {group.name}
                    </span>
                    <span className="text-white/30 text-3xl group-hover:text-white/60 transition-opacity">→</span>
                  </button>
                </li>
              ))}
            </ul>
          ) : !showCreate ? (
            <div className="text-center py-20">
              <p className="text-2xl text-white/40 italic mb-10">
                no groups yet.
              </p>
              <button
                onClick={() => setShowCreate(true)}
                className="text-2xl text-white/60 hover:text-white transition-opacity"
              >
                create your first group →
              </button>
            </div>
          ) : null}
        </section>
      </div>
    </main>
  );
}
