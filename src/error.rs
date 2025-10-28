use thiserror::Error;
use std::time::Duration;

/// Comprehensive error types for the blocks production library
/// 
/// This enum provides detailed error information to help developers
/// understand and handle different failure scenarios effectively.
#[derive(Error, Debug)]
pub enum BlockProductionError {
    /// HTTP client errors (network, DNS, connection issues)
    #[error("HTTP request failed: {source}")]
    Http {
        source: reqwest::Error,
        /// Additional context about the request that failed
        context: Option<String>,
    },

    /// JSON serialization/deserialization errors
    #[error("JSON processing failed: {source}")]
    Json {
        source: serde_json::Error,
        /// The data that failed to serialize/deserialize (truncated if large)
        data_sample: Option<String>,
    },

    /// RPC endpoint returned an error response
    #[error("RPC error ({code}): {message}")]
    Rpc {
        /// RPC error code
        code: i32,
        /// Error message from RPC
        message: String,
        /// The RPC method that failed
        method: String,
        /// Raw RPC response for debugging
        raw_response: Option<String>,
    },

    /// Invalid configuration provided to client
    #[error("Configuration error: {message}")]
    Config {
        /// Detailed error message
        message: String,
        /// Configuration field that caused the error
        field: Option<String>,
        /// Suggested fix
        suggestion: Option<String>,
    },

    /// Rate limiting triggered
    #[error("Rate limit exceeded: {requests} requests in {window:?}, limit is {limit} per {window:?}")]
    RateLimit {
        /// Current number of requests
        requests: u32,
        /// Rate limit window
        window: Duration,
        /// Maximum allowed requests
        limit: u32,
        /// Time until rate limit resets
        retry_after: Option<Duration>,
    },

    /// Request timeout occurred
    #[error("Request timeout after {duration:?}")]
    Timeout {
        /// Duration after which timeout occurred
        duration: Duration,
        /// Operation that timed out
        operation: String,
        /// Whether this was a connection timeout or read timeout
        timeout_type: TimeoutType,
    },

    /// Invalid slot range provided
    #[error("Invalid slot range: {message}")]
    InvalidSlotRange {
        /// Detailed error message
        message: String,
        /// The invalid range that was provided
        provided_range: Option<(u64, u64)>,
        /// Valid range bounds if known
        valid_range: Option<(u64, u64)>,
    },

    /// No data available from RPC
    #[error("No block production data available for the requested range")]
    NoData {
        /// Slot range that was requested
        requested_range: Option<(u64, u64)>,
        /// Reason why no data is available
        reason: Option<String>,
    },

    /// Retry attempts exhausted
    #[error("Operation failed after {attempts} retry attempts over {total_duration:?}")]
    RetryExhausted {
        /// Number of retry attempts made
        attempts: u32,
        /// Total time spent retrying
        total_duration: Duration,
        /// The last error that occurred
        last_error: Box<BlockProductionError>,
        /// All errors encountered during retries
        error_history: Vec<String>,
    },

    /// Invalid validator public key format
    #[error("Invalid validator public key: {pubkey}")]
    InvalidValidator {
        /// The invalid public key
        pubkey: String,
        /// Expected format
        expected_format: String,
    },

    /// Connection failed to RPC endpoint
    #[error("Failed to connect to RPC endpoint: {endpoint}")]
    ConnectionFailed {
        /// RPC endpoint URL
        endpoint: String,
        /// Underlying connection error
        source: Box<dyn std::error::Error + Send + Sync>,
        /// Whether the endpoint appears to be reachable
        endpoint_reachable: Option<bool>,
    },

    /// RPC response parsing failed
    #[error("Failed to parse RPC response: {reason}")]
    ResponseParsing {
        /// Reason for parsing failure
        reason: String,
        /// Raw response content (truncated)
        response_sample: Option<String>,
        /// Expected response structure
        expected_structure: Option<String>,
    },

    /// Internal library error (should not normally occur)
    #[error("Internal error: {message}")]
    Internal {
        /// Error message
        message: String,
        /// Location where error occurred
        location: Option<String>,
        /// Additional debugging context
        debug_context: Option<String>,
    },

    /// Authentication or authorization failed
    #[error("Authentication failed: {message}")]
    Auth {
        /// Error message
        message: String,
        /// Whether API key is missing or invalid
        auth_type: AuthErrorType,
    },

    /// General error with custom message (use sparingly)
    #[error("Error: {message}")]
    General {
        /// Error message
        message: String,
        /// Optional error category for filtering
        category: Option<ErrorCategory>,
    },
}

/// Types of timeout errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimeoutType {
    /// Connection timeout (failed to establish connection)
    Connection,
    /// Read timeout (connection established but no response)
    Read,
    /// Overall request timeout
    Request,
}

/// Types of authentication errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthErrorType {
    /// API key is missing
    MissingApiKey,
    /// API key is invalid or expired
    InvalidApiKey,
    /// Rate limit exceeded for API key
    QuotaExceeded,
    /// IP address blocked
    IpBlocked,
}

/// Error categories for filtering and handling
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Network-related errors (retryable)
    Network,
    /// Configuration errors (not retryable)
    Configuration,
    /// Data validation errors (not retryable)
    Validation,
    /// RPC-specific errors (may be retryable)
    Rpc,
    /// Rate limiting (retryable after delay)
    RateLimit,
    /// Authentication (retryable with different credentials)
    Authentication,
}

/// Trait for additional error context and handling hints
pub trait ErrorExt {
    /// Whether this error is likely to be resolved by retrying
    fn is_retryable(&self) -> bool;
    
    /// Whether this error indicates a configuration problem
    fn is_config_error(&self) -> bool;
    
    /// Whether this error is transient (temporary)
    fn is_transient(&self) -> bool;
    
    /// Get suggested retry delay if error is retryable
    fn retry_delay(&self) -> Option<Duration>;
    
    /// Get error category for filtering
    fn category(&self) -> ErrorCategory;
    
    /// Get debugging hints for developers
    fn debug_hints(&self) -> Vec<String>;
}

impl ErrorExt for BlockProductionError {
    fn is_retryable(&self) -> bool {
        match self {
            Self::Http { source, .. } => {
                // Network errors are generally retryable
                source.is_timeout() || source.is_connect() || 
                source.status().map_or(true, |s| s.is_server_error())
            },
            Self::Timeout { .. } | Self::RateLimit { .. } | Self::ConnectionFailed { .. } => true,
            Self::Rpc { code, .. } => {
                // Some RPC errors are retryable
                *code == -32603 || // Internal error
                *code == -32000    // Server error
            },
            Self::Json { .. } | 
            Self::Config { .. } | 
            Self::InvalidSlotRange { .. } | 
            Self::InvalidValidator { .. } |
            Self::Auth { .. } |
            Self::NoData { .. } |
            Self::RetryExhausted { .. } |
            Self::ResponseParsing { .. } |
            Self::Internal { .. } => false,
            Self::General { category, .. } => {
                matches!(category, Some(ErrorCategory::Network | ErrorCategory::RateLimit))
            },
        }
    }
    
    fn is_config_error(&self) -> bool {
        matches!(self, 
            Self::Config { .. } | 
            Self::InvalidSlotRange { .. } | 
            Self::InvalidValidator { .. }
        )
    }
    
    fn is_transient(&self) -> bool {
        match self {
            Self::Http { source, .. } => source.is_timeout() || source.is_connect(),
            Self::Timeout { .. } | Self::RateLimit { .. } | Self::ConnectionFailed { .. } => true,
            _ => false,
        }
    }
    
    fn retry_delay(&self) -> Option<Duration> {
        match self {
            Self::RateLimit { retry_after, .. } => {
                retry_after.or(Some(Duration::from_secs(60)))
            },
            Self::Timeout { .. } => Some(Duration::from_secs(5)),
            Self::ConnectionFailed { .. } => Some(Duration::from_secs(2)),
            Self::Http { source, .. } if source.is_timeout() => Some(Duration::from_secs(3)),
            _ if self.is_retryable() => Some(Duration::from_secs(1)),
            _ => None,
        }
    }
    
    fn category(&self) -> ErrorCategory {
        match self {
            Self::Http { .. } | Self::ConnectionFailed { .. } | Self::Timeout { .. } => {
                ErrorCategory::Network
            },
            Self::Config { .. } => ErrorCategory::Configuration,
            Self::InvalidSlotRange { .. } | Self::InvalidValidator { .. } => {
                ErrorCategory::Validation
            },
            Self::Rpc { .. } | Self::ResponseParsing { .. } => ErrorCategory::Rpc,
            Self::RateLimit { .. } => ErrorCategory::RateLimit,
            Self::Auth { .. } => ErrorCategory::Authentication,
            Self::Json { .. } | Self::NoData { .. } | Self::RetryExhausted { .. } | 
            Self::Internal { .. } => ErrorCategory::Network, // Default fallback
            Self::General { category, .. } => {
                category.clone().unwrap_or(ErrorCategory::Network)
            },
        }
    }
    
    fn debug_hints(&self) -> Vec<String> {
        let mut hints = Vec::new();
        
        match self {
            Self::Http { source, .. } => {
                if source.is_timeout() {
                    hints.push("Try increasing timeout duration".to_string());
                    hints.push("Check network connectivity".to_string());
                } else if source.is_connect() {
                    hints.push("Verify RPC endpoint URL is correct".to_string());
                    hints.push("Check if RPC service is running".to_string());
                }
            },
            Self::RateLimit { limit, window, .. } => {
                hints.push(format!("Reduce request frequency to under {limit} per {window:?}"));
                hints.push("Consider using a private RPC endpoint for higher limits".to_string());
            },
            Self::Config { suggestion: Some(suggestion), .. } => {
                hints.push(suggestion.clone());
            },
            Self::Auth { auth_type, .. } => {
                match auth_type {
                    AuthErrorType::MissingApiKey => {
                        hints.push("Add API key to request headers".to_string());
                    },
                    AuthErrorType::InvalidApiKey => {
                        hints.push("Verify API key is correct and not expired".to_string());
                    },
                    AuthErrorType::QuotaExceeded => {
                        hints.push("Upgrade RPC plan or wait for quota reset".to_string());
                    },
                    AuthErrorType::IpBlocked => {
                        hints.push("Contact RPC provider to unblock IP address".to_string());
                    },
                }
            },
            Self::Timeout { timeout_type, duration, .. } => {
                hints.push(format!("Request timed out after {duration:?}"));
                match timeout_type {
                    TimeoutType::Request => {
                        hints.push("Consider increasing request timeout".to_string());
                        hints.push("Check network latency to RPC endpoint".to_string());
                    },
                    TimeoutType::Connection => {
                        hints.push("Check network connectivity".to_string());
                        hints.push("Verify RPC endpoint is reachable".to_string());
                    },
                    TimeoutType::Read => {
                        hints.push("RPC server is not responding".to_string());
                        hints.push("Try a different RPC endpoint".to_string());
                    },
                }
            },
            _ => {},
        }
        
        hints
    }
}

/// Result type alias for convenience
pub type Result<T> = std::result::Result<T, BlockProductionError>;

impl From<tokio::time::error::Elapsed> for BlockProductionError {
    fn from(_elapsed: tokio::time::error::Elapsed) -> Self {
        Self::Timeout {
            duration: Duration::from_secs(30), // Default timeout
            operation: "RPC request".to_string(),
            timeout_type: TimeoutType::Request,
        }
    }
}

impl From<reqwest::Error> for BlockProductionError {
    fn from(error: reqwest::Error) -> Self {
        Self::Http {
            source: error,
            context: None,
        }
    }
}

impl From<serde_json::Error> for BlockProductionError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json {
            source: error,
            data_sample: None,
        }
    }
}

/// Helper for creating configuration errors with suggestions
impl BlockProductionError {
    pub fn config_error(message: &str, field: Option<&str>, suggestion: Option<&str>) -> Self {
        Self::Config {
            message: message.to_string(),
            field: field.map(String::from),
            suggestion: suggestion.map(String::from),
        }
    }
    
    #[must_use] 
    pub const fn rate_limit_error(requests: u32, limit: u32, window: Duration) -> Self {
        Self::RateLimit {
            requests,
            window,
            limit,
            retry_after: Some(window),
        }
    }
    
    #[must_use] 
    pub fn connection_failed(endpoint: &str, source: Box<dyn std::error::Error + Send + Sync>) -> Self {
        Self::ConnectionFailed {
            endpoint: endpoint.to_string(),
            source,
            endpoint_reachable: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_error_ext_retryable() {
        // Network errors should be retryable
        let timeout_error = BlockProductionError::Timeout {
            duration: Duration::from_secs(30),
            operation: "test".to_string(),
            timeout_type: TimeoutType::Request,
        };
        assert!(timeout_error.is_retryable());

        // Configuration errors should not be retryable
        let config_error = BlockProductionError::Config {
            message: "Invalid config".to_string(),
            field: Some("endpoint".to_string()),
            suggestion: Some("Fix the endpoint".to_string()),
        };
        assert!(!config_error.is_retryable());
    }

    #[test]
    fn test_error_ext_transient() {
        let rate_limit = BlockProductionError::RateLimit {
            requests: 100,
            window: Duration::from_secs(60),
            limit: 50,
            retry_after: Some(Duration::from_secs(30)),
        };
        assert!(rate_limit.is_transient());

        let json_error = BlockProductionError::Json {
            source: serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
            data_sample: None,
        };
        assert!(!json_error.is_transient());
    }

    #[test]
    fn test_error_ext_config_error() {
        let config_error = BlockProductionError::Config {
            message: "Bad config".to_string(),
            field: None,
            suggestion: None,
        };
        assert!(config_error.is_config_error());

        let http_error = BlockProductionError::Timeout {
            timeout_type: TimeoutType::Request,
            duration: Duration::from_secs(30),
            operation: "Test timeout".to_string(),
        };
        assert!(!http_error.is_config_error());
    }

    #[test]
    fn test_error_retry_delay() {
        let timeout_error = BlockProductionError::Timeout {
            duration: Duration::from_secs(30),
            operation: "test".to_string(),
            timeout_type: TimeoutType::Request,
        };
        let delay = timeout_error.retry_delay();
        assert!(delay.is_some());
        let delay = delay.unwrap();
        assert!(delay >= Duration::from_secs(1));
        assert!(delay <= Duration::from_secs(10));

        let rate_limit = BlockProductionError::RateLimit {
            requests: 100,
            window: Duration::from_secs(60),
            limit: 50,
            retry_after: Some(Duration::from_secs(45)),
        };
        assert_eq!(rate_limit.retry_delay(), Some(Duration::from_secs(45)));
    }

    #[test]
    fn test_error_debug_hints() {
        let config_error = BlockProductionError::Config {
            message: "Invalid endpoint".to_string(),
            field: Some("rpc_endpoint".to_string()),
            suggestion: Some("Use a valid URL".to_string()),
        };
        let hints = config_error.debug_hints();
        assert!(!hints.is_empty());
        assert!(hints[0].contains("Use a valid URL"));

        let timeout_error = BlockProductionError::Timeout {
            duration: Duration::from_secs(5),
            operation: "test".to_string(),
            timeout_type: TimeoutType::Request,
        };
        let hints = timeout_error.debug_hints();
        assert!(!hints.is_empty());
    }

    #[test]
    fn test_error_category() {
        let config_error = BlockProductionError::Config {
            message: "test".to_string(),
            field: None,
            suggestion: None,
        };
        assert_eq!(config_error.category(), ErrorCategory::Configuration);

        let http_error = BlockProductionError::Timeout {
            timeout_type: TimeoutType::Request,
            duration: Duration::from_secs(30),
            operation: "Test timeout".to_string(),
        };
        assert_eq!(http_error.category(), ErrorCategory::Network);

        let json_error = BlockProductionError::Json {
            source: serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err(),
            data_sample: None,
        };
        assert_eq!(json_error.category(), ErrorCategory::Network);
    }

    #[test]
    fn test_timeout_type_debug() {
        assert_eq!(format!("{:?}", TimeoutType::Request), "Request");
        assert_eq!(format!("{:?}", TimeoutType::Connection), "Connection");
    }

    #[test]
    fn test_auth_error_type_debug() {
        assert_eq!(format!("{:?}", AuthErrorType::InvalidApiKey), "InvalidApiKey");
        assert_eq!(format!("{:?}", AuthErrorType::QuotaExceeded), "QuotaExceeded");
    }

    #[test]
    fn test_error_category_debug() {
        assert_eq!(format!("{:?}", ErrorCategory::Network), "Network");
        assert_eq!(format!("{:?}", ErrorCategory::Configuration), "Configuration");
        assert_eq!(format!("{:?}", ErrorCategory::Validation), "Validation");
        assert_eq!(format!("{:?}", ErrorCategory::Authentication), "Authentication");
        assert_eq!(format!("{:?}", ErrorCategory::RateLimit), "RateLimit");
    }

    #[test]
    fn test_constructor_methods() {
        let rate_limit = BlockProductionError::rate_limit_error(100, 50, Duration::from_secs(60));
        match rate_limit {
            BlockProductionError::RateLimit { requests, window, limit, .. } => {
                assert_eq!(requests, 100);
                assert_eq!(window, Duration::from_secs(60));
                assert_eq!(limit, 50);
            }
            _ => panic!("Expected RateLimit error"),
        }

        let connection_failed = BlockProductionError::connection_failed(
            "https://test.com", 
            Box::new(std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout"))
        );
        match connection_failed {
            BlockProductionError::ConnectionFailed { endpoint, .. } => {
                assert_eq!(endpoint, "https://test.com");
            }
            _ => panic!("Expected ConnectionFailed error"),
        }
    }

    #[test]
    fn test_error_display() {
        let config_error = BlockProductionError::Config {
            message: "Invalid endpoint URL".to_string(),
            field: Some("rpc_endpoint".to_string()),
            suggestion: Some("Use https:// prefix".to_string()),
        };
        let display = format!("{}", config_error);
        assert!(display.contains("Configuration error"));
        assert!(display.contains("Invalid endpoint URL"));
    }

    #[test]
    fn test_error_debug() {
        let timeout_error = BlockProductionError::Timeout {
            duration: Duration::from_secs(30),
            operation: "fetch data".to_string(),
            timeout_type: TimeoutType::Request,
        };
        let debug = format!("{:?}", timeout_error);
        assert!(debug.contains("Timeout"));
        assert!(debug.contains("30s"));
    }

    #[test]
    fn test_from_conversions() {
        // Test that we can convert from serde_json errors
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let block_error: BlockProductionError = json_error.into();
        match block_error {
            BlockProductionError::Json { .. } => {
                // Expected
            }
            _ => panic!("Expected Json error"),
        }
    }
}