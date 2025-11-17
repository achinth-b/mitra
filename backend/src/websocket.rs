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
}

impl WebSocketServer {
    /// Create a new WebSocket server
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000); // Buffer up to 1000 messages

        Self {
            tx,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
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
            for _client_id in subscribers {
                // Send to broadcast channel (all subscribers will receive)
                if let Err(e) = self.tx.send(message.clone()) {
                    warn!("Failed to broadcast message: {}", e);
                }
            }
        }
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
        let subscriptions = self.subscriptions.clone();
        let tx_clone = self.tx.clone();
        
        tokio::spawn(async move {
            while let Some(msg) = ws_receiver.next().await {
                match msg {
                    Ok(Message::Text(text)) => {
                        // Parse subscription message
                        if let Ok(sub_msg) = serde_json::from_str::<WsMessage>(&text) {
                            match sub_msg {
                                WsMessage::Subscribe { channel } => {
                                    info!("Client {} subscribed to {}", client_id, channel);
                                    let mut subs = subscriptions.write().await;
                                    subs.entry(channel).or_insert_with(Vec::new).push(client_id);
                                }
                                WsMessage::Unsubscribe { channel } => {
                                    info!("Client {} unsubscribed from {}", client_id, channel);
                                    let mut subs = subscriptions.write().await;
                                    if let Some(subscribers) = subs.get_mut(&channel) {
                                        subscribers.retain(|&id| id != client_id);
                                    }
                                }
                                _ => {
                                    warn!("Unexpected message type from client {}", client_id);
                                }
                            }
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

            // Clean up subscriptions
            let mut subs = subscriptions.write().await;
            for subscribers in subs.values_mut() {
                subscribers.retain(|&id| id != client_id);
            }
        });

        // Spawn task to send messages to client
        tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(json) => json,
                    Err(e) => {
                        error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };

                if let Err(e) = ws_sender.send(Message::Text(json)).await {
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
        bet_id: Uuid,
        user_wallet: String,
        outcome: String,
        shares: f64,
        price: f64,
    ) {
        let message = WsMessage::BetExecuted {
            bet_id: bet_id.to_string(),
            user: user_wallet,
            outcome,
            shares,
            price,
        };

        // Broadcast to all event subscribers
        // TODO: Get event_id from bet and broadcast to specific channel
        if let Err(e) = self.tx.send(message) {
            warn!("Failed to broadcast bet executed: {}", e);
        }
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

impl Default for WebSocketServer {
    fn default() -> Self {
        Self::new()
    }
}

