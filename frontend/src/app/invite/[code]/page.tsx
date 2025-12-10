'use client';

import { useEffect, useState } from 'react';
import { useRouter, useParams } from 'next/navigation';
import { useAuthStore } from '@/store/auth';
import { getInviteByCode, joinGroupByInvite, getGroups } from '@/lib/api';

export default function InvitePage() {
    const router = useRouter();
    const params = useParams();
    const code = params.code as string;

    const { user, checkAuth, isInitialized } = useAuthStore();
    const [status, setStatus] = useState<'loading' | 'joining' | 'success' | 'error'>('loading');
    const [error, setError] = useState<string | null>(null);
    const [groupName, setGroupName] = useState<string>('');

    useEffect(() => {
        if (!isInitialized) {
            checkAuth();
        }
    }, [checkAuth, isInitialized]);

    useEffect(() => {
        if (!isInitialized) return;

        // If not logged in, redirect to login with return URL
        if (!user.isLoggedIn) {
            // Store invite code to join after login
            localStorage.setItem('pending_invite', code);
            router.push('/');
            return;
        }

        // Validate invite code
        const invite = getInviteByCode(code);
        if (!invite) {
            setStatus('error');
            setError('This invite link is invalid or has expired.');
            return;
        }

        // Get group name
        getGroups(user.walletAddress!).then(groups => {
            const group = groups.find(g => g.groupId === invite.groupId);
            if (group) {
                setGroupName(group.name);
            }
        });

        setStatus('joining');
    }, [isInitialized, user.isLoggedIn, user.walletAddress, code, router]);

    const handleJoin = async () => {
        if (!user.walletAddress || !user.email) return;

        setStatus('joining');

        const result = await joinGroupByInvite(code, user.walletAddress, user.email);

        if (result.success && result.groupId) {
            setStatus('success');
            // Clear pending invite
            localStorage.removeItem('pending_invite');
            // Redirect to group after brief delay
            setTimeout(() => {
                router.push(`/group/${result.groupId}`);
            }, 1500);
        } else {
            setStatus('error');
            setError(result.error || 'Failed to join group');
        }
    };

    if (status === 'loading') {
        return (
            <main className="min-h-screen flex items-center justify-center">
                <p className="text-3xl text-white/70 italic">loading...</p>
            </main>
        );
    }

    if (status === 'error') {
        return (
            <main className="min-h-screen flex flex-col items-center justify-center px-8">
                <p className="text-2xl text-red-400 mb-8">{error}</p>
                <button
                    onClick={() => router.push('/dashboard')}
                    className="text-xl text-white/60 hover:text-white transition-opacity"
                >
                    go to dashboard →
                </button>
            </main>
        );
    }

    if (status === 'success') {
        return (
            <main className="min-h-screen flex flex-col items-center justify-center px-8">
                <p className="text-3xl text-white mb-4">you're in!</p>
                <p className="text-xl text-white/60 italic">redirecting to group...</p>
            </main>
        );
    }

    return (
        <main className="min-h-screen flex flex-col items-center justify-center px-8 text-center">
            <h1 className="text-4xl md:text-5xl text-white mb-6">
                join {groupName || 'group'}
            </h1>
            <p className="text-xl text-white/60 mb-12">
                you've been invited to join this betting group
            </p>

            <button
                onClick={handleJoin}
                className="text-2xl text-white/80 hover:text-white transition-opacity mb-8"
            >
                join group →
            </button>

            <button
                onClick={() => router.push('/dashboard')}
                className="text-lg text-white/40 hover:text-white/60 transition-opacity"
            >
                no thanks
            </button>
        </main>
    );
}
