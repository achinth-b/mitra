use rust_decimal::Decimal;
use std::collections::HashMap;
use thiserror::Error;

/// Error types for AMM operations
#[derive(Error, Debug)]
pub enum AmmError {
    #[error("Invalid outcome: {0}")]
    InvalidOutcome(String),

    #[error("Invalid amount: {0}")]
    InvalidAmount(String),

    #[error("Price out of bounds: {0}")]
    PriceOutOfBounds(String),

    #[error("Insufficient liquidity")]
    InsufficientLiquidity,

    #[error("Calculation error: {0}")]
    CalculationError(String),
}

/// Result type for AMM operations
pub type AmmResult<T> = Result<T, AmmError>;

/// Logarithmic Market Scoring Rule (LMSR) AMM
/// 
/// Uses the formula: P(outcome) = exp(shares[outcome]/b) / sum(exp(shares[i]/b))
/// where b is the liquidity parameter
pub struct LmsrAmm {
    /// Liquidity parameter (typically 100-1000 USDC)
    pub liquidity_parameter: Decimal,
    /// Current shares for each outcome
    shares: HashMap<String, Decimal>,
    /// Minimum price (0.01)
    min_price: Decimal,
    /// Maximum price (0.99)
    max_price: Decimal,
}

impl LmsrAmm {
    /// Create a new LMSR AMM with initial liquidity
    /// 
    /// # Arguments
    /// * `liquidity_parameter` - The 'b' parameter (e.g., 100.0)
    /// * `outcomes` - List of possible outcomes (e.g., ["YES", "NO"])
    pub fn new(liquidity_parameter: Decimal, outcomes: Vec<String>) -> AmmResult<Self> {
        if outcomes.is_empty() {
            return Err(AmmError::InvalidOutcome("Outcomes cannot be empty".to_string()));
        }

        if liquidity_parameter <= Decimal::ZERO {
            return Err(AmmError::InvalidAmount("Liquidity parameter must be positive".to_string()));
        }

        // Initialize shares to zero for all outcomes
        let shares: HashMap<String, Decimal> = outcomes
            .into_iter()
            .map(|outcome| (outcome, Decimal::ZERO))
            .collect();

        Ok(Self {
            liquidity_parameter,
            shares,
            min_price: Decimal::new(1, 2), // 0.01
            max_price: Decimal::new(99, 2), // 0.99
        })
    }

    /// Get current prices for all outcomes
    pub fn get_prices(&self) -> AmmResult<HashMap<String, Decimal>> {
        self.calculate_prices()
    }

    /// Calculate current prices using LMSR formula
    /// 
    /// Handles edge cases:
    /// - Zero liquidity (all shares = 0): Equal prices for all outcomes
    /// - First bet: Prices adjust from equal distribution
    fn calculate_prices(&self) -> AmmResult<HashMap<String, Decimal>> {
        if self.shares.is_empty() {
            return Err(AmmError::InvalidOutcome("No outcomes defined".to_string()));
        }

        // Edge case: Zero liquidity (all shares are zero)
        // Return equal prices for all outcomes
        let total_shares: Decimal = self.shares.values().sum();
        if total_shares == Decimal::ZERO {
            let equal_price = Decimal::ONE / Decimal::from(self.shares.len() as u64);
            let mut prices = HashMap::new();
            for outcome in self.shares.keys() {
                prices.insert(outcome.clone(), equal_price);
            }
            return Ok(prices);
        }

        // Calculate exp(shares[i]/b) for each outcome
        let mut exp_values: Vec<(String, Decimal)> = Vec::new();
        let mut sum_exp = Decimal::ZERO;

        for (outcome, shares) in &self.shares {
            // exp(shares / b)
            let exp_value = self.exp_approximation(*shares / self.liquidity_parameter)?;
            exp_values.push((outcome.clone(), exp_value));
            sum_exp += exp_value;
        }

        if sum_exp == Decimal::ZERO {
            // Fallback to equal prices if calculation fails
            let equal_price = Decimal::ONE / Decimal::from(self.shares.len() as u64);
            let mut prices = HashMap::new();
            for outcome in self.shares.keys() {
                prices.insert(outcome.clone(), equal_price);
            }
            return Ok(prices);
        }

        // Calculate prices: P(i) = exp(i) / sum(exp)
        let mut prices = HashMap::new();
        for (outcome, exp_value) in exp_values {
            let price = exp_value / sum_exp;
            // Constrain price between min_price and max_price
            let constrained_price = price.max(self.min_price).min(self.max_price);
            prices.insert(outcome, constrained_price);
        }

        // Normalize prices to sum to 1.0 (after constraints)
        self.normalize_prices(&mut prices)?;

        Ok(prices)
    }

    /// Approximate exp(x) using Taylor series expansion
    /// exp(x) ≈ 1 + x + x²/2! + x³/3! + x⁴/4!
    fn exp_approximation(&self, x: Decimal) -> AmmResult<Decimal> {
        // For small x, use Taylor series
        // For large x, this approximation may not be accurate enough
        // In production, consider using a more robust math library
        
        let one = Decimal::ONE;
        let x_squared = x * x;
        let x_cubed = x_squared * x;
        let x_fourth = x_cubed * x;

        let result = one 
            + x 
            + x_squared / Decimal::new(2, 0)
            + x_cubed / Decimal::new(6, 0)
            + x_fourth / Decimal::new(24, 0);

        // Ensure result is positive
        Ok(result.max(Decimal::new(1, 10))) // Minimum 0.0000000001
    }

    /// Normalize prices so they sum to 1.0
    fn normalize_prices(&self, prices: &mut HashMap<String, Decimal>) -> AmmResult<()> {
        let sum: Decimal = prices.values().sum();
        if sum == Decimal::ZERO {
            return Err(AmmError::CalculationError("Cannot normalize: sum is zero".to_string()));
        }

        // Scale all prices proportionally
        for price in prices.values_mut() {
            *price = *price / sum;
            // Re-apply constraints
            *price = (*price).max(self.min_price).min(self.max_price);
        }

        // Final normalization after constraints
        let final_sum: Decimal = prices.values().sum();
        if final_sum > Decimal::ZERO {
            for price in prices.values_mut() {
                *price = *price / final_sum;
            }
        }

        Ok(())
    }

    /// Calculate shares and cost for buying a given amount
    /// 
    /// Uses iterative method to solve LMSR cost function:
    /// C(q) = b * ln(sum(exp(q_i/b)))
    /// 
    /// # Arguments
    /// * `outcome` - The outcome to buy shares for
    /// * `amount_usdc` - Amount of USDC to spend
    /// 
    /// # Returns
    /// (shares_received, price_per_share, new_prices)
    /// 
    /// # Edge Cases Handled
    /// - First bet (zero liquidity): Uses equal price approximation
    /// - Small amounts: Ensures minimum shares received
    pub fn calculate_buy(
        &self,
        outcome: &str,
        amount_usdc: Decimal,
    ) -> AmmResult<(Decimal, Decimal, HashMap<String, Decimal>)> {
        if !self.shares.contains_key(outcome) {
            return Err(AmmError::InvalidOutcome(format!("Outcome '{}' not found", outcome)));
        }

        if amount_usdc <= Decimal::ZERO {
            return Err(AmmError::InvalidAmount("Amount must be positive".to_string()));
        }

        // Get current price
        let current_prices = self.calculate_prices()?;
        let current_price = current_prices
            .get(outcome)
            .ok_or_else(|| AmmError::InvalidOutcome(format!("Outcome '{}' not in prices", outcome)))?;

        // Calculate cost before purchase
        let cost_before = self.calculate_cost()?;

        // Use iterative method to find shares that match the cost
        // Start with approximation: shares = amount / current_price
        let mut shares_received = amount_usdc / *current_price;
        let mut iterations = 0;
        let max_iterations = 10;
        let tolerance = Decimal::new(1, 4); // 0.0001 USDC tolerance

        // Iteratively refine shares to match exact cost
        while iterations < max_iterations {
            // Create temporary state with new shares
            let mut test_shares = self.shares.clone();
            *test_shares.get_mut(outcome).unwrap() += shares_received;

            // Calculate cost after purchase
            let temp_amm = Self {
                liquidity_parameter: self.liquidity_parameter,
                shares: test_shares,
                min_price: self.min_price,
                max_price: self.max_price,
            };
            let cost_after = temp_amm.calculate_cost()?;
            let cost_diff = cost_after - cost_before;

            // Check if we're close enough
            let error = (cost_diff - amount_usdc).abs();
            if error < tolerance {
                break;
            }

            // Adjust shares based on error
            // If cost_diff is too high, reduce shares; if too low, increase shares
            let adjustment = (amount_usdc - cost_diff) / *current_price;
            shares_received += adjustment;

            // Ensure shares are positive
            if shares_received <= Decimal::ZERO {
                shares_received = Decimal::new(1, 6); // Minimum 0.000001 shares
            }

            iterations += 1;
        }

        // Ensure minimum shares received (edge case: very small amounts)
        if shares_received < Decimal::new(1, 6) {
            shares_received = Decimal::new(1, 6);
        }

        // Create new state with updated shares
        let mut new_shares = self.shares.clone();
        *new_shares.get_mut(outcome).unwrap() += shares_received;

        // Calculate new prices
        let temp_amm = Self {
            liquidity_parameter: self.liquidity_parameter,
            shares: new_shares,
            min_price: self.min_price,
            max_price: self.max_price,
        };
        let new_prices = temp_amm.calculate_prices()?;

        // Get new price for this outcome
        let new_price = new_prices
            .get(outcome)
            .ok_or_else(|| AmmError::CalculationError("Failed to get new price".to_string()))?;

        Ok((shares_received, *new_price, new_prices))
    }

    /// Calculate the cost function C(q) = b * ln(sum(exp(q_i/b)))
    /// 
    /// This is used for exact share calculation in buy operations
    fn calculate_cost(&self) -> AmmResult<Decimal> {
        let mut sum_exp = Decimal::ZERO;

        for shares in self.shares.values() {
            let exp_value = self.exp_approximation(*shares / self.liquidity_parameter)?;
            sum_exp += exp_value;
        }

        if sum_exp <= Decimal::ZERO {
            return Err(AmmError::CalculationError("Sum of exp values must be positive".to_string()));
        }

        // C(q) = b * ln(sum_exp)
        // Using approximation: ln(x) ≈ (x-1) - (x-1)²/2 + (x-1)³/3 for x near 1
        // For better accuracy, use: ln(x) = 2 * ((x-1)/(x+1)) + 2/3 * ((x-1)/(x+1))³ + ...
        let x = sum_exp;
        let ln_approx = self.ln_approximation(x)?;
        let cost = self.liquidity_parameter * ln_approx;

        Ok(cost)
    }

    /// Approximate ln(x) using series expansion
    /// ln(x) ≈ 2 * ((x-1)/(x+1)) + 2/3 * ((x-1)/(x+1))³ + ...
    fn ln_approximation(&self, x: Decimal) -> AmmResult<Decimal> {
        if x <= Decimal::ZERO {
            return Err(AmmError::CalculationError("Cannot calculate ln of non-positive number".to_string()));
        }

        // For x near 1, use series expansion
        if (x - Decimal::ONE).abs() < Decimal::new(1, 1) {
            let t = (x - Decimal::ONE) / (x + Decimal::ONE);
            let t_squared = t * t;
            let t_cubed = t_squared * t;
            
            let ln = Decimal::new(2, 0) * (t + t_cubed / Decimal::new(3, 0));
            return Ok(ln);
        }

        // For larger x, use approximation: ln(x) ≈ (x-1) - (x-1)²/2
        let diff = x - Decimal::ONE;
        let ln = diff - (diff * diff) / Decimal::new(2, 0);
        Ok(ln.max(Decimal::new(-10, 0)).min(Decimal::new(10, 0))) // Clamp to reasonable range
    }

    /// Update shares after a bet is placed
    pub fn update_shares(&mut self, outcome: &str, shares: Decimal) -> AmmResult<()> {
        if !self.shares.contains_key(outcome) {
            return Err(AmmError::InvalidOutcome(format!("Outcome '{}' not found", outcome)));
        }

        *self.shares.get_mut(outcome).unwrap() += shares;
        Ok(())
    }

    /// Get current shares for an outcome
    pub fn get_shares(&self, outcome: &str) -> Option<Decimal> {
        self.shares.get(outcome).copied()
    }

    /// Get all current shares
    pub fn get_all_shares(&self) -> &HashMap<String, Decimal> {
        &self.shares
    }

    /// Get total liquidity (sum of all shares)
    pub fn get_total_liquidity(&self) -> Decimal {
        self.shares.values().sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lmsr_creation() {
        let amm = LmsrAmm::new(
            Decimal::new(100, 0),
            vec!["YES".to_string(), "NO".to_string()],
        ).unwrap();

        assert_eq!(amm.shares.len(), 2);
    }

    #[test]
    fn test_get_prices() {
        let amm = LmsrAmm::new(
            Decimal::new(100, 0),
            vec!["YES".to_string(), "NO".to_string()],
        ).unwrap();

        let prices = amm.get_prices().unwrap();
        assert_eq!(prices.len(), 2);
        
        // Prices should sum to approximately 1.0
        let sum: Decimal = prices.values().sum();
        assert!(sum >= Decimal::new(99, 2)); // Allow for rounding
        assert!(sum <= Decimal::new(101, 2));
    }

    #[test]
    fn test_calculate_buy() {
        let amm = LmsrAmm::new(
            Decimal::new(100, 0),
            vec!["YES".to_string(), "NO".to_string()],
        ).unwrap();

        let (shares, price, new_prices) = amm.calculate_buy("YES", Decimal::new(10, 0)).unwrap();
        
        assert!(shares > Decimal::ZERO);
        assert!(price >= Decimal::new(1, 2)); // >= 0.01
        assert!(price <= Decimal::new(99, 2)); // <= 0.99
        assert_eq!(new_prices.len(), 2);
    }

    #[test]
    fn test_zero_liquidity_prices() {
        // Test that zero liquidity returns equal prices
        let amm = LmsrAmm::new(
            Decimal::new(100, 0),
            vec!["YES".to_string(), "NO".to_string()],
        ).unwrap();

        let prices = amm.get_prices().unwrap();
        assert_eq!(prices.len(), 2);
        
        // With zero liquidity, prices should be equal (0.5 each)
        let yes_price = prices.get("YES").unwrap();
        let no_price = prices.get("NO").unwrap();
        
        // Allow for small rounding differences
        let diff = (*yes_price - *no_price).abs();
        assert!(diff < Decimal::new(1, 3)); // Less than 0.001 difference
    }

    #[test]
    fn test_first_bet() {
        // Test first bet scenario (zero liquidity -> first bet)
        let mut amm = LmsrAmm::new(
            Decimal::new(100, 0),
            vec!["YES".to_string(), "NO".to_string()],
        ).unwrap();

        // Get initial prices (should be equal)
        let initial_prices = amm.get_prices().unwrap();
        let initial_yes = initial_prices.get("YES").unwrap();

        // Place first bet
        let (shares, price, new_prices) = amm.calculate_buy("YES", Decimal::new(10, 0)).unwrap();
        
        assert!(shares > Decimal::ZERO);
        assert!(price > *initial_yes); // Price should increase after buying YES
        
        // Update AMM state
        amm.update_shares("YES", shares).unwrap();
        
        // Verify prices changed
        let updated_prices = amm.get_prices().unwrap();
        let updated_yes = updated_prices.get("YES").unwrap();
        assert!(updated_yes > initial_yes);
    }

    #[test]
    fn test_price_constraints() {
        let mut amm = LmsrAmm::new(
            Decimal::new(100, 0),
            vec!["YES".to_string(), "NO".to_string()],
        ).unwrap();

        // Place very large bet to push price to limit
        let (shares, price, _) = amm.calculate_buy("YES", Decimal::new(10000, 0)).unwrap();
        amm.update_shares("YES", shares).unwrap();

        let prices = amm.get_prices().unwrap();
        let yes_price = prices.get("YES").unwrap();
        
        // Price should be constrained between 0.01 and 0.99
        assert!(*yes_price >= Decimal::new(1, 2)); // >= 0.01
        assert!(*yes_price <= Decimal::new(99, 2)); // <= 0.99
    }
}

