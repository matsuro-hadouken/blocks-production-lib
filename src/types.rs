use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

/// RPC response wrapper for getBlockProduction calls
#[derive(Debug, Deserialize)]
pub struct RpcResponse {
    pub result: BlockProductionResult,
}

/// Block production result from RPC
#[derive(Debug, Deserialize)]
pub struct BlockProductionResult {
    pub value: BlockProductionValue,
}

/// Block production data with validator information and slot range
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockProductionValue {
    /// Map of validator public key to (leader_slots, blocks_produced)
    pub by_identity: HashMap<String, (u64, u64)>,
    /// Slot range for the data
    pub range: SlotRange,
}

/// Slot range information
#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SlotRange {
    pub first_slot: u64,
    pub last_slot: u64,
}

impl SlotRange {
    /// Calculate the total number of slots in the range
    pub fn slot_count(&self) -> u64 {
        self.last_slot.saturating_sub(self.first_slot)
    }
}

/// Individual validator skip rate data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidatorSkipRate {
    /// Validator public key
    pub pubkey: String,
    /// Number of leader slots assigned
    pub leader_slots: u64,
    /// Number of blocks actually produced
    pub blocks_produced: u64,
    /// Number of missed slots
    pub missed_slots: u64,
    /// Skip rate as percentage (0.0 to 100.0)
    pub skip_rate_percent: f64,
}

impl ValidatorSkipRate {
    /// Create a new ValidatorSkipRate from raw data
    pub fn new(pubkey: String, leader_slots: u64, blocks_produced: u64) -> Self {
        let missed_slots = leader_slots.saturating_sub(blocks_produced);
        let skip_rate_percent = if leader_slots > 0 {
            (missed_slots as f64 / leader_slots as f64) * 100.0
        } else {
            0.0
        };

        Self {
            pubkey,
            leader_slots,
            blocks_produced,
            missed_slots,
            skip_rate_percent,
        }
    }

    /// Check if validator has perfect performance (0% skip rate)
    pub fn is_perfect(&self) -> bool {
        self.skip_rate_percent == 0.0 && self.leader_slots > 0
    }

    /// Check if validator has concerning skip rate (> 5%)
    pub fn is_concerning(&self) -> bool {
        self.skip_rate_percent > 5.0
    }

    /// Check if validator is significant (has enough slots to matter for network analysis)
    /// Threshold: >= 50 slots (represents real network participants, not test validators)
    pub fn is_significant(&self) -> bool {
        self.leader_slots >= 50
    }

    /// Check if validator is high-stake (>1000 slots, these are the major network contributors)
    pub fn is_high_stake(&self) -> bool {
        self.leader_slots > 1000
    }

    /// Check if validator is low activity (< 10 slots, likely test or inactive)
    pub fn is_low_activity(&self) -> bool {
        self.leader_slots < 10
    }

    /// Check if validator is completely offline (100% skip rate)
    pub fn is_offline(&self) -> bool {
        self.skip_rate_percent >= 100.0
    }

    /// Get validator significance weight for weighted calculations
    /// Uses logarithmic scaling to prevent huge validators from dominating
    /// but still gives more weight to validators with more slots
    pub fn significance_weight(&self) -> f64 {
        if self.leader_slots == 0 {
            0.0
        } else if self.leader_slots < 50 {
            // Low significance for small validators
            0.1
        } else {
            // Logarithmic scaling: more slots = more weight, but with diminishing returns
            (self.leader_slots as f64).ln() / 10.0
        }
    }
}

/// Aggregated statistics for all validators with weighted and significance-based metrics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkipRateStatistics {
    /// Total number of validators tracked
    pub total_validators: usize,
    /// Total leader slots across all validators
    pub total_leader_slots: u64,
    /// Total blocks produced across all validators
    pub total_blocks_produced: u64,
    /// Total missed slots across all validators
    pub total_missed_slots: u64,
    /// Overall network skip rate (weighted by all slots)
    pub overall_skip_rate_percent: f64,
    /// Simple average skip rate across all validators (unweighted)
    pub average_skip_rate_percent: f64,
    /// Median skip rate
    pub median_skip_rate_percent: f64,
    /// Weighted skip rate (slots-weighted, excludes low-significance validators)
    pub weighted_skip_rate_percent: f64,
    /// Significant validators skip rate (only validators with meaningful slot counts)
    pub significant_validators_skip_rate_percent: f64,
    /// High-stake skip rate (only validators with >1000 slots)
    pub high_stake_skip_rate_percent: f64,
    /// Number of validators with perfect performance (0% skip rate)
    pub perfect_validators: usize,
    /// Number of validators with concerning skip rate (> 5%)
    pub concerning_validators: usize,
    /// Number of validators with 100% skip rate (completely offline)
    pub offline_validators: usize,
    /// Number of validators with low activity (< 10 leader slots)
    pub low_activity_validators: usize,
    /// Number of validators with high activity (> 1000 leader slots)
    pub high_activity_validators: usize,
    /// Number of significant validators (>= 50 slots, represents real network participants)
    pub significant_validators: usize,
    /// Skip rate at 90th percentile (useful for identifying outliers)
    pub skip_rate_90th_percentile: f64,
    /// Skip rate at 95th percentile
    pub skip_rate_95th_percentile: f64,
    /// Skip rate at 90th percentile for significant validators only
    pub significant_skip_rate_90th_percentile: f64,
    /// Skip rate at 95th percentile for significant validators only
    pub significant_skip_rate_95th_percentile: f64,
    /// Network efficiency (percentage of assigned slots that were produced)
    pub network_efficiency_percent: f64,
    /// Significance-weighted network efficiency (excluding noise from tiny validators)
    pub weighted_network_efficiency_percent: f64,
}

/// Distribution data for plotting histograms and percentile charts
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SkipRateDistribution {
    /// Skip rate buckets (0-1%, 1-2%, 2-5%, 5-10%, 10-25%, 25-50%, 50-100%)
    pub buckets: Vec<DistributionBucket>,
    /// Percentile values (every 10th percentile: P10, P20, ..., P90, P95, P99)
    pub percentiles: Vec<PercentileData>,
    /// Ready-to-plot arrays for frontend
    pub plot_data: DistributionPlotData,
}

/// Single bucket in skip rate distribution
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DistributionBucket {
    /// Range label (e.g., "0-1%", "5-10%")
    pub range_label: String,
    /// Lower bound (inclusive)
    pub min_percent: f64,
    /// Upper bound (exclusive, except for last bucket)
    pub max_percent: f64,
    /// Number of validators in this bucket
    pub validator_count: usize,
    /// Percentage of total validators
    pub percentage_of_total: f64,
    /// Total slots from validators in this bucket
    pub total_slots: u64,
}

/// Percentile data point
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PercentileData {
    /// Percentile (e.g., 50, 90, 95, 99)
    pub percentile: u8,
    /// Skip rate value at this percentile
    pub skip_rate_percent: f64,
}

/// Ready-to-plot data arrays for frontend frameworks
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DistributionPlotData {
    /// X-axis labels for histogram ["0-1%", "1-2%", ...]
    pub histogram_labels: Vec<String>,
    /// Y-axis values for histogram [count1, count2, ...]
    pub histogram_values: Vec<usize>,
    /// X-axis values for percentile chart [10, 20, 30, ..., 95, 99]
    pub percentile_x: Vec<u8>,
    /// Y-axis values for percentile chart [skip_rate_p10, skip_rate_p20, ...]
    pub percentile_y: Vec<f64>,
}

/// Time-series friendly data for tracking validator performance over time
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ValidatorPerformanceSnapshot {
    /// Timestamp of the snapshot
    pub timestamp: DateTime<Utc>,
    /// Slot range for this snapshot
    pub slot_range: SlotRange,
    /// Validator pubkey
    pub validator_pubkey: String,
    /// Skip rate for this time period
    pub skip_rate_percent: f64,
    /// Leader slots in this period
    pub leader_slots: u64,
    /// Performance category
    pub performance_category: ValidatorPerformanceCategory,
}

/// Categories for easy frontend filtering and color coding
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
pub enum ValidatorPerformanceCategory {
    /// 0% skip rate
    Perfect,
    /// 0-1% skip rate
    Excellent,
    /// 1-3% skip rate  
    Good,
    /// 3-5% skip rate
    Average,
    /// 5-10% skip rate
    Concerning,
    /// 10-25% skip rate
    Poor,
    /// 25%+ skip rate
    Critical,
    /// 100% skip rate
    Offline,
    /// < 10 slots (not enough data)
    Insufficient,
}

impl ValidatorPerformanceCategory {
    /// Get category from skip rate
    pub fn from_skip_rate(skip_rate: f64, leader_slots: u64) -> Self {
        if leader_slots < 10 {
            Self::Insufficient
        } else if skip_rate >= 100.0 {
            Self::Offline
        } else if skip_rate >= 25.0 {
            Self::Critical
        } else if skip_rate >= 10.0 {
            Self::Poor
        } else if skip_rate >= 5.0 {
            Self::Concerning
        } else if skip_rate >= 3.0 {
            Self::Average
        } else if skip_rate >= 1.0 {
            Self::Good
        } else if skip_rate > 0.0 {
            Self::Excellent
        } else {
            Self::Perfect
        }
    }

    /// Get color hex code for frontend
    pub fn color_hex(&self) -> &'static str {
        match self {
            Self::Perfect => "#22c55e",      // Green
            Self::Excellent => "#84cc16",    // Light green
            Self::Good => "#eab308",         // Yellow
            Self::Average => "#f97316",      // Orange
            Self::Concerning => "#ef4444",   // Red
            Self::Poor => "#dc2626",         // Dark red
            Self::Critical => "#991b1b",     // Very dark red
            Self::Offline => "#374151",      // Gray
            Self::Insufficient => "#9ca3af", // Light gray
        }
    }

    /// Get display label for frontend
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::Perfect => "Perfect (0%)",
            Self::Excellent => "Excellent (0-1%)",
            Self::Good => "Good (1-3%)",
            Self::Average => "Average (3-5%)",
            Self::Concerning => "Concerning (5-10%)",
            Self::Poor => "Poor (10-25%)",
            Self::Critical => "Critical (25%+)",
            Self::Offline => "Offline (100%)",
            Self::Insufficient => "Insufficient Data",
        }
    }
}

/// Network health summary optimized for dashboard displays
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkHealthSummary {
    /// Overall network health score (0-100)
    pub health_score: f64,
    /// Primary status for dashboard
    pub status: NetworkStatus,
    /// Key metrics for dashboard cards
    pub key_metrics: DashboardMetrics,
    /// Alert conditions
    pub alerts: Vec<NetworkAlert>,
}

/// Network status for easy dashboard display
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum NetworkStatus {
    Healthy,
    Warning,
    Critical,
    Degraded,
}

impl NetworkStatus {
    pub fn color_hex(&self) -> &'static str {
        match self {
            Self::Healthy => "#22c55e",
            Self::Warning => "#eab308", 
            Self::Critical => "#ef4444",
            Self::Degraded => "#f97316",
        }
    }
}

/// Key metrics formatted for dashboard cards
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DashboardMetrics {
    /// Network skip rate with trend indicator
    pub network_skip_rate: MetricCard,
    /// Active validators count with trend
    pub active_validators: MetricCard,
    /// Network efficiency with trend
    pub network_efficiency: MetricCard,
    /// Concerning validators count with trend
    pub concerning_validators: MetricCard,
}

/// Individual metric card data
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetricCard {
    /// Current value
    pub value: String,
    /// Previous value for comparison (optional)
    pub previous_value: Option<String>,
    /// Trend direction
    pub trend: TrendDirection,
    /// Color indicator
    pub color: String,
    /// Additional context
    pub subtitle: String,
}

/// Trend indicators for metrics
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TrendDirection {
    Up,
    Down,
    Stable,
    Unknown,
}

/// Alert conditions for monitoring
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NetworkAlert {
    /// Alert severity
    pub severity: AlertSeverity,
    /// Alert message
    pub message: String,
    /// When the alert was triggered
    pub triggered_at: DateTime<Utc>,
    /// Alert category
    pub category: AlertCategory,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AlertCategory {
    SkipRate,
    ValidatorCount,
    NetworkEfficiency,
    Performance,
}

impl std::fmt::Display for AlertCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SkipRate => write!(f, "Skip Rate"),
            Self::ValidatorCount => write!(f, "Validator Count"),
            Self::NetworkEfficiency => write!(f, "Network Efficiency"),
            Self::Performance => write!(f, "Performance"),
        }
    }
}

/// Complete block production data for production use
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockProductionData {
    /// List of validator skip rates
    pub validators: Vec<ValidatorSkipRate>,
    /// Aggregated statistics
    pub statistics: SkipRateStatistics,
    /// Distribution data for plotting
    pub distribution: SkipRateDistribution,
    /// Network health summary for dashboards
    pub network_health: NetworkHealthSummary,
    /// Performance snapshots for time-series tracking
    pub performance_snapshots: Vec<ValidatorPerformanceSnapshot>,
    /// Slot range for the data
    pub slot_range: SlotRange,
    /// When the data was fetched
    pub fetched_at: DateTime<Utc>,
}

/// Debug version with additional raw data
#[derive(Debug, Serialize, Deserialize)]
pub struct BlockProductionDataDebug {
    /// Production data
    pub production_data: BlockProductionData,
    /// Raw RPC response for debugging
    pub raw_rpc_data: serde_json::Value,
    /// Request parameters used
    pub request_params: serde_json::Value,
    /// Response metadata
    pub response_metadata: ResponseMetadata,
}

/// Metadata about the RPC response
#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// RPC endpoint used
    pub rpc_endpoint: String,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Number of retry attempts made
    pub retry_attempts: u32,
    /// Whether rate limiting was applied
    pub rate_limited: bool,
}

/// Request parameters for getBlockProduction
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockProductionRequest {
    /// Specific slot range (optional)
    pub range: Option<SlotRange>,
    /// Commitment level (optional) - "processed", "confirmed", or "finalized"
    pub commitment: Option<String>,
}

impl Default for BlockProductionRequest {
    fn default() -> Self {
        Self {
            range: None,
            commitment: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validator_skip_rate_creation() {
        let validator = ValidatorSkipRate::new(
            "test_validator".to_string(),
            100,
            95,
        );

        assert_eq!(validator.pubkey, "test_validator");
        assert_eq!(validator.leader_slots, 100);
        assert_eq!(validator.blocks_produced, 95);
        assert_eq!(validator.missed_slots, 5);
        assert_eq!(validator.skip_rate_percent, 5.0);
    }

    #[test]
    fn test_validator_skip_rate_perfect() {
        let perfect = ValidatorSkipRate::new(
            "perfect".to_string(),
            100,
            100,
        );

        assert!(perfect.is_perfect());
        assert!(!perfect.is_concerning());
        assert_eq!(perfect.skip_rate_percent, 0.0);
    }

    #[test]
    fn test_validator_skip_rate_concerning() {
        let concerning = ValidatorSkipRate::new(
            "concerning".to_string(),
            100,
            85,
        );

        assert!(!concerning.is_perfect());
        assert!(concerning.is_concerning());
        assert_eq!(concerning.skip_rate_percent, 15.0);
    }

    #[test]
    fn test_validator_skip_rate_edge_cases() {
        // Zero leader slots
        let zero_slots = ValidatorSkipRate::new(
            "zero".to_string(),
            0,
            0,
        );
        assert_eq!(zero_slots.skip_rate_percent, 0.0);
        assert!(!zero_slots.is_perfect()); // Not perfect because no slots
        assert!(!zero_slots.is_concerning());

        // Impossible case (blocks > leader slots) - should be handled gracefully
        let impossible = ValidatorSkipRate::new(
            "impossible".to_string(),
            50,
            100,
        );
        assert_eq!(impossible.missed_slots, 0); // saturating_sub prevents underflow
    }

    #[test]
    fn test_validator_significance_categories() {
        let low_activity = ValidatorSkipRate::new("low".to_string(), 5, 5);
        assert!(low_activity.is_low_activity());
        assert!(!low_activity.is_significant());
        assert!(!low_activity.is_high_stake());

        let significant = ValidatorSkipRate::new("significant".to_string(), 100, 95);
        assert!(!significant.is_low_activity());
        assert!(significant.is_significant());
        assert!(!significant.is_high_stake());

        let high_stake = ValidatorSkipRate::new("high_stake".to_string(), 2000, 1900);
        assert!(!high_stake.is_low_activity());
        assert!(high_stake.is_significant());
        assert!(high_stake.is_high_stake());
    }

    #[test]
    fn test_validator_offline_detection() {
        let offline = ValidatorSkipRate::new("offline".to_string(), 100, 0);
        assert!(offline.is_offline());
        assert_eq!(offline.skip_rate_percent, 100.0);

        let online = ValidatorSkipRate::new("online".to_string(), 100, 95);
        assert!(!online.is_offline());
    }

    #[test]
    fn test_validator_significance_weight() {
        let zero_slots = ValidatorSkipRate::new("zero".to_string(), 0, 0);
        assert_eq!(zero_slots.significance_weight(), 0.0);

        let small = ValidatorSkipRate::new("small".to_string(), 10, 10);
        assert_eq!(small.significance_weight(), 0.1);

        let significant = ValidatorSkipRate::new("significant".to_string(), 100, 95);
        let weight = significant.significance_weight();
        assert!(weight > 0.1);
        assert!(weight < 1.0); // Should be reasonable
    }

    #[test]
    fn test_slot_range() {
        let range = SlotRange {
            first_slot: 1000,
            last_slot: 2000,
        };
        assert_eq!(range.slot_count(), 1000);

        // Edge case: same slot
        let same = SlotRange {
            first_slot: 1000,
            last_slot: 1000,
        };
        assert_eq!(same.slot_count(), 0);
    }

    #[test]
    fn test_validator_performance_category() {
        assert_eq!(
            ValidatorPerformanceCategory::from_skip_rate(0.0, 100),
            ValidatorPerformanceCategory::Perfect
        );

        assert_eq!(
            ValidatorPerformanceCategory::from_skip_rate(0.5, 100),
            ValidatorPerformanceCategory::Excellent
        );

        assert_eq!(
            ValidatorPerformanceCategory::from_skip_rate(2.0, 100),
            ValidatorPerformanceCategory::Good
        );

        assert_eq!(
            ValidatorPerformanceCategory::from_skip_rate(4.0, 100),
            ValidatorPerformanceCategory::Average
        );

        assert_eq!(
            ValidatorPerformanceCategory::from_skip_rate(7.0, 100),
            ValidatorPerformanceCategory::Concerning
        );

        assert_eq!(
            ValidatorPerformanceCategory::from_skip_rate(15.0, 100),
            ValidatorPerformanceCategory::Poor
        );

        assert_eq!(
            ValidatorPerformanceCategory::from_skip_rate(50.0, 100),
            ValidatorPerformanceCategory::Critical
        );

        assert_eq!(
            ValidatorPerformanceCategory::from_skip_rate(100.0, 100),
            ValidatorPerformanceCategory::Offline
        );

        assert_eq!(
            ValidatorPerformanceCategory::from_skip_rate(5.0, 5),
            ValidatorPerformanceCategory::Insufficient
        );
    }

    #[test]
    fn test_performance_category_display() {
        let perfect = ValidatorPerformanceCategory::Perfect;
        assert_eq!(perfect.display_label(), "Perfect (0%)");
        assert_eq!(perfect.color_hex(), "#22c55e");

        let concerning = ValidatorPerformanceCategory::Concerning;
        assert_eq!(concerning.display_label(), "Concerning (5-10%)");
        assert_eq!(concerning.color_hex(), "#ef4444");
    }

    #[test]
    fn test_network_status() {
        let healthy = NetworkStatus::Healthy;
        assert_eq!(healthy.color_hex(), "#22c55e");

        let critical = NetworkStatus::Critical;
        assert_eq!(critical.color_hex(), "#ef4444");
    }

    #[test]
    fn test_alert_category_display() {
        let skip_rate = AlertCategory::SkipRate;
        assert_eq!(format!("{}", skip_rate), "Skip Rate");

        let performance = AlertCategory::Performance;
        assert_eq!(format!("{}", performance), "Performance");
    }

    #[test]
    fn test_block_production_request_default() {
        let request = BlockProductionRequest::default();
        assert!(request.range.is_none());
        assert!(request.commitment.is_none());
    }

    #[test]
    fn test_rpc_response_structure() {
        // Test the internal RPC response structures
        let by_identity = std::collections::HashMap::new();
        let range = SlotRange {
            first_slot: 1000,
            last_slot: 2000,
        };

        let value = BlockProductionValue {
            by_identity,
            range: range.clone(),
        };

        let result = BlockProductionResult { value };
        let response = RpcResponse { result };

        // Verify structure can be created
        assert_eq!(response.result.value.range.first_slot, 1000);
        assert_eq!(response.result.value.range.last_slot, 2000);
    }

    #[test]
    fn test_serialization() {
        // Test that our main types can be serialized/deserialized
        let validator = ValidatorSkipRate::new(
            "test".to_string(),
            100,
            95,
        );

        let json = serde_json::to_string(&validator).unwrap();
        let deserialized: ValidatorSkipRate = serde_json::from_str(&json).unwrap();

        assert_eq!(validator.pubkey, deserialized.pubkey);
        assert_eq!(validator.skip_rate_percent, deserialized.skip_rate_percent);
    }

    #[test]
    fn test_percentile_data() {
        let percentile = PercentileData {
            percentile: 95,
            skip_rate_percent: 10.5,
        };

        assert_eq!(percentile.percentile, 95);
        assert_eq!(percentile.skip_rate_percent, 10.5);
    }

    #[test]
    fn test_distribution_bucket() {
        let bucket = DistributionBucket {
            range_label: "5-10%".to_string(),
            min_percent: 5.0,
            max_percent: 10.0,
            validator_count: 15,
            percentage_of_total: 7.5,
            total_slots: 1500,
        };

        assert_eq!(bucket.range_label, "5-10%");
        assert_eq!(bucket.validator_count, 15);
    }
}