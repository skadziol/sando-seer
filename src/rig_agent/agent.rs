use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use tracing::{info, error, debug};
use rand::Rng;

use crate::listen_bot::mempool_scanner::SwapTransaction;
use crate::evaluator::OpportunityScore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDecision {
    pub opportunity_score: f64,
    pub action: String, // "enter", "skip"
    pub risk_level: String, // "low", "medium", "high"
    pub reasoning: String,
}

pub struct RigAgent {
    api_key: Option<String>,
}

impl RigAgent {
    pub fn new(api_key: Option<String>) -> Self {
        Self { api_key }
    }
    
    pub async fn evaluate_opportunity(
        &self,
        transaction: &SwapTransaction,
        market_data: Option<String>,
        sentiment_data: Option<String>,
    ) -> Result<AgentDecision> {
        info!("Evaluating opportunity with RIG agent...");
        
        // For now, always use simulation
        info!("Using simulation mode");
        Ok(self.simulate_agent_decision(transaction))
    }
    
    fn format_context(
        &self,
        transaction: &SwapTransaction,
        market_data: Option<String>,
        sentiment_data: Option<String>,
    ) -> String {
        let mut context = format!(
            "Transaction Details:\n\
            - Token In: {}\n\
            - Token Out: {}\n\
            - Amount: {} {}\n\
            - Expected Output: ~ {} {}\n\
            - Slippage: {}%\n\
            - DEX: {}\n\
            - Wallet: {}\n",
            transaction.token_in,
            transaction.token_out,
            transaction.amount_in,
            transaction.token_in,
            transaction.estimated_amount_out,
            transaction.token_out,
            transaction.slippage * 100.0,
            transaction.pool_name,
            transaction.wallet_address
        );
        
        // Add market data if available
        if let Some(data) = market_data {
            context.push_str("\nMarket Data:\n");
            context.push_str(&data);
        }
        
        // Add sentiment data if available
        if let Some(data) = sentiment_data {
            context.push_str("\nSentiment Data:\n");
            context.push_str(&data);
        }
        
        context
    }
    
    fn simulate_agent_decision(&self, transaction: &SwapTransaction) -> AgentDecision {
        // Simplified simulation logic - in reality, this would be done by the LLM
        let mut rng = rand::thread_rng();
        
        // Higher score for larger transactions (whales)
        let size_factor = (transaction.amount_in / 100.0).min(1.0) * 0.4;
        
        // Higher score for higher slippage (potential for MEV)
        let slippage_factor = (transaction.slippage * 100.0).min(1.0) * 0.3;
        
        // Pool-specific factor (simplified)
        let pool_factor = match transaction.pool_name.as_str() {
            "Orca" => 0.2,
            "Raydium" => 0.15,
            _ => 0.1,
        };
        
        // Token pair specific factor (simplified)
        let pair_factor = if transaction.token_in == "SOL" && transaction.token_out == "USDC" {
            0.2 // Well-known liquid pair
        } else if transaction.token_in == "USDC" && transaction.token_out == "BONK" {
            0.3 // More volatile, more opportunity
        } else {
            0.1
        };
        
        let final_score = size_factor + slippage_factor + pool_factor + pair_factor;
        
        // Add some randomness for testing purposes
        let random_factor = rng.gen::<f64>() * 0.2;
        
        let opportunity_score = (final_score + random_factor).min(1.0);
        
        let action = if opportunity_score > 0.7 { "enter" } else { "skip" };
        let risk_level = if opportunity_score > 0.85 { 
            "high" 
        } else if opportunity_score > 0.75 { 
            "medium" 
        } else { 
            "low" 
        };
        
        let reasoning = format!(
            "Transaction analysis: {} {} -> {} on {}. \
            Transaction size and slippage suggest {}.",
            transaction.amount_in,
            transaction.token_in,
            transaction.token_out,
            transaction.pool_name,
            if opportunity_score > 0.7 { "potential opportunity" } else { "low probability of success" }
        );
        
        AgentDecision {
            opportunity_score,
            action: action.to_string(),
            risk_level: risk_level.to_string(),
            reasoning,
        }
    }
}