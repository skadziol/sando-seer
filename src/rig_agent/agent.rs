use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use tracing::{info, error, debug};
use rig::agent::{Agent, AgentConfig};
use rig::providers::openai::OpenAIProvider;

use crate::listen_bot::SwapTransaction;
use crate::evaluator::OpportunityScore;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDecision {
    pub opportunity_score: f64,
    pub action: String, // "enter", "skip"
    pub risk_level: String, // "low", "medium", "high"
    pub reasoning: String,
}

pub struct RigAgent {
    agent: Option<Agent>,
}

impl RigAgent {
    pub fn new(api_key: Option<String>) -> Self {
        // Only initialize the rig agent if we have an API key
        let agent = api_key.map(|key| {
            let provider = OpenAIProvider::new(key);
            let config = AgentConfig {
                name: "SandoSeer MEV Oracle".to_string(),
                description: "AI agent for detecting MEV opportunities on Solana".to_string(),
                ..Default::default()
            };
            
            Agent::new(config, Box::new(provider))
        });
        
        Self {
            agent,
        }
    }
    
    pub async fn evaluate_opportunity(
        &self,
        transaction: &SwapTransaction,
        market_data: Option<String>,
        sentiment_data: Option<String>,
    ) -> Result<AgentDecision> {
        info!("Evaluating opportunity with RIG agent...");
        
        // If we don't have an agent configured, use the simulation
        if self.agent.is_none() {
            info!("No RIG agent configured, using simulation");
            return Ok(self.simulate_agent_decision(transaction));
        }
        
        let agent = self.agent.as_ref().unwrap();
        
        // Format the context for the agent
        let context = self.format_context(transaction, market_data, sentiment_data);
        
        // Create the prompt for the agent
        let prompt = format!(
            "You are an expert in MEV (Maximal Extractable Value) detection on the Solana blockchain. \
            Your goal is to evaluate whether the following transaction presents a profitable MEV opportunity.\n\n\
            === TRANSACTION CONTEXT ===\n{}\n\n\
            Based on this information, evaluate whether this is a good MEV opportunity. \
            Return your answer in JSON format with the following fields:\n\
            - opportunity_score: a number between 0 and 1 indicating the quality of the opportunity\n\
            - action: either 'enter' or 'skip'\n\
            - risk_level: 'low', 'medium', or 'high'\n\
            - reasoning: a brief explanation of your decision",
            context
        );
        
        // Send the prompt to the agent
        let response = agent.prompt(prompt).await?;
        
        // Parse the JSON response
        let decision: AgentDecision = match serde_json::from_str(&response) {
            Ok(decision) => decision,
            Err(e) => {
                error!("Failed to parse agent response as JSON: {}", e);
                error!("Raw response: {}", response);
                
                // Fall back to simulation
                info!("Falling back to simulated decision");
                self.simulate_agent_decision(transaction)
            }
        };
        
        Ok(decision)
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
        let random_factor = rand::random::<f64>() * 0.2;
        
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