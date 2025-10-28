use tracing_subscriber::{
    fmt::{self, time::ChronoUtc},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use std::io;

/// Initialize structured logging for the library
/// 
/// This sets up comprehensive logging with:
/// - JSON formatting for production environments
/// - Pretty formatting for development
/// - Configurable log levels via RUST_LOG environment variable
/// - Timestamp and source location information
pub fn init_logging() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            // Default log levels for different components
            EnvFilter::new("blocks_production_lib=info,warn")
        });

    // Check if we're in a production environment
    let is_production = std::env::var("ENVIRONMENT")
        .map(|env| env == "production" || env == "prod")
        .unwrap_or(false);

    if is_production {
        // JSON logging for production
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .json()
                    .with_timer(ChronoUtc::rfc_3339())
                    .with_target(true)
                    .with_line_number(true)
                    .with_file(true)
                    .with_writer(io::stderr)
            )
            .try_init()?;
    } else {
        // Pretty logging for development
        tracing_subscriber::registry()
            .with(env_filter)
            .with(
                fmt::layer()
                    .pretty()
                    .with_timer(ChronoUtc::rfc_3339())
                    .with_target(true)
                    .with_line_number(true)
                    .with_file(false) // Less verbose for development
                    .with_writer(io::stderr)
            )
            .try_init()?;
    }

    tracing::info!("Logging initialized successfully");
    Ok(())
}

/// Initialize minimal logging for tests
pub fn init_test_logging() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .with_env_filter(EnvFilter::new("blocks_production_lib=debug"))
        .try_init();
}

/// Initialize logging with custom configuration
pub fn init_custom_logging(
    level: &str, 
    format: LogFormat,
    include_location: bool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let env_filter = EnvFilter::new(level);

    match format {
        LogFormat::Json => {
            let layer = fmt::layer()
                .json()
                .with_timer(ChronoUtc::rfc_3339())
                .with_target(true)
                .with_writer(io::stderr);

            if include_location {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(layer.with_line_number(true).with_file(true))
                    .try_init()?;
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(layer.with_line_number(false).with_file(false))
                    .try_init()?;
            }
        },
        LogFormat::Pretty => {
            let layer = fmt::layer()
                .pretty()
                .with_timer(ChronoUtc::rfc_3339())
                .with_target(true)
                .with_writer(io::stderr);

            if include_location {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(layer.with_line_number(true).with_file(true))
                    .try_init()?;
            } else {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(layer.with_line_number(false).with_file(false))
                    .try_init()?;
            }
        },
        LogFormat::Compact => {
            let layer = fmt::layer()
                .compact()
                .with_timer(ChronoUtc::rfc_3339())
                .with_target(true)
                .with_writer(io::stderr);

            tracing_subscriber::registry()
                .with(env_filter)
                .with(layer.with_line_number(include_location).with_file(include_location))
                .try_init()?;
        },
    }

    tracing::info!("Custom logging initialized successfully");
    Ok(())
}

/// Available log formats
#[derive(Debug, Clone, PartialEq)]
pub enum LogFormat {
    /// JSON structured logging (best for production)
    Json,
    /// Pretty formatted logging (best for development)
    Pretty,
    /// Compact single-line logging
    Compact,
}

/// Logging configuration builder
pub struct LoggingConfig {
    level: String,
    format: LogFormat,
    include_location: bool,
    include_spans: bool,
}

impl LoggingConfig {
    pub fn new() -> Self {
        Self {
            level: "info".to_string(),
            format: LogFormat::Pretty,
            include_location: false,
            include_spans: true,
        }
    }

    pub fn level(mut self, level: &str) -> Self {
        self.level = level.to_string();
        self
    }

    pub fn format(mut self, format: LogFormat) -> Self {
        self.format = format;
        self
    }

    pub fn include_location(mut self, include: bool) -> Self {
        self.include_location = include;
        self
    }

    pub fn include_spans(mut self, include: bool) -> Self {
        self.include_spans = include;
        self
    }

    pub fn init(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        init_custom_logging(&self.level, self.format, self.include_location)
    }
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_logging_config_builder() {
        let config = LoggingConfig::new()
            .level("debug")
            .format(LogFormat::Json)
            .include_location(true)
            .include_spans(false);

        assert_eq!(config.level, "debug");
        assert_eq!(config.format, LogFormat::Json);
        assert_eq!(config.include_location, true);
        assert_eq!(config.include_spans, false);
    }

    #[test]
    fn test_logging_config_default() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, "info");
        assert_eq!(config.format, LogFormat::Pretty);
        assert_eq!(config.include_location, false);
        assert_eq!(config.include_spans, true);
    }

    #[test]
    fn test_log_format_equality() {
        assert_eq!(LogFormat::Json, LogFormat::Json);
        assert_eq!(LogFormat::Pretty, LogFormat::Pretty);
        assert_eq!(LogFormat::Compact, LogFormat::Compact);
        assert_ne!(LogFormat::Json, LogFormat::Pretty);
    }

    #[test]
    fn test_log_format_debug() {
        let json_format = LogFormat::Json;
        let debug_str = format!("{:?}", json_format);
        assert_eq!(debug_str, "Json");
    }

    #[test]
    fn test_log_format_clone() {
        let original = LogFormat::Json;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_init_test_logging() {
        // This should not panic
        init_test_logging();
        // Can be called multiple times safely
        init_test_logging();
    }

    #[test]
    fn test_logging_config_chaining() {
        let config = LoggingConfig::new()
            .level("trace")
            .format(LogFormat::Compact)
            .include_location(true)
            .include_spans(false)
            .level("warn"); // Should override previous level

        assert_eq!(config.level, "warn");
        assert_eq!(config.format, LogFormat::Compact);
        assert_eq!(config.include_location, true);
        assert_eq!(config.include_spans, false);
    }

    #[test]
    fn test_format_variants() {
        // Test all format variants exist and are different
        let json = LogFormat::Json;
        let pretty = LogFormat::Pretty;
        let compact = LogFormat::Compact;

        assert_ne!(json, pretty);
        assert_ne!(pretty, compact);
        assert_ne!(json, compact);
    }
}