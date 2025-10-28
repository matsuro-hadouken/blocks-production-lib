use blocks_production_lib::{BlockProductionClient, Result, ValidatorSkipRate};

#[tokio::main]
async fn main() -> Result<()> {
    let client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .public_rpc_config()
        .build()?;
    
    // Fetch structured data ready for database insertion
    let data = client.fetch_block_production().await?;
    
    println!("=== DATA PROVIDER OUTPUT ===\n");
    
    // 1. VALIDATOR RECORDS (for validator_skip_rates table)
    println!("VALIDATOR RECORDS: {} records", data.validators.len());
    println!("Sample validator record:");
    if let Some(validator) = data.validators.first() {
        println!("  pubkey: {}", validator.pubkey);
        println!("  leader_slots: {}", validator.leader_slots);
        println!("  blocks_produced: {}", validator.blocks_produced);
        println!("  missed_slots: {}", validator.missed_slots);
        println!("  skip_rate_percent: {:.2}%", validator.skip_rate_percent);
    }
    println!();
    
    // 2. NETWORK STATISTICS (for network_statistics table)
    println!("NETWORK STATISTICS:");
    let stats = &data.statistics;
    println!("  total_validators: {}", stats.total_validators);
    println!("  total_leader_slots: {}", stats.total_leader_slots);
    println!("  total_blocks_produced: {}", stats.total_blocks_produced);
    println!("  overall_skip_rate_percent: {:.2}%", stats.overall_skip_rate_percent);
    println!("  perfect_validators: {}", stats.perfect_validators);
    println!("  concerning_validators: {}", stats.concerning_validators);
    println!("  skip_rate_95th_percentile: {:.2}%", stats.skip_rate_95th_percentile);
    println!();
    
    // 3. DISTRIBUTION DATA (for skip_rate_distribution table as JSONB)
    println!("DISTRIBUTION DATA:");
    println!("  buckets: {} distribution buckets", data.distribution.buckets.len());
    println!("  percentiles: {} percentile points", data.distribution.percentiles.len());
    
    // Show detailed percentile data structure
    println!("  PERCENTILE BREAKDOWN:");
    for percentile in data.distribution.percentiles.iter() {
        println!("    p{}: {:.3}% skip rate", percentile.percentile, percentile.skip_rate_percent);
    }
    
    // Show bucket distribution with validator counts
    println!("  BUCKET DISTRIBUTION:");
    for bucket in data.distribution.buckets.iter().take(3) {
        println!("    {}: {} validators ({:.1}% of network)", 
                 bucket.range_label, bucket.validator_count,
                 (bucket.validator_count as f64 / stats.total_validators as f64) * 100.0);
    }
    if data.distribution.buckets.len() > 3 {
        println!("    ... {} more buckets", data.distribution.buckets.len() - 3);
    }
    println!();
    
    // 4. NETWORK HEALTH (for network_health table)
    println!("NETWORK HEALTH:");
    let health = &data.network_health;
    println!("  health_score: {:.1}/100", health.health_score);
    println!("  status: {:?}", health.status);
    println!("  active_alerts: {}", health.alerts.len());
    println!();
    
    // 5. PERFORMANCE SNAPSHOTS (for validator_performance table)
    println!("PERFORMANCE SNAPSHOTS: {} records", data.performance_snapshots.len());
    println!("  WEIGHTED PERFORMANCE METRICS:");
    
    // Group and show performance categories
    let mut category_counts = std::collections::HashMap::new();
    let mut category_skip_rates = std::collections::HashMap::new();
    let mut total_leader_slots_by_category = std::collections::HashMap::new();
    
    for snapshot in &data.performance_snapshots {
        *category_counts.entry(snapshot.performance_category.clone()).or_insert(0) += 1;
        category_skip_rates.entry(snapshot.performance_category.clone())
            .or_insert(Vec::new())
            .push(snapshot.skip_rate_percent);
        *total_leader_slots_by_category.entry(snapshot.performance_category.clone())
            .or_insert(0) += snapshot.leader_slots;
    }
    
    for (category, count) in category_counts {
        let skip_rates = &category_skip_rates[&category];
        let avg_skip_rate = skip_rates.iter().sum::<f64>() / skip_rates.len() as f64;
        let total_slots = total_leader_slots_by_category[&category];
        let network_weight = (total_slots as f64 / stats.total_leader_slots as f64) * 100.0;
        
        println!("    {:?}: {} validators, {:.2}% avg skip rate, {:.1}% network weight", 
                 category, count, avg_skip_rate, network_weight);
    }
    
    // Show detailed sample with all fields
    if let Some(snapshot) = data.performance_snapshots.first() {
        println!("  SAMPLE RECORD STRUCTURE:");
        println!("    validator_pubkey: {}", snapshot.validator_pubkey);
        println!("    leader_slots: {} (assigned)", snapshot.leader_slots);
        println!("    blocks_produced: {} (actual)", snapshot.blocks_produced);
        println!("    missed_slots: {}", snapshot.leader_slots - snapshot.blocks_produced);
        println!("    skip_rate_percent: {:.3}%", snapshot.skip_rate_percent);
        println!("    performance_category: {:?}", snapshot.performance_category);
        println!("    timestamp: {}", snapshot.timestamp);
        println!("    slot_range: {} to {}", snapshot.slot_range.first_slot, snapshot.slot_range.last_slot);
    }
    println!();
    
    // 6. METADATA (for all tables)
    println!("METADATA:");
    println!("  slot_range: {} to {}", data.slot_range.first_slot, data.slot_range.last_slot);
    println!("  fetched_at: {}", data.fetched_at);
    println!("  slot_count: {}", data.slot_range.slot_count());
    println!();
    
    // 7. JSON SERIALIZATION (for JSONB fields)
    println!("JSON SERIALIZATION EXAMPLES:");
    
    // Distribution buckets as JSONB
    let json_buckets = serde_json::to_string_pretty(&data.distribution.buckets[..2]).unwrap();
    println!("Distribution buckets (for skip_rate_distribution.buckets JSONB):");
    println!("{}", json_buckets);
    
    // Percentiles as JSONB
    let json_percentiles = serde_json::to_string_pretty(&data.distribution.percentiles.iter().skip(6).collect::<Vec<_>>()).unwrap();
    println!("\nPercentiles (for skip_rate_distribution.percentiles JSONB, showing higher percentiles):");
    println!("{}", json_percentiles);
    
    // Network health alerts as JSONB
    if !data.network_health.alerts.is_empty() {
        let json_alerts = serde_json::to_string_pretty(&data.network_health.alerts).unwrap();
        println!("\nHealth alerts (for network_health.alerts JSONB):");
        println!("{}", json_alerts);
    }
    
    println!();
    
    println!("Data extraction complete.");
    println!("See DATA_MAP.md for complete table schemas.");
    
    // 8. WORST VALIDATOR ANALYSIS
    println!("\nWORST VALIDATOR ANALYSIS:");
    
    // Strategy 1: Most network damage (missed slots × impact)
    let mut network_damage_scores: Vec<_> = data.validators.iter()
        .filter(|v| v.missed_slots > 0)
        .map(|v| {
            let damage_score = v.missed_slots as f64 * (v.leader_slots as f64 / stats.total_leader_slots as f64) * 10000.0;
            (v, damage_score)
        })
        .collect();
    network_damage_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    if let Some((worst_by_damage, damage_score)) = network_damage_scores.first() {
        println!("  MOST NETWORK DAMAGE:");
        println!("    validator: {}", worst_by_damage.pubkey);
        println!("    damage_score: {:.2} (missed_slots × network_weight × 10000)", damage_score);
        println!("    missed_slots: {} out of {} assigned", worst_by_damage.missed_slots, worst_by_damage.leader_slots);
        println!("    skip_rate: {:.2}%", worst_by_damage.skip_rate_percent);
        println!("    network_weight: {:.3}%", (worst_by_damage.leader_slots as f64 / stats.total_leader_slots as f64) * 100.0);
    }
    
    // Strategy 2: Worst high-stake validator (>1000 slots with bad performance)
    let high_stake_bad: Vec<_> = data.validators.iter()
        .filter(|v| v.leader_slots > 1000 && v.skip_rate_percent > 5.0)
        .collect();
    
    if !high_stake_bad.is_empty() {
        let worst_high_stake = high_stake_bad.iter()
            .max_by(|a, b| a.skip_rate_percent.partial_cmp(&b.skip_rate_percent).unwrap())
            .unwrap();
            
        println!("  WORST HIGH-STAKE VALIDATOR (>1000 slots):");
        println!("    validator: {}", worst_high_stake.pubkey);
        println!("    skip_rate: {:.2}%", worst_high_stake.skip_rate_percent);
        println!("    leader_slots: {} (high-stake)", worst_high_stake.leader_slots);
        println!("    missed_slots: {}", worst_high_stake.missed_slots);
        println!("    blocks_lost: {} (significant network impact)", worst_high_stake.missed_slots);
    } else {
        println!("  WORST HIGH-STAKE VALIDATOR: None found (no high-stake validators with >5% skip rate)");
    }
    
    // Strategy 3: Most absolute missed slots
    if let Some(worst_by_missed) = data.validators.iter().max_by_key(|v| v.missed_slots) {
        if worst_by_missed.missed_slots > 0 {
            println!("  MOST ABSOLUTE MISSED SLOTS:");
            println!("    validator: {}", worst_by_missed.pubkey);
            println!("    missed_slots: {} (absolute worst)", worst_by_missed.missed_slots);
            println!("    leader_slots: {}", worst_by_missed.leader_slots);
            println!("    skip_rate: {:.2}%", worst_by_missed.skip_rate_percent);
            println!("    efficiency: {:.1}% (blocks_produced/leader_slots)", 
                     (worst_by_missed.blocks_produced as f64 / worst_by_missed.leader_slots as f64) * 100.0);
        }
    }
    
    // Strategy 4: Worst performer by category analysis
    println!("  TOP 3 WORST BY CATEGORY:");
    let critical_validators: Vec<_> = data.performance_snapshots.iter()
        .filter(|v| matches!(v.performance_category, 
            blocks_production_lib::types::ValidatorPerformanceCategory::Critical |
            blocks_production_lib::types::ValidatorPerformanceCategory::Offline
        ))
        .collect();
    
    for (i, validator) in critical_validators.iter().take(3).enumerate() {
        println!("    {}. {} -> {:.1}% skip rate ({} missed of {} slots)",
                 i + 1,
                 validator.validator_pubkey,
                 validator.skip_rate_percent,
                 validator.leader_slots - validator.blocks_produced,
                 validator.leader_slots);
    }
    
    // Strategy 5: Network impact ranking
    println!("  NETWORK IMPACT RANKING (worst validators by total impact):");
    let mut impact_ranking: Vec<_> = data.validators.iter()
        .filter(|v| v.skip_rate_percent > 1.0)
        .map(|v| {
            let network_share = (v.leader_slots as f64 / stats.total_leader_slots as f64) * 100.0;
            let impact_score = v.skip_rate_percent * network_share;
            (v, impact_score, network_share)
        })
        .collect();
    impact_ranking.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    for (i, (validator, impact_score, network_share)) in impact_ranking.iter().take(5).enumerate() {
        println!("    {}. {} -> {:.3} impact score ({:.2}% skip × {:.3}% network share)",
                 i + 1,
                 validator.pubkey,
                 impact_score,
                 validator.skip_rate_percent,
                 network_share);
    }
    
    println!();

    // OFFLINE VALIDATORS SECTION (for QA validation)
    let offline_validators: Vec<&ValidatorSkipRate> = data.validators
        .iter()
        .filter(|v| v.is_offline())
        .collect();
        
    if offline_validators.is_empty() {
        println!("OFFLINE VALIDATORS: None found - excellent network health!");
    } else {
        println!("OFFLINE VALIDATORS: {} completely offline validators", offline_validators.len());
        for (i, validator) in offline_validators.iter().take(10).enumerate() {
            println!("   {}. {}: {} assigned slots, 0 blocks produced", 
                i + 1, validator.pubkey, validator.leader_slots);
        }
        if offline_validators.len() > 10 {
            println!("   ... and {} more offline validators", offline_validators.len() - 10);
        }
    }
    
    println!();
    
    Ok(())
}