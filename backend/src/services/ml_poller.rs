use crate::amm::LmsrAmm;
use crate::error::{AppError, AppResult};
use crate::models::{Event, EventStatus};
use crate::repositories::{EventRepository, BetRepository};
use crate::websocket::WebSocketServer;
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use tracing::{error, info, warn};
use uuid::Uuid;

/// ML service poller that queries ML service and broadcasts price updates
pub struct MlPoller {
    ml_service_url: String,
    event_repo: Arc<EventRepository>,
    bet_repo: Arc<BetRepository>,
    ws_server: Arc<WebSocketServer>,
    poll_interval: Duration,
    price_change_threshold: f64, // Minimum price change to trigger broadcast (e.g., 0.01 = 1%)
    last_prices: Arc<tokio::sync::RwLock<HashMap<Uuid, HashMap<String, f64>>>>,
}

impl MlPoller {
    /// Create a new ML poller
    pub fn new(
        ml_service_url: String,
        event_repo: Arc<EventRepository>,
        bet_repo: Arc<BetRepository>,
        ws_server: Arc<WebSocketServer>,
    ) -> Self {
        Self {
            ml_service_url,
            event_repo,
            bet_repo,
            ws_server,
            poll_interval: Duration::from_secs(3), // Default: 3 seconds
            price_change_threshold: 0.01, // 1% change threshold
            last_prices: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }

    /// Set poll interval
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Set price change threshold
    pub fn with_price_threshold(mut self, threshold: f64) -> Self {
        self.price_change_threshold = threshold;
        self
    }

    /// Start polling ML service
    pub async fn start(self) {
        let mut interval = time::interval(self.poll_interval);
        info!("ML poller started, polling every {:?}", self.poll_interval);

        loop {
            interval.tick().await;

            if let Err(e) = self.poll_and_broadcast().await {
                error!("Error in ML poller: {}", e);
            }
        }
    }

    /// Poll ML service and broadcast updates if prices changed significantly
    async fn poll_and_broadcast(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Get all active events
        let active_events = self.event_repo.find_active_events().await?;

        if active_events.is_empty() {
            return Ok(());
        }

        for event in active_events {
            // Calculate current prices using AMM
            let current_prices = self.calculate_current_prices(&event).await?;

            // Check if prices changed significantly
            let should_broadcast = self.should_broadcast_price_update(event.id, &current_prices).await
                .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e))) as Box<dyn std::error::Error>)?;

            if should_broadcast {
                // Query ML service for recommendations (optional)
                let recommended_prices = self.query_ml_service(&event, &current_prices).await
                    .map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, format!("{}", e))) as Box<dyn std::error::Error>)?;

                // Use ML recommendations if available, otherwise use AMM prices
                let prices_to_broadcast = recommended_prices.unwrap_or(current_prices.clone());

                // Broadcast price update
                let prices_f64: HashMap<String, f64> = prices_to_broadcast
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_f64().unwrap_or(0.0)))
                    .collect();

                self.ws_server
                    .broadcast_price_update(event.id, prices_f64)
                    .await;

                // Update last known prices
                let mut last_prices = self.last_prices.write().await;
                last_prices.insert(event.id, current_prices);
            }
        }

        Ok(())
    }

    /// Calculate current prices using AMM
    async fn calculate_current_prices(&self, event: &Event) -> Result<HashMap<String, Decimal>, Box<dyn std::error::Error>> {
        // Get all bets for event
        let bets = self.bet_repo.find_by_event(event.id).await
            .map_err(|e| format!("Database error: {}", e))?;

        // Initialize AMM
        let outcomes = event.outcomes_vec();
        let mut amm = LmsrAmm::new(Decimal::new(100, 0), outcomes.clone())
            .map_err(|e| format!("AMM error: {}", e))?;

        // Update AMM with existing shares
        for bet in &bets {
            amm.update_shares(&bet.outcome, bet.shares)
                .map_err(|e| format!("AMM error: {}", e))?;
        }

        // Get prices
        let prices = amm.get_prices()
            .map_err(|e| format!("AMM error: {}", e))?;

        Ok(prices)
    }

    /// Check if price change is significant enough to broadcast
    async fn should_broadcast_price_update(
        &self,
        event_id: Uuid,
        current_prices: &HashMap<String, Decimal>,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        let last_prices = self.last_prices.read().await;

        if let Some(last) = last_prices.get(&event_id) {
            // Check if any price changed by more than threshold
            for (outcome, current_price) in current_prices {
                if let Some(last_price) = last.get(outcome) {
                    let current_f64 = current_price.to_f64().unwrap_or(0.0);
                    let last_f64 = last_price.to_f64().unwrap_or(0.0);

                    if last_f64 > 0.0 {
                        let change = (current_f64 - last_f64).abs() / last_f64;
                        if change >= self.price_change_threshold {
                            return true;
                        }
                    }
                } else {
                    // New outcome, broadcast
                    return true;
                }
            }
            Ok(false)
        } else {
            // First time seeing this event, broadcast
            Ok(true)
        }
    }

    /// Query ML service for price recommendations
    async fn query_ml_service(
        &self,
        event: &Event,
        current_prices: &HashMap<String, Decimal>,
    ) -> Result<Option<HashMap<String, Decimal>>, Box<dyn std::error::Error>> {
        use reqwest::Client;

        let client = Client::new();
        let prices_f64: HashMap<String, f64> = current_prices
            .iter()
            .map(|(k, v)| (k.clone(), v.to_f64().unwrap_or(0.0)))
            .collect();

        // Get volume
        let total_volume = self.bet_repo
            .get_total_volume_for_event(event.id)
            .await?
            .unwrap_or(Decimal::ZERO);

        let bet_count = self.bet_repo.count_by_event(event.id).await?;

        // Calculate time since creation
        let time_since_creation = chrono::Utc::now()
            .signed_duration_since(event.created_at.and_utc())
            .num_seconds() as f64 / 3600.0; // Convert to hours

        let request_body = serde_json::json!({
            "event_id": event.id.to_string(),
            "current_prices": prices_f64,
            "total_volume": total_volume.to_f64().unwrap_or(0.0),
            "bet_count": bet_count,
            "time_since_creation": time_since_creation,
        });

        match client
            .post(&format!("{}/predict-prices", self.ml_service_url))
            .json(&request_body)
            .timeout(Duration::from_secs(2))
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    if let Ok(result) = response.json::<serde_json::Value>().await {
                        if let Some(recommended) = result.get("recommended_prices") {
                            let recommended_map: HashMap<String, f64> =
                                serde_json::from_value(recommended.clone())?;
                            
                            // Convert back to Decimal
                            let recommended_decimal: HashMap<String, Decimal> = recommended_map
                                .into_iter()
                                .map(|(k, v)| {
                                    // Convert f64 to Decimal
                                    Decimal::try_from(v as f64)
                                        .unwrap_or_else(|_| Decimal::from_f64(v).unwrap_or(Decimal::ZERO))
                                })
                                .collect();

                            return Ok(Some(recommended_decimal));
                        }
                    }
                }
            }
            Err(e) => {
                warn!("ML service query failed: {}", e);
            }
        }

        Ok(None)
    }
}

