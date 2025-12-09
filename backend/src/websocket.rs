use crate::error::{AppError, AppResult};
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{error, info, warn};
use uuid::Uuid;

/// WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum WsMessage {
    #[serde(rename = "subscribe")]
    Subscribe {
        channel: String, // "event:{id}", "group:{id}", "user:{wallet}"
    },
    #[serde(rename = "unsubscribe")]
    Unsubscribe {
        channel: String,
    },
    #[serde(rename = "price_update")]
    PriceUpdate {
        event_id: String,
        prices: HashMap<String, f64>,
        timestamp: i64,
    },
    #[serde(rename = "bet_executed")]
    BetExecuted {
        bet_id: String,
        user: String,
        outcome: String,
        shares: f64,
        price: f64,
    },
    #[serde(rename = "event_settled")]
    EventSettled {
        event_id: String,
        winning_outcome: String,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
    },
}

/// WebSocket server for real-time updates
pub struct WebSocketServer {
    /// Broadcast sender for sending messages to all clients
    tx: broadcast::Sender<WsMessage>,
    /// Active subscriptions: channel -> set of client IDs
    subscriptions: Arc<RwLock<HashMap<String, Vec<Uuid>>>>,
    /// Client subscriptions: client_id -> set of channels
    client_channels: Arc<RwLock<HashMap<Uuid, Vec<String>>>>,
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000); // Buffer up to 1000 messages

        Self {
            tx,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            client_channels: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get broadcast sender
    pub fn sender(&self) -> broadcast::Sender<WsMessage> {
        self.tx.clone()
    }

    /// Broadcast a message to all subscribers of a channel
    pub async fn broadcast_to_channel(&self, channel: &str, message: WsMessage) {
        let subscriptions = self.subscriptions.read().await;
        
        if let Some(subscribers) = subscriptions.get(channel) {
            let count = subscribers.len();
            if count > 0 {
                info!("Broadcasting to {} subscribers on channel {}", count, channel);
                // Send to broadcast channel (all subscribers will receive)
                if let Err(e) = self.tx.send(message.clone()) {
                    warn!("Failed to broadcast message: {}", e);
                }
            }
        }
    }

    /// Subscribe a client to a channel
    pub async fn subscribe(&self, client_id: Uuid, channel: String) {
        let channel_clone = channel.clone();
        let mut subscriptions = self.subscriptions.write().await;
        let mut client_channels = self.client_channels.write().await;

        // Add client to channel
        subscriptions
            .entry(channel.clone())
            .or_insert_with(Vec::new)
            .push(client_id);

        // Track channel for client
        client_channels
            .entry(client_id)
            .or_insert_with(Vec::new)
            .push(channel.clone());

        info!("Client {} subscribed to {}", client_id, channel_clone);
    }

    /// Unsubscribe a client from a channel
    pub async fn unsubscribe(&self, client_id: Uuid, channel: &str) {
        let mut subscriptions = self.subscriptions.write().await;
        let mut client_channels = self.client_channels.write().await;

        // Remove client from channel
        if let Some(subscribers) = subscriptions.get_mut(channel) {
            subscribers.retain(|&id| id != client_id);
        }

        // Remove channel from client's list
        if let Some(channels) = client_channels.get_mut(&client_id) {
            channels.retain(|c| c != channel);
        }

        info!("Client {} unsubscribed from {}", client_id, channel);
    }

    /// Get all channels a client is subscribed to
    pub async fn get_client_channels(&self, client_id: Uuid) -> Vec<String> {
        let client_channels = self.client_channels.read().await;
        client_channels.get(&client_id).cloned().unwrap_or_default()
    }

    /// Handle a new WebSocket connection
    pub async fn handle_connection(
        &self,
        stream: tokio::net::TcpStream,
    ) -> AppResult<()> {
        let ws_stream = accept_async(stream)
            .await
            .map_err(|e| AppError::Message(format!("WebSocket handshake failed: {}", e)))?;

        let (mut ws_sender, mut ws_receiver) = ws_stream.split();
        let mut rx = self.tx.subscribe();
        let client_id = Uuid::new_v4();

        info!("New WebSocket connection: {}", client_id);

        // Spawn task to handle incoming messages
        let ws_server = self.clone();
        
        // Clone sender for use in the spawned task
        let ws_server_for_receiver = ws_server.clone();
        
        // Send welcome message
        let welcome = serde_json::json!({
            "type": "connected",
            "client_id": client_id.to_string(),
            "message": "Connected to Mitra WebSocket server"
        });
        if let Err(e) = ws_sender.send(Message::Text(welcome.to_string())).await {
            warn!("Failed to send welcome message: {}", e);
        }
        
        // Need to wrap sender in Arc<Mutex> to share between tasks
        let ws_sender = std::sync::Arc::new(tokio::sync::Mutex::new(ws_sender));
        let ws_sender_for_receiver = ws_sender.clone();
        
        tokio::spawn(async move {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        // Parse subscription message
                        if let Ok(sub_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match sub_msg {
                                WsMessage::Subscribe { channel } => {
                                    ws_server_for_receiver.subscribe(client_id, channel.clone()).await;
                                    // Send acknowledgment
                                    let ack = serde_json::json!({
                                        "type": "subscribed",
                                        "channel": channel
                                    });
                                    let mut sender = ws_sender_for_receiver.lock().await;
                                    if let Err(e) = sender.send(Message::Text(ack.to_string())).await {
                                        warn!("Failed to send ack: {}", e);
                                    }
                                }
                                WsMessage::Unsubscribe { channel } => {
                                    ws_server_for_receiver.unsubscribe(client_id, &channel).await;
                                    // Send acknowledgment
                                    let ack = serde_json::json!({
                                        "type": "unsubscribed",
                                        "channel": channel
                                    });
                                    let mut sender = ws_sender_for_receiver.lock().await;
                                    if let Err(e) = sender.send(Message::Text(ack.to_string())).await {
                                        warn!("Failed to send ack: {}", e);
                                    }
                                }
                                _ => {
                                    warn!("Unexpected message type from client {}", client_id);
                                }
                            }
                        } else {
                            warn!("Failed to parse message from client {}: {}", client_id, text);
                            // Send error response
                            let err = serde_json::json!({
                                "type": "error",
                                "message": "Invalid message format"
                            });
                            let mut sender = ws_sender_for_receiver.lock().await;
                            let _ = sender.send(Message::Text(err.to_string())).await;
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket connection closed: {}", client_id);
                        break;
                    }
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Clean up all subscriptions for this client
            let channels = ws_server_for_receiver.get_client_channels(client_id).await;
            for channel in channels {
                ws_server_for_receiver.unsubscribe(client_id, &channel).await;
            }
        });

        // Spawn task to send broadcast messages to client
        let ws_server_clone = self.clone();
        let ws_sender_for_broadcast = ws_sender.clone();
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                // Check if client is subscribed to relevant channel
                let should_send = match &msg {
                    WsMessage::PriceUpdate { event_id, .. } => {
                        let channel = format!("event:{}", event_id);
                        ws_server_clone.is_client_subscribed(client_id, &channel).await
                    }
                    WsMessage::BetExecuted { .. } => {
                        // For bet_executed, we need event_id - for now send to all
                        // TODO: Filter by event subscription
                        true
                    }
                    WsMessage::EventSettled { event_id, .. } => {
                        let channel = format!("event:{}", event_id);
                        ws_server_clone.is_client_subscribed(client_id, &channel).await
                    }
                    _ => false, // Don't forward subscription/unsubscribe messages
                };

                if !should_send {
                    continue;
                }

                let json = match serde_json::to_string(&msg) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };

                let mut sender = ws_sender_for_broadcast.lock().await;
                if let Err(e) = sender.send(Message::Text(json)).await {
                    error!("Failed to send message to client {}: {}", client_id, e);
                    break;
                }
            }
        });

        Ok(())
    }

    /// Broadcast price update
    pub async fn broadcast_price_update(
        &self,
        event_id: Uuid,
        prices: HashMap<String, f64>,
    ) {
        let message = WsMessage::PriceUpdate {
            event_id: event_id.to_string(),
            prices,
            timestamp: chrono::Utc::now().timestamp(),
        };

        let channel = format!("event:{}", event_id);
        self.broadcast_to_channel(&channel, message).await;
    }

    /// Broadcast bet executed
    pub async fn broadcast_bet_executed(
        &self,
        event_id: Uuid,
        bet_id: Uuid,
        user_wallet: String,
        outcome: String,
        shares: f64,
        price: f64,
    ) {
        let user_wallet_clone = user_wallet.clone();
        let message = WsMessage::BetExecuted {
            bet_id: bet_id.to_string(),
            user: user_wallet,
            outcome,
            shares,
            price,
        };

        // Broadcast to event subscribers and user subscribers
        let event_channel = format!("event:{}", event_id);
        self.broadcast_to_channel(&event_channel, message.clone()).await;

        // Also broadcast to user's own channel
        let user_channel = format!("user:{}", user_wallet_clone);
        self.broadcast_to_channel(&user_channel, message).await;
    }

    /// Check if client is subscribed to a channel
    async fn is_client_subscribed(&self, client_id: Uuid, channel: &str) -> bool {
        let subscriptions = self.subscriptions.read().await;
        if let Some(subscribers) = subscriptions.get(channel) {
            subscribers.contains(&client_id)
        } else {
            false
        }
    }

    /// Broadcast to group subscribers
    pub async fn broadcast_to_group(
        &self,
        group_id: Uuid,
        message: WsMessage,
    ) {
        let channel = format!("group:{}", group_id);
        self.broadcast_to_channel(&channel, message).await;
    }

    /// Broadcast to user subscribers
    pub async fn broadcast_to_user(
        &self,
        user_wallet: &str,
        message: WsMessage,
    ) {
        let channel = format!("user:{}", user_wallet);
        self.broadcast_to_channel(&channel, message).await;
    }

    /// Broadcast event settled
    pub async fn broadcast_event_settled(
        &self,
        event_id: Uuid,
        winning_outcome: String,
    ) {
        let message = WsMessage::EventSettled {
            event_id: event_id.to_string(),
            winning_outcome,
        };

        let channel = format!("event:{}", event_id);
        self.broadcast_to_channel(&channel, message).await;
    }
}

impl Clone for WebSocketServer {
    fn clone(&self) -> Self {
        Self {
            tx: self.tx.clone(),
            subscriptions: Arc::clone(&self.subscriptions),
            client_channels: Arc::clone(&self.client_channels),
        }
    }
}

impl Default for WebSocketServer {
    fn default() -> Self {
        Self::new()
    }
}

