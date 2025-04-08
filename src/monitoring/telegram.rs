use anyhow::{Result, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::{info, error};

use crate::listen_bot::TradeDecision;

pub struct TelegramNotifier {
    bot_token: Option<String>,
    chat_id: Option<String>,
    client: Client,
}

impl TelegramNotifier {
    pub fn new(bot_token: Option<String>, chat_id: Option<String>) -> Self {
        Self {
            bot_token,
            chat_id,
            client: Client::new(),
        }
    }
    
    pub async fn send_notification(&self, message: &str) -> Result<()> {
        // Check if Telegram integration is configured
        let (bot_token, chat_id) = match (&self.bot_token, &self.chat_id) {
            (Some(token), Some(chat)) => (token, chat),
            _ => {
                info!("Telegram notification skipped: Bot token or chat ID not configured");
                return Ok(());
            }
        };
        
        info!("Sending Telegram notification");
        
        // Construct the Telegram API URL
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            bot_token
        );
        
        // Send the request
        let response = self.client
            .post(&url)
            .form(&[
                ("chat_id", chat_id.as_str()),
                ("text", message),
                ("parse_mode", "HTML"),
            ])
            .send()
            .await?;
        
        // Check if the request was successful
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Failed to send Telegram notification: {}", error_text);
            return Err(anyhow::anyhow!("Telegram API error: {}", error_text));
        }
        
        info!("Telegram notification sent successfully");
        Ok(())
    }
    
    pub async fn notify_trade_execution(&self, decision: &TradeDecision, tx_signature: &str) -> Result<()> {
        let message = format!(
            "<b>🚀 SandoSeer Trade Executed</b>\n\n\
            Strategy: <b>{}</b>\n\
            Token Pair: <b>{} → {}</b>\n\
            Amount: <b>{} {}</b>\n\
            Confidence: <b>{}%</b>\n\
            Risk Level: <b>{}/3</b>\n\
            TX: <code>{}</code>",
            decision.strategy,
            decision.token_in,
            decision.token_out,
            decision.amount_in,
            decision.token_in,
            (decision.confidence_score * 100.0) as u8,
            decision.risk_level,
            tx_signature
        );
        
        self.send_notification(&message).await
    }
    
    pub async fn notify_opportunity_detected(&self, decision: &TradeDecision) -> Result<()> {
        let message = format!(
            "<b>👀 SandoSeer Opportunity Detected</b>\n\n\
            Strategy: <b>{}</b>\n\
            Token Pair: <b>{} → {}</b>\n\
            Amount: <b>{} {}</b>\n\
            Confidence: <b>{}%</b>\n\
            Risk Level: <b>{}/3</b>",
            decision.strategy,
            decision.token_in,
            decision.token_out,
            decision.amount_in,
            decision.token_in,
            (decision.confidence_score * 100.0) as u8,
            decision.risk_level
        );
        
        self.send_notification(&message).await
    }
}