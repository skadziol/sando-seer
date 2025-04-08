use anyhow::Result;
use sandoseer::config;
use sandoseer::listen_bot::{SwapTransaction, TransactionExecutor};
use sandoseer::rig_agent::{RigAgent, SentimentAnalyzer, MarketDataCollector};
use sandoseer::evaluator::{OpportunityScorer, DecisionMaker};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    dotenv::dotenv().ok();
    let config = config::load_config().await?;
    
    // Initialize components
    let transaction_executor = TransactionExecutor::new(&config.rpc_url, true)?; // Simulation mode
    let rig_agent = RigAgent::new(config.rig_api_key.clone());
    let sentiment_analyzer = SentimentAnalyzer::new();
    let market_data_collector = MarketDataCollector::new();
    let opportunity_scorer = OpportunityScorer::new(0.7, 2); // 0.7 min score, max risk level 2
    let decision_maker = DecisionMaker::new(0.1); // Minimum 0.1 SOL profitability
    
    // Simulate an incoming transaction
    let transaction = SwapTransaction {
        token_in: "SOL".to_string(),
        token_out: "USDC".to_string(),
        amount_in: 20.0,
        estimated_amount_out: 700.0,
        slippage: 0.02,
        pool_name: "Orca".to_string(),
        wallet_address: "8JUjWjAyXTMB4ZXcV7nk9myvZ1HuZvxV7L6hx9ZYbFcz".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };
    
    println!("Analyzing transaction: {:?}", transaction);
    
    // 1. Gather market data
    let tokens = vec![
        transaction.token_in.clone(),
        transaction.token_out.clone(),
    ];
    let market_data = market_data_collector.get_market_data(&tokens).await?;
    println!("Market data: {:?}", market_data);
    
    // 2. Get sentiment data
    let sentiment = sentiment_analyzer.get_token_sentiment(&transaction.token_out).await?;
    println!("Sentiment data: {:?}", sentiment);
    
    // 3. Get AI evaluation
    let agent_decision = rig_agent.evaluate_opportunity(
        &transaction,
        Some(serde_json::to_string(&market_data)?),
        Some(serde_json::to_string(&sentiment)?),
    ).await?;
    println!("Agent decision: {:?}", agent_decision);
    
    // 4. Score the opportunity
    let opportunity_score = opportunity_scorer.calculate_score(
        &transaction,
        &agent_decision,
        Some(serde_json::to_string(&market_data)?),
    );
    println!("Opportunity score: {:?}", opportunity_score);
    
    // 5. Make the final decision
    if !opportunity_scorer.should_execute(&opportunity_score) {
        println!("Opportunity score too low, skipping");
        return Ok(());
    }
    
    let trade_decision = match decision_maker.make_decision(
        &transaction,
        &opportunity_score,
        &agent_decision,
    ) {
        Some(decision) => decision,
        None => {
            println!("Decision maker chose not to execute trade");
            return Ok(());
        }
    };
    println!("Trade decision: {:?}", trade_decision);
    
    // 6. Execute the trade
    println!("Executing trade...");
    
    // Simulate the transaction first
    match transaction_executor.simulate_transaction(&trade_decision).await {
        Ok(true) => {
            println!("Transaction simulation successful, proceeding with execution");
        }
        Ok(false) => {
            println!("Transaction simulation failed, skipping execution");
            return Ok(());
        }
        Err(e) => {
            println!("Transaction simulation error: {}", e);
            return Ok(());
        }
    }
    
    // Actually execute the trade
    match transaction_executor.execute_trade(trade_decision.clone()).await {
        Ok(tx_signature) => {
            println!("Trade executed successfully! Signature: {}", tx_signature);
        }
        Err(e) => {
            println!("Trade execution failed: {}", e);
        }
    }
    
    Ok(())
}