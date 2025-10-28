use blocks_production_lib::{BlockProductionClient, Result, init_logging};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging using the library's built-in logging
    init_logging().unwrap();

    println!("Comprehensive Statistics Analysis\n");

    // Create client
    let client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .public_rpc_config()
        .build()?;

    // Fetch data
    println!("Fetching block production data...");
    let data = client.fetch_block_production().await?;
    let stats = &data.statistics;

    println!("Data fetched successfully!\n");

    // Comprehensive statistics breakdown
    println!("COMPREHENSIVE STATISTICS ANALYSIS");
    println!("=====================================\n");

    // Basic metrics
    println!("Basic Metrics:");
    println!("   Total Validators: {}", stats.total_validators);
    println!("   Total Leader Slots: {}", stats.total_leader_slots);
    println!("   Total Blocks Produced: {}", stats.total_blocks_produced);
    println!("   Total Missed Slots: {}", stats.total_missed_slots);
    println!("   Slot Range: {} ({} slots)", 
        data.slot_range.slot_count(), data.slot_range.slot_count());
    println!();

    // Skip rate analysis
    println!("Skip Rate Analysis:");
    println!("   Overall Skip Rate: {:.4}%", stats.overall_skip_rate_percent);
    println!("   Average Skip Rate: {:.4}%", stats.average_skip_rate_percent);
    println!("   Median Skip Rate: {:.4}%", stats.median_skip_rate_percent);
    println!("   Weighted Skip Rate: {:.4}%", stats.weighted_skip_rate_percent);
    println!("   Significant Skip Rate: {:.4}%", stats.significant_validators_skip_rate_percent);
    println!();

    // Performance distribution
    println!("Performance Distribution:");
    println!("   Perfect Validators (0% skip): {} ({:.1}%)", 
        stats.perfect_validators,
        (stats.perfect_validators as f64 / stats.total_validators as f64) * 100.0
    );
    println!("   Concerning Validators (>5% skip): {} ({:.1}%)", 
        stats.concerning_validators,
        (stats.concerning_validators as f64 / stats.total_validators as f64) * 100.0
    );
    
    let good_validators = stats.total_validators - stats.perfect_validators - stats.concerning_validators;
    println!("   Good Validators (0-5% skip): {} ({:.1}%)", 
        good_validators,
        (good_validators as f64 / stats.total_validators as f64) * 100.0
    );
    println!();

    // Network health indicators
    println!("Network Health Indicators:");
    let network_efficiency = (stats.total_blocks_produced as f64 / stats.total_leader_slots as f64) * 100.0;
    println!("   Network Efficiency: {:.2}%", network_efficiency);
    
    let avg_slots_per_validator = stats.total_leader_slots as f64 / stats.total_validators as f64;
    println!("   Average Slots per Validator: {:.1}", avg_slots_per_validator);
    
    println!("   Data Freshness: {}", data.fetched_at.format("%Y-%m-%d %H:%M:%S UTC"));
    println!();

    // Detailed skip rate distribution
    println!("Skip Rate Distribution:");
    let mut skip_rate_buckets = HashMap::new();
    
    for validator in &data.validators {
        let bucket = match validator.skip_rate_percent {
            rate if rate == 0.0 => "0.0% (Perfect)",
            rate if rate <= 1.0 => "0.1-1.0%",
            rate if rate <= 2.0 => "1.1-2.0%",
            rate if rate <= 5.0 => "2.1-5.0%",
            rate if rate <= 10.0 => "5.1-10.0%",
            rate if rate <= 20.0 => "10.1-20.0%",
            rate if rate <= 50.0 => "20.1-50.0%",
            _ => ">50.0%",
        };
        
        *skip_rate_buckets.entry(bucket).or_insert(0) += 1;
    }

    let bucket_order = [
        "0.0% (Perfect)", "0.1-1.0%", "1.1-2.0%", "2.1-5.0%", 
        "5.1-10.0%", "10.1-20.0%", "20.1-50.0%", ">50.0%"
    ];

    for bucket in bucket_order.iter() {
        if let Some(&count) = skip_rate_buckets.get(bucket) {
            let percentage = (count as f64 / stats.total_validators as f64) * 100.0;
            println!("   {:<15}: {:4} validators ({:5.1}%)", bucket, count, percentage);
        }
    }
    println!();

    // Performance analysis
    println!("Performance Analysis:");
    
    println!("   Perfect Performers:");
    let perfect = client.get_perfect_validators().await?;
    for (i, validator) in perfect.iter().take(5).enumerate() {
        println!("     {}. {}: {:.4}% ({}/{} slots)", 
            i + 1, 
            &validator.pubkey[..8], 
            validator.skip_rate_percent,
            validator.blocks_produced,
            validator.leader_slots
        );
    }

    println!("   Concerning Performers:");
    let concerning = client.get_concerning_validators().await?;
    for (i, validator) in concerning.iter().take(5).enumerate() {
        println!("     {}. {}: {:.4}% ({}/{} slots)", 
            i + 1, 
            &validator.pubkey[..8], 
            validator.skip_rate_percent,
            validator.blocks_produced,
            validator.leader_slots
        );
    }
    println!();

    // Leader slot distribution analysis
    println!("Leader Slot Distribution:");
    let mut slot_buckets = HashMap::new();
    
    for validator in &data.validators {
        let bucket = match validator.leader_slots {
            0 => "0 slots",
            1..=10 => "1-10 slots",
            11..=50 => "11-50 slots",
            51..=100 => "51-100 slots",
            101..=500 => "101-500 slots",
            501..=1000 => "501-1000 slots",
            _ => ">1000 slots",
        };
        
        *slot_buckets.entry(bucket).or_insert(0) += 1;
    }

    let slot_bucket_order = [
        "0 slots", "1-10 slots", "11-50 slots", "51-100 slots", 
        "101-500 slots", "501-1000 slots", ">1000 slots"
    ];

    for bucket in slot_bucket_order.iter() {
        if let Some(&count) = slot_buckets.get(bucket) {
            let percentage = (count as f64 / stats.total_validators as f64) * 100.0;
            println!("   {:<15}: {:4} validators ({:5.1}%)", bucket, count, percentage);
        }
    }
    println!();

    // Calculate some advanced metrics
    println!("Advanced Metrics:");
    
    // Weighted skip rate (by leader slots)
    let weighted_skip_sum: f64 = data.validators.iter()
        .map(|v| v.skip_rate_percent * v.leader_slots as f64)
        .sum();
    let weighted_skip_rate = weighted_skip_sum / stats.total_leader_slots as f64;
    println!("   Weighted Skip Rate: {:.4}% (weighted by leader slots)", weighted_skip_rate);
    
    // Skip rate variance
    let variance: f64 = data.validators.iter()
        .map(|v| (v.skip_rate_percent - stats.average_skip_rate_percent).powi(2))
        .sum::<f64>() / data.validators.len() as f64;
    let std_deviation = variance.sqrt();
    println!("   Skip Rate Std Deviation: {:.4}%", std_deviation);
    
    // Active validator ratio (validators with >0 leader slots)
    let active_validators = data.validators.iter().filter(|v| v.leader_slots > 0).count();
    let active_ratio = (active_validators as f64 / stats.total_validators as f64) * 100.0;
    println!("   Active Validator Ratio: {:.1}% ({}/{})", 
        active_ratio, active_validators, stats.total_validators);

    println!("\nComprehensive analysis complete!");
    Ok(())
}