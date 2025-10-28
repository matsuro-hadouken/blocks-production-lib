# Examples

This directory contains comprehensive examples demonstrating how to use the blocks-production-lib.

## Available Examples

### 1. Basic Usage (`basic_usage.rs`)

Demonstrates the fundamental features of the library:
- Creating a client with public RPC configuration
- Testing RPC connection
- Fetching block production data
- Analyzing overall statistics
- Getting top and bottom performers
- Identifying concerning validators

Run with:
```bash
cargo run --example basic_usage
```

Output includes:
- Overall network statistics (total validators, skip rates, etc.)
- Top 10 best performing validators
- Bottom 10 worst performing validators
- Validators with concerning skip rates (>5%)
- Perfect validators (0% skip rate)

### 2. Advanced Configuration (`advanced_config.rs`)

Shows advanced configuration options and different client setups:
- Enterprise, high-frequency, and custom configurations
- Provider-specific optimizations
- Custom headers and timeouts
- Fetching data for specific slot ranges
- Fetching data for specific validators
- Debug format output with raw RPC data

Run with:
```bash
cargo run --example advanced_config
```

Features demonstrated:
- Multiple client configurations
- Custom slot range queries
- Validator-specific queries
- Debug output with metadata
- Provider preset configurations

### 3. Statistics Analysis (`statistics_analysis.rs`)

Comprehensive statistical analysis of block production data:
- Detailed performance distribution
- Skip rate buckets and histograms
- Network health indicators
- Advanced metrics calculations
- Leader slot distribution analysis

Run with:
```bash
cargo run --example statistics_analysis
```

Analysis includes:
- Performance distribution across validators
- Skip rate distribution in buckets
- Network efficiency metrics
- Weighted skip rates
- Statistical variance and standard deviation
- Active validator ratios

## Configuration Examples

### Public RPC (Conservative)
```rust
let client = BlockProductionClient::builder()
    .rpc_endpoint("https://api.mainnet-beta.solana.com")
    .public_rpc_config()
    .build()?;
```

### Private RPC (Higher Limits)
```rust
let client = BlockProductionClient::builder()
    .rpc_endpoint("https://your-private-rpc.com")
    .private_rpc_config()
    .build()?;
```

### High Frequency (Optimized)
```rust
let client = BlockProductionClient::builder()
    .rpc_endpoint("https://fast-rpc.com")
    .high_frequency_config()
    .build()?;
```

### Custom Configuration
```rust
let client = BlockProductionClient::builder()
    .rpc_endpoint("https://custom-rpc.com")
    .timeout(Duration::from_secs(60))
    .retry_attempts(5)
    .rate_limit(25)
    .max_concurrent_requests(50)
    .add_header("Authorization", "Bearer token")
    .build()?;
```

## Data Analysis Examples

### Performance Metrics
```rust
// Get statistics
let data = client.fetch_block_production().await?;
println!("Overall skip rate: {:.2}%", data.statistics.overall_skip_rate_percent);
println!("Perfect validators: {}", data.statistics.perfect_validators);

// Get specific groups
let concerning = client.get_concerning_validators().await?;
let perfect = client.get_perfect_validators().await?;
let top_10 = client.get_top_validators(10).await?;
```

### Custom Analysis
```rust
// Filter validators by criteria
let high_volume_validators: Vec<_> = data.validators
    .iter()
    .filter(|v| v.leader_slots > 1000)
    .collect();

// Calculate custom metrics
let weighted_skip_rate = data.validators
    .iter()
    .map(|v| v.skip_rate_percent * v.leader_slots as f64)
    .sum::<f64>() / data.statistics.total_leader_slots as f64;
```

## Error Handling Examples

### Basic Error Handling
```rust
match client.fetch_block_production().await {
    Ok(data) => println!("Success: {} validators", data.statistics.total_validators),
    Err(e) => eprintln!("Error: {}", e),
}
```

### Detailed Error Handling
```rust
use blocks_production_lib::BlockProductionError;

match client.fetch_block_production().await {
    Ok(data) => process_data(data),
    Err(BlockProductionError::Http(e)) => {
        eprintln!("HTTP error: {}", e);
        // Retry logic or fallback
    },
    Err(BlockProductionError::Rpc { message }) => {
        eprintln!("RPC error: {}", message);
        // Handle RPC-specific issues
    },
    Err(BlockProductionError::Timeout) => {
        eprintln!("Request timed out");
        // Increase timeout or retry
    },
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Running Examples

All examples can be run from the project root:

```bash
# Run all examples
cargo run --example basic_usage
cargo run --example advanced_config
cargo run --example statistics_analysis

# Run with logging
RUST_LOG=debug cargo run --example basic_usage

# Run specific example with custom RPC (set environment variable)
RPC_ENDPOINT=https://your-custom-rpc.com cargo run --example basic_usage
```

## Environment Variables

Examples support the following environment variables:

- `RPC_ENDPOINT`: Custom RPC endpoint to use
- `RUST_LOG`: Logging level (trace, debug, info, warn, error)

## Tips for Using Examples

1. Start with basic_usage.rs to understand the core functionality
2. Use advanced_config.rs to learn about configuration options
3. Explore statistics_analysis.rs for in-depth data analysis
4. Check the console output for detailed statistics and analysis
5. Modify examples to experiment with different configurations
6. Use environment variables to test with your own RPC endpoints

## Expected Output

Examples will show real-time Solana validator performance data including:
- Current validator count and slot ranges
- Skip rate statistics and distributions  
- Top and bottom performing validators
- Network health indicators
- Detailed performance analysis

Note: Output will vary based on current network conditions and the specific epoch being analyzed.