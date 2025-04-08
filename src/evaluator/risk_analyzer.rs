use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

use crate::listen_bot::SwapTransaction;
use crate::rig_agent::{SentimentData, MarketData};

pub struct RiskAnalyzer {
    // Configuration for risk analysis
    risk_tolerance: u8, // 1-3, where 3 is highest risk tolerance
}

impl RiskAnalyzer {
    pub fn new(risk_tolerance: u8) -> Self {
        Self {
            risk_tolerance: risk_tolerance.clamp(1, 3),
        }
    }
    
    pub fn analyze_risk(
        &self,
        transaction: &SwapTransaction,
        sentiment: Option<&SentimentData>,
        market_data: Option<&MarketData>,
    ) -> u8 {
        info!("Analyzing risk for transaction...");
        
        // Base risk level starts at medium (2)
        let mut risk_level = 2;
        
        // Adjust risk based on transaction size
        // Larger transactions are riskier
        if transaction.amount_in > 100.0 {
            risk_level += 1;
        } else if transaction.amount_in < 10.0 {
            risk_level -= 1;
        }
        
        // Adjust risk based on slippage
        // Higher slippage means higher risk
        if transaction.slippage > 0.03 {
            risk_level += 1;
        } else if transaction.slippage < 0.01 {
            risk_level -= 1;
        }
        
        // Adjust risk based on sentiment if available
        if let Some(sentiment_data) = sentiment {
            // Negative sentiment increases risk
            if sentiment_data.sentiment_score < -0.3 {
                risk_level += 1;
            }
            // Very positive sentiment decreases risk
            else if sentiment_data.sentiment_score > 0.7 {
                risk_level -= 1;
            }
        }
        
        // Adjust risk based on market data if available
        if let Some(market) = market_data {
            // Try to find the token price data
            if let Some(token_price) = market.prices.iter().find(|p| p.token == transaction.token_out) {
                // High volatility (large 24h change) increases risk
                if token_price.change_24h.abs() > 10.0 {
                    risk_level += 1;
                }
            }
        }
        
        // Clamp the risk level between 1 and 3
        risk_level.clamp(1, 3)
    }
    
    pub fn is_within_risk_tolerance(&self, risk_level: u8) -> bool {
        // Check if the calculated risk level is within our tolerance
        risk_level <= self.risk_tolerance
    }
}