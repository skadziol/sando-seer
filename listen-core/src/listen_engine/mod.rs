use anyhow::Result;
use futures::Stream;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_client::nonblocking::pubsub_client::PubsubClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use futures::stream::{self, StreamExt};
use std::pin::Pin;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};
use crate::{
    model::tx::Transaction,
    router::dexes::DexName,
};

#[derive(Debug, Clone)]
pub struct ListenEngineConfig {
    pub rpc_url: String,
    pub commitment: String,
    pub ws_url: Option<String>,
}

impl Default for ListenEngineConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            commitment: "confirmed".to_string(),
            ws_url: None,
        }
    }
}

pub struct ListenEngine {
    config: ListenEngineConfig,
    client: Arc<RpcClient>,
}

impl ListenEngine {
    pub fn new(config: ListenEngineConfig) -> Result<Self> {
        let client = Arc::new(RpcClient::new(config.rpc_url.clone()));
        Ok(Self { config, client })
    }

    pub async fn stream_dex_swaps(
        &self,
        dexes: Vec<DexName>,
    ) -> Result<Pin<Box<dyn Stream<Item = Transaction> + Send + '_>>> {
        info!("Starting DEX swap monitoring for DEXes: {:?}", dexes);
        
        // Get websocket URL, fallback to default if not provided
        let ws_url = self.config.ws_url.clone()
            .unwrap_or_else(|| "wss://api.mainnet-beta.solana.com".to_string());
        info!("Connecting to Solana websocket at: {}", ws_url);

        // Create a stream that processes transactions
        let client = Arc::clone(&self.client);
        let dexes = Arc::new(dexes);

        let stream = stream::unfold(
            (ws_url, client, dexes),
            |(ws_url, client, dexes)| async move {
                // Create new PubsubClient for each iteration
                match PubsubClient::new(&ws_url).await {
                    Ok(pubsub_client) => {
                        info!("Successfully connected to Solana websocket");
                        match pubsub_client.slot_subscribe().await {
                            Ok((mut notifications, _unsubscribe)) => {
                                info!("Successfully subscribed to slot notifications");
                                if let Some(notification) = notifications.next().await {
                                    let slot = notification.slot;
                                    info!("Received new slot: {}", slot);
                                    
                                    // For now, just create a placeholder transaction
                                    // In a real implementation, we would:
                                    // 1. Get transactions in the slot
                                    // 2. Filter for DEX transactions
                                    // 3. Parse swap information
                                    let transaction = Transaction {
                                        signature: "placeholder".to_string(),
                                        signer: solana_sdk::pubkey::new_rand(), // Placeholder
                                        dex: Some(DexName::Jupiter),
                                        swap_info: None,
                                        block_time: Some(chrono::Utc::now().timestamp()),
                                        slot: Some(slot),
                                    };
                                    
                                    info!("Processed transaction in slot {}: {:?}", slot, transaction);
                                    Some((transaction, (ws_url, client, dexes)))
                                } else {
                                    warn!("Slot notification stream ended, attempting to reconnect...");
                                    sleep(Duration::from_secs(1)).await;
                                    Some((
                                        Transaction {
                                            signature: "reconnecting".to_string(),
                                            signer: solana_sdk::pubkey::new_rand(),
                                            dex: None,
                                            swap_info: None,
                                            block_time: Some(chrono::Utc::now().timestamp()),
                                            slot: None,
                                        },
                                        (ws_url, client, dexes),
                                    ))
                                }
                            }
                            Err(e) => {
                                error!("Failed to subscribe to slot notifications: {:?}", e);
                                warn!("Attempting to reconnect after subscription error...");
                                sleep(Duration::from_secs(1)).await;
                                Some((
                                    Transaction {
                                        signature: "subscription_error".to_string(),
                                        signer: solana_sdk::pubkey::new_rand(),
                                        dex: None,
                                        swap_info: None,
                                        block_time: Some(chrono::Utc::now().timestamp()),
                                        slot: None,
                                    },
                                    (ws_url, client, dexes),
                                ))
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to connect to Solana websocket: {:?}", e);
                        warn!("Attempting to reconnect after connection error...");
                        sleep(Duration::from_secs(1)).await;
                        Some((
                            Transaction {
                                signature: "connection_error".to_string(),
                                signer: solana_sdk::pubkey::new_rand(),
                                dex: None,
                                swap_info: None,
                                block_time: Some(chrono::Utc::now().timestamp()),
                                slot: None,
                            },
                            (ws_url, client, dexes),
                        ))
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }

    pub async fn get_transaction_details(&self, signature_str: &str) -> Result<Transaction> {
        info!("Fetching transaction details for signature: {}", signature_str);
        let signature = Signature::from_str(signature_str)?;
        
        let tx = self.client.get_transaction_with_config(
            &signature,
            RpcTransactionConfig {
                commitment: Some(CommitmentConfig::from_str(&self.config.commitment)?),
                encoding: None,
                max_supported_transaction_version: None,
            },
        )?;

        info!("Successfully fetched transaction details for signature: {}", signature_str);
        Ok(Transaction {
            signature: signature_str.to_string(),
            signer: solana_sdk::pubkey::new_rand(), // Placeholder
            dex: None,
            swap_info: None,
            block_time: tx.block_time,
            slot: Some(tx.slot),
        })
    }
}
