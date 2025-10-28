use blocks_production_lib::BlockProductionClient;
use criterion::{criterion_group, criterion_main, Criterion, BenchmarkId};
use serde_json::json;
use std::time::Duration;
use tokio::runtime::Runtime;
use wiremock::{matchers::{method, path}, Mock, MockServer, ResponseTemplate};

// Benchmark client creation
fn bench_client_creation(c: &mut Criterion) {
    c.bench_function("client_creation", |b| {
        b.iter(|| {
            BlockProductionClient::builder()
                .rpc_endpoint("https://api.mainnet-beta.solana.com")
                .build()
                .unwrap()
        })
    });
}

// Benchmark configuration building
fn bench_config_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("config_building");
    
    group.bench_function("simple_config", |b| {
        b.iter(|| {
            BlockProductionClient::builder()
                .rpc_endpoint("https://test.com")
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap()
        })
    });

    group.bench_function("complex_config", |b| {
        b.iter(|| {
            BlockProductionClient::builder()
                .rpc_endpoint("https://test.com")
                .timeout(Duration::from_secs(30))
                .retry_attempts(5)
                .rate_limit(10)
                .max_concurrent_requests(20)
                .add_header("Authorization", "Bearer token")
                .add_header("User-Agent", "benchmark-client")
                .build()
                .unwrap()
        })
    });

    group.bench_function("preset_configs", |b| {
        b.iter(|| {
            let _public = BlockProductionClient::builder()
                .public_rpc_config()
                .rpc_endpoint("https://test.com")
                .build()
                .unwrap();
            
            let _private = BlockProductionClient::builder()
                .private_rpc_config() 
                .rpc_endpoint("https://test.com")
                .build()
                .unwrap();
        })
    });

    group.finish();
}

// Benchmark data processing
fn bench_data_processing(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("data_processing", |b| {
        b.to_async(&rt).iter(|| async {
            let mock_server = MockServer::start().await;

            // Create mock response with realistic data size
            let mut by_identity = serde_json::Map::new();
            for i in 0..1000 {
                by_identity.insert(
                    format!("validator_{}", i),
                    json!([100, 95]) // [leader_slots, blocks_produced]
                );
            }

            let mock_response = json!({
                "jsonrpc": "2.0",
                "result": {
                    "value": {
                        "byIdentity": by_identity,
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

            let _result = client.fetch_block_production().await.unwrap();
        })
    });
}

// Benchmark different payload sizes
fn bench_payload_sizes(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    let mut group = c.benchmark_group("payload_sizes");

    for validator_count in [10, 100, 500, 1000, 2000].iter() {
        group.bench_with_input(
            BenchmarkId::new("validators", validator_count),
            validator_count,
            |b, &validator_count| {
                b.to_async(&rt).iter(|| async move {
                    let mock_server = MockServer::start().await;

                    let mut by_identity = serde_json::Map::new();
                    for i in 0..validator_count {
                        by_identity.insert(
                            format!("validator_{}", i),
                            json!([100, 95])
                        );
                    }

                    let mock_response = json!({
                        "jsonrpc": "2.0",
                        "result": {
                            "value": {
                                "byIdentity": by_identity,
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

                    let _result = client.fetch_block_production().await.unwrap();
                })
            },
        );
    }

    group.finish();
}

// Benchmark error handling overhead
fn bench_error_handling(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("error_handling_overhead", |b| {
        b.to_async(&rt).iter(|| async {
            let mock_server = MockServer::start().await;

            // Mock server error
            Mock::given(method("POST"))
                .and(path("/"))
                .respond_with(ResponseTemplate::new(500))
                .mount(&mock_server)
                .await;

            let client = BlockProductionClient::builder()
                .rpc_endpoint(&mock_server.uri())
                .retry_attempts(1) // Single attempt for consistent timing
                .build()
                .unwrap();

            let _result = client.test_connection().await; // Will fail
        })
    });
}

// Benchmark statistics calculation
fn bench_statistics_calculation(c: &mut Criterion) {
    use blocks_production_lib::ValidatorSkipRate;
    
    let mut group = c.benchmark_group("statistics");

    for validator_count in [100, 500, 1000, 2000].iter() {
        group.bench_with_input(
            BenchmarkId::new("calculate_stats", validator_count),
            validator_count,
            |b, &validator_count| {
                let validators: Vec<ValidatorSkipRate> = (0..validator_count)
                    .map(|i| ValidatorSkipRate::new(
                        format!("validator_{}", i),
                        100,
                        95,
                    ))
                    .collect();

                b.iter(|| {
                    // Simulate statistics calculation by processing validators
                    let total_slots: u64 = validators.iter().map(|v| v.leader_slots).sum();
                    let total_produced: u64 = validators.iter().map(|v| v.blocks_produced).sum();
                    let _skip_rate = ((total_slots - total_produced) as f64 / total_slots as f64) * 100.0;
                })
            },
        );
    }

    group.finish();
}

// Benchmark concurrent requests
fn bench_concurrent_requests(c: &mut Criterion) {
    let rt = Runtime::new().unwrap();
    
    c.bench_function("concurrent_requests", |b| {
        b.to_async(&rt).iter(|| async {
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
                .max_concurrent_requests(10)
                .build()
                .unwrap();

            // Make 10 concurrent requests
            let client = std::sync::Arc::new(client);
            let mut handles = Vec::new();
            for _ in 0..10 {
                let client_clone = client.clone();
                let handle = tokio::spawn(async move {
                    client_clone.test_connection().await
                });
                handles.push(handle);
            }

            futures::future::join_all(handles).await;
        })
    });
}

criterion_group!(
    benches,
    bench_client_creation,
    bench_config_building,
    bench_data_processing,
    bench_payload_sizes,
    bench_error_handling,
    bench_statistics_calculation,
    bench_concurrent_requests
);

criterion_main!(benches);