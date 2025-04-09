use thiserror::Error;
use solana_client::client_error::ClientError;
use std::io;

#[derive(Error, Debug)]
pub enum SandoSeerError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("RPC error: {0}")]
    Rpc(#[from] ClientError),

    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("Wallet error: {0}")]
    Wallet(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Monitoring error: {0}")]
    Monitoring(String),

    #[error("Strategy error: {0}")]
    Strategy(String),

    #[error("Market data error: {0}")]
    MarketData(String),

    #[error("RIG API error: {0}")]
    RigApi(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

impl SandoSeerError {
    pub fn config_error(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    pub fn wallet_error(msg: impl Into<String>) -> Self {
        Self::Wallet(msg.into())
    }

    pub fn transaction_error(msg: impl Into<String>) -> Self {
        Self::Transaction(msg.into())
    }

    pub fn monitoring_error(msg: impl Into<String>) -> Self {
        Self::Monitoring(msg.into())
    }

    pub fn strategy_error(msg: impl Into<String>) -> Self {
        Self::Strategy(msg.into())
    }

    pub fn market_data_error(msg: impl Into<String>) -> Self {
        Self::MarketData(msg.into())
    }

    pub fn rig_api_error(msg: impl Into<String>) -> Self {
        Self::RigApi(msg.into())
    }

    pub fn validation_error(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, SandoSeerError>; 