use std::time::Duration;
use std::num::NonZeroU32;
use governor::{Quota, RateLimiter};

/// Rate limiter type alias for easier use
type AppRateLimiter = RateLimiter<
    governor::state::direct::NotKeyed,
    governor::state::InMemoryState,
    governor::clock::DefaultClock,
>;

/// Configuration for the `BlockProductionClient`
#[derive(Debug)]
pub struct ClientConfig {
    /// RPC endpoint URL
    pub rpc_endpoint: String,
    /// Request timeout
    pub timeout: Duration,
    /// Number of retry attempts
    pub retry_attempts: u32,
    /// Rate limiter (requests per second)
    pub rate_limiter: Option<AppRateLimiter>,
    /// Maximum concurrent requests
    pub max_concurrent_requests: usize,
    /// Custom HTTP headers
    pub headers: std::collections::HashMap<String, String>,
}

impl Clone for ClientConfig {
    fn clone(&self) -> Self {
        Self {
            rpc_endpoint: self.rpc_endpoint.clone(),
            timeout: self.timeout,
            retry_attempts: self.retry_attempts,
            rate_limiter: None, // Cannot clone rate limiter due to internal state
            max_concurrent_requests: self.max_concurrent_requests,
            headers: self.headers.clone(),
        }
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            rpc_endpoint: "https://api.mainnet-beta.solana.com".to_string(),
            timeout: Duration::from_secs(30),
            retry_attempts: 3,
            rate_limiter: None,
            max_concurrent_requests: 10,
            headers: std::collections::HashMap::new(),
        }
    }
}

impl ClientConfig {
    /// Create a new config builder
    #[must_use] 
    pub fn builder() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
    }

    /// Create a pre-configured client for public RPC endpoints
    #[must_use]
    pub fn public_rpc_config() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
            .timeout(Duration::from_secs(60))
            .retry_attempts(5)
            .rate_limit(2) // Conservative rate limiting for public RPCs
            .max_concurrent_requests(5)
    }

    /// Configuration optimized for private/paid RPC endpoints
    #[must_use] 
    pub fn private_rpc_config() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
            .timeout(Duration::from_secs(30))
            .retry_attempts(3)
            .rate_limit(10) // Higher rate limit for private RPCs
            .max_concurrent_requests(20)
    }

    /// Configuration for high-frequency applications
    #[must_use] 
    pub fn high_frequency_config() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
            .timeout(Duration::from_secs(15))
            .retry_attempts(2)
            .rate_limit(50)
            .max_concurrent_requests(50)
    }

    /// Configuration for batch processing
    #[must_use] 
    pub fn batch_processing_config() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
            .timeout(Duration::from_secs(120))
            .retry_attempts(5)
            .rate_limit(5)
            .max_concurrent_requests(100)
    }

    /// Development configuration with verbose settings
    #[must_use] 
    pub fn development_config() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
            .timeout(Duration::from_secs(60))
            .retry_attempts(1)
            .rate_limit(1)
            .max_concurrent_requests(5)
    }

    /// Enterprise configuration for production use
    #[must_use] 
    pub fn enterprise_config() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
            .timeout(Duration::from_secs(45))
            .retry_attempts(3)
            .rate_limit(25)
            .max_concurrent_requests(30)
    }

    /// Auto-detect optimal configuration based on RPC endpoint
    #[must_use] 
    pub fn auto_config(rpc_endpoint: &str) -> ClientConfigBuilder {
        let builder = if rpc_endpoint.contains("mainnet-beta.solana.com") {
            Self::public_rpc_config()
        } else if rpc_endpoint.contains("helius") {
            Self::helius_config()
        } else if rpc_endpoint.contains("quicknode") {
            Self::quicknode_config()
        } else if rpc_endpoint.contains("alchemy") {
            Self::alchemy_config()
        } else {
            Self::private_rpc_config()
        };

        builder.rpc_endpoint(rpc_endpoint.to_string())
    }

    /// Pre-configured client for Helius endpoints
    #[must_use]
    pub fn helius_config() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
            .timeout(Duration::from_secs(30))
            .retry_attempts(3)
            .rate_limit(20)
            .max_concurrent_requests(25)
    }

    /// QuickNode-optimized configuration
    #[must_use]
    pub fn quicknode_config() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
            .timeout(Duration::from_secs(30))
            .retry_attempts(3)
            .rate_limit(15)
            .max_concurrent_requests(20)
    }

    /// Alchemy-optimized configuration
    #[must_use]
    pub fn alchemy_config() -> ClientConfigBuilder {
        ClientConfigBuilder::new()
            .timeout(Duration::from_secs(30))
            .retry_attempts(3)
            .rate_limit(25)
            .max_concurrent_requests(30)
    }
}

/// Builder for `ClientConfig`
#[derive(Debug)]
pub struct ClientConfigBuilder {
    config: ClientConfig,
}

impl ClientConfigBuilder {
    #[must_use] 
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
        }
    }

    #[must_use]
    pub fn rpc_endpoint(mut self, endpoint: String) -> Self {
        self.config.rpc_endpoint = endpoint;
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
    pub fn add_header(mut self, key: String, value: String) -> Self {
        self.config.headers.insert(key, value);
        self
    }

    pub fn build(self) -> ClientConfig {
        self.config
    }
}

impl Default for ClientConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.rpc_endpoint, "https://api.mainnet-beta.solana.com");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.max_concurrent_requests, 10);
        assert!(config.headers.is_empty());
        assert!(config.rate_limiter.is_none());
    }

    #[test]
    fn test_preset_configurations() {
        // Test public RPC config
        let public_builder = ClientConfig::public_rpc_config();
        let config = public_builder.rpc_endpoint("https://test.com".to_string()).build();
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.retry_attempts, 5);
        assert_eq!(config.max_concurrent_requests, 5);

        // Test private RPC config
        let private_builder = ClientConfig::private_rpc_config();
        let config = private_builder.rpc_endpoint("https://test.com".to_string()).build();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.max_concurrent_requests, 20);

        // Test high frequency config
        let hf_builder = ClientConfig::high_frequency_config();
        let config = hf_builder.rpc_endpoint("https://test.com".to_string()).build();
        assert_eq!(config.timeout, Duration::from_secs(15));
        assert_eq!(config.retry_attempts, 2);
        assert_eq!(config.max_concurrent_requests, 50);

        // Test batch processing config
        let batch_builder = ClientConfig::batch_processing_config();
        let config = batch_builder.rpc_endpoint("https://test.com".to_string()).build();
        assert_eq!(config.timeout, Duration::from_secs(120));
        assert_eq!(config.retry_attempts, 5);
        assert_eq!(config.max_concurrent_requests, 100);
    }

    #[test]
    fn test_provider_specific_configs() {
        // Test Helius config
        let helius_builder = ClientConfig::helius_config();
        let config = helius_builder.rpc_endpoint("https://test.com".to_string()).build();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_concurrent_requests, 25);

        // Test QuickNode config
        let quicknode_builder = ClientConfig::quicknode_config();
        let config = quicknode_builder.rpc_endpoint("https://test.com".to_string()).build();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_concurrent_requests, 20);

        // Test Alchemy config
        let alchemy_builder = ClientConfig::alchemy_config();
        let config = alchemy_builder.rpc_endpoint("https://test.com".to_string()).build();
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_concurrent_requests, 30);
    }

    #[test]
    fn test_auto_config_detection() {
        // Test auto-detection for known providers
        let mainnet_config = ClientConfig::auto_config("https://api.mainnet-beta.solana.com").build();
        assert_eq!(mainnet_config.timeout, Duration::from_secs(60)); // Public RPC config

        let helius_config = ClientConfig::auto_config("https://rpc.helius.xyz").build();
        assert_eq!(helius_config.timeout, Duration::from_secs(30)); // Helius config

        let quicknode_config = ClientConfig::auto_config("https://api.quicknode.com").build();
        assert_eq!(quicknode_config.timeout, Duration::from_secs(30)); // QuickNode config

        let alchemy_config = ClientConfig::auto_config("https://solana-mainnet.g.alchemy.com").build();
        assert_eq!(alchemy_config.timeout, Duration::from_secs(30)); // Alchemy config

        // Test fallback to private RPC config
        let unknown_config = ClientConfig::auto_config("https://unknown-provider.com").build();
        assert_eq!(unknown_config.timeout, Duration::from_secs(30)); // Private RPC config
    }

    #[test]
    fn test_client_config_builder() {
        let config = ClientConfigBuilder::new()
            .rpc_endpoint("https://test.com".to_string())
            .timeout(Duration::from_secs(45))
            .retry_attempts(4)
            .rate_limit(20)
            .max_concurrent_requests(15)
            .add_header("Authorization".to_string(), "Bearer token".to_string())
            .add_header("User-Agent".to_string(), "test-client".to_string())
            .build();

        assert_eq!(config.rpc_endpoint, "https://test.com");
        assert_eq!(config.timeout, Duration::from_secs(45));
        assert_eq!(config.retry_attempts, 4);
        assert_eq!(config.max_concurrent_requests, 15);
        assert_eq!(config.headers.len(), 2);
        assert_eq!(config.headers.get("Authorization"), Some(&"Bearer token".to_string()));
        assert_eq!(config.headers.get("User-Agent"), Some(&"test-client".to_string()));
        assert!(config.rate_limiter.is_some());
    }

    #[test]
    fn test_rate_limiter_creation() {
        let config = ClientConfigBuilder::new()
            .rate_limit(10)
            .build();
        assert!(config.rate_limiter.is_some());

        let config_no_limit = ClientConfigBuilder::new()
            .rate_limit(0)
            .build();
        assert!(config_no_limit.rate_limiter.is_none());
    }

    #[test]
    fn test_builder_default() {
        let builder = ClientConfigBuilder::default();
        let config = builder.build();
        
        // Should have same defaults as ClientConfig::default()
        assert_eq!(config.rpc_endpoint, "https://api.mainnet-beta.solana.com");
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.retry_attempts, 3);
        assert_eq!(config.max_concurrent_requests, 10);
    }

    #[test]
    fn test_config_debug() {
        let config = ClientConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("ClientConfig"));
        assert!(debug_str.contains("timeout"));
    }

    #[test]
    fn test_config_clone() {
        let original = ClientConfigBuilder::new()
            .rpc_endpoint("https://test.com".to_string())
            .timeout(Duration::from_secs(45))
            .add_header("test".to_string(), "value".to_string())
            .build();

        let cloned = original.clone();
        
        assert_eq!(original.rpc_endpoint, cloned.rpc_endpoint);
        assert_eq!(original.timeout, cloned.timeout);
        assert_eq!(original.headers, cloned.headers);
    }

    #[test]
    fn test_development_config() {
        let dev_config = ClientConfig::development_config().build();
        assert_eq!(dev_config.timeout, Duration::from_secs(60));
        assert_eq!(dev_config.retry_attempts, 1);
        assert_eq!(dev_config.max_concurrent_requests, 5);
    }

    #[test]
    fn test_enterprise_config() {
        let enterprise_config = ClientConfig::enterprise_config().build();
        assert_eq!(enterprise_config.timeout, Duration::from_secs(45));
        assert_eq!(enterprise_config.retry_attempts, 3);
        assert_eq!(enterprise_config.max_concurrent_requests, 30);
    }
}