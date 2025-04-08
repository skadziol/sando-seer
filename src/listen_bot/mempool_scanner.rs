use anyhow::{Result, Context, anyhow};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, error, debug};
use listen_core::listen_engine::{ListenEngine, ListenEngineConfig};
use listen_core::router::dexes::{Dex, DexName};
use listen_core::model::tx::Transaction as ListenTransaction;
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use futures::StreamExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapTransaction {
    pub token_in: String,
    pub token_out: String,
    pub amount_in: f64,
    pub estimated_amount_out: f64,
    pub slippage: f64,
    pub pool_name: String,
    pub wallet_address: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone)]
pub struct MempoolScanner {
    rpc_url: String,
    tx_sender: mpsc::Sender<SwapTransaction>,
}

impl MempoolScanner {
    pub fn new(
        rpc_url: String,
        tx_sender: mpsc::Sender<SwapTransaction>,
    ) -> Self {
        Self {
            rpc_url,
            tx_sender,
        }
    }
    
    pub async fn start_scanning(&self) -> Result<()> {
        info!("Starting mempool scanning using listen-engine...");
        
        // Configure listen-engine
        let listen_config = ListenEngineConfig {
            rpc_url: self.rpc_url.clone(),
            commitment: "confirmed".to_string(),
            ws_url: None,
        };
        
        // Initialize listen-engine
        let listen_engine = ListenEngine::new(listen_config)?;
        
        // Set up DEX stream filters - focusing on major Solana DEXes
        let dexes = vec![
            DexName::Orca,
            DexName::Raydium,
            DexName::Jupiter,
        ];
        
        info!("Starting transaction stream for DEXes: {:?}", dexes);
        
        // Create a stream of swap transactions
        let mut tx_stream = listen_engine.stream_dex_swaps(dexes).await?;
        
        info!("Transaction stream established, monitoring for swaps...");
        
        // Process transactions as they come in
        while let Some(tx) = tx_stream.next().await {
            match self.process_transaction(tx).await {
                Ok(Some(swap_tx)) => {
                    // Send the parsed transaction to the channel
                    if let Err(e) = self.tx_sender.send(swap_tx).await {
                        error!("Failed to send transaction to channel: {}", e);
                    }
                },
                Ok(None) => {
                    // Transaction wasn't relevant, skip
                    continue;
                },
                Err(e) => {
                    error!("Failed to process transaction: {}", e);
                }
            }
        }
        
        Err(anyhow!("Transaction stream ended unexpectedly"))
    }
    
    async fn process_transaction(&self, tx: ListenTransaction) -> Result<Option<SwapTransaction>> {
        // Extract the relevant swap data from the listen transaction
        
        // Check if it's a swap transaction
        let swap_info = match tx.swap_info {
            Some(info) => info,
            None => return Ok(None), // Not a swap transaction
        };
        
        // Extract the token information
        let token_in = swap_info.token_in.symbol.unwrap_or_else(|| swap_info.token_in.mint.to_string());
        let token_out = swap_info.token_out.symbol.unwrap_or_else(|| swap_info.token_out.mint.to_string());
        
        // Extract the amounts
        let amount_in = swap_info.amount_in as f64 / 10f64.powi(swap_info.token_in.decimals as i32);
        let amount_out = swap_info.amount_out as f64 / 10f64.powi(swap_info.token_out.decimals as i32);
        
        // Estimate slippage based on expected vs actual (simplified)
        let slippage = if let Some(expected_out) = swap_info.expected_out {
            let expected_amount = expected_out as f64 / 10f64.powi(swap_info.token_out.decimals as i32);
            if expected_amount > 0.0 {
                (expected_amount - amount_out).abs() / expected_amount
            } else {
                0.01 // Default slippage if we can't calculate
            }
        } else {
            0.01 // Default slippage
        };
        
        // Extract wallet address
        let wallet_address = tx.signer.to_string();
        
        // Determine the pool name from the DEX info
        let pool_name = match tx.dex {
            Some(dex) => dex.to_string(),
            None => "Unknown".to_string(),
        };
        
        // Create the SwapTransaction
        let swap_tx = SwapTransaction {
            token_in,
            token_out,
            amount_in,
            estimated_amount_out: amount_out,
            slippage,
            pool_name,
            wallet_address,
            timestamp: chrono::Utc::now().timestamp(),
        };
        
        debug!("Processed swap transaction: {:?}", swap_tx);
        
        Ok(Some(swap_tx))
    }
}