use blocks_production_lib::{BlockProductionClient, Result, init_logging};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging using the library's built-in logging
    init_logging().unwrap();

    println!("🔧 Advanced Configuration Example\n");

    // Example 1: Custom configuration for enterprise use
    println!("1️⃣  Enterprise Configuration:");
    let enterprise_client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .auto_config("https://api.mainnet-beta.solana.com") // Auto-detect optimal settings
        .build()?;

    println!("   Testing enterprise client...");
    match enterprise_client.test_connection().await {
        Ok(true) => println!("   Enterprise client connected"),
        _ => println!("   Enterprise client failed"),
    }

    // Example 2: High-frequency configuration
    println!("\n2️⃣  High-Frequency Configuration:");
    let hf_client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .high_frequency_config()
        .build()?;

    println!("   Testing high-frequency client...");
    match hf_client.test_connection().await {
        Ok(true) => println!("   High-frequency client connected"),
        _ => println!("   High-frequency client failed"),
    }

    // Example 3: Custom headers and timeout
    println!("\n3️⃣  Custom Headers and Timeout:");
    let custom_client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .add_header("User-Agent", "BlockProductionLib/1.0")
        .add_header("X-Custom-Header", "MyApp")
        .timeout(std::time::Duration::from_secs(45))
        .retry_attempts(5)
        .rate_limit(3) // 3 requests per second
        .build()?;

    println!("   Testing custom client...");
    match custom_client.test_connection().await {
        Ok(true) => println!("   Custom client connected"),
        _ => println!("   Custom client failed"),
    }

    // Example 4: Fetch data for specific slot range
    println!("\n4️⃣  Fetching Specific Slot Range:");
    let data = enterprise_client.fetch_block_production().await?;
    let current_range = &data.slot_range;
    
    // Fetch data for the last 1000 slots
    let custom_range_start = current_range.last_slot.saturating_sub(1000);
    println!("   Fetching slots {} to {}", custom_range_start, current_range.last_slot);
    
    match enterprise_client.fetch_block_production_range(custom_range_start, current_range.last_slot).await {
        Ok(range_data) => {
            println!("   Got data for {} validators in custom range", range_data.statistics.total_validators);
            println!("   Skip rate for range: {:.2}%", range_data.statistics.overall_skip_rate_percent);
        }
        Err(e) => println!("   Failed to fetch range data: {}", e),
    }

    // Example 5: Fetch data for specific validators
    println!("\n5️⃣  Fetching Specific Validators:");
    let all_data = enterprise_client.fetch_block_production().await?;
    
    // Get a few validator pubkeys from the data
    let sample_validators: Vec<String> = all_data.validators
        .iter()
        .take(5)
        .map(|v| v.pubkey.clone())
        .collect();

    if !sample_validators.is_empty() {
        println!("   Fetching data for {} specific validators...", sample_validators.len());
        match enterprise_client.fetch_validator_skip_rates(sample_validators.clone()).await {
            Ok(validator_data) => {
                println!("   Got data for {} validators:", validator_data.len());
                for validator in validator_data {
                    println!("     {}: {:.2}% skip rate", 
                        &validator.pubkey[..8], validator.skip_rate_percent);
                }
            }
            Err(e) => println!("   Failed to fetch validator data: {}", e),
        }
    }

    // Example 6: Debug format with raw RPC data
    println!("\n6️⃣  Debug Format Example:");
    let debug_data = enterprise_client.fetch_block_production_debug(
        blocks_production_lib::BlockProductionRequest::default()
    ).await?;

    println!("   Debug data retrieved:");
    println!("     RPC endpoint: {}", debug_data.response_metadata.rpc_endpoint);
    println!("     Response time: {}ms", debug_data.response_metadata.response_time_ms);
    println!("     Raw data keys: {:?}", debug_data.raw_rpc_data.as_object().unwrap().keys().collect::<Vec<_>>());
    println!("     Production validators: {}", debug_data.production_data.validators.len());

    // Example 7: Provider-specific configurations
    println!("\n7️⃣  Provider-Specific Configurations:");
    
    println!("   Helius optimized config:");
    let helius_config = blocks_production_lib::ClientConfig::helius_config().build();
    println!("     Timeout: {:?}", helius_config.timeout);
    println!("     Retry attempts: {}", helius_config.retry_attempts);

    println!("   QuickNode optimized config:");
    let quicknode_config = blocks_production_lib::ClientConfig::quicknode_config().build();
    println!("     Timeout: {:?}", quicknode_config.timeout);
    println!("     Retry attempts: {}", quicknode_config.retry_attempts);

    println!("   Alchemy optimized config:");
    let alchemy_config = blocks_production_lib::ClientConfig::alchemy_config().build();
    println!("     Timeout: {:?}", alchemy_config.timeout);
    println!("     Retry attempts: {}", alchemy_config.retry_attempts);

    println!("\n✨ Advanced configuration examples complete!");
    Ok(())
}