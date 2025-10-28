use blocks_production_lib::{
    BlockProductionClient, BlockProductionError, ValidatorSkipRate, SlotRange,
    ErrorExt, LoggingConfig, LogFormat,
};
use blocks_production_lib::error::ErrorCategory;
use serde_json::json;
use std::time::Duration;
use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate, Request,
};

#[tokio::test]
async fn test_client_creation() {
    let client = BlockProductionClient::builder()
        .rpc_endpoint("https://api.mainnet-beta.solana.com")
        .timeout(Duration::from_secs(30))
        .retry_attempts(3)
        .build();
    
    assert!(client.is_ok());
}

#[tokio::test]
async fn test_client_builder_configuration() {
    let _client = BlockProductionClient::builder()
        .rpc_endpoint("https://custom-rpc.com")
        .timeout(Duration::from_secs(60))
        .retry_attempts(5)
        .rate_limit(10)
        .max_concurrent_requests(20)
        .add_header("Authorization", "Bearer token")
        .build()
        .unwrap();

    // The client should be created successfully with custom config
    // We can't directly access config fields, but creation success indicates proper setup
    assert!(true);
}

#[tokio::test]
async fn test_preset_configurations() {
    // Test all preset configurations compile and create clients
    let configs = [
        BlockProductionClient::builder().public_rpc_config(),
        BlockProductionClient::builder().private_rpc_config(),
        BlockProductionClient::builder().high_frequency_config(),
        BlockProductionClient::builder().batch_processing_config(),
    ];

    for config_builder in configs {
        let client = config_builder
            .rpc_endpoint("https://test.com")
            .build();
        assert!(client.is_ok());
    }
}

#[tokio::test]
async fn test_validator_skip_rate_calculation() {
    // Test perfect performance
    let perfect = ValidatorSkipRate::new(
        "perfect_validator".to_string(),
        100, // leader slots
        100, // blocks produced
    );
    assert_eq!(perfect.missed_slots, 0);
    assert_eq!(perfect.skip_rate_percent, 0.0);
    assert!(perfect.is_perfect());
    assert!(!perfect.is_concerning());

    // Test concerning performance
    let concerning = ValidatorSkipRate::new(
        "bad_validator".to_string(),
        100, // leader slots
        85,  // blocks produced
    );
    assert_eq!(concerning.missed_slots, 15);
    assert_eq!(concerning.skip_rate_percent, 15.0);
    assert!(!concerning.is_perfect());
    assert!(concerning.is_concerning());

    // Test edge case: no leader slots
    let no_slots = ValidatorSkipRate::new(
        "no_slots_validator".to_string(),
        0, // leader slots
        0, // blocks produced
    );
    assert_eq!(no_slots.missed_slots, 0);
    assert_eq!(no_slots.skip_rate_percent, 0.0);
    assert!(!no_slots.is_perfect()); // not perfect because no slots
    assert!(!no_slots.is_concerning());
}

#[tokio::test]
async fn test_slot_range_calculations() {
    let range = SlotRange {
        first_slot: 1000,
        last_slot: 2500,
    };
    assert_eq!(range.slot_count(), 1500);

    // Test edge case
    let same_slot = SlotRange {
        first_slot: 1000,
        last_slot: 1000,
    };
    assert_eq!(same_slot.slot_count(), 0);
}

#[tokio::test]
async fn test_invalid_slot_range() {
    let mock_server = MockServer::start().await;
    
    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .build()
        .unwrap();

    // Test invalid range (first_slot >= last_slot)
    let result = client.fetch_block_production_range(2000, 1000).await;
    assert!(matches!(result, Err(BlockProductionError::InvalidSlotRange { .. })));

    let result = client.fetch_block_production_range(1000, 1000).await;
    assert!(matches!(result, Err(BlockProductionError::InvalidSlotRange { .. })));
}

#[tokio::test]
async fn test_mock_rpc_success() {
    let mock_server = MockServer::start().await;

    // Mock successful getBlockProduction response
    let mock_response = json!({
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
    });

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .build()
        .unwrap();

    let result = client.fetch_block_production().await;
    assert!(result.is_ok());

    let data = result.unwrap();
    assert_eq!(data.validators.len(), 3);
    assert_eq!(data.statistics.total_validators, 3);
    assert_eq!(data.statistics.total_leader_slots, 350);
    assert_eq!(data.statistics.total_blocks_produced, 335);
    assert_eq!(data.statistics.perfect_validators, 1); // validator3
    assert_eq!(data.slot_range.first_slot, 1000);
    assert_eq!(data.slot_range.last_slot, 2000);
}

#[tokio::test]
async fn test_mock_rpc_error() {
    let mock_server = MockServer::start().await;

    // Mock RPC error response
    let error_response = json!({
        "jsonrpc": "2.0",
        "error": {
            "code": -32601,
            "message": "Method not found"
        },
        "id": 1
    });

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&error_response))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .build()
        .unwrap();

    let result = client.fetch_block_production().await;
    assert!(matches!(result, Err(BlockProductionError::Rpc { .. })));
}

#[tokio::test]
async fn test_mock_http_error() {
    let mock_server = MockServer::start().await;

    // Mock HTTP error
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .retry_attempts(1) // Reduce retries for faster test
        .build()
        .unwrap();

    let result = client.fetch_block_production().await;
    assert!(matches!(result, Err(BlockProductionError::Http { .. })));
}

#[tokio::test]
async fn test_mock_no_data() {
    let mock_server = MockServer::start().await;

    // Mock response with no validator data
    let empty_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "value": {
                "byIdentity": {},
                "range": {
                    "firstSlot": 1000,
                    "lastSlot": 2000
                }
            }
        },
        "id": 1
    });

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&empty_response))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .build()
        .unwrap();

    let result = client.fetch_block_production().await;
    assert!(matches!(result, Err(BlockProductionError::NoData { .. })));
}

#[tokio::test]
async fn test_debug_format() {
    let mock_server = MockServer::start().await;

    let mock_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "value": {
                "byIdentity": {
                    "validator1": [100, 95]
                },
                "range": {
                    "firstSlot": 1000,
                    "lastSlot": 2000
                }
            }
        },
        "id": 1
    });

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .build()
        .unwrap();

    let result = client.fetch_block_production_debug(
        blocks_production_lib::BlockProductionRequest::default()
    ).await;
    
    assert!(result.is_ok());
    let debug_data = result.unwrap();
    
    assert_eq!(debug_data.production_data.validators.len(), 1);
    assert!(debug_data.raw_rpc_data.get("result").is_some());
    assert_eq!(debug_data.response_metadata.rpc_endpoint, mock_server.uri());
    assert!(debug_data.response_metadata.response_time_ms > 0);
}

#[tokio::test]
async fn test_statistics_calculation() {
    let mock_server = MockServer::start().await;

    // Create test data with known statistics
    let mock_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "value": {
                "byIdentity": {
                    "perfect_validator": [100, 100],    // 0% skip rate
                    "good_validator": [100, 98],        // 2% skip rate
                    "concerning_validator": [100, 90],  // 10% skip rate
                    "bad_validator": [100, 80]          // 20% skip rate
                },
                "range": {
                    "firstSlot": 1000,
                    "lastSlot": 2000
                }
            }
        },
        "id": 1
    });

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .build()
        .unwrap();

    let result = client.fetch_block_production().await.unwrap();
    let stats = &result.statistics;

    assert_eq!(stats.total_validators, 4);
    assert_eq!(stats.total_leader_slots, 400);
    assert_eq!(stats.total_blocks_produced, 368);
    assert_eq!(stats.total_missed_slots, 32);
    assert_eq!(stats.overall_skip_rate_percent, 8.0); // 32/400 * 100
    assert_eq!(stats.perfect_validators, 1);
    assert_eq!(stats.concerning_validators, 2); // 10% and 20% are > 5%
    assert_eq!(stats.skip_rate_95th_percentile, 10.0);  // 95th percentile of [0, 2, 10, 20]
    
    // Average should be (0 + 2 + 10 + 20) / 4 = 8.0
    assert_eq!(stats.average_skip_rate_percent, 8.0);
    
    // Median of [0, 2, 10, 20] should be (2 + 10) / 2 = 6.0
    assert_eq!(stats.median_skip_rate_percent, 6.0);
}

#[tokio::test]
async fn test_connection_test() {
    let mock_server = MockServer::start().await;

    // Mock health check response
    let health_response = json!({
        "jsonrpc": "2.0",
        "result": "ok",
        "id": 1
    });

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&health_response))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .build()
        .unwrap();

    let result = client.test_connection().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), true);
}

#[tokio::test]
async fn test_error_handling_comprehensive() {
    // Initialize test logging
    blocks_production_lib::init_test_logging();

    let mock_server = MockServer::start().await;

    // Test 1: Network timeout
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_delay(Duration::from_secs(10)))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .timeout(Duration::from_millis(500)) // Short timeout for test
        .retry_attempts(1)
        .build()
        .unwrap();

    let result = client.test_connection().await;
    assert!(result.is_err());
    
    if let Err(e) = result {
        assert!(e.is_retryable());
        assert!(e.is_transient());
        assert!(!e.is_config_error());
        assert_eq!(e.category(), ErrorCategory::Network);
    }
}

#[tokio::test]
async fn test_error_analysis_traits() {
    let config_error = BlockProductionError::Config {
        message: "Invalid endpoint".to_string(),
        field: Some("endpoint".to_string()),
        suggestion: Some("Use https:// prefix".to_string()),
    };

    assert!(!config_error.is_retryable());
    assert!(!config_error.is_transient());
    assert!(config_error.is_config_error());
    
    let hints = config_error.debug_hints();
    assert!(!hints.is_empty());
    assert!(hints[0].contains("https://"));
}

#[tokio::test]
async fn test_rate_limiting_behavior() {
    let mock_server = MockServer::start().await;

    // Mock rate limit response
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(429).insert_header("Retry-After", "1"))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .retry_attempts(1)
        .build()
        .unwrap();

    let result = client.test_connection().await;
    assert!(result.is_err());
    
    if let Err(e) = result {
        match e {
            BlockProductionError::RateLimit { retry_after, .. } => {
                assert!(retry_after.is_some());
            }
            BlockProductionError::ConnectionFailed { source, .. } => {
                // Check if the source error contains RateLimit information
                let error_string = source.to_string();
                assert!(error_string.contains("Rate limit") || error_string.contains("rate limit"));
            }
            _ => panic!("Expected RateLimit error, got {:?}", e),
        }
    }
}

#[tokio::test]
async fn test_invalid_json_response() {
    let mock_server = MockServer::start().await;

    // Mock invalid JSON response
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .build()
        .unwrap();

    let result = client.test_connection().await;
    assert!(result.is_err());
    
    if let Err(e) = result {
        match e {
            BlockProductionError::ResponseParsing { .. } => {
                // Expected
            }
            BlockProductionError::ConnectionFailed { source, .. } => {
                // Check if the source error contains parsing information
                let error_string = source.to_string();
                assert!(error_string.contains("parsing") || error_string.contains("JSON") || error_string.contains("decode"));
            }
            _ => panic!("Expected ResponseParsing error, got {:?}", e),
        }
    }
}

#[tokio::test]
async fn test_configuration_validation() {
    // Test empty endpoint
    let result = BlockProductionClient::new("");
    assert!(result.is_err());
    
    if let Err(e) = result {
        assert!(e.is_config_error());
        assert!(!e.is_retryable());
    }

    // Test invalid URL format
    let result = BlockProductionClient::new("not-a-url");
    assert!(result.is_err());
    
    if let Err(e) = result {
        assert!(e.is_config_error());
        assert!(!e.is_retryable());
    }
}

#[tokio::test]
async fn test_custom_headers() {
    let mock_server = MockServer::start().await;

    // Verify custom headers are sent
    Mock::given(method("POST"))
        .and(path("/"))
        .and(|req: &Request| {
            req.headers.get("Authorization").is_some() &&
            req.headers.get("User-Agent").is_some()
        })
        .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
            "jsonrpc": "2.0",
            "result": "ok",
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .add_header("Authorization", "Bearer test-token")
        .add_header("User-Agent", "test-client/1.0")
        .build()
        .unwrap();

    let result = client.test_connection().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_statistics_edge_cases() {
    let mock_server = MockServer::start().await;

    // Mock response with edge case data (zero slots, perfect validators, etc.)
    let mock_response = json!({
        "jsonrpc": "2.0",
        "result": {
            "value": {
                "byIdentity": {
                    "zero_slots_validator": [0, 0],
                    "perfect_validator": [100, 100],
                    "offline_validator": [100, 0],
                    "low_activity_validator": [5, 4]
                },
                "range": {
                    "firstSlot": 1000,
                    "lastSlot": 2000
                }
            }
        },
        "id": 1
    });

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&mock_response))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .build()
        .unwrap();

    let result = client.fetch_block_production().await.unwrap();
    
    // Verify edge cases are handled correctly
    let validators = &result.validators;
    
    // Find specific validators
    let zero_slots = validators.iter().find(|v| v.pubkey == "zero_slots_validator").unwrap();
    assert_eq!(zero_slots.skip_rate_percent, 0.0);
    assert!(!zero_slots.is_perfect()); // No slots = not perfect
    
    let perfect = validators.iter().find(|v| v.pubkey == "perfect_validator").unwrap();
    assert!(perfect.is_perfect());
    assert!(!perfect.is_concerning());
    
    let offline = validators.iter().find(|v| v.pubkey == "offline_validator").unwrap();
    assert!(offline.is_offline());
    assert_eq!(offline.skip_rate_percent, 100.0);
    
    let low_activity = validators.iter().find(|v| v.pubkey == "low_activity_validator").unwrap();
    assert!(low_activity.is_low_activity());
    assert!(!low_activity.is_significant());
}

#[tokio::test]
async fn test_concurrent_requests() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&json!({
            "jsonrpc": "2.0",
            "result": "ok",
            "id": 1
        })))
        .mount(&mock_server)
        .await;

    let client = BlockProductionClient::builder()
        .rpc_endpoint(&mock_server.uri())
        .max_concurrent_requests(5)
        .build()
        .unwrap();

    // Make multiple concurrent requests
    let client = std::sync::Arc::new(client);
    let mut handles = Vec::new();
    for _ in 0..10 {
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            client_clone.test_connection().await
        });
        handles.push(handle);
    }

    // Wait for all requests to complete
    let results: Vec<_> = futures::future::join_all(handles).await;
    
    // All should succeed (some may be queued due to concurrency limits)
    for result in results {
        assert!(result.is_ok());
        assert!(result.unwrap().is_ok());
    }
}

#[tokio::test]
async fn test_logging_configuration() {
    // Test that logging can be configured without panicking
    let config_result = LoggingConfig::new()
        .level("debug")
        .format(LogFormat::Json)
        .include_location(true)
        .init();

    // Should not panic, but may fail if already initialized
    // That's okay for tests
    let _ = config_result;
}

#[tokio::test]
async fn test_auto_config_detection() {
    // Test auto-configuration for different RPC providers
    let mainnet_client = BlockProductionClient::builder()
        .auto_config("https://api.mainnet-beta.solana.com")
        .build();
    assert!(mainnet_client.is_ok());

    let helius_client = BlockProductionClient::builder()
        .auto_config("https://rpc.helius.xyz/test")
        .build();
    assert!(helius_client.is_ok());

    let quicknode_client = BlockProductionClient::builder()
        .auto_config("https://api.quicknode.com/test")
        .build();
    assert!(quicknode_client.is_ok());
}
