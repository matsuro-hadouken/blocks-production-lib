use blocks_production_lib::BlockProductionClient;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Block Production Plotting Data");
    println!("===============================\n");

    // Create client
    let client = BlockProductionClient::new("https://api.mainnet-beta.solana.com")?;

    // Fetch recent block production data
    println!("Fetching block production data...\n");
    let data = client.fetch_block_production().await?;

    // 1. DASHBOARD METRICS (Ready for cards/widgets)
    println!("DASHBOARD METRICS");
    println!("-----------------");
    println!("Network Status: {:?} ({})", 
             data.network_health.status, 
             data.network_health.status.color_hex());
    println!("Health Score: {:.1}/100", data.network_health.health_score);
    
    println!("\nKey Metrics:");
    println!("  Skip Rate: {} ({})", 
             data.network_health.key_metrics.network_skip_rate.value,
             data.network_health.key_metrics.network_skip_rate.color);
    println!("  Active Validators: {}", 
             data.network_health.key_metrics.active_validators.value);
    println!("  Network Efficiency: {}", 
             data.network_health.key_metrics.network_efficiency.value);
    println!("  Concerning Validators: {}", 
             data.network_health.key_metrics.concerning_validators.value);

    // 2. HISTOGRAM DATA (Ready for bar charts)
    println!("\nHISTOGRAM DATA");
    println!("--------------");
    println!("Skip Rate Distribution:");
    for bucket in &data.distribution.buckets {
        println!("  {}: {} validators ({:.1}% of total, {} slots)", 
                 bucket.range_label, 
                 bucket.validator_count,
                 bucket.percentage_of_total,
                 bucket.total_slots);
    }

    println!("\nPlot Arrays:");
    println!("  Labels: {:?}", data.distribution.plot_data.histogram_labels);
    println!("  Values: {:?}", data.distribution.plot_data.histogram_values);

    // 3. PERCENTILE DATA (Ready for line charts)
    println!("\nPERCENTILE DATA");
    println!("---------------");
    println!("X-axis (percentiles): {:?}", data.distribution.plot_data.percentile_x);
    println!("Y-axis (skip rates): {:?}", data.distribution.plot_data.percentile_y.iter().map(|v| format!("{:.2}%", v)).collect::<Vec<_>>());

    // 4. TIME-SERIES DATA (Ready for time charts)
    println!("\nTIME-SERIES DATA");
    println!("----------------");
    println!("Total snapshots: {}", data.performance_snapshots.len());
    
    // Show sample of performance categories for plotting
    let mut category_counts = std::collections::HashMap::new();
    for snapshot in &data.performance_snapshots {
        *category_counts.entry(&snapshot.performance_category).or_insert(0) += 1;
    }
    
    println!("Performance Categories:");
    for (category, count) in category_counts {
        println!("  {:?}: {} validators ({})", 
                 category, 
                 count, 
                 category.color_hex());
    }

    // 5. ALERTS (Ready for notification systems)
    println!("\nALERTS");
    println!("------");
    if data.network_health.alerts.is_empty() {
        println!("No alerts");
    } else {
        for alert in &data.network_health.alerts {
            println!("  {:?}: {} ({:?})", 
                     alert.severity, 
                     alert.message,
                     alert.category);
        }
    }

    // 6. EXAMPLE JSON FOR FRONTEND
    println!("\nJSON OUTPUT");
    println!("-----------");
    
    // Create a simplified structure that frontend would typically need
    let frontend_data = serde_json::json!({
        "timestamp": data.fetched_at,
        "network": {
            "status": data.network_health.status,
            "health_score": data.network_health.health_score,
            "skip_rate": data.statistics.overall_skip_rate_percent,
            "efficiency": data.statistics.network_efficiency_percent
        },
        "charts": {
            "histogram": {
                "labels": data.distribution.plot_data.histogram_labels,
                "values": data.distribution.plot_data.histogram_values
            },
            "percentiles": {
                "x": data.distribution.plot_data.percentile_x,
                "y": data.distribution.plot_data.percentile_y
            }
        },
        "metrics": {
            "total_validators": data.statistics.total_validators,
            "significant_validators": data.statistics.significant_validators,
            "concerning_validators": data.statistics.concerning_validators,
            "perfect_validators": data.statistics.perfect_validators
        },
        "alerts": data.network_health.alerts.len(),
        "slot_range": data.slot_range
    });

    println!("{}", serde_json::to_string_pretty(&frontend_data)?);

    Ok(())
}