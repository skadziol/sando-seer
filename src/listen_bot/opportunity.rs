use anyhow::Result;
use listen_core::model::tx::Transaction as ListenTransaction;
use listen_core::router::{Router, RouterConfig, quote::QuoteResponse};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tracing::{debug, error};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OpportunityType {
    Sandwich,
    Arbitrage,
    TokenSnipe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MEVOpportunity {
    pub opportunity_type: OpportunityType,
    pub token_in: Pubkey,
    pub token_out: Pubkey,
    pub amount_in: u64,
    pub expected_profit: f64,
    pub risk_score: f64,
    #[serde(skip)]
    pub quote: Option<QuoteResponse>,
}

pub struct OpportunityDetector {
    router: Router,
    min_profit_threshold: f64,
    max_risk_threshold: f64,
}

impl OpportunityDetector {
    pub fn new(rpc_url: String, min_profit_threshold: f64, max_risk_threshold: f64) -> Result<Self> {
        let router_config = RouterConfig {
            rpc_url,
            commitment: "confirmed".to_string(),
        };
        
        let router = Router::new(router_config)?;
        
        Ok(Self {
            router,
            min_profit_threshold,
            max_risk_threshold,
        })
    }
    
    pub async fn analyze_sandwich_opportunity(&self, tx: &ListenTransaction) -> Result<Option<MEVOpportunity>> {
        // Extract swap info
        let swap_info = match &tx.swap_info {
            Some(info) => info,
            None => return Ok(None),
        };
        
        // Get token prices and liquidity info
        let token_in = Pubkey::from_str(&swap_info.token_in.mint.to_string())?;
        let token_out = Pubkey::from_str(&swap_info.token_out.mint.to_string())?;
        
        // Get best quote for potential frontrun
        let quote = self.router.get_best_quote(
            &token_in,
            &token_out,
            swap_info.amount_in,
            None,
        ).await?;
        
        // Calculate potential profit
        let profit = self.calculate_sandwich_profit(&quote, swap_info.amount_in)?;
        
        // Calculate risk score based on various factors
        let risk_score = self.calculate_risk_score(&quote, &tx)?;
        
        // Check if opportunity meets our criteria
        if profit >= self.min_profit_threshold && risk_score <= self.max_risk_threshold {
            return Ok(Some(MEVOpportunity {
                opportunity_type: OpportunityType::Sandwich,
                token_in,
                token_out,
                amount_in: swap_info.amount_in,
                expected_profit: profit,
                risk_score,
                quote: Some(quote),
            }));
        }
        
        Ok(None)
    }
    
    fn calculate_sandwich_profit(&self, quote: &QuoteResponse, amount_in: u64) -> Result<f64> {
        // Calculate the potential profit from a sandwich trade
        let price_impact = 0.01; // Assume 1% price impact for now
        let gas_cost = 0.001; // Assume fixed gas cost in SOL
        
        // Simple profit calculation: 
        // - Assume we can capture half of the price impact
        // - Subtract gas costs and a safety margin
        let gross_profit = (amount_in as f64 * price_impact * 0.5) - gas_cost;
        let safety_margin = gross_profit * 0.1; // 10% safety margin
        
        Ok(gross_profit - safety_margin)
    }
    
    fn calculate_risk_score(&self, quote: &QuoteResponse, tx: &ListenTransaction) -> Result<f64> {
        let mut risk_score = 0.5; // Base risk score
        
        // Adjust based on amount size
        if let Some(swap_info) = &tx.swap_info {
            let amount_size_risk = (swap_info.amount_in as f64 / 1_000_000.0).min(0.3);
            risk_score += amount_size_risk;
        }
        
        // Adjust based on token popularity/liquidity
        // For now, use a simplified approach
        risk_score += 0.1; // Add some risk for unknown factors
        
        // Ensure risk score stays between 0 and 1
        Ok(risk_score.max(0.0).min(1.0))
    }
} 