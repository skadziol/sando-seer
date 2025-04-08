use anyhow::{Result, Context};
use tracing::{info, debug};

use crate::listen_bot::{SwapTransaction, TradeDecision};
use crate::rig_agent::AgentDecision;
use super::OpportunityScore;

pub struct DecisionMaker {
    min_profitability: f64,
}

impl DecisionMaker {
    pub fn new(min_profitability: f64) -> Self {
        Self {
            min_profitability,
        }
    }
    
    pub fn make_decision(
        &self,
        transaction: &SwapTransaction,
        score: &OpportunityScore,
        agent_decision: &AgentDecision,
    ) -> Option<TradeDecision> {
        info!("Making final trade decision...");
        
        // Only proceed if the agent suggests entering
        if agent_decision.action != "enter" {
            debug!("Agent decided to skip this opportunity");
            return None;
        }
        
        // Check if profitability meets our minimum threshold
        if score.profitability < self.min_profitability {
            debug!("Profitability too low: {}", score.profitability);
            return None;
        }
        
        // Determine the best strategy based on the opportunity
        let strategy = self.determine_strategy(transaction, score);
        
        // Calculate a reasonable minimum output amount with some buffer
        let expected_min_out = transaction.estimated_amount_out * (1.0 - transaction.slippage * 1.5);
        
        Some(TradeDecision {
            token_in: transaction.token_in.clone(),
            token_out: transaction.token_out.clone(),
            amount_in: transaction.amount_in,
            expected_min_out,
            confidence_score: score.confidence,
            risk_level: score.risk_level,
            strategy,
        })
    }
    
    fn determine_strategy(&self, transaction: &SwapTransaction, score: &OpportunityScore) -> String {
        // Simplified strategy selection
        // In a real system, this would be much more sophisticated
        
        if transaction.slippage > 0.03 && transaction.amount_in > 5.0 {
            // High slippage + decent size = potential for sandwich
            "sandwich".to_string()
        } else if score.mev_score > 0.85 {
            // Very high score might indicate a good snipe opportunity
            "snipe".to_string()
        } else {
            // Default to arbitrage for most situations
            "arbitrage".to_string()
        }
    }
}