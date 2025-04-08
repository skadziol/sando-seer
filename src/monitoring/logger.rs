use anyhow::{Result, Context};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use tracing::{info, error};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeLog {
    pub timestamp: DateTime<Utc>,
    pub token_in: String,
    pub token_out: String,
    pub amount_in: f64,
    pub amount_out: Option<f64>,
    pub strategy: String,
    pub tx_signature: Option<String>,
    pub success: bool,
    pub profit: Option<f64>,
    pub notes: Option<String>,
}

pub struct Logger {
    log_path: PathBuf,
}

impl Logger {
    pub fn new(log_dir: Option<&str>) -> Result<Self> {
        let log_dir = log_dir.unwrap_or("./logs");
        
        // Ensure the log directory exists
        std::fs::create_dir_all(log_dir)?;
        
        let log_path = PathBuf::from(log_dir).join("trades.json");
        
        Ok(Self {
            log_path,
        })
    }
    
    pub async fn log_trade(&self, log_entry: TradeLog) -> Result<()> {
        info!("Logging trade: {:?}", log_entry);
        
        // Convert the log entry to JSON
        let json = serde_json::to_string_pretty(&log_entry)?;
        
        // Check if the file exists
        let file_exists = self.log_path.exists();
        
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .append(true)
            .open(&self.log_path)?;
        
        // If the file is new, start a JSON array
        if !file_exists {
            file.write_all(b"[\n")?;
        } else {
            // If the file exists, add a comma after the last entry
            file.write_all(b",\n")?;
        }
        
        // Write the new entry
        file.write_all(json.as_bytes())?;
        
        // We don't close the array bracket here because we want to be able to append later
        // In a real application, you'd have a proper JSON array handling mechanism
        
        Ok(())
    }
    
    pub async fn get_trade_history(&self) -> Result<Vec<TradeLog>> {
        if !self.log_path.exists() {
            return Ok(Vec::new());
        }
        
        // Read the file
        let content = std::fs::read_to_string(&self.log_path)?;
        
        // Fix the JSON if needed (add closing bracket)
        let fixed_content = if !content.trim().ends_with("]") {
            format!("{}]", content)
        } else {
            content
        };
        
        // Parse the JSON
        let logs: Vec<TradeLog> = serde_json::from_str(&fixed_content)?;
        
        Ok(logs)
    }
}