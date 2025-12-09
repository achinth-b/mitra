'use client';

import { create } from 'zustand';
import type { FriendGroup, Event, Prices } from '@/types';

interface AppState {
  // Groups
  groups: FriendGroup[];
  selectedGroup: FriendGroup | null;
  
  // Events
  events: Event[];
  selectedEvent: Event | null;
  
  // Prices (real-time)
  prices: Record<string, Prices>; // eventId -> prices
  
  // UI State
  isCreatingGroup: boolean;
  isCreatingEvent: boolean;
  isPlacingBet: boolean;
  
  // Actions
  setGroups: (groups: FriendGroup[]) => void;
  addGroup: (group: FriendGroup) => void;
  selectGroup: (group: FriendGroup | null) => void;
  
  setEvents: (events: Event[]) => void;
  addEvent: (event: Event) => void;
  selectEvent: (event: Event | null) => void;
  updateEventStatus: (eventId: string, status: Event['status']) => void;
  
  updatePrices: (eventId: string, prices: Prices) => void;
  
  setCreatingGroup: (value: boolean) => void;
  setCreatingEvent: (value: boolean) => void;
  setPlacingBet: (value: boolean) => void;
}

export const useAppStore = create<AppState>((set) => ({
  // Initial state
  groups: [],
  selectedGroup: null,
  events: [],
  selectedEvent: null,
  prices: {},
  isCreatingGroup: false,
  isCreatingEvent: false,
  isPlacingBet: false,

  // Group actions
  setGroups: (groups) => set({ groups }),
  addGroup: (group) => set((state) => ({ 
    groups: [...state.groups, group] 
  })),
  selectGroup: (group) => set({ selectedGroup: group }),

  // Event actions
  setEvents: (events) => set({ events }),
  addEvent: (event) => set((state) => ({ 
    events: [...state.events, event] 
  })),
  selectEvent: (event) => set({ selectedEvent: event }),
  updateEventStatus: (eventId, status) => set((state) => ({
    events: state.events.map((e) => 
      e.eventId === eventId ? { ...e, status } : e
    ),
  })),

  // Price updates (real-time)
  updatePrices: (eventId, prices) => set((state) => ({
    prices: { ...state.prices, [eventId]: prices },
  })),

  // UI actions
  setCreatingGroup: (value) => set({ isCreatingGroup: value }),
  setCreatingEvent: (value) => set({ isCreatingEvent: value }),
  setPlacingBet: (value) => set({ isPlacingBet: value }),
}));

