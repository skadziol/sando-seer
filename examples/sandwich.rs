use anyhow::Result;
use sandoseer::config;
use sandoseer::listen_bot::{SwapTransaction, TransactionExecutor, TradeDecision};

#[tokio::main]
async fn main() -> Result<()> {
    // Load configuration
    dotenv::dotenv().ok();
    let config = config::load_config().await?;
    
    // Initialize the transaction executor in simulation mode
    let transaction_executor = TransactionExecutor::new(&config.rpc_url, true)?;
    
    println!("Sandwich MEV Example");
    println!("-------------------");
    
    // 1. Detect a large swap transaction
    let victim_tx = SwapTransaction {
        token_in: "USDC".to_string(),
        token_out: "SOL".to_string(),
        amount_in: 50000.0, // $50,000 USDC
        estimated_amount_out: 1400.0, // ~1400 SOL
        slippage: 0.05,
        pool_name: "Orca".to_string(),
        wallet_address: "9rgeN6mbhCVbnZPpMBg2QCFhYJnuRyGrqnKULNbreAha".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };
    
    println!("Detected victim transaction:");
    println!("  {} USDC -> SOL with {}% slippage", victim_tx.amount_in, victim_tx.slippage * 100.0);
    println!("  Pool: {}", victim_tx.pool_name);
    
    // 2. Execute front-running transaction
    let front_run = TradeDecision {
        token_in: "SOL".to_string(),
        token_out: "USDC".to_string(),
        amount_in: 50.0, // 50 SOL
        expected_min_out: 1750.0, // Expect at least $1,750 USDC
        confidence_score: 0.9,
        risk_level: 2,
        strategy: "sandwich_front".to_string(),
    };
    
    println!("\nExecuting front-run transaction:");
    println!("  {} SOL -> USDC", front_run.amount_in);
    
    let front_sig = match transaction_executor.execute_trade(front_run).await {
        Ok(sig) => {
            println!("  Front-run successful! Signature: {}", sig);
            sig
        },
        Err(e) => {
            println!("  Front-run failed: {}", e);
            return Ok(());
        }
    };
    
    // 3. Wait for victim transaction (in real scenario)
    println!("\nWaiting for victim transaction to execute...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    
    // 4. Execute back-running transaction
    let back_run = TradeDecision {
        token_in: "USDC".to_string(),
        token_out: "SOL".to_string(),
        amount_in: 1800.0, // $1,800 USDC (slightly more than we got from front-run)
        expected_min_out: 48.0, // Expect at least 48 SOL
        confidence_score: 0.9,
        risk_level: 2,
        strategy: "sandwich_back".to_string(),
    };
    
    println("\nExecuting back-run transaction:");
    println!("  {} USDC -> SOL", back_run.amount_in);
    
    match transaction_executor.execute_trade(back_run).await {
        Ok(sig) => {
            println!("  Back-run successful! Signature: {}", sig);
            println!("\nSandwich complete!");
            println!("  Started with: 50 SOL");
            println!("  Ended with: ~52 SOL");
            println!("  Profit: ~2 SOL");
        },
        Err(e) => {
            println!("  Back-run failed: {}", e);
        }
    };
    
    Ok(())
}