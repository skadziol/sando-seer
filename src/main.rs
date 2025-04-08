mod config;
mod listen_bot;
mod rig_agent;
mod evaluator;
mod monitoring;

use anyhow::{Result, Context};
use clap::{Parser, Subcommand};
use dotenv::dotenv;
use tokio::sync::mpsc;
use tracing::{info, error, Level};
use tracing_subscriber::FmtSubscriber;

use listen_bot::{MempoolScanner, SwapTransaction, TransactionExecutor};
use rig_agent::{RigAgent, SentimentAnalyzer, MarketDataCollector};
use evaluator::{OpportunityScorer, RiskAnalyzer, DecisionMaker};
use monitoring::{Logger, TelegramNotifier};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start monitoring for MEV opportunities
    Start {
        /// Run in simulation mode (no real transactions)
        #[arg(long)]
        sim: bool,
    },
    /// Initialize wallet and configuration
    Init,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenv().ok();
    
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)
        .context("setting default subscriber failed")?;
    
    info!("Starting SandoSeer - AI MEV Oracle");
    
    let cli = Cli::parse();
    
    match &cli.command {
        Some(Commands::Start { sim }) => {
            info!("Starting MEV monitoring...");
            if *sim {
                info!("Running in SIMULATION mode - no real transactions will be executed");
            }
            
            // Start the main application loop
            run_sandoseer(*sim).await?;
        }
        Some(Commands::Init) => {
            info!("Initializing SandoSeer configuration...");
            config::initialize_config().await?;
        }
        None => {
            info!("No command specified. Use --help for available commands.");
        }
    }
    
    Ok(())
}

async fn run_sandoseer(simulation_mode: bool) -> Result<()> {
    info!("Loading configuration...");
    let config = config::load_config().await?;
    
    // Create channels for communication between components
    let (tx_sender, mut tx_receiver) = mpsc::channel::<SwapTransaction>(100);
    
    // Initialize components
    let mempool_scanner = MempoolScanner::new(config.rpc_url.clone(), tx_sender);
    let transaction_executor = TransactionExecutor::new(&config.rpc_url, simulation_mode)?;
    let rig_agent = RigAgent::new(config.rig_api_key.clone());
    let sentiment_analyzer = SentimentAnalyzer::new();
    let market_data_collector = MarketDataCollector::new();
    let opportunity_scorer = OpportunityScorer::new(config.min_opportunity_score, config.max_risk_level);
    let risk_analyzer = RiskAnalyzer::new(config.max_risk_level);
    let decision_maker = DecisionMaker::new(0.5); // Minimum 0.5 SOL profitability
    let logger = Logger::new(None)?;
    let telegram = TelegramNotifier::new(
        config.telegram_bot_token.clone(),
        config.telegram_chat_id.clone(),
    );
    
    // Start the mempool scanner
    tokio::spawn(async move {
        if let Err(e) = mempool_scanner.start_scanning().await {
            error!("Mempool scanner error: {}", e);
        }
    });
    
    info!("SandoSeer is running. Monitoring for MEV opportunities...");
    
    // Main processing loop
    while let Some(transaction) = tx_receiver.recv().await {
        info!("Received transaction: {:?}", transaction);
        
        // 1. Gather market data
        let tokens = vec![
            transaction.token_in.clone(),
            transaction.token_out.clone(),
        ];
        let market_data = match market_data_collector.get_market_data(&tokens).await {
            Ok(data) => Some(data),
            Err(e) => {
                error!("Failed to get market data: {}", e);
                None
            }
        };
        
        // 2. Get sentiment data
        let sentiment = match sentiment_analyzer.get_token_sentiment(&transaction.token_out).await {
            Ok(data) => Some(data),
            Err(e) => {
                error!("Failed to get sentiment data: {}", e);
                None
            }
        };
        
        // 3. Get AI evaluation
        let agent_decision = match rig_agent.evaluate_opportunity(
            &transaction,
            market_data.as_ref().map(|d| serde_json::to_string(d).unwrap_or_default()),
            sentiment.as_ref().map(|s| serde_json::to_string(s).unwrap_or_default()),
        ).await {
            Ok(decision) => decision,
            Err(e) => {
                error!("Failed to get agent decision: {}", e);
                continue;
            }
        };
        
        // 4. Score the opportunity
        let opportunity_score = opportunity_scorer.calculate_score(
            &transaction,
            &agent_decision,
            market_data.as_ref().map(|d| serde_json::to_string(d).unwrap_or_default()),
        );
        
        // 5. Make the final decision
        if !opportunity_scorer.should_execute(&opportunity_score) {
            info!("Opportunity score too low, skipping");
            continue;
        }
        
        let trade_decision = match decision_maker.make_decision(
            &transaction,
            &opportunity_score,
            &agent_decision,
        ) {
            Some(decision) => decision,
            None => {
                info!("Decision maker chose not to execute trade");
                continue;
            }
        };
        
        // 6. Execute the trade
        info!("Executing trade: {:?}", trade_decision);
        
        // Notify about detected opportunity
        if let Err(e) = telegram.notify_opportunity_detected(&trade_decision).await {
            error!("Failed to send Telegram notification: {}", e);
        }
        
        // Simulate the transaction first to check if it would succeed
        match transaction_executor.simulate_transaction(&trade_decision).await {
            Ok(true) => {
                info!("Transaction simulation successful, proceeding with execution");
            }
            Ok(false) => {
                info!("Transaction simulation failed, skipping execution");
                continue;
            }
            Err(e) => {
                error!("Transaction simulation error: {}", e);
                continue;
            }
        }
        
        // Actually execute the trade
        match transaction_executor.execute_trade(trade_decision.clone()).await {
            Ok(tx_signature) => {
                info!("Trade executed successfully! Signature: {}", tx_signature);
                
                // Log the successful trade
                let trade_log = monitoring::logger::TradeLog {
                    timestamp: chrono::Utc::now(),
                    token_in: trade_decision.token_in.clone(),
                    token_out: trade_decision.token_out.clone(),
                    amount_in: trade_decision.amount_in,
                    amount_out: Some(trade_decision.expected_min_out),
                    strategy: trade_decision.strategy.clone(),
                    tx_signature: Some(tx_signature.clone()),
                    success: true,
                    profit: None, // We don't know the actual profit yet
                    notes: Some(format!("Confidence: {}", trade_decision.confidence_score)),
                };
                
                if let Err(e) = logger.log_trade(trade_log).await {
                    error!("Failed to log trade: {}", e);
                }
                
                // Send success notification
                if let Err(e) = telegram.notify_trade_execution(&trade_decision, &tx_signature).await {
                    error!("Failed to send Telegram notification: {}", e);
                }
            }
            Err(e) => {
                error!("Trade execution failed: {}", e);
                
                // Log the failed trade
                let trade_log = monitoring::logger::TradeLog {
                    timestamp: chrono::Utc::now(),
                    token_in: trade_decision.token_in.clone(),
                    token_out: trade_decision.token_out.clone(),
                    amount_in: trade_decision.amount_in,
                    amount_out: None,
                    strategy: trade_decision.strategy,
                    tx_signature: None,
                    success: false,
                    profit: None,
                    notes: Some(format!("Error: {}", e)),
                };
                
                if let Err(e) = logger.log_trade(trade_log).await {
                    error!("Failed to log trade: {}", e);
                }
            }
        }
        
        // Add a small delay to avoid overwhelming resources
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    }
    
    Ok(())
}