use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    signature::Keypair,
    transaction::Transaction,
    pubkey::Pubkey,
};
use std::sync::Arc;
use std::str::FromStr;
use tracing::{info, error, debug};
use listen_core::router::{
    Router, RouterConfig,
    dexes::{Dex, DexName},
    quote::QuoteResponse,
};
use listen_core::model::token::Token;

use crate::config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeDecision {
    pub token_in: String,
    pub token_out: String,
    pub amount_in: f64,
    pub expected_min_out: f64,
    pub confidence_score: f64,
    pub risk_level: u8,
    pub strategy: String, // "arbitrage", "sandwich", "snipe"
}

pub struct TransactionExecutor {
    rpc_client: Arc<RpcClient>,
    keypair: Keypair,
    simulation_mode: bool,
    router: Router,
}

impl TransactionExecutor {
    pub fn new(rpc_url: &str, simulation_mode: bool) -> Result<Self> {
        let rpc_client = Arc::new(RpcClient::new_with_commitment(
            rpc_url.to_string(),
            CommitmentConfig::confirmed(),
        ));
        
        let keypair = config::get_keypair()?;
        
        // Initialize the listen Router
        let router_config = RouterConfig {
            rpc_url: rpc_url.to_string(),
            commitment: "confirmed".to_string(),
        };
        
        let router = Router::new(router_config)?;
        
        Ok(Self {
            rpc_client,
            keypair,
            simulation_mode,
            router,
        })
    }
    
    pub async fn execute_trade(&self, decision: TradeDecision) -> Result<String> {
        info!("Executing trade: {:?}", decision);
        
        if self.simulation_mode {
            info!("[SIMULATION] Would execute trade for {} {} -> {}", 
                  decision.amount_in, 
                  decision.token_in, 
                  decision.token_out);
            
            // In simulation mode, just return a fake signature
            return Ok("SIM_TX_SIGNATURE".to_string());
        }
        
        // Convert token symbols to mints
        let token_in_mint = self.get_token_mint(&decision.token_in)?;
        let token_out_mint = self.get_token_mint(&decision.token_out)?;
        
        info!("Executing swap: {} {} -> {}", 
              decision.amount_in, 
              decision.token_in, 
              decision.token_out);
        
        // Calculate the input amount in raw units
        let amount_in_raw = self.to_raw_amount(decision.amount_in, &token_in_mint).await?;
        
        // Calculate minimum output amount
        let min_out_raw = self.to_raw_amount(decision.expected_min_out, &token_out_mint).await?;
        
        // Get the best quote
        let quote = self.router.get_best_quote(
            &token_in_mint, 
            &token_out_mint, 
            amount_in_raw,
            Some(min_out_raw),
        ).await?;
        
        // Execute the swap
        let signature = match decision.strategy.as_str() {
            "sandwich_front" | "sandwich_back" => {
                // For sandwich strategies, we need more precise routing
                self.execute_sandwich_leg(&quote, &self.keypair).await?
            },
            _ => {
                // For regular swaps, use standard routing
                self.router.swap(
                    &quote,
                    &self.keypair,
                ).await?
            }
        };
        
        info!("Trade executed successfully! Signature: {}", signature);
        
        Ok(signature)
    }
    
    pub async fn simulate_transaction(&self, decision: &TradeDecision) -> Result<bool> {
        info!("Simulating transaction before execution...");
        
        // Convert token symbols to mints
        let token_in_mint = self.get_token_mint(&decision.token_in)?;
        let token_out_mint = self.get_token_mint(&decision.token_out)?;
        
        // Calculate the input amount in raw units
        let amount_in_raw = self.to_raw_amount(decision.amount_in, &token_in_mint).await?;
        
        // Calculate minimum output amount
        let min_out_raw = self.to_raw_amount(decision.expected_min_out, &token_out_mint).await?;
        
        // Get the best quote
        let quote = self.router.get_best_quote(
            &token_in_mint, 
            &token_out_mint, 
            amount_in_raw,
            Some(min_out_raw),
        ).await?;
        
        // Simulate the transaction
        let simulation = self.router.simulate_swap(&quote, &self.keypair).await?;
        
        // Check if simulation was successful
        if simulation.err.is_some() {
            error!("Transaction simulation failed: {:?}", simulation.err);
            return Ok(false);
        }
        
        info!("Transaction simulation successful");
        Ok(true)
    }
    
    async fn execute_sandwich_leg(&self, quote: &QuoteResponse, keypair: &Keypair) -> Result<String> {
        // Execute with high priority for sandwich transactions
        let signature = self.router.swap_with_priority(
            &quote,
            &keypair,
            // Use high priority for sandwich transactions
            Some(solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(
                1000, // Higher priority fee
            )),
        ).await?;
        
        Ok(signature)
    }
    
    fn get_token_mint(&self, token_symbol: &str) -> Result<Pubkey> {
        // This is a simplified version - in a real implementation,
        // you would have a token registry or lookup service
        
        // Map common token symbols to their mint addresses
        let mint_address = match token_symbol {
            "SOL" => "So11111111111111111111111111111111111111112",
            "USDC" => "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            "BONK" => "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
            // Add other tokens as needed
            _ => return Err(anyhow!("Unknown token symbol: {}", token_symbol)),
        };
        
        Pubkey::from_str(mint_address)
            .context(format!("Invalid mint address for token: {}", token_symbol))
    }
    
    async fn to_raw_amount(&self, amount: f64, mint: &Pubkey) -> Result<u64> {
        // Get token info to determine decimals
        let token_info = self.router.get_token_info(mint).await?;
        let decimals = token_info.decimals;
        
        // Convert to raw amount
        let raw_amount = (amount * 10f64.powi(decimals as i32)) as u64;
        
        Ok(raw_amount)
    }
}