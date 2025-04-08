use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use tracing::{info, debug};

use crate::listen_bot::mempool_scanner::SwapTransaction;
use crate::rig_agent::agent::AgentDecision;
use crate::listen_bot::transaction::TradeDecision;
use super::OpportunityScore;

pub struct OpportunityScorer {
    min_opportunity_score: f64,
    max_risk_level: u8,
}

impl OpportunityScorer {
    pub fn new(min_opportunity_score: f64, max_risk_level: u8) -> Self {
        Self {
            min_opportunity_score,
            max_risk_level,
        }
    }
    
    pub fn calculate_score(
        &self,
        transaction: &SwapTransaction,
        agent_decision: &AgentDecision,
        market_data: Option<String>,
    ) -> OpportunityScore {
        info!("Calculating opportunity score...");
        
        // Extract the base score from the agent's decision
        let base_score = agent_decision.opportunity_score;
        
        // Convert risk level string to numeric value
        let risk_level = match agent_decision.risk_level.as_str() {
            "low" => 1,
            "medium" => 2,
            "high" => 3,
            _ => 0,
        };
        
        // Calculate confidence based on agent's score
        let confidence = if base_score > 0.9 {
            0.95
        } else if base_score > 0.8 {
            0.85
        } else if base_score > 0.7 {
            0.75
        } else {
            0.5
        };
        
        // Estimate profitability based on slippage and transaction size
        // (In a real system, this would be much more sophisticated)
        let profitability = transaction.slippage * transaction.amount_in * 0.01;
        
        OpportunityScore {
            mev_score: base_score,
            confidence,
            profitability,
            risk_level,
        }
    }
    
    pub fn should_execute(&self, score: &OpportunityScore) -> bool {
        // Check if the opportunity meets our criteria
        score.mev_score >= self.min_opportunity_score 
            && score.risk_level <= self.max_risk_level
    }
}