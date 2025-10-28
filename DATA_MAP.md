# Data Provider Map

This library processes Solana block production data and outputs structured records for PostgreSQL ingestion. All data structures are designed for change-detection storage patterns.

## Primary Data Output

The main function `fetch_block_production()` returns a `BlockProductionData` struct containing all processed data:

```rust
pub struct BlockProductionData {
    pub validators: Vec<ValidatorSkipRate>,           // Individual validator records
    pub statistics: SkipRateStatistics,              // Network-wide aggregated stats  
    pub distribution: SkipRateDistribution,          // Distribution analysis
    pub network_health: NetworkHealthSummary,        // Health indicators
    pub performance_snapshots: Vec<ValidatorPerformanceSnapshot>, // Performance categorization
    pub slot_range: SlotRange,                       // Slot range metadata
    pub fetched_at: DateTime<Utc>,                   // Timestamp for record tracking
}
```

## Database Table Schemas

### 1. Validator Records (`validators` field)

**Table: `validator_skip_rates`**
```sql
CREATE TABLE validator_skip_rates (
    id SERIAL PRIMARY KEY,
    pubkey VARCHAR(44) NOT NULL,           -- Validator public key
    leader_slots BIGINT NOT NULL,          -- Assigned leader slots
    blocks_produced BIGINT NOT NULL,       -- Successfully produced blocks
    missed_slots BIGINT NOT NULL,          -- Missed slots (calculated)
    skip_rate_percent DOUBLE PRECISION,    -- Skip rate percentage
    first_slot BIGINT NOT NULL,            -- Slot range start
    last_slot BIGINT NOT NULL,             -- Slot range end
    fetched_at TIMESTAMP WITH TIME ZONE,   -- Data collection timestamp
    
    UNIQUE(pubkey, first_slot, last_slot)  -- Only record changes
);
```

**Rust Data Structure:**
```rust
pub struct ValidatorSkipRate {
    pub pubkey: String,                    // VARCHAR(44)
    pub leader_slots: u64,                 // BIGINT
    pub blocks_produced: u64,              // BIGINT
    pub missed_slots: u64,                 // BIGINT (calculated field)
    pub skip_rate_percent: f64,            // DOUBLE PRECISION
}
```

### 2. Network Statistics (`statistics` field)

**Table: `network_statistics`**
```sql
CREATE TABLE network_statistics (
    id SERIAL PRIMARY KEY,
    first_slot BIGINT NOT NULL,
    last_slot BIGINT NOT NULL,
    fetched_at TIMESTAMP WITH TIME ZONE,
    
    -- Basic metrics
    total_validators INTEGER,
    total_leader_slots BIGINT,
    total_blocks_produced BIGINT,
    total_missed_slots BIGINT,
    
    -- Skip rate analysis
    overall_skip_rate_percent DOUBLE PRECISION,
    average_skip_rate_percent DOUBLE PRECISION,
    median_skip_rate_percent DOUBLE PRECISION,
    weighted_skip_rate_percent DOUBLE PRECISION,
    significant_validators_skip_rate_percent DOUBLE PRECISION,
    high_stake_skip_rate_percent DOUBLE PRECISION,
    
    -- Validator categorization
    perfect_validators INTEGER,           -- 0% skip rate
    concerning_validators INTEGER,        -- >5% skip rate
    offline_validators INTEGER,           -- 100% skip rate
    low_activity_validators INTEGER,      -- <50 slots
    high_activity_validators INTEGER,     -- >1000 slots
    significant_validators INTEGER,       -- >=50 slots
    
    -- Percentile analysis
    skip_rate_90th_percentile DOUBLE PRECISION,
    skip_rate_95th_percentile DOUBLE PRECISION,
    significant_skip_rate_90th_percentile DOUBLE PRECISION,
    significant_skip_rate_95th_percentile DOUBLE PRECISION,
    
    -- Network efficiency
    network_efficiency_percent DOUBLE PRECISION,
    weighted_network_efficiency_percent DOUBLE PRECISION,
    
    UNIQUE(first_slot, last_slot)
);
```

### 3. Distribution Data (`distribution` field)

**Table: `skip_rate_distribution`**
```sql
CREATE TABLE skip_rate_distribution (
    id SERIAL PRIMARY KEY,
    first_slot BIGINT NOT NULL,
    last_slot BIGINT NOT NULL,
    fetched_at TIMESTAMP WITH TIME ZONE,
    
    -- Distribution buckets (JSON)
    buckets JSONB,                        -- Array of distribution buckets
    percentiles JSONB,                    -- Array of percentile data
    plot_data JSONB,                      -- Ready-to-plot arrays
    
    UNIQUE(first_slot, last_slot)
);
```

**Bucket Structure (JSONB):**
```json
{
  "range_start": 0.0,
  "range_end": 1.0,
  "range_label": "0-1%",
  "validator_count": 245,
  "percentage_of_total": 65.2
}
```

### 4. Network Health (`network_health` field)

**Table: `network_health`**
```sql
CREATE TABLE network_health (
    id SERIAL PRIMARY KEY,
    first_slot BIGINT NOT NULL,
    last_slot BIGINT NOT NULL,
    fetched_at TIMESTAMP WITH TIME ZONE,
    
    health_score DOUBLE PRECISION,        -- 0-100 health score
    status VARCHAR(20),                   -- Healthy/Warning/Critical
    key_metrics JSONB,                    -- Dashboard metrics
    alerts JSONB,                         -- Active alerts array
    
    UNIQUE(first_slot, last_slot)
);
```

### 5. Performance Snapshots (`performance_snapshots` field)

**Table: `validator_performance`**
```sql
CREATE TABLE validator_performance (
    id SERIAL PRIMARY KEY,
    validator_pubkey VARCHAR(44) NOT NULL,
    first_slot BIGINT NOT NULL,
    last_slot BIGINT NOT NULL,
    timestamp_collected TIMESTAMP WITH TIME ZONE,
    
    skip_rate_percent DOUBLE PRECISION,
    leader_slots BIGINT,
    performance_category VARCHAR(20),     -- Perfect/Excellent/Good/Average/Concerning/Poor/Critical/Offline
    
    UNIQUE(validator_pubkey, first_slot, last_slot)
);
```

## Change Detection Strategy

Since you only record changes, implement these strategies:

### 1. Hash-Based Change Detection
```rust
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

impl BlockProductionData {
    pub fn calculate_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        // Hash only the changing data, not timestamps
        self.validators.hash(&mut hasher);
        self.statistics.hash(&mut hasher);
        hasher.finish()
    }
}
```

### 2. Differential Updates
```sql
-- Example: Only insert validator records if skip rate changed by >0.1%
INSERT INTO validator_skip_rates (pubkey, skip_rate_percent, ...)
SELECT * FROM new_data
WHERE NOT EXISTS (
    SELECT 1 FROM validator_skip_rates v 
    WHERE v.pubkey = new_data.pubkey 
    AND ABS(v.skip_rate_percent - new_data.skip_rate_percent) < 0.1
);
```

## Data Collection Pattern

```rust
use blocks_production_lib::BlockProductionClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .public_rpc_config()
        .build()?;
    
    // Fetch structured data
    let data = client.fetch_block_production().await?;
    
    // Data is ready for PostgreSQL ingestion
    // - data.validators -> validator_skip_rates table
    // - data.statistics -> network_statistics table  
    // - data.distribution -> skip_rate_distribution table
    // - data.network_health -> network_health table
    // - data.performance_snapshots -> validator_performance table
    
    println!("Collected {} validator records", data.validators.len());
    println!("Slot range: {} to {}", data.slot_range.first_slot, data.slot_range.last_slot);
    
    Ok(())
}
```

## Raw JSON Output Sample

For reference, here's what the raw data looks like:

```json
{
  "validators": [
    {
      "pubkey": "7Np41oeYqPefeNQEHSv1UDhYrehxin3NStELsSKCT4K2",
      "leader_slots": 152,
      "blocks_produced": 150,
      "missed_slots": 2,
      "skip_rate_percent": 1.32
    }
  ],
  "statistics": {
    "total_validators": 1842,
    "total_leader_slots": 432000,
    "total_blocks_produced": 427890,
    "overall_skip_rate_percent": 0.95,
    "perfect_validators": 1205,
    "concerning_validators": 23
  },
  "slot_range": {
    "first_slot": 280000000,
    "last_slot": 280432000
  },
  "fetched_at": "2025-10-28T15:30:45.123Z"
}
```

## Key Design Decisions for Data Providers

1. **Immutable Records**: Each fetch represents a point-in-time snapshot
2. **Calculated Fields**: Skip rates and percentages pre-calculated for efficiency  
3. **Typed Data**: All numeric fields properly typed (u64, f64) for database precision
4. **Timestamp Tracking**: `fetched_at` field for temporal ordering
5. **Slot Range Context**: Every record includes slot range for data correlation
6. **No Business Logic**: Pure data transformation, no application-specific processing

This library transforms raw Solana RPC data into structured records for PostgreSQL ingestion.