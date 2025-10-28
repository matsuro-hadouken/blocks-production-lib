use blocks_production_lib::{
    BlockProductionClient, 
    LoggingConfig, 
    LogFormat,
    ErrorExt,
};
use tracing::{info, warn, error};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Initialize comprehensive logging
    LoggingConfig::new()
        .level("debug")
        .format(LogFormat::Pretty)
        .include_location(true)
        .init()
        .map_err(|e| format!("Failed to initialize logging: {}", e))?;

    info!("Starting error handling and logging demonstration");

    // Test 1: Valid endpoint
    info!("=== Test 1: Valid RPC Endpoint ===");
    test_valid_endpoint().await;

    // Test 2: Invalid endpoint
    info!("=== Test 2: Invalid RPC Endpoint ===");
    test_invalid_endpoint().await;

    // Test 3: Configuration errors
    info!("=== Test 3: Configuration Errors ===");
    test_configuration_errors().await;

    // Test 4: Rate limiting simulation
    info!("=== Test 4: Rate Limiting ===");
    test_rate_limiting().await;

    // Test 5: Error handling patterns
    info!("=== Test 5: Error Handling Patterns ===");
    test_error_handling_patterns().await;

    info!("Error handling and logging demonstration completed");
    Ok(())
}

async fn test_valid_endpoint() {
    info!("Testing connection to valid endpoint");
    
    match BlockProductionClient::new("https://api.mainnet-beta.solana.com") {
        Ok(client) => {
            info!("Client created successfully");
            
            match client.test_connection().await {
                Ok(true) => info!("Connection test passed"),
                Ok(false) => warn!("Connection test returned false"),
                Err(e) => error!(error = %e, "Connection test failed"),
            }
            
            // Try to fetch actual data
            match client.fetch_block_production().await {
                Ok(data) => info!(
                    total_validators = data.statistics.total_validators,
                    skip_rate = %format!("{:.2}%", data.statistics.overall_skip_rate_percent),
                    "Successfully fetched block production data"
                ),
                Err(e) => {
                    error!(error = %e, "Failed to fetch block production data");
                    log_error_details(&e);
                }
            }
        },
        Err(e) => {
            error!(error = %e, "Failed to create client");
            log_error_details(&e);
        }
    }
}

async fn test_invalid_endpoint() {
    info!("Testing various invalid endpoints");
    
    let invalid_endpoints = vec![
        "",
        "not-a-url",
        "http://definitely-not-a-real-solana-rpc-endpoint.invalid",
        "https://httpstat.us/500", // Returns 500 error
        "https://httpstat.us/429", // Returns rate limit error
    ];
    
    for endpoint in invalid_endpoints {
        info!(endpoint = endpoint, "Testing invalid endpoint");
        
        match BlockProductionClient::new(endpoint) {
            Ok(client) => {
                warn!(endpoint = endpoint, "Client creation succeeded unexpectedly");
                
                match client.test_connection().await {
                    Ok(_) => warn!("Connection unexpectedly succeeded"),
                    Err(e) => {
                        info!(error = %e, "Connection failed as expected");
                        log_error_details(&e);
                    }
                }
            },
            Err(e) => {
                info!(error = %e, "Client creation failed as expected");
                log_error_details(&e);
            }
        }
    }
}

async fn test_configuration_errors() {
    info!("Testing configuration error scenarios");
    
    // Test invalid headers
    match BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .add_header("Invalid\nHeader", "value")
        .build() 
    {
        Ok(_) => warn!("Invalid header was accepted unexpectedly"),
        Err(e) => {
            info!(error = %e, "Invalid header rejected as expected");
            log_error_details(&e);
        }
    }
    
    // Test very short timeout
    let client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .timeout(Duration::from_millis(1)) // Unrealistically short
        .build();
    
    match client {
        Ok(client) => {
            warn!("Short timeout client created");
            match client.test_connection().await {
                Ok(_) => warn!("Connection succeeded despite short timeout"),
                Err(e) => {
                    info!(error = %e, "Connection failed due to short timeout");
                    log_error_details(&e);
                }
            }
        },
        Err(e) => {
            error!(error = %e, "Failed to create client with short timeout");
            log_error_details(&e);
        }
    }
}

async fn test_rate_limiting() {
    info!("Testing rate limiting functionality");
    
    let client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .rate_limit(1) // Very restrictive rate limit
        .build();
        
    match client {
        Ok(client) => {
            info!("Rate-limited client created");
            
            // Make multiple rapid requests
            for i in 1..=3 {
                info!(request_number = i, "Making request");
                let start = std::time::Instant::now();
                
                match client.test_connection().await {
                    Ok(_) => info!(
                        request_number = i,
                        duration_ms = start.elapsed().as_millis(),
                        "Request completed"
                    ),
                    Err(e) => {
                        warn!(
                            request_number = i,
                            duration_ms = start.elapsed().as_millis(),
                            error = %e,
                            "Request failed"
                        );
                        log_error_details(&e);
                    }
                }
            }
        },
        Err(e) => {
            error!(error = %e, "Failed to create rate-limited client");
            log_error_details(&e);
        }
    }
}

async fn test_error_handling_patterns() {
    info!("Demonstrating error handling patterns");
    
    // Create a client that will likely fail
    let client = BlockProductionClient::builder()
        .rpc_endpoint("https://httpstat.us/500")
        .retry_attempts(2)
        .build();
        
    match client {
        Ok(client) => {
            match client.test_connection().await {
                Ok(_) => warn!("Unexpected success"),
                Err(e) => {
                    error!(error = %e, "Demonstrating comprehensive error handling");
                    
                    // Show all error handling capabilities
                    log_error_details(&e);
                    demonstrate_error_analysis(&e);
                }
            }
        },
        Err(e) => {
            error!(error = %e, "Client creation failed");
            log_error_details(&e);
        }
    }
}

fn log_error_details(error: &blocks_production_lib::BlockProductionError) {
    info!("=== Detailed Error Analysis ===");
    info!(error_type = ?error, "Error type");
    info!(error_message = %error, "Error message");
    info!(is_retryable = error.is_retryable(), "Is retryable");
    info!(is_config_error = error.is_config_error(), "Is configuration error");
    info!(is_transient = error.is_transient(), "Is transient");
    info!(category = ?error.category(), "Error category");
    
    if let Some(retry_delay) = error.retry_delay() {
        info!(retry_delay_secs = retry_delay.as_secs(), "Suggested retry delay");
    }
    
    let hints = error.debug_hints();
    if !hints.is_empty() {
        info!("Debug hints:");
        for (i, hint) in hints.iter().enumerate() {
            info!(hint_number = i + 1, hint = hint, "Debug hint");
        }
    }
}

fn demonstrate_error_analysis(error: &blocks_production_lib::BlockProductionError) {
    info!("=== Error Handling Strategy ===");
    
    if error.is_retryable() {
        if let Some(delay) = error.retry_delay() {
            info!(
                delay_secs = delay.as_secs(),
                "Error is retryable, suggested delay"
            );
        } else {
            info!("Error is retryable with default backoff");
        }
    } else {
        info!("Error is not retryable");
    }
    
    if error.is_config_error() {
        warn!("This is a configuration error - check your settings");
    }
    
    match error.category() {
        blocks_production_lib::error::ErrorCategory::Network => {
            info!("Network error - check connectivity and endpoint");
        },
        blocks_production_lib::error::ErrorCategory::Configuration => {
            warn!("Configuration error - review client settings");
        },
        blocks_production_lib::error::ErrorCategory::RateLimit => {
            info!("Rate limit error - reduce request frequency");
        },
        blocks_production_lib::error::ErrorCategory::Authentication => {
            error!("Authentication error - check API credentials");
        },
        blocks_production_lib::error::ErrorCategory::Validation => {
            error!("Validation error - check input parameters");
        },
        blocks_production_lib::error::ErrorCategory::Rpc => {
            warn!("RPC error - check endpoint and request format");
        },
    }
}