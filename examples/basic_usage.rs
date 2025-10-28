use blocks_production_lib::{BlockProductionClient, Result};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    println!("Testing Solana Block Production Library\n");

    // Create client with public RPC configuration
    let client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .public_rpc_config()
        .build()?;

    // Test connection
    println!("Testing RPC connection...");
    match client.test_connection().await {
        Ok(true) => println!("RPC connection successful\n"),
        Ok(false) => {
            println!("RPC connection failed\n");
            return Ok(());
        }
        Err(e) => {
            println!("Connection error: {}\n", e);
            return Ok(());
        }
    }

    // Fetch block production data
    println!("Fetching block production data...");
    let data = client.fetch_block_production().await?;

    // Display overall statistics
    println!("Network Statistics:");
    println!("   Slot range: {} - {} ({} slots)",
        data.slot_range.first_slot, 
        data.slot_range.last_slot, 
        data.slot_range.slot_count()
    );
    println!("   Total validators: {}", data.statistics.total_validators);
    println!("   Significant validators (≥50 slots): {}", data.statistics.significant_validators);
    println!("   Total leader slots: {}", data.statistics.total_leader_slots);
    println!("   Network efficiency: {:.2}%", data.statistics.network_efficiency_percent);
    println!();

    // Skip rate analysis with different weighting methods
    println!("Skip Rate Analysis:");
    println!("   Overall skip rate (all validators): {:.3}%", data.statistics.overall_skip_rate_percent);
    println!("   Significant validators skip rate (≥50 slots): {:.3}%", data.statistics.significant_validators_skip_rate_percent);
    println!("   High-stake validators skip rate (>1000 slots): {:.3}%", data.statistics.high_stake_skip_rate_percent);
    println!("   Weighted skip rate (significance-weighted): {:.3}%", data.statistics.weighted_skip_rate_percent);
    println!("   Simple average (unweighted): {:.3}%", data.statistics.average_skip_rate_percent);
    println!("   Median skip rate: {:.3}%", data.statistics.median_skip_rate_percent);
    println!();

    // Percentile analysis
    println!("Percentile Analysis:");
    println!("   All validators - 90th percentile: {:.3}%", data.statistics.skip_rate_90th_percentile);
    println!("   All validators - 95th percentile: {:.3}%", data.statistics.skip_rate_95th_percentile);
    println!("   Significant validators - 90th percentile: {:.3}%", data.statistics.significant_skip_rate_90th_percentile);
    println!("   Significant validators - 95th percentile: {:.3}%", data.statistics.significant_skip_rate_95th_percentile);
    println!();

    // Validator categorization
    println!("Validator Categories:");
    println!("   Perfect validators (0% skip): {}", data.statistics.perfect_validators);
    println!("   Concerning validators (>5% skip): {}", data.statistics.concerning_validators);
    println!("   Offline validators (100% skip): {}", data.statistics.offline_validators);
    println!("   Low activity validators (<10 slots): {}", data.statistics.low_activity_validators);
    println!("   High activity validators (>1000 slots): {}", data.statistics.high_activity_validators);
    println!();

    // Show high-activity validators - these are the ones that matter most
    println!("🔥 High-Activity Validators Performance (>1000 slots):");
    let high_activity = client.get_high_activity_validators().await?;
    if high_activity.is_empty() {
        println!("   No high-activity validators found.");
    } else {
        println!("   Found {} high-activity validators:", high_activity.len());
        for (i, validator) in high_activity.iter().take(10).enumerate() {
            println!("{}. {}", i + 1, &validator.pubkey[..8]);
            println!("   Leader slots: {}, Skip rate: {:.2}%", 
                validator.leader_slots, validator.skip_rate_percent);
        }
        if high_activity.len() > 10 {
            println!("   ... and {} more high-activity validators", high_activity.len() - 10);
        }
    }
    println!();

    // Show moderate performers - these provide useful insights
    println!("� Moderate Performers (1-5% skip rate) - The Interesting Ones:");
    let moderate = client.get_moderate_performers().await?;
    if moderate.is_empty() {
        println!("   No moderate performers found - all validators are either perfect or concerning.");
    } else {
        println!("   Found {} moderate performers:", moderate.len());
        for (i, validator) in moderate.iter().take(10).enumerate() {
            println!("{}. {}", i + 1, &validator.pubkey[..8]);
            println!("   Leader slots: {}, Skip rate: {:.2}%", 
                validator.leader_slots, validator.skip_rate_percent);
        }
        if moderate.len() > 10 {
            println!("   ... and {} more moderate performers", moderate.len() - 10);
        }
    }
    println!();

    // Show worst percentile - actionable for stake management
    println!("⚠️  Worst Percentile Validators (95th+ percentile, excluding offline):");
    let worst_percentile = client.get_worst_percentile_validators().await?;
    if worst_percentile.is_empty() {
        println!("   No validators in worst percentile (excluding completely offline).");
    } else {
        println!("   Found {} validators performing worse than 95% of network:", worst_percentile.len());
        for (i, validator) in worst_percentile.iter().take(10).enumerate() {
            println!("{}. {}", i + 1, &validator.pubkey[..8]);
            println!("   Leader slots: {}, Skip rate: {:.2}%", 
                validator.leader_slots, validator.skip_rate_percent);
        }
        if worst_percentile.len() > 10 {
            println!("   ... and {} more in worst percentile", worst_percentile.len() - 10);
        }
    }
    println!();

    // Show concerning validators
    println!("⚠️  Validators with Concerning Skip Rates (>5%):");
    let concerning = client.get_concerning_validators().await?;
    if concerning.is_empty() {
        println!("   None! All validators performing well.");
    } else {
        println!("   Found {} concerning validators:", concerning.len());
        for (i, validator) in concerning.iter().take(5).enumerate() {
            println!("   {}. {}: {:.2}% skip rate ({} slots)", 
                i + 1, &validator.pubkey[..8], validator.skip_rate_percent, validator.leader_slots);
        }
        if concerning.len() > 5 {
            println!("   ... and {} more concerning validators", concerning.len() - 5);
        }
    }
    println!();

    // Show offline validators
    println!("💀 Completely Offline Validators (100% skip rate):");
    let offline = client.get_offline_validators().await?;
    if offline.is_empty() {
        println!("   None! No completely offline validators.");
    } else {
        println!("   Found {} completely offline validators", offline.len());
        // Just show count and a few examples, not full list
        for (i, validator) in offline.iter().take(3).enumerate() {
            println!("   {}. {}: {} assigned slots, 0 blocks produced", 
                i + 1, &validator.pubkey[..8], validator.leader_slots);
        }
        if offline.len() > 3 {
            println!("   ... and {} more offline validators", offline.len() - 3);
        }
    }
    println!();

    println!("\n✨ Analysis complete!");
    Ok(())
}