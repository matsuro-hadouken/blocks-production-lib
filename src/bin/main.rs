use blocks_production_lib::{BlockProductionClient, Result, AlertSeverity};
use clap::Parser;
use colored::*;
use std::process;

/// Solana Block Production CLI - Get validator performance statistics
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// RPC endpoint to use
    #[arg(short, long, default_value = "https://api.mainnet-beta.solana.com")]
    endpoint: String,

    /// Use quiet output (minimal info)
    #[arg(short, long)]
    quiet: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run_cli(cli).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn run_cli(cli: Cli) -> Result<()> {
    let client = BlockProductionClient::builder()
        .rpc_endpoint(&cli.endpoint)
        .public_rpc_config()
        .build()?;

    // Test connection first
    if !cli.quiet {
        println!("\x1b[36m[INFO]\x1b[0m Connecting to {}...", cli.endpoint);
    }
    
    client.test_connection().await?;
    
    if !cli.quiet {
        println!("\x1b[32m[SUCCESS]\x1b[0m Connected successfully\n");
    }

    // Fetch data
    let data = client.fetch_block_production().await?;
    let stats = &data.statistics;

    if cli.quiet {
        // Minimal output for scripts
        println!("validators={} slots={} skip_rate={:.2}% perfect={} concerning={} offline={}", 
            stats.total_validators,
            stats.total_leader_slots,
            stats.overall_skip_rate_percent,
            stats.perfect_validators,
            stats.concerning_validators,
            stats.offline_validators
        );
        return Ok(());
    }

    // Header
    println!("\x1b[1;35mSOLANA BLOCK PRODUCTION REPORT\x1b[0m");
    println!("═══════════════════════════════════");
    
    // Basic network stats
    println!("\n\x1b[1;34mNETWORK OVERVIEW:\x1b[0m");
    println!("   Total Validators: {}", stats.total_validators);
    println!("   Total Leader Slots: {}", stats.total_leader_slots);
    println!("   Total Blocks Produced: {}", stats.total_blocks_produced);
    println!("   Total Missed Slots: {}", stats.total_missed_slots);
    println!("   Network Skip Rate: {:.2}%", stats.overall_skip_rate_percent);
    println!("   Network Efficiency: {:.2}%", (stats.total_blocks_produced as f64 / stats.total_leader_slots as f64) * 100.0);
    println!("   Network Health Score: {:.1}/100", data.network_health.health_score);
    println!("   Status: {:?}", data.network_health.status);
    
    // Enhanced validator performance summary
    println!("\n\x1b[1;33mVALIDATOR PERFORMANCE BREAKDOWN:\x1b[0m");
    println!("   Perfect Performers (0% skip): {} ({:.1}%)", 
        stats.perfect_validators,
        (stats.perfect_validators as f64 / stats.total_validators as f64) * 100.0
    );
    println!("   Concerning Validators (>5% skip): {} ({:.1}%)", 
        stats.concerning_validators,
        (stats.concerning_validators as f64 / stats.total_validators as f64) * 100.0
    );
    println!("   Dead Validators (100% skip): {} ({:.1}%)", 
        stats.offline_validators,
        (stats.offline_validators as f64 / stats.total_validators as f64) * 100.0
    );
    println!("   High-Activity Validators (>1000 slots): {} ({:.1}%)", 
        stats.high_activity_validators,
        (stats.high_activity_validators as f64 / stats.total_validators as f64) * 100.0
    );
    println!("   Significant Validators (≥50 slots): {} ({:.1}%)", 
        stats.significant_validators,
        (stats.significant_validators as f64 / stats.total_validators as f64) * 100.0
    );

    // Statistical analysis
    println!("\n\x1b[1;32mSTATISTICAL ANALYSIS:\x1b[0m");
    println!("   Average Skip Rate: {:.4}%", stats.average_skip_rate_percent);
    println!("   Median Skip Rate: {:.4}%", stats.median_skip_rate_percent);
    println!("   Weighted Skip Rate: {:.4}% (by slot count)", stats.weighted_skip_rate_percent);
    println!("   95th Percentile Skip Rate: {:.4}%", stats.skip_rate_95th_percentile);
    println!("   Significant Validators Skip Rate: {:.4}%", stats.significant_validators_skip_rate_percent);

    // Distribution overview
    println!("\n\x1b[1;36mSKIP RATE DISTRIBUTION:\x1b[0m");
    let mut non_empty_buckets = 0;
    for bucket in &data.distribution.buckets {
        if bucket.validator_count > 0 {
            println!("   {}: {} validators ({:.1}% of network, {} total slots)", 
                bucket.range_label,
                bucket.validator_count,
                bucket.percentage_of_total,
                bucket.total_slots
            );
            non_empty_buckets += 1;
        }
        if non_empty_buckets >= 5 { // Show max 5 buckets to keep it manageable
            break;
        }
    }

    // Network health alerts
    if !data.network_health.alerts.is_empty() {
        println!("\n\x1b[1;31mNETWORK ALERTS:\x1b[0m");
        for alert in &data.network_health.alerts {
            let (color, level) = match alert.severity {
                AlertSeverity::Critical => ("\x1b[31m", "[CRITICAL]"),
                AlertSeverity::Warning => ("\x1b[33m", "[WARNING]"),
                AlertSeverity::Info => ("\x1b[36m", "[INFO]"),
            };
            println!("   {}{}\x1b[0m {}", color, level, alert.message);
        }
    } else {
        println!("\n\x1b[32m[SUCCESS]\x1b[0m NO NETWORK ALERTS - Excellent network health!");
    }

    // Show top 10 most problematic validators
    let mut problematic_validators: Vec<_> = data.validators.iter()
        .filter(|v| v.skip_rate_percent > 1.0 && v.leader_slots > 0)
        .map(|v| {
            let network_share = (v.leader_slots as f64 / stats.total_leader_slots as f64) * 100.0;
            let impact_score = v.skip_rate_percent * network_share;
            (v, impact_score, network_share)
        })
        .collect();
    
    // Sort by impact score descending (skip_rate × network_share)
    problematic_validators.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

    if problematic_validators.is_empty() {
        println!("\n{}", "NO PROBLEMATIC VALIDATORS FOUND - EXCELLENT NETWORK HEALTH".green().bold());
    } else {
        println!("\n{}", "TOP 10 MOST PROBLEMATIC VALIDATORS".red().bold());
        println!("{:<47} {:<10} {:<8} {:<10} {:<12} {:<10}", 
                 "Validator Public Key".white().bold(),
                 "Skip Rate".white().bold(), 
                 "Slots".white().bold(),
                 "Missed".white().bold(),
                 "Network%".white().bold(),
                 "Impact".white().bold());
        println!("{}", "-".repeat(97).white());
        
        for (i, (validator, impact_score, network_share)) in problematic_validators.iter().take(10).enumerate() {
            let (_skip_color, format_fn): (&str, fn(&str) -> colored::ColoredString) = 
                if validator.skip_rate_percent >= 50.0 { 
                    ("red", |s| s.red())
                } else if validator.skip_rate_percent >= 20.0 { 
                    ("yellow", |s| s.yellow()) 
                } else if validator.skip_rate_percent >= 5.0 {
                    ("magenta", |s| s.magenta())
                } else { 
                    ("white", |s| s.white())
                };
            
            println!("   {:<2} {:<44} {:<10} {:<8} {:<10} {:<12} {:<10}",
                     format!("{}.", i + 1).white(),
                     format_fn(&validator.pubkey),
                     format_fn(&format!("{:.2}%", validator.skip_rate_percent)),
                     validator.leader_slots.to_string().white(),
                     format_fn(&validator.missed_slots.to_string()),
                     format!("{:.3}%", network_share).cyan(),
                     format_fn(&format!("{:.3}", impact_score)));
        }
        
        // Show high-stake validators with concerning performance
        let high_stake_bad: Vec<_> = data.validators.iter()
            .filter(|v| v.leader_slots > 1000 && v.skip_rate_percent > 5.0)
            .collect();
            
        if !high_stake_bad.is_empty() {
            println!("\n{}", "HIGH-STAKE VALIDATORS WITH CONCERNING PERFORMANCE (>1000 slots, >5% skip):".yellow());
            for validator in high_stake_bad.iter().take(3) {
                println!("   {} -> {:.2}% skip rate, {} slots, {} blocks lost",
                         validator.pubkey.red(),
                         validator.skip_rate_percent,
                         validator.leader_slots,
                         validator.missed_slots);
            }
        }
    }

    // Metadata
    println!("\n\x1b[1;90mDATA METADATA:\x1b[0m");
    println!("   Slot Range: {} to {}", data.slot_range.first_slot, data.slot_range.last_slot);
    println!("   Slot Count: {}", data.slot_range.slot_count());
    println!("   Fetched At: {}", data.fetched_at.format("%Y-%m-%d %H:%M:%S UTC"));

    Ok(())
}