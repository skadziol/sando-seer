use anyhow::{Result, Context};
use serde::{Deserialize, Serialize};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{Keypair, read_keypair_file, Signer};
use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::{info, warn};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub rpc_url: String,
    pub ws_url: Option<String>,
    pub wallet_path: PathBuf,
    pub rig_api_key: Option<String>,
    pub telegram_bot_token: Option<String>,
    pub telegram_chat_id: Option<String>,
    
    // MEV strategy settings
    pub min_opportunity_score: f64,
    pub max_risk_level: u8,
    pub target_tokens: Vec<String>,
    pub min_profit_threshold: f64,
    pub max_risk_threshold: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.mainnet-beta.solana.com".to_string(),
            ws_url: None,
            wallet_path: PathBuf::from("~/.config/solana/id.json"),
            rig_api_key: None,
            telegram_bot_token: None,
            telegram_chat_id: None,
            min_opportunity_score: 0.8,
            max_risk_level: 2, // 0-3 scale where 3 is highest risk
            target_tokens: vec![
                "SOL".to_string(), 
                "USDC".to_string(),
                "BONK".to_string(),
            ],
            min_profit_threshold: 0.01, // 1% minimum profit
            max_risk_threshold: 0.5,    // 50% maximum risk
        }
    }
}

pub async fn load_config() -> Result<Config> {
    let mut config = Config::default();
    
    // Override defaults with environment variables
    if let Ok(rpc_url) = env::var("SOLANA_RPC_URL") {
        config.rpc_url = rpc_url;
    }
    
    if let Ok(wallet_path) = env::var("WALLET_PATH") {
        config.wallet_path = PathBuf::from(wallet_path);
    }
    
    if let Ok(rig_api_key) = env::var("RIG_API_KEY") {
        config.rig_api_key = Some(rig_api_key);
    }
    
    if let Ok(telegram_bot_token) = env::var("TELEGRAM_BOT_TOKEN") {
        config.telegram_bot_token = Some(telegram_bot_token);
    }
    
    if let Ok(telegram_chat_id) = env::var("TELEGRAM_CHAT_ID") {
        config.telegram_chat_id = Some(telegram_chat_id);
    }
    
    // Additional configuration loading logic can be added here
    
    Ok(config)
}

pub async fn initialize_config() -> Result<()> {
    info!("Initializing configuration...");
    
    // Check if we can connect to the Solana RPC
    let rpc_url = env::var("SOLANA_RPC_URL")
        .unwrap_or_else(|_| "https://api.mainnet-beta.solana.com".to_string());
    
    info!("Connecting to Solana RPC at: {}", rpc_url);
    let client = solana_client::rpc_client::RpcClient::new_with_commitment(
        rpc_url,
        CommitmentConfig::confirmed(),
    );
    
    match client.get_version() {
        Ok(version) => info!("Connected to Solana node version: {}", version.solana_core),
        Err(e) => warn!("Could not connect to Solana RPC: {}", e),
    }
    
    // Check the wallet
    let wallet_path = env::var("WALLET_PATH")
        .unwrap_or_else(|_| "~/.config/solana/id.json".to_string());
    
    let expanded_path = shellexpand::tilde(&wallet_path).to_string();
    
    match read_keypair_file(&expanded_path) {
        Ok(keypair) => {
            let pubkey = keypair.pubkey();
            info!("Using wallet: {}", pubkey);
            
            // Try to get the balance
            match client.get_balance(&pubkey) {
                Ok(balance) => {
                    let sol_balance = balance as f64 / 1_000_000_000.0;
                    info!("Wallet balance: {} SOL", sol_balance);
                    
                    if sol_balance < 0.1 {
                        warn!("Wallet balance is low. You might need more SOL for transactions.");
                    }
                },
                Err(e) => warn!("Could not get wallet balance: {}", e),
            }
        },
        Err(e) => warn!("Could not read wallet at {}: {}", expanded_path, e),
    }
    
    info!("Configuration initialized successfully!");
    Ok(())
}

pub fn get_rpc_url() -> Result<String> {
    env::var("SOLANA_RPC_URL")
        .context("SOLANA_RPC_URL environment variable not set")
}

pub fn get_keypair() -> Result<Keypair> {
    let wallet_path = env::var("WALLET_PATH")
        .context("WALLET_PATH environment variable not set")?;
    
    let expanded_path = shellexpand::tilde(&wallet_path).to_string();
    
    read_keypair_file(&expanded_path)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair from {}: {}", expanded_path, e))
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        
        if !config_path.exists() {
            info!("Config file not found, creating default config at {:?}", config_path);
            let default_config = Config::default();
            default_config.save()?;
            return Ok(default_config);
        }
        
        let config_str = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file at {:?}", config_path))?;
            
        let config: Config = serde_json::from_str(&config_str)
            .with_context(|| "Failed to parse config file")?;
            
        Ok(config)
    }
    
    pub fn save(&self) -> Result<()> {
        let config_path = Self::get_config_path()?;
        
        // Ensure the directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory at {:?}", parent))?;
        }
        
        let config_str = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize config")?;
            
        fs::write(&config_path, config_str)
            .with_context(|| format!("Failed to write config file to {:?}", config_path))?;
            
        Ok(())
    }
    
    fn get_config_path() -> Result<PathBuf> {
        let home_dir = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .with_context(|| "Failed to determine home directory")?;
            
        Ok(PathBuf::from(home_dir).join(".config").join("sandoseer").join("config.json"))
    }
}