# Blocks Production Library

[![Crates.io](https://img.shields.io/crates/v/blocks-production-lib.svg)](https://crates.io/crates/blocks-production-lib)
[![Documentation](https://docs.rs/blocks-production-lib/badge.svg)](https://docs.rs/blocks-production-lib)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A Rust library for fetching Solana block production data and calculating validator skip rates. Built following best practices with comprehensive error handling, rate limiting, and flexible configuration options.

## Features

- Simple API for fetching block production data
- Comprehensive statistics including detailed skip rate analysis and performance metrics
- Weighted skip rate algorithms with significance-based filtering to exclude test validators
- Plotting-ready data structures with pre-calculated arrays for charts and visualizations
- Distribution analysis with histogram buckets and percentile calculations
- Network health monitoring with automated alerts and status indicators
- Dashboard-ready metrics with color codes and trend indicators
- Time-series data support for tracking validator performance over time
- Flexible configuration with multiple preset configurations for different use cases
- Retry logic with built-in retry mechanisms and exponential backoff
- Rate limiting with configurable rate limiting to respect RPC endpoints
- Error handling with comprehensive error types and detailed messages
- Well tested with extensive unit and integration tests using mock RPC responses
- Production ready with both production and debug output formats

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
blocks-production-lib = "0.1.0"
tokio = { version = "1.0", features = ["full"] }
```

### Basic Usage

```rust
use blocks_production_lib::BlockProductionClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client with automatic configuration
    let client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .public_rpc_config()
        .build()?;
    
    // Test connection
    client.test_connection().await?;
    
    // Fetch block production data
    let data = client.fetch_block_production().await?;
    
    println!("Total validators: {}", data.statistics.total_validators);
    println!("Overall skip rate: {:.2}%", data.statistics.overall_skip_rate_percent);
    println!("Network health: {:?}", data.network_health.status);
    
    // Get significant validators (filters out test validators)
    let significant = client.get_significant_validators().await?;
    println!("Significant validators: {}", significant.len());
    
    Ok(())
}
```

## Plotting and Visualization Features

The library provides comprehensive plotting-ready data structures that require zero calculations from frontend engineers:

### Ready-to-Plot Data

```rust
use blocks_production_lib::BlockProductionClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BlockProductionClient::new("https://api.mainnet-beta.solana.com")?;
    let data = client.fetch_block_production().await?;

    // Histogram data for bar charts
    let histogram = &data.distribution.plot_data;
    println!("Chart labels: {:?}", histogram.histogram_labels);
    println!("Chart values: {:?}", histogram.histogram_values);
    
    // Percentile data for line charts
    println!("Percentile X: {:?}", histogram.percentile_x);
    println!("Percentile Y: {:?}", histogram.percentile_y);
    
    // Dashboard metrics with colors
    let metrics = &data.network_health.key_metrics;
    println!("Skip rate: {} (color: {})", 
             metrics.network_skip_rate.value,
             metrics.network_skip_rate.color);
    
    // Performance categories with color coding
    for snapshot in &data.performance_snapshots {
        println!("Validator {}: {:?} ({})", 
                 &snapshot.validator_pubkey[..8],
                 snapshot.performance_category,
                 snapshot.performance_category.color_hex());
    }
    
    Ok(())
}
```

### Distribution Analysis

```rust
// Skip rate distribution in buckets
for bucket in &data.distribution.buckets {
    println!("{}: {} validators ({:.1}%)", 
             bucket.range_label,
             bucket.validator_count,
             bucket.percentage_of_total);
}

// Percentile analysis
for percentile in &data.distribution.percentiles {
    println!("P{}: {:.2}%", 
             percentile.percentile,
             percentile.skip_rate_percent);
}
```

### Network Health Dashboard

```rust
// Health summary
println!("Network status: {:?}", data.network_health.status);
println!("Health score: {:.1}/100", data.network_health.health_score);

// Alerts
for alert in &data.network_health.alerts {
    println!("{:?}: {}", alert.severity, alert.message);
}

// Dashboard cards
let cards = &data.network_health.key_metrics;
println!("Active validators: {}", cards.active_validators.value);
println!("Network efficiency: {}", cards.network_efficiency.value);
```

### Weighted Skip Rate Analysis

The library uses sophisticated algorithms to provide meaningful skip rate analysis:

```rust
let stats = &data.statistics;

// Multiple skip rate perspectives
println!("Overall: {:.2}%", stats.overall_skip_rate_percent);
println!("Significant validators: {:.2}%", stats.significant_validators_skip_rate_percent);
println!("High-stake validators: {:.2}%", stats.high_stake_skip_rate_percent);
println!("Weighted: {:.2}%", stats.weighted_skip_rate_percent);

// Percentile analysis
println!("95th percentile: {:.2}%", stats.skip_rate_95th_percentile);
println!("95th percentile (significant): {:.2}%", stats.significant_skip_rate_95th_percentile);
```
```

## Configuration Options

### Preset Configurations

The library provides several preset configurations optimized for different scenarios:

```rust
// For public RPC endpoints (conservative rate limiting)
let client = BlockProductionClient::builder()
    .rpc_endpoint("https://api.mainnet-beta.solana.com")
    .public_rpc_config()
    .build()?;

// For private/paid RPC endpoints (higher rate limits)
let client = BlockProductionClient::builder()
    .rpc_endpoint("https://your-private-rpc.com")
    .private_rpc_config()
    .build()?;

// For high-frequency applications
let client = BlockProductionClient::builder()
    .rpc_endpoint("https://your-rpc.com")
    .high_frequency_config()
    .build()?;

// Auto-detect optimal settings based on endpoint
let client = BlockProductionClient::builder()
    .auto_config("https://api.mainnet-beta.solana.com")
    .build()?;
```

### Provider-Specific Optimizations

```rust
// Optimized for specific RPC providers
let helius_client = BlockProductionClient::builder()
    .auto_config("https://mainnet.helius-rpc.com/?api-key=YOUR_KEY")
    .build()?;

let quicknode_client = BlockProductionClient::builder()
    .auto_config("https://your-endpoint.quiknode.pro/")
    .build()?;
```

### Custom Configuration

```rust
let client = BlockProductionClient::builder()
    .rpc_endpoint("https://custom-rpc.com")
    .timeout(Duration::from_secs(60))
    .retry_attempts(5)
    .rate_limit(10) // 10 requests per second
    .max_concurrent_requests(20)
    .add_header("Authorization", "Bearer your-token")
    .build()?;
```

## API Methods

### Basic Data Fetching

```rust
// Fetch all validator data with plotting structures
let data = client.fetch_block_production().await?;

// Test RPC connection
let is_connected = client.test_connection().await?;
```

### Validator Analysis

```rust
// Get validators by performance categories
let concerning = client.get_concerning_validators().await?;        // >5% skip rate
let perfect = client.get_perfect_validators().await?;              // 0% skip rate  
let offline = client.get_offline_validators().await?;              // 100% skip rate
let significant = client.get_significant_validators().await?;      // >=50 slots
let high_activity = client.get_high_activity_validators().await?;  // >1000 slots

// Get performance groups
let moderate = client.get_moderate_performers().await?;            // 1-5% skip rate
let worst_percentile = client.get_worst_percentile_validators().await?; // Bottom 10%
```

### Data Structures

The library returns comprehensive data structures optimized for different use cases:

```rust
pub struct BlockProductionData {
    pub validators: Vec<ValidatorSkipRate>,           // Individual validator data
    pub statistics: SkipRateStatistics,              // Aggregated network statistics  
    pub distribution: SkipRateDistribution,          // Histogram and percentile data
    pub network_health: NetworkHealthSummary,        // Dashboard and alerting data
    pub performance_snapshots: Vec<ValidatorPerformanceSnapshot>, // Time-series data
    pub slot_range: SlotRange,                       // Slot range information
    pub fetched_at: DateTime<Utc>,                   // Timestamp
}
```

### Debug Information

```rust
// Get debug data with raw RPC response
let debug_data = client.fetch_block_production_debug(
    blocks_production_lib::BlockProductionRequest::default()
).await?;

println!("Response time: {}ms", debug_data.response_metadata.response_time_ms);
println!("Raw RPC data: {}", debug_data.raw_rpc_data);
```

## Data Structures

### Enhanced Statistics with Weighted Analysis

```rust
pub struct SkipRateStatistics {
    // Basic metrics
    pub total_validators: usize,
    pub total_leader_slots: u64,
    pub total_blocks_produced: u64,
    pub total_missed_slots: u64,
    
    // Skip rate analysis
    pub overall_skip_rate_percent: f64,                    // Network-wide skip rate
    pub average_skip_rate_percent: f64,                    // Simple average
    pub median_skip_rate_percent: f64,                     // Median skip rate
    pub weighted_skip_rate_percent: f64,                   // Weighted by slots
    pub significant_validators_skip_rate_percent: f64,     // Significant validators only
    pub high_stake_skip_rate_percent: f64,                 // High-stake validators only
    
    // Validator categorization
    pub perfect_validators: usize,                         // 0% skip rate
    pub concerning_validators: usize,                      // >5% skip rate
    pub offline_validators: usize,                         // 100% skip rate
    pub significant_validators: usize,                     // >=50 slots
    pub high_activity_validators: usize,                   // >1000 slots
    
    // Percentile analysis
    pub skip_rate_90th_percentile: f64,
    pub skip_rate_95th_percentile: f64,
    pub significant_skip_rate_90th_percentile: f64,
    pub significant_skip_rate_95th_percentile: f64,
    
    // Network efficiency
    pub network_efficiency_percent: f64,
    pub weighted_network_efficiency_percent: f64,
}
```

### Plotting Data Structures

```rust
pub struct SkipRateDistribution {
    pub buckets: Vec<DistributionBucket>,        // Histogram buckets
    pub percentiles: Vec<PercentileData>,        // Percentile data points
    pub plot_data: DistributionPlotData,         // Ready-to-plot arrays
}

pub struct DistributionPlotData {
    pub histogram_labels: Vec<String>,           // ["0-1%", "1-2%", ...]
    pub histogram_values: Vec<usize>,            // [count1, count2, ...]
    pub percentile_x: Vec<u8>,                   // [10, 20, 30, ...]
    pub percentile_y: Vec<f64>,                  // [skip_rate_p10, ...]
}
```

### Network Health Monitoring

```rust
pub struct NetworkHealthSummary {
    pub health_score: f64,                       // 0-100 health score
    pub status: NetworkStatus,                   // Healthy/Warning/Critical
    pub key_metrics: DashboardMetrics,           // Dashboard cards
    pub alerts: Vec<NetworkAlert>,               // Active alerts
}

pub struct DashboardMetrics {
    pub network_skip_rate: MetricCard,
    pub active_validators: MetricCard,
    pub network_efficiency: MetricCard,
    pub concerning_validators: MetricCard,
}

pub struct MetricCard {
    pub value: String,                           // Formatted value
    pub color: String,                           // Hex color code
    pub trend: TrendDirection,                   // Up/Down/Stable
    pub subtitle: String,                        // Description
}
```

### Time-Series Data

```rust
pub struct ValidatorPerformanceSnapshot {
    pub timestamp: DateTime<Utc>,
    pub validator_pubkey: String,
    pub skip_rate_percent: f64,
    pub leader_slots: u64,
    pub performance_category: ValidatorPerformanceCategory,
}

pub enum ValidatorPerformanceCategory {
    Perfect,      // 0% skip rate
    Excellent,    // 0-1% skip rate
    Good,         // 1-3% skip rate
    Average,      // 3-5% skip rate
    Concerning,   // 5-10% skip rate
    Poor,         // 10-25% skip rate
    Critical,     // 25%+ skip rate
    Offline,      // 100% skip rate
}
```

### Individual Validator Data

```rust
pub struct ValidatorSkipRate {
    pub pubkey: String,
    pub leader_slots: u64,
    pub blocks_produced: u64,
    pub missed_slots: u64,
    pub skip_rate_percent: f64,
}
```

## Examples

The library includes comprehensive examples:

```bash
# Basic usage with weighted skip rate analysis
cargo run --example basic_usage

# Advanced configuration and client setups
cargo run --example advanced_config

# Comprehensive statistics and performance analysis
cargo run --example statistics_analysis

# Plotting and visualization data structures
cargo run --example plotting_data
```

Each example demonstrates different aspects of the library:

- `basic_usage.rs`: Core functionality with weighted metrics and significant validator filtering
- `advanced_config.rs`: Different client configurations and provider optimizations  
- `statistics_analysis.rs`: In-depth statistical analysis with percentiles and distributions
- `plotting_data.rs`: Ready-to-use data structures for charts, dashboards, and frontend integration

## Error Handling

The library provides detailed error types for proper error handling:

```rust
use blocks_production_lib::{BlockProductionError, Result};

match client.fetch_block_production().await {
    Ok(data) => println!("Success: {} validators", data.statistics.total_validators),
    Err(BlockProductionError::Http(e)) => eprintln!("HTTP error: {}", e),
    Err(BlockProductionError::Rpc { message }) => eprintln!("RPC error: {}", message),
    Err(BlockProductionError::Timeout) => eprintln!("Request timed out"),
    Err(BlockProductionError::RateLimit) => eprintln!("Rate limit exceeded"),
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Testing

Run the test suite:

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests only
cargo test --test integration_tests

# With output
cargo test -- --nocapture
```

## Performance Considerations

- Rate limiting: Automatically applied based on configuration to respect RPC limits
- Retry logic: Exponential backoff prevents overwhelming endpoints during failures
- Concurrent requests: Configurable concurrency limits for batch operations
- Timeouts: Prevent hanging requests with configurable timeouts

## Use Cases

- Validator monitoring: Track validator performance over time with time-series data
- Network health analysis: Monitor overall Solana network block production with health scores
- Stake pool management: Analyze validator performance for delegation decisions using weighted metrics
- Research and analytics: Historical analysis of network performance with percentile distributions
- Alerting systems: Detect validators with concerning skip rates using automated alerts
- Dashboard development: Build monitoring dashboards with ready-to-plot data structures
- Data visualization: Create charts and graphs using pre-calculated histogram and percentile data
- Frontend integration: Implement validator analytics with zero-calculation data structures
- Cluster analytics: Generate content and reports using significance-weighted algorithms
- Real-time monitoring: Track network health in production environments with 1-second polling

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Changelog

### v0.1.0
- Initial release
- Basic block production data fetching
- Comprehensive statistics calculation
- Multiple configuration presets
- Rate limiting and retry logic
- Extensive test coverage
- Production and debug output formats