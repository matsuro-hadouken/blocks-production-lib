use crate::{
    config::ClientConfig,
    error::{BlockProductionError, Result, TimeoutType, AuthErrorType},
    types::{BlockProductionData, BlockProductionRequest, BlockProductionDataDebug, ResponseMetadata, ValidatorSkipRate, SlotRange, RpcResponse, SkipRateStatistics, SkipRateDistribution, DistributionBucket, PercentileData, DistributionPlotData, NetworkHealthSummary, NetworkStatus, DashboardMetrics, MetricCard, TrendDirection, NetworkAlert, AlertSeverity, AlertCategory, ValidatorPerformanceSnapshot, ValidatorPerformanceCategory},
};
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde_json::json;
use std::time::{Duration, Instant};
use tokio::time::timeout;
use tracing::{debug, error, info, warn, trace, instrument};

/// Client for fetching Solana block production data and calculating skip rates
#[derive(Debug)]
pub struct BlockProductionClient {
    config: ClientConfig,
    http_client: Client,
}

impl BlockProductionClient {
    /// Create a new client with default configuration
    #[instrument(fields(endpoint = rpc_endpoint))]
    pub fn new(rpc_endpoint: &str) -> Result<Self> {
        info!(endpoint = rpc_endpoint, "Creating new BlockProductionClient");
        
        // Validate RPC endpoint format
        if rpc_endpoint.is_empty() {
            return Err(BlockProductionError::config_error(
                "RPC endpoint cannot be empty",
                Some("rpc_endpoint"),
                Some("Provide a valid Solana RPC endpoint URL"),
            ));
        }

        if !rpc_endpoint.starts_with("http://") && !rpc_endpoint.starts_with("https://") {
            return Err(BlockProductionError::config_error(
                "RPC endpoint must start with http:// or https://",
                Some("rpc_endpoint"),
                Some("Use a complete URL like https://api.mainnet-beta.solana.com"),
            ));
        }

        let config = ClientConfig::builder()
            .rpc_endpoint(rpc_endpoint.to_string())
            .build();
        
        Self::from_config(config)
    }

    /// Create a client from configuration
    #[instrument(skip(config), fields(endpoint = %config.rpc_endpoint))]
    pub fn from_config(config: ClientConfig) -> Result<Self> {
        debug!(
            endpoint = %config.rpc_endpoint,
            timeout_ms = config.timeout.as_millis(),
            retry_attempts = config.retry_attempts,
            "Creating client from configuration"
        );

        let mut headers = reqwest::header::HeaderMap::new();
        
        // Add custom headers with validation
        for (key, value) in &config.headers {
            trace!(header_name = key, "Adding custom header");
            
            let header_name = reqwest::header::HeaderName::from_bytes(key.as_bytes())
                .map_err(|e| BlockProductionError::config_error(
                    &format!("Invalid header name '{key}': {e}"),
                    Some("headers"),
                    Some("Use valid HTTP header names (alphanumeric and hyphens)"),
                ))?;
                
            let header_value = reqwest::header::HeaderValue::from_str(value)
                .map_err(|e| BlockProductionError::config_error(
                    &format!("Invalid header value '{value}': {e}"),
                    Some("headers"),
                    Some("Header values must be valid ASCII"),
                ))?;
                
            headers.insert(header_name, header_value);
        }

        debug!(header_count = headers.len(), "Added custom headers");

        let http_client = Client::builder()
            .timeout(config.timeout)
            .default_headers(headers)
            .user_agent("blocks-production-lib/0.1.0")
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()
            .map_err(|e| BlockProductionError::config_error(
                &format!("Failed to create HTTP client: {e}"),
                None,
                Some("Check timeout and header configuration"),
            ))?;

        info!(
            endpoint = %config.rpc_endpoint,
            "Successfully created BlockProductionClient"
        );

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Create a client builder for custom configuration
    pub fn builder() -> ClientBuilder {
        debug!("Creating client builder");
        ClientBuilder::new()
    }

    /// Test RPC endpoint connectivity
    #[instrument(skip(self), fields(endpoint = %self.config.rpc_endpoint))]
    pub async fn test_connection(&self) -> Result<bool> {
        info!("Testing RPC endpoint connectivity");
        
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getHealth"
        });

        let start_time = Instant::now();
        
        match self.make_rpc_request(request).await {
            Ok(response) => {
                let duration = start_time.elapsed();
                let has_result = response.get("result").is_some();
                
                if has_result {
                    info!(
                        response_time_ms = duration.as_millis(),
                        "RPC endpoint is healthy"
                    );
                } else {
                    warn!(
                        response_time_ms = duration.as_millis(),
                        response = %response,
                        "RPC endpoint responded but without expected result"
                    );
                }
                
                Ok(has_result)
            },
            Err(e) => {
                error!(
                    error = %e,
                    duration_ms = start_time.elapsed().as_millis(),
                    "RPC endpoint connectivity test failed"
                );
                
                // Return a more informative error for connection tests
                Err(BlockProductionError::ConnectionFailed {
                    endpoint: self.config.rpc_endpoint.clone(),
                    source: Box::new(e),
                    endpoint_reachable: Some(false),
                })
            }
        }
    }

    /// Fetch block production data for all validators
    pub async fn fetch_block_production(&self) -> Result<BlockProductionData> {
        self.fetch_block_production_with_params(BlockProductionRequest::default())
            .await
    }

    /// Fetch block production data with specific parameters
    pub async fn fetch_block_production_with_params(
        &self,
        params: BlockProductionRequest,
    ) -> Result<BlockProductionData> {
        let start_time = Instant::now();
        let rpc_response = self.fetch_raw_block_production(&params).await?;
        
        let production_data = Self::process_block_production_response(rpc_response, start_time)?;
        Ok(production_data)
    }

    /// Fetch block production data in debug format with raw RPC data
    pub async fn fetch_block_production_debug(
        &self,
        params: BlockProductionRequest,
    ) -> Result<BlockProductionDataDebug> {
        let start_time = Instant::now();
        let request_json = Self::build_rpc_request(&params);
        let rpc_response = self.fetch_raw_block_production(&params).await?;
        
        let production_data = Self::process_block_production_response(rpc_response.clone(), start_time)?;
        #[allow(clippy::cast_possible_truncation)]
        let response_time = start_time.elapsed().as_millis() as u64;

        Ok(BlockProductionDataDebug {
            production_data,
            raw_rpc_data: rpc_response,
            request_params: request_json,
            response_metadata: ResponseMetadata {
                rpc_endpoint: self.config.rpc_endpoint.clone(),
                response_time_ms: response_time,
                retry_attempts: 0, // TODO: Track actual retry attempts
                rate_limited: false, // TODO: Track if rate limiting was applied
            },
        })
    }

    /// Fetch skip rates for specific validators only
    pub async fn fetch_validator_skip_rates(
        &self,
        validator_pubkeys: Vec<String>,
    ) -> Result<Vec<ValidatorSkipRate>> {
        // Since getBlockProduction doesn't support filtering by identity,
        // we fetch all data and filter client-side
        let data = self.fetch_block_production().await?;
        
        let validator_set: std::collections::HashSet<String> = validator_pubkeys.into_iter().collect();
        let filtered_validators: Vec<ValidatorSkipRate> = data.validators
            .into_iter()
            .filter(|v| validator_set.contains(&v.pubkey))
            .collect();
            
        Ok(filtered_validators)
    }

    /// Fetch block production data for a specific slot range
    pub async fn fetch_block_production_range(
        &self,
        first_slot: u64,
        last_slot: u64,
    ) -> Result<BlockProductionData> {
        if first_slot >= last_slot {
            return Err(BlockProductionError::InvalidSlotRange {
                message: format!(
                    "Invalid slot range: first_slot ({first_slot}) must be less than last_slot ({last_slot})"
                ),
                provided_range: Some((first_slot, last_slot)),
                valid_range: None,
            });
        }

        let params = BlockProductionRequest {
            range: Some(SlotRange { first_slot, last_slot }),
            commitment: None,
        };

        self.fetch_block_production_with_params(params).await
    }

    /// Get validators with concerning skip rates (> 5%)
    pub async fn get_concerning_validators(&self) -> Result<Vec<ValidatorSkipRate>> {
        let data = self.fetch_block_production().await?;
        Ok(data.validators.into_iter()
            .filter(super::types::ValidatorSkipRate::is_concerning)
            .collect())
    }

    /// Get validators with perfect performance (0% skip rate)
    pub async fn get_perfect_validators(&self) -> Result<Vec<ValidatorSkipRate>> {
        let data = self.fetch_block_production().await?;
        Ok(data.validators.into_iter()
            .filter(super::types::ValidatorSkipRate::is_perfect)
            .collect())
    }

    /// Get validators that are completely offline (100% skip rate)
    pub async fn get_offline_validators(&self) -> Result<Vec<ValidatorSkipRate>> {
        let data = self.fetch_block_production().await?;
        Ok(data.validators.into_iter()
            .filter(super::types::ValidatorSkipRate::is_offline)
            .collect())
    }

    /// Get significant validators (>= 50 slots) - these represent real network participants
    pub async fn get_significant_validators(&self) -> Result<Vec<ValidatorSkipRate>> {
        let data = self.fetch_block_production().await?;
        let mut validators: Vec<ValidatorSkipRate> = data.validators.into_iter()
            .filter(super::types::ValidatorSkipRate::is_significant)
            .collect();
        
        // Sort by skip rate (ascending) - better performers first
        validators.sort_by(|a, b| a.skip_rate_percent.partial_cmp(&b.skip_rate_percent).unwrap_or(std::cmp::Ordering::Equal));
        Ok(validators)
    }

    /// Get validators with moderate skip rates (between 1% and 5%) - these are the interesting ones
    pub async fn get_moderate_performers(&self) -> Result<Vec<ValidatorSkipRate>> {
        let data = self.fetch_block_production().await?;
        let mut validators: Vec<ValidatorSkipRate> = data.validators.into_iter()
            .filter(|v| v.skip_rate_percent > 0.0 && v.skip_rate_percent <= 5.0)
            .collect();
        
        // Sort by skip rate (ascending) to show best moderate performers first
        validators.sort_by(|a, b| a.skip_rate_percent.partial_cmp(&b.skip_rate_percent).unwrap_or(std::cmp::Ordering::Equal));
        Ok(validators)
    }

    /// Get high-activity validators (>1000 leader slots) sorted by skip rate - these are the important ones
    pub async fn get_high_activity_validators(&self) -> Result<Vec<ValidatorSkipRate>> {
        let data = self.fetch_block_production().await?;
        let mut validators: Vec<ValidatorSkipRate> = data.validators.into_iter()
            .filter(|v| v.leader_slots > 1000)
            .collect();
        
        // Sort by skip rate (ascending) - lower skip rate = better performance
        validators.sort_by(|a, b| a.skip_rate_percent.partial_cmp(&b.skip_rate_percent).unwrap_or(std::cmp::Ordering::Equal));
        Ok(validators)
    }

    /// Get validators in the worst percentile (95th percentile and above) - actionable for stake removal
    pub async fn get_worst_percentile_validators(&self) -> Result<Vec<ValidatorSkipRate>> {
        let data = self.fetch_block_production().await?;
        let percentile_95 = data.statistics.skip_rate_95th_percentile;
        
        let mut validators: Vec<ValidatorSkipRate> = data.validators.into_iter()
            .filter(|v| v.skip_rate_percent >= percentile_95 && v.skip_rate_percent < 100.0) // Exclude completely offline
            .collect();
        
        // Sort by skip rate (descending) - worst first
        validators.sort_by(|a, b| b.skip_rate_percent.partial_cmp(&a.skip_rate_percent).unwrap_or(std::cmp::Ordering::Equal));
        Ok(validators)
    }

    // Internal methods

    async fn fetch_raw_block_production(
        &self,
        params: &BlockProductionRequest,
    ) -> Result<serde_json::Value> {
        let request = Self::build_rpc_request(params);
        self.make_rpc_request(request).await
    }

    fn build_rpc_request(params: &BlockProductionRequest) -> serde_json::Value {
        let mut rpc_params = serde_json::Map::new();

        if let Some(range) = &params.range {
            rpc_params.insert(
                "range".to_string(),
                json!({
                    "firstSlot": range.first_slot,
                    "lastSlot": range.last_slot
                }),
            );
        }

        if let Some(commitment) = &params.commitment {
            rpc_params.insert("commitment".to_string(), json!(commitment));
        }

        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "getBlockProduction",
            "params": if rpc_params.is_empty() { json!([]) } else { json!([rpc_params]) }
        })
    }

    #[instrument(skip(self, request), fields(endpoint = %self.config.rpc_endpoint))]
    async fn make_rpc_request(&self, request: serde_json::Value) -> Result<serde_json::Value> {
        let request_id = request.get("id").and_then(serde_json::Value::as_u64).unwrap_or(0);
        let method = request.get("method").and_then(|v| v.as_str()).unwrap_or("unknown");
        
        info!(
            request_id = request_id,
            method = method,
            endpoint = %self.config.rpc_endpoint,
            "Starting RPC request"
        );
        
        // Apply rate limiting if configured
        if let Some(rate_limiter) = &self.config.rate_limiter {
            debug!("Checking rate limiter");
            let start_wait = Instant::now();
            rate_limiter.until_ready().await;
            let wait_duration = start_wait.elapsed();
            
            if wait_duration > Duration::from_millis(10) {
                debug!(
                    wait_duration_ms = wait_duration.as_millis(),
                    "Rate limiter delayed request"
                );
            }
        } else {
            trace!("No rate limiting configured");
        }

        let mut error_history = Vec::new();
        let total_start = Instant::now();
        let max_attempts = self.config.retry_attempts;

        for attempt in 1..=max_attempts {
            let attempt_start = Instant::now();
            
            debug!(
                attempt = attempt,
                max_attempts = max_attempts,
                timeout_ms = self.config.timeout.as_millis(),
                "Attempting RPC request"
            );

            // Make the HTTP request with timeout
            let http_result = timeout(
                self.config.timeout,
                self.http_client
                    .post(&self.config.rpc_endpoint)
                    .json(&request)
                    .send(),
            ).await;

            let response = match http_result {
                Ok(Ok(response)) => {
                    debug!(
                        status = response.status().as_u16(),
                        attempt_duration_ms = attempt_start.elapsed().as_millis(),
                        "HTTP request completed"
                    );
                    response
                },
                Ok(Err(e)) => {
                    let error_msg = format!("HTTP error on attempt {attempt}: {e}");
                    error_history.push(error_msg.clone());
                    
                    warn!(
                        attempt = attempt,
                        error = %e,
                        attempt_duration_ms = attempt_start.elapsed().as_millis(),
                        "HTTP request failed"
                    );

                    // Determine if this is a retryable error
                    if e.is_timeout() {
                        if attempt == max_attempts {
                            return Err(BlockProductionError::Timeout {
                                duration: self.config.timeout,
                                operation: format!("RPC {method} request"),
                                timeout_type: TimeoutType::Request,
                            });
                        }
                        continue;
                    } else if e.is_connect() {
                        if attempt == max_attempts {
                            return Err(BlockProductionError::ConnectionFailed {
                                endpoint: self.config.rpc_endpoint.clone(),
                                source: Box::new(e),
                                endpoint_reachable: None,
                            });
                        }
                        continue;
                    }
                    return Err(BlockProductionError::Http {
                        source: e,
                        context: Some(format!("RPC {method} request attempt {attempt}")),
                    });
                },
                Err(_) => {
                    let error_msg = format!("Request timeout on attempt {} after {:?}", attempt, self.config.timeout);
                    error_history.push(error_msg.clone());
                    
                    warn!(
                        attempt = attempt,
                        timeout_ms = self.config.timeout.as_millis(),
                        "Request timed out"
                    );

                    if attempt == max_attempts {
                        return Err(BlockProductionError::Timeout {
                            duration: self.config.timeout,
                            operation: format!("RPC {method} request"),
                            timeout_type: TimeoutType::Request,
                        });
                    }
                    continue;
                }
            };

            // Check HTTP status
            let status = response.status();
            if !status.is_success() {
                let error_msg = format!("HTTP {} error on attempt {}", status.as_u16(), attempt);
                error_history.push(error_msg.clone());
                
                warn!(
                    attempt = attempt,
                    status = status.as_u16(),
                    "HTTP request returned error status"
                );

                // Handle specific HTTP status codes
                match status.as_u16() {
                    429 => {
                        let retry_after = response
                            .headers()
                            .get("retry-after")
                            .and_then(|v| v.to_str().ok())
                            .and_then(|s| s.parse::<u64>().ok())
                            .map(Duration::from_secs);
                        
                        debug!(
                            retry_after_secs = retry_after.as_ref().map(std::time::Duration::as_secs),
                            "Rate limit exceeded, will retry"
                        );

                        if attempt == max_attempts {
                            return Err(BlockProductionError::RateLimit {
                                requests: 0, // Unknown from this context
                                window: Duration::from_secs(60), // Default window
                                limit: 0, // Unknown from this context  
                                retry_after,
                            });
                        }
                        
                        // Wait for retry delay
                        if let Some(delay) = retry_after {
                            tokio::time::sleep(delay).await;
                        } else {
                            tokio::time::sleep(Duration::from_millis(100 * u64::from(attempt))).await;
                        }
                        continue;
                    },
                    401 | 403 => {
                        error!(status = status.as_u16(), "Authentication failed");
                        return Err(BlockProductionError::Auth {
                            message: format!("HTTP {}: Authentication failed", status.as_u16()),
                            auth_type: if status.as_u16() == 401 {
                                AuthErrorType::InvalidApiKey
                            } else {
                                AuthErrorType::QuotaExceeded
                            },
                        });
                    },
                    500..=599 => {
                        // Server errors are retryable
                        if attempt == max_attempts {
                            return Err(BlockProductionError::Http {
                                source: response.error_for_status().unwrap_err(),
                                context: Some(format!("Server error on attempt {attempt}")),
                            });
                        }
                        continue;
                    },
                    _ => {
                        return Err(BlockProductionError::Http {
                            source: response.error_for_status().unwrap_err(),
                            context: Some(format!("HTTP error {} on attempt {}", status.as_u16(), attempt)),
                        });
                    }
                }
            }

            // Parse JSON response
            let json_response: serde_json::Value = match response.json::<serde_json::Value>().await {
                Ok(json) => {
                    debug!(
                        attempt = attempt,
                        response_size = json.to_string().len(),
                        attempt_duration_ms = attempt_start.elapsed().as_millis(),
                        "Successfully parsed JSON response"
                    );
                    json
                },
                Err(e) => {
                    let error_msg = format!("JSON parsing error on attempt {attempt}: {e}");
                    error_history.push(error_msg);
                    
                    error!(
                        attempt = attempt,
                        error = %e,
                        "Failed to parse JSON response"
                    );

                    return Err(BlockProductionError::ResponseParsing {
                        reason: format!("Invalid JSON response: {e}"),
                        response_sample: None,
                        expected_structure: Some("Valid JSON object with 'result' field".to_string()),
                    });
                }
            };

            // Check for RPC errors in response
            if let Some(error) = json_response.get("error") {
                #[allow(clippy::cast_possible_truncation)]
                let error_code = error.get("code").and_then(serde_json::Value::as_i64).unwrap_or(-1) as i32;
                let error_message = error.get("message").and_then(|v| v.as_str()).unwrap_or("Unknown RPC error");
                
                let error_msg = format!("RPC error {error_code} on attempt {attempt}: {error_message}");
                error_history.push(error_msg);
                
                warn!(
                    attempt = attempt,
                    rpc_error_code = error_code,
                    rpc_error_message = error_message,
                    "RPC returned error response"
                );

                return Err(BlockProductionError::Rpc {
                    code: error_code,
                    message: error_message.to_string(),
                    method: method.to_string(),
                    raw_response: Some(json_response.to_string()),
                });
            }

            // Success!
            info!(
                request_id = request_id,
                method = method,
                attempt = attempt,
                total_duration_ms = total_start.elapsed().as_millis(),
                attempt_duration_ms = attempt_start.elapsed().as_millis(),
                "RPC request completed successfully"
            );

            return Ok(json_response);
        }

        // If we get here, all attempts failed
        error!(
            request_id = request_id,
            method = method,
            total_attempts = max_attempts,
            total_duration_ms = total_start.elapsed().as_millis(),
            "All retry attempts exhausted"
        );

        Err(BlockProductionError::RetryExhausted {
            attempts: max_attempts,
            total_duration: total_start.elapsed(),
            last_error: Box::new(BlockProductionError::Timeout {
                duration: self.config.timeout,
                operation: format!("RPC {method} request"),
                timeout_type: TimeoutType::Request,
            }),
            error_history,
        })
    }

    fn process_block_production_response(
        response: serde_json::Value,
        _start_time: Instant,
    ) -> Result<BlockProductionData> {
        let rpc_response: RpcResponse = serde_json::from_value(response)?;
        let value = rpc_response.result.value;
        let slot_range = value.range.clone();

        if value.by_identity.is_empty() {
            return Err(BlockProductionError::NoData {
                requested_range: Some((slot_range.first_slot, slot_range.last_slot)),
                reason: Some("No block production data available for the specified slot range".to_string()),
            });
        }

        let timestamp = Utc::now();

        // Process validator data
        let mut validators: Vec<ValidatorSkipRate> = value
            .by_identity
            .into_iter()
            .map(|(pubkey, (leader_slots, blocks_produced))| {
                ValidatorSkipRate::new(pubkey, leader_slots, blocks_produced)
            })
            .collect();

        // Sort by skip rate (ascending - best performers first)
        validators.sort_by(|a, b| a.skip_rate_percent.partial_cmp(&b.skip_rate_percent).unwrap_or(std::cmp::Ordering::Equal));

        // Calculate all the data structures
        let statistics = Self::calculate_statistics(&validators);
        let distribution = Self::calculate_distribution(&validators);
        let network_health = Self::calculate_network_health(&statistics, &validators);
        let performance_snapshots = Self::create_performance_snapshots(&validators, &slot_range, timestamp);

        Ok(BlockProductionData {
            validators,
            statistics,
            distribution,
            network_health,
            performance_snapshots,
            slot_range,
            fetched_at: timestamp,
        })
    }

    #[allow(clippy::cast_precision_loss)]
    fn calculate_statistics(validators: &[ValidatorSkipRate]) -> SkipRateStatistics {
        let total_validators = validators.len();
        let total_leader_slots: u64 = validators.iter().map(|v| v.leader_slots).sum();
        let total_blocks_produced: u64 = validators.iter().map(|v| v.blocks_produced).sum();
        let total_missed_slots: u64 = validators.iter().map(|v| v.missed_slots).sum();

        // Basic network metrics
        let overall_skip_rate_percent = if total_leader_slots > 0 {
            (total_missed_slots as f64 / total_leader_slots as f64) * 100.0
        } else {
            0.0
        };

        let network_efficiency_percent = if total_leader_slots > 0 {
            (total_blocks_produced as f64 / total_leader_slots as f64) * 100.0
        } else {
            0.0
        };

        // Filter significant validators (>= 50 slots)
        let significant_validators: Vec<&ValidatorSkipRate> = validators.iter()
            .filter(|v| v.is_significant())
            .collect();

        // Filter high-stake validators (> 1000 slots)
        let high_stake_validators: Vec<&ValidatorSkipRate> = validators.iter()
            .filter(|v| v.is_high_stake())
            .collect();

        // Calculate weighted metrics (using slot counts as weights, but capped to prevent dominance)
        let mut weighted_skip_sum = 0.0;
        let mut total_weight = 0.0;
        
        for validator in &significant_validators {
            let weight = validator.significance_weight();
            weighted_skip_sum += validator.skip_rate_percent * weight;
            total_weight += weight;
        }
        
        let weighted_skip_rate_percent = if total_weight > 0.0 {
            weighted_skip_sum / total_weight
        } else {
            0.0
        };

        // Calculate significant validators metrics
        let significant_total_slots: u64 = significant_validators.iter().map(|v| v.leader_slots).sum();
        let significant_total_produced: u64 = significant_validators.iter().map(|v| v.blocks_produced).sum();
        let significant_total_missed: u64 = significant_validators.iter().map(|v| v.missed_slots).sum();
        
        let significant_validators_skip_rate_percent = if significant_total_slots > 0 {
            (significant_total_missed as f64 / significant_total_slots as f64) * 100.0
        } else {
            0.0
        };

        let weighted_network_efficiency_percent = if significant_total_slots > 0 {
            (significant_total_produced as f64 / significant_total_slots as f64) * 100.0
        } else {
            0.0
        };

        // Calculate high-stake metrics
        let high_stake_total_slots: u64 = high_stake_validators.iter().map(|v| v.leader_slots).sum();
        let high_stake_total_missed: u64 = high_stake_validators.iter().map(|v| v.missed_slots).sum();
        
        let high_stake_skip_rate_percent = if high_stake_total_slots > 0 {
            (high_stake_total_missed as f64 / high_stake_total_slots as f64) * 100.0
        } else {
            0.0
        };

        // Basic statistics
        let skip_rates: Vec<f64> = validators.iter().map(|v| v.skip_rate_percent).collect();
        let average_skip_rate_percent = if skip_rates.is_empty() {
            0.0
        } else {
            skip_rates.iter().sum::<f64>() / skip_rates.len() as f64
        };

        // Calculate median for all validators
        let mut sorted_rates = skip_rates;
        sorted_rates.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median_skip_rate_percent = if sorted_rates.is_empty() {
            0.0
        } else if sorted_rates.len() % 2 == 0 {
            let mid = sorted_rates.len() / 2;
            (sorted_rates[mid - 1] + sorted_rates[mid]) / 2.0
        } else {
            sorted_rates[sorted_rates.len() / 2]
        };

        // Calculate percentiles for all validators
        let skip_rate_90th_percentile = Self::calculate_percentile(&sorted_rates, 0.90);
        let skip_rate_95th_percentile = Self::calculate_percentile(&sorted_rates, 0.95);

        // Calculate percentiles for significant validators only
        let mut significant_skip_rates: Vec<f64> = significant_validators.iter()
            .map(|v| v.skip_rate_percent)
            .collect();
        significant_skip_rates.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        let significant_skip_rate_90th_percentile = Self::calculate_percentile(&significant_skip_rates, 0.90);
        let significant_skip_rate_95th_percentile = Self::calculate_percentile(&significant_skip_rates, 0.95);

        // Count different validator categories
        let perfect_validators = validators.iter().filter(|v| v.is_perfect()).count();
        let concerning_validators = validators.iter().filter(|v| v.is_concerning()).count();
        let offline_validators = validators.iter().filter(|v| v.is_offline()).count();
        let low_activity_validators = validators.iter().filter(|v| v.is_low_activity()).count();
        let high_activity_validators = validators.iter().filter(|v| v.is_high_stake()).count();
        let significant_validators_count = significant_validators.len();

        SkipRateStatistics {
            total_validators,
            total_leader_slots,
            total_blocks_produced,
            total_missed_slots,
            overall_skip_rate_percent,
            average_skip_rate_percent,
            median_skip_rate_percent,
            weighted_skip_rate_percent,
            significant_validators_skip_rate_percent,
            high_stake_skip_rate_percent,
            perfect_validators,
            concerning_validators,
            offline_validators,
            low_activity_validators,
            high_activity_validators,
            significant_validators: significant_validators_count,
            skip_rate_90th_percentile,
            skip_rate_95th_percentile,
            significant_skip_rate_90th_percentile,
            significant_skip_rate_95th_percentile,
            network_efficiency_percent,
            weighted_network_efficiency_percent,
        }
    }

    fn calculate_percentile(sorted_values: &[f64], percentile: f64) -> f64 {
        if sorted_values.is_empty() {
            return 0.0;
        }
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let index = ((sorted_values.len() as f64 - 1.0) * percentile) as usize;
        sorted_values[index.min(sorted_values.len() - 1)]
    }

    /// Calculate skip rate distribution for plotting
    #[allow(clippy::cast_precision_loss)]
    fn calculate_distribution(validators: &[ValidatorSkipRate]) -> SkipRateDistribution {
        // Define bucket ranges - separate Perfect (0%) from other performers
        let bucket_ranges = vec![
            (0.0, 0.0, "Perfect (0%)"),
            (0.0001, 1.0, "Excellent (0.1-1%)"),
            (1.0, 2.0, "Good (1-2%)"),
            (2.0, 5.0, "Average (2-5%)"),
            (5.0, 10.0, "Concerning (5-10%)"),
            (10.0, 25.0, "Poor (10-25%)"),
            (25.0, 50.0, "Critical (25-50%)"),
            (50.0, 99.9, "Failing (50-99%)"),
            (100.0, 100.0, "Dead (100%)"),
        ];

        let total_validators = validators.len();
        let mut buckets = Vec::new();
        let mut histogram_labels = Vec::new();
        let mut histogram_values = Vec::new();

        // Calculate buckets
        for (min_percent, max_percent, label) in bucket_ranges {
            let count = validators.iter().filter(|v| {
                if label == "Perfect (0%)" {
                    // Exact match for perfect performers
                    v.skip_rate_percent == 0.0
                } else if label == "Dead (100%)" {
                    // Exact match for completely dead validators (missed all slots)
                    v.skip_rate_percent == 100.0
                } else if label == "Failing (50-99%)" {
                    // Range for failing but not completely dead
                    v.skip_rate_percent >= min_percent && v.skip_rate_percent < max_percent
                } else {
                    // Standard range: inclusive lower, exclusive upper
                    v.skip_rate_percent >= min_percent && v.skip_rate_percent < max_percent
                }
            }).count();

            let total_slots: u64 = validators.iter()
                .filter(|v| {
                    if label == "Perfect (0%)" {
                        v.skip_rate_percent == 0.0
                    } else if label == "Dead (100%)" {
                        v.skip_rate_percent == 100.0
                    } else if label == "Failing (50-99%)" {
                        v.skip_rate_percent >= min_percent && v.skip_rate_percent < max_percent
                    } else {
                        v.skip_rate_percent >= min_percent && v.skip_rate_percent < max_percent
                    }
                })
                .map(|v| v.leader_slots)
                .sum();

            let percentage_of_total = if total_validators > 0 {
                (count as f64 / total_validators as f64) * 100.0
            } else {
                0.0
            };

            buckets.push(DistributionBucket {
                range_label: label.to_string(),
                min_percent,
                max_percent,
                validator_count: count,
                percentage_of_total,
                total_slots,
            });

            histogram_labels.push(label.to_string());
            histogram_values.push(count);
        }

        // Calculate percentiles
        let percentiles_to_calculate = vec![10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99];
        let mut skip_rates: Vec<f64> = validators.iter().map(|v| v.skip_rate_percent).collect();
        skip_rates.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        
        let mut percentiles = Vec::new();
        let mut percentile_x = Vec::new();
        let mut percentile_y = Vec::new();

        for p in percentiles_to_calculate {
            let value = Self::calculate_percentile(&skip_rates, f64::from(p) / 100.0);
            percentiles.push(PercentileData {
                percentile: p,
                skip_rate_percent: value,
            });
            percentile_x.push(p);
            percentile_y.push(value);
        }

        let plot_data = DistributionPlotData {
            histogram_labels,
            histogram_values,
            percentile_x,
            percentile_y,
        };

        SkipRateDistribution {
            buckets,
            percentiles,
            plot_data,
        }
    }

    /// Calculate network health summary for dashboards
    #[allow(clippy::cast_precision_loss)]
    fn calculate_network_health(statistics: &SkipRateStatistics, validators: &[ValidatorSkipRate]) -> NetworkHealthSummary {
        // Calculate health score (0-100)
        let health_score = Self::calculate_health_score(statistics);
        
        // Determine status
        let status = if health_score >= 90.0 {
            NetworkStatus::Healthy
        } else if health_score >= 75.0 {
            NetworkStatus::Warning
        } else if health_score >= 50.0 {
            NetworkStatus::Degraded
        } else {
            NetworkStatus::Critical
        };

        // Create dashboard metrics
        let key_metrics = DashboardMetrics {
            network_skip_rate: MetricCard {
                value: format!("{:.2}%", statistics.overall_skip_rate_percent),
                previous_value: None, // Would be populated with historical data
                trend: TrendDirection::Unknown,
                color: if statistics.overall_skip_rate_percent < 1.0 {
                    "#22c55e".to_string()
                } else if statistics.overall_skip_rate_percent < 3.0 {
                    "#eab308".to_string()
                } else {
                    "#ef4444".to_string()
                },
                subtitle: "Network skip rate".to_string(),
            },
            active_validators: MetricCard {
                value: statistics.significant_validators.to_string(),
                previous_value: None,
                trend: TrendDirection::Unknown,
                color: "#22c55e".to_string(),
                subtitle: "Active validators".to_string(),
            },
            network_efficiency: MetricCard {
                value: format!("{:.1}%", statistics.network_efficiency_percent),
                previous_value: None,
                trend: TrendDirection::Unknown,
                color: if statistics.network_efficiency_percent > 99.0 {
                    "#22c55e".to_string()
                } else if statistics.network_efficiency_percent > 97.0 {
                    "#eab308".to_string()
                } else {
                    "#ef4444".to_string()
                },
                subtitle: "Network efficiency".to_string(),
            },
            concerning_validators: MetricCard {
                value: statistics.concerning_validators.to_string(),
                previous_value: None,
                trend: TrendDirection::Unknown,
                color: if statistics.concerning_validators == 0 {
                    "#22c55e".to_string()
                } else if statistics.concerning_validators < 10 {
                    "#eab308".to_string()
                } else {
                    "#ef4444".to_string()
                },
                subtitle: "Concerning validators".to_string(),
            },
        };

        // Generate alerts
        let mut alerts = Vec::new();
        let timestamp = Utc::now();

        if statistics.overall_skip_rate_percent > 5.0 {
            alerts.push(NetworkAlert {
                severity: AlertSeverity::Critical,
                message: format!("High network skip rate: {:.2}%", statistics.overall_skip_rate_percent),
                triggered_at: timestamp,
                category: AlertCategory::SkipRate,
            });
        }

        if statistics.concerning_validators > 20 {
            // Calculate impact of concerning validators
            let concerning_validators_data: Vec<_> = validators.iter()
                .filter(|v| v.is_concerning())
                .collect();
            
            let total_concerning_slots: u64 = concerning_validators_data.iter()
                .map(|v| v.leader_slots)
                .sum();
            
            let significant_concerning = concerning_validators_data.iter()
                .filter(|v| v.is_significant())
                .count();
                
            let concerning_network_impact = (total_concerning_slots as f64 / statistics.total_leader_slots as f64) * 100.0;
            
            // Only alert if the impact is meaningful
            if concerning_network_impact > 2.0 || significant_concerning > 10 {
                let severity = if concerning_network_impact > 5.0 { AlertSeverity::Critical } else { AlertSeverity::Warning };
                alerts.push(NetworkAlert {
                    severity,
                    message: format!("{} validators have concerning skip rates ({} significant validators, {:.1}% network impact, {} total missed slots)", 
                        statistics.concerning_validators,
                        significant_concerning,
                        concerning_network_impact,
                        concerning_validators_data.iter().map(|v| v.missed_slots).sum::<u64>()
                    ),
                    triggered_at: timestamp,
                    category: AlertCategory::ValidatorCount,
                });
            }
        }

        if statistics.network_efficiency_percent < 95.0 {
            alerts.push(NetworkAlert {
                severity: AlertSeverity::Warning,
                message: format!("Low network efficiency: {:.1}%", statistics.network_efficiency_percent),
                triggered_at: timestamp,
                category: AlertCategory::NetworkEfficiency,
            });
        }

        // Check for high-impact individual validators
        let high_impact_bad_validators: Vec<_> = validators.iter()
            .filter(|v| v.skip_rate_percent > 10.0 && v.leader_slots > 1000)
            .collect();
            
        if !high_impact_bad_validators.is_empty() {
            let total_missed_slots: u64 = high_impact_bad_validators.iter()
                .map(|v| v.missed_slots)
                .sum();
            
            alerts.push(NetworkAlert {
                severity: AlertSeverity::Critical,
                message: format!("{} high-stake validators have >10% skip rates (missed {} slots total)", 
                    high_impact_bad_validators.len(),
                    total_missed_slots
                ),
                triggered_at: timestamp,
                category: AlertCategory::ValidatorCount,
            });
        }

        NetworkHealthSummary {
            health_score,
            status,
            key_metrics,
            alerts,
        }
    }

    /// Calculate overall health score
    fn calculate_health_score(statistics: &SkipRateStatistics) -> f64 {
        // Weighted scoring system
        let skip_rate_score = ((5.0 - statistics.overall_skip_rate_percent.min(5.0)) / 5.0) * 40.0;
        let efficiency_score = (statistics.network_efficiency_percent / 100.0) * 30.0;
        let validator_health_score = if statistics.total_validators > 0 {
            ((statistics.total_validators - statistics.concerning_validators) as f64 / statistics.total_validators as f64) * 30.0
        } else {
            0.0
        };
        
        (skip_rate_score + efficiency_score + validator_health_score).min(100.0)
    }

    /// Create performance snapshots for time-series data
    fn create_performance_snapshots(validators: &[ValidatorSkipRate], slot_range: &SlotRange, timestamp: DateTime<Utc>) -> Vec<ValidatorPerformanceSnapshot> {
        validators.iter().map(|validator| {
            let category = ValidatorPerformanceCategory::from_skip_rate(
                validator.skip_rate_percent, 
                validator.leader_slots
            );

            ValidatorPerformanceSnapshot {
                timestamp,
                slot_range: slot_range.clone(),
                validator_pubkey: validator.pubkey.clone(),
                skip_rate_percent: validator.skip_rate_percent,
                leader_slots: validator.leader_slots,
                blocks_produced: validator.blocks_produced,
                performance_category: category,
            }
        }).collect()
    }
}

/// Builder for `BlockProductionClient`
pub struct ClientBuilder {
    config: ClientConfig,
}

impl ClientBuilder {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }

    #[must_use]
    pub fn rpc_endpoint(mut self, endpoint: &str) -> Self {
        self.config.rpc_endpoint = endpoint.to_string();
        self
    }

    #[must_use]
    pub const fn timeout(mut self, timeout: Duration) -> Self {
        self.config.timeout = timeout;
        self
    }

    #[must_use]
    pub const fn retry_attempts(mut self, attempts: u32) -> Self {
        self.config.retry_attempts = attempts;
        self
    }

    #[must_use]
    pub fn rate_limit(mut self, requests_per_second: u32) -> Self {
        use std::num::NonZeroU32;
        use governor::{Quota, RateLimiter};
        
        if let Ok(non_zero) = NonZeroU32::try_from(requests_per_second) {
            let quota = Quota::per_second(non_zero);
            self.config.rate_limiter = Some(RateLimiter::direct(quota));
        }
        self
    }

    #[must_use]
    pub const fn max_concurrent_requests(mut self, max: usize) -> Self {
        self.config.max_concurrent_requests = max;
        self
    }

    #[must_use]
    pub fn add_header(mut self, key: &str, value: &str) -> Self {
        self.config.headers.insert(key.to_string(), value.to_string());
        self
    }

    /// Use preset configuration for public RPC endpoints
    #[must_use]
    pub fn public_rpc_config(mut self) -> Self {
        self.config = ClientConfig::public_rpc_config().build();
        self
    }

    /// Use preset configuration for private RPC endpoints
    #[must_use]
    pub fn private_rpc_config(mut self) -> Self {
        self.config = ClientConfig::private_rpc_config().build();
        self
    }

    /// Use preset configuration for high-frequency applications
    #[must_use]
    pub fn high_frequency_config(mut self) -> Self {
        self.config = ClientConfig::high_frequency_config().build();
        self
    }

    /// Use preset configuration for batch processing
    #[must_use]
    pub fn batch_processing_config(mut self) -> Self {
        self.config = ClientConfig::batch_processing_config().build();
        self
    }

    /// Auto-detect optimal configuration based on RPC endpoint
    #[must_use]
    pub fn auto_config(mut self, rpc_endpoint: &str) -> Self {
        self.config = ClientConfig::auto_config(rpc_endpoint).build();
        self
    }

    pub fn build(self) -> Result<BlockProductionClient> {
        BlockProductionClient::from_config(self.config)
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}