//! # Blocks Production Library
//!
//! A Rust library for fetching Solana block production data and calculating validator skip rates.
//! 
//! ## Features
//! 
//! - Fetch block production data from Solana RPC endpoints
//! - Calculate validator skip rates and performance statistics
//! - Support for custom RPC endpoints with rate limiting and retry logic
//! - Production and debug output formats
//! - Configurable client with preset configurations for different use cases
//! 
//! ## Quick Start
//! 
//! ```rust
//! use blocks_production_lib::BlockProductionClient;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = BlockProductionClient::builder()
//!         .rpc_endpoint("https://api.mainnet-beta.solana.com")
//!         .public_rpc_config()
//!         .build()?;
//!     
//!     // Test connection
//!     client.test_connection().await?;
//!     
//!     // Fetch block production data
//!     let data = client.fetch_block_production().await?;
//!     
//!     println!("Total validators: {}", data.statistics.total_validators);
//!     println!("Network efficiency: {:.2}%", data.statistics.network_efficiency_percent);
//!     println!("95th percentile skip rate: {:.2}%", data.statistics.skip_rate_95th_percentile);
//!     
//!     // Get actionable insights
//!     let high_activity = client.get_high_activity_validators().await?;
//!     println!("High-activity validators: {}", high_activity.len());
//!     
//!     let concerning = client.get_concerning_validators().await?;
//!     println!("Validators needing attention: {}", concerning.len());
//!     
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod types;
pub mod logging;

// Re-export main types for convenience
pub use client::{BlockProductionClient, ClientBuilder};
pub use config::{ClientConfig, ClientConfigBuilder};
pub use error::{BlockProductionError, Result, ErrorExt};
pub use logging::{init_logging, init_test_logging, LoggingConfig, LogFormat};

/// Test utilities for mocking and testing
#[cfg(test)]
pub mod test_utils {
    use crate::types::*;

    /// Creates mock block production data for testing
    pub fn create_mock_block_production_data() -> BlockProductionData {
        let mut validators = Vec::new();
        
        // Create test validators with different performance levels
        validators.push(ValidatorSkipRate::new(
            "perfect_validator".to_string(),
            100,
            100,
        ));
        
        validators.push(ValidatorSkipRate::new(
            "good_validator".to_string(),
            100,
            98,
        ));
        
        validators.push(ValidatorSkipRate::new(
            "concerning_validator".to_string(),
            100,
            85,
        ));

        let slot_range = SlotRange {
            first_slot: 1000,
            last_slot: 2000,
        };

        // Create minimal required data structures
        let statistics = SkipRateStatistics {
            total_validators: validators.len(),
            total_leader_slots: validators.iter().map(|v| v.leader_slots).sum(),
            total_blocks_produced: validators.iter().map(|v| v.blocks_produced).sum(),
            total_missed_slots: validators.iter().map(|v| v.missed_slots).sum(),
            overall_skip_rate_percent: 5.0,
            average_skip_rate_percent: 5.0,
            median_skip_rate_percent: 5.0,
            weighted_skip_rate_percent: 5.0,
            significant_validators_skip_rate_percent: 5.0,
            high_stake_skip_rate_percent: 5.0,
            perfect_validators: 1,
            concerning_validators: 1,
            offline_validators: 0,
            low_activity_validators: 0,
            high_activity_validators: 0,
            significant_validators: validators.len(),
            skip_rate_90th_percentile: 10.0,
            skip_rate_95th_percentile: 15.0,
            significant_skip_rate_90th_percentile: 10.0,
            significant_skip_rate_95th_percentile: 15.0,
            network_efficiency_percent: 95.0,
            weighted_network_efficiency_percent: 95.0,
        };

        let distribution = SkipRateDistribution {
            buckets: vec![],
            percentiles: vec![],
            plot_data: DistributionPlotData {
                histogram_labels: vec![],
                histogram_values: vec![],
                percentile_x: vec![],
                percentile_y: vec![],
            },
        };

        let network_health = NetworkHealthSummary {
            health_score: 85.0,
            status: NetworkStatus::Healthy,
            key_metrics: DashboardMetrics {
                network_skip_rate: MetricCard {
                    value: "5.0%".to_string(),
                    previous_value: None,
                    trend: TrendDirection::Stable,
                    color: "#22c55e".to_string(),
                    subtitle: "Network skip rate".to_string(),
                },
                active_validators: MetricCard {
                    value: "3".to_string(),
                    previous_value: None,
                    trend: TrendDirection::Stable,
                    color: "#22c55e".to_string(),
                    subtitle: "Active validators".to_string(),
                },
                network_efficiency: MetricCard {
                    value: "95.0%".to_string(),
                    previous_value: None,
                    trend: TrendDirection::Up,
                    color: "#22c55e".to_string(),
                    subtitle: "Network efficiency".to_string(),
                },
                concerning_validators: MetricCard {
                    value: "1".to_string(),
                    previous_value: None,
                    trend: TrendDirection::Down,
                    color: "#eab308".to_string(),
                    subtitle: "Concerning validators".to_string(),
                },
            },
            alerts: vec![],
        };

        BlockProductionData {
            validators,
            statistics,
            distribution,
            network_health,
            performance_snapshots: vec![],
            slot_range,
            fetched_at: chrono::Utc::now(),
        }
    }

    /// Creates mock RPC response for testing
    pub fn create_mock_rpc_response() -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "result": {
                "value": {
                    "byIdentity": {
                        "validator1": [100, 95],
                        "validator2": [200, 190],
                        "validator3": [50, 50]
                    },
                    "range": {
                        "firstSlot": 1000,
                        "lastSlot": 2000
                    }
                }
            },
            "id": 1
        })
    }

    /// Creates mock percentile data for testing
    pub fn create_mock_percentile_data() -> Vec<PercentileData> {
        let _validators = vec![
            ("validator1".to_string(), 5.0),
            ("validator2".to_string(), 10.0),
            ("validator3".to_string(), 15.0),
            ("validator4".to_string(), 20.0),
        ];

        // Return mock percentile data for testing
        vec![
            PercentileData {
                percentile: 50,
                skip_rate_percent: 10.0,
            },
            PercentileData {
                percentile: 90,
                skip_rate_percent: 18.0,
            },
            PercentileData {
                percentile: 95,
                skip_rate_percent: 19.0,
            },
        ]
    }
}
pub use types::{
    BlockProductionData, BlockProductionDataDebug, BlockProductionRequest,
    SkipRateStatistics, SlotRange, ValidatorSkipRate, ResponseMetadata,
    SkipRateDistribution, NetworkHealthSummary, ValidatorPerformanceSnapshot,
    ValidatorPerformanceCategory,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_skip_rate_calculation() {
        let validator = ValidatorSkipRate::new(
            "test_pubkey".to_string(),
            100, // leader slots
            95,  // blocks produced
        );

        assert_eq!(validator.missed_slots, 5);
        assert_eq!(validator.skip_rate_percent, 5.0);
        assert!(!validator.is_perfect());
        assert!(!validator.is_concerning()); // exactly 5%, not > 5%
    }

    #[test]
    fn test_validator_perfect_performance() {
        let validator = ValidatorSkipRate::new(
            "perfect_validator".to_string(),
            100, // leader slots
            100, // blocks produced
        );

        assert_eq!(validator.missed_slots, 0);
        assert_eq!(validator.skip_rate_percent, 0.0);
        assert!(validator.is_perfect());
        assert!(!validator.is_concerning());
    }

    #[test]
    fn test_validator_concerning_performance() {
        let validator = ValidatorSkipRate::new(
            "bad_validator".to_string(),
            100, // leader slots
            90,  // blocks produced
        );

        assert_eq!(validator.missed_slots, 10);
        assert_eq!(validator.skip_rate_percent, 10.0);
        assert!(!validator.is_perfect());
        assert!(validator.is_concerning());
    }

    #[test]
    fn test_slot_range() {
        let range = SlotRange {
            first_slot: 1000,
            last_slot: 2000,
        };

        assert_eq!(range.slot_count(), 1000);
    }

    #[test]
    fn test_config_builder() {
        let config = ClientConfig::builder()
            .rpc_endpoint("https://custom-rpc.com".to_string())
            .retry_attempts(5)
            .build();

        assert_eq!(config.rpc_endpoint, "https://custom-rpc.com");
        assert_eq!(config.retry_attempts, 5);
    }
}
