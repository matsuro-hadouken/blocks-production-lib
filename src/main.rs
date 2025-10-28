use blocks_production_lib::{BlockProductionClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 Blocks Production Library Demo\n");
    
    println!("This is a simple demo. For comprehensive examples, run:");
    println!("  cargo run --example basic_usage");
    println!("  cargo run --example advanced_config"); 
    println!("  cargo run --example statistics_analysis");
    println!();

    // Quick test
    let client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .public_rpc_config()
        .build()?;

    println!("🔗 Testing connection...");
    match client.test_connection().await {
        Ok(true) => {
            println!("✅ Connection successful!");
            
            // Quick stats
            let data = client.fetch_block_production().await?;
            println!("📊 Quick stats:");
            println!("   Total validators: {}", data.statistics.total_validators);
            println!("   Overall skip rate: {:.2}%", data.statistics.overall_skip_rate_percent);
        }
        Ok(false) => println!("❌ Connection failed"),
        Err(e) => println!("❌ Error: {}", e),
    }

    println!("\n✨ Demo complete! Run the examples for more details.");
    Ok(())
}
