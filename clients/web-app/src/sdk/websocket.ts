/**
 * TypeScript WebSocket SDK for Mitra Prediction Market
 * 
 * Provides React hooks and utilities for real-time updates
 */

export type WsMessage =
  | { type: 'price_update'; event_id: string; prices: Record<string, number>; timestamp: number }
  | { type: 'bet_executed'; bet_id: string; user: string; outcome: string; shares: number; price: number }
  | { type: 'event_settled'; event_id: string; winning_outcome: string }
  | { type: 'error'; message: string };

export type SubscriptionChannel = 
  | `event:${string}`
  | `group:${string}`
  | `user:${string}`;

export class MitraWebSocketClient {
  private ws: WebSocket | null = null;
  private url: string;
  private subscriptions: Set<SubscriptionChannel> = new Set();
  private messageHandlers: Map<string, Set<(msg: WsMessage) => void>> = new Map();
  private reconnectAttempts = 0;
  private maxReconnectAttempts = 5;
  private reconnectDelay = 1000;

  constructor(url: string) {
    this.url = url;
  }

  /**
   * Connect to WebSocket server
   */
  connect(): Promise<void> {
    return new Promise((resolve, reject) => {
      try {
        this.ws = new WebSocket(this.url);

        this.ws.onopen = () => {
          console.log('WebSocket connected');
          this.reconnectAttempts = 0;
          
          // Resubscribe to all channels
          this.subscriptions.forEach(channel => {
            this.subscribe(channel);
          });

          resolve();
        };

        this.ws.onmessage = (event) => {
          try {
            const message: WsMessage = JSON.parse(event.data);
            this.handleMessage(message);
          } catch (e) {
            console.error('Failed to parse WebSocket message:', e);
          }
        };

        this.ws.onerror = (error) => {
          console.error('WebSocket error:', error);
          reject(error);
        };

        this.ws.onclose = () => {
          console.log('WebSocket disconnected');
          this.ws = null;
          this.attemptReconnect();
        };
      } catch (e) {
        reject(e);
      }
    });
  }

  /**
   * Disconnect from WebSocket server
   */
  disconnect(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.subscriptions.clear();
  }

  /**
   * Subscribe to a channel
   */
  subscribe(channel: SubscriptionChannel): void {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      console.warn('WebSocket not connected, subscription will be sent on connect');
      this.subscriptions.add(channel);
      return;
    }

    this.subscriptions.add(channel);
    this.send({
      type: 'subscribe',
      channel,
    });
  }

  /**
   * Unsubscribe from a channel
   */
  unsubscribe(channel: SubscriptionChannel): void {
    this.subscriptions.delete(channel);
    
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.send({
        type: 'unsubscribe',
        channel,
      });
    }
  }

  /**
   * Subscribe to event updates
   */
  subscribeToEvent(eventId: string): void {
    this.subscribe(`event:${eventId}`);
  }

  /**
   * Subscribe to group updates
   */
  subscribeToGroup(groupId: string): void {
    this.subscribe(`group:${groupId}`);
  }

  /**
   * Subscribe to user updates
   */
  subscribeToUser(walletAddress: string): void {
    this.subscribe(`user:${walletAddress}`);
  }

  /**
   * Register a message handler
   */
  onMessage(type: string, handler: (msg: WsMessage) => void): () => void {
    if (!this.messageHandlers.has(type)) {
      this.messageHandlers.set(type, new Set());
    }
    this.messageHandlers.get(type)!.add(handler);

    // Return unsubscribe function
    return () => {
      this.messageHandlers.get(type)?.delete(handler);
    };
  }

  /**
   * Handle incoming message
   */
  private handleMessage(message: WsMessage): void {
    // Call type-specific handlers
    const handlers = this.messageHandlers.get(message.type);
    if (handlers) {
      handlers.forEach(handler => handler(message));
    }

    // Call all handlers
    const allHandlers = this.messageHandlers.get('*');
    if (allHandlers) {
      allHandlers.forEach(handler => handler(message));
    }
  }

  /**
   * Send a message to the server
   */
  private send(message: { type: string; channel: string }): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  /**
   * Attempt to reconnect
   */
  private attemptReconnect(): void {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.error('Max reconnection attempts reached');
      return;
    }

    this.reconnectAttempts++;
    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts - 1); // Exponential backoff

    setTimeout(() => {
      console.log(`Attempting to reconnect (${this.reconnectAttempts}/${this.maxReconnectAttempts})...`);
      this.connect().catch(console.error);
    }, delay);
  }

  /**
   * Check if connected
   */
  isConnected(): boolean {
    return this.ws !== null && this.ws.readyState === WebSocket.OPEN;
  }
}

/**
 * React hook for WebSocket connection
 */
export function useMitraWebSocket(url: string) {
  const [client] = React.useState(() => new MitraWebSocketClient(url));
  const [connected, setConnected] = React.useState(false);

  React.useEffect(() => {
    client.connect().then(() => setConnected(true)).catch(console.error);

    return () => {
      client.disconnect();
      setConnected(false);
    };
  }, [client, url]);

  return { client, connected };
}

/**
 * React hook for event price updates
 */
export function useEventPrices(client: MitraWebSocketClient, eventId: string) {
  const [prices, setPrices] = React.useState<Record<string, number>>({});

  React.useEffect(() => {
    client.subscribeToEvent(eventId);

    const unsubscribe = client.onMessage('price_update', (msg) => {
      if (msg.type === 'price_update' && msg.event_id === eventId) {
        setPrices(msg.prices);
      }
    });

    return () => {
      client.unsubscribe(`event:${eventId}`);
      unsubscribe();
    };
  }, [client, eventId]);

  return prices;
}

// Note: This requires React to be imported
// In actual implementation, add: import React from 'react';

