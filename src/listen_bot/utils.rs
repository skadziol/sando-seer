use anyhow::{Result, Context};
use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;
use tracing::error;

/// Converts a token symbol to its mint address
pub fn token_symbol_to_mint(symbol: &str) -> Result<Pubkey> {
    // Common token symbols to their mint addresses mapping
    let mint_address = match symbol.to_uppercase().as_str() {
        "SOL" => "So11111111111111111111111111111111111111112",
        "USDC" => "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
        "BONK" => "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
        "USDT" => "Es9vMFrzaCERmJfrF4H2FYD4KCoNkY11McCe8BenwNYB",
        "RAY" => "4k3Dyjzvzp8eMZWUXbBCjEvwSkkk59S5iCNLY3QrkX6R",
        "SRM" => "SRMuApVNdxXokk5GT7XD5cUUgXMBCoAz2LHeuAoKWRt",
        "MNGO" => "MangoCzJ36AjZyKwVj3VnYU4GTonjfVEnJmvvWaxLac",
        _ => return Err(anyhow::anyhow!("Unknown token symbol: {}", symbol)),
    };
    
    Pubkey::from_str(mint_address)
        .context(format!("Invalid mint address for token: {}", symbol))
}

/// Calculates estimated slippage based on expected vs actual output
pub fn calculate_slippage(expected: Option<u64>, actual: u64) -> f64 {
    match expected {
        Some(expected_amount) if expected_amount > 0 => {
            (expected_amount as f64 - actual as f64).abs() / expected_amount as f64
        }
        _ => 0.01, // Default 1% slippage if we can't calculate
    }
}

/// Formats a wallet address for display (shortens it)
pub fn format_wallet_address(address: &str) -> String {
    if address.len() > 12 {
        format!("{}...{}", &address[0..6], &address[address.len()-6..])
    } else {
        address.to_string()
    }
}

/// Checks if a transaction might be from a known MEV bot
pub fn is_known_mev_bot(wallet_address: &str) -> bool {
    // List of known MEV bot addresses (this is just an example - would need to be updated)
    let known_bots = [
        "JUP2jxvXaqu7NQY1GmNF4m1vodw12LVXYxbFL2uJvfo", // Jupiter aggregator
        "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin", // Serum program
    ];
    
    known_bots.contains(&wallet_address)
}

/// Estimates the potential profitability of a sandwich attack
pub fn estimate_sandwich_profit(amount_in: f64, slippage: f64, pool_depth: Option<f64>) -> f64 {
    let base_profit = amount_in * slippage;
    
    // If we know the pool depth, we can make a more accurate estimate
    if let Some(depth) = pool_depth {
        let impact_factor = (amount_in / depth).min(1.0);
        base_profit * impact_factor * 0.8 // 80% of theoretical maximum to account for gas and risks
    } else {
        // Conservative estimate without pool depth knowledge
        base_profit * 0.5
    }
}