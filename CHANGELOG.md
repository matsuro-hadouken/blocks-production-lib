# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-10-27

### Added
- Initial release of blocks-production-lib
- `BlockProductionClient` with builder pattern for configuration
- Comprehensive statistics calculation for validator skip rates
- Multiple preset configurations:
  - Public RPC configuration (conservative rate limiting)
  - Private RPC configuration (higher rate limits) 
  - High-frequency configuration (optimized for frequent requests)
  - Batch processing configuration (optimized for bulk operations)
  - Development configuration (verbose settings)
  - Enterprise configuration (production use)
- Provider-specific optimizations:
  - Helius RPC optimization
  - QuickNode RPC optimization  
  - Alchemy RPC optimization
  - Auto-detection based on endpoint URL
- Rate limiting with configurable requests per second
- Retry logic with exponential backoff
- Comprehensive error handling with custom error types
- Support for custom HTTP headers and timeouts
- Production and debug output formats
- Methods for performance analysis:
  - Get top/bottom performers
  - Get validators with concerning skip rates (>5%)
  - Get validators with perfect performance (0% skip rate)
  - Fetch data for specific validators or slot ranges
- Extensive test coverage:
  - Unit tests for all core functionality
  - Integration tests with mock RPC responses
  - Tests for error conditions and edge cases
- Comprehensive examples:
  - Basic usage example
  - Advanced configuration example
  - Statistics analysis example
- Documentation:
  - Complete API documentation
  - Usage examples
  - Configuration guides
  - Error handling examples

### Technical Details
- Built with async/await using tokio runtime
- Uses reqwest for HTTP client with connection pooling
- Governor crate for rate limiting
- Tokio-retry for retry logic with exponential backoff
- Thiserror for structured error handling
- Chrono for timestamp handling
- Serde for JSON serialization/deserialization
- Comprehensive logging support
- Memory-efficient data structures
- Support for concurrent request limiting

### Dependencies
- reqwest 0.12+ (HTTP client)
- tokio 1.48+ (async runtime)
- serde 1.0+ (serialization)
- thiserror 2.0+ (error handling)
- governor 0.10+ (rate limiting)
- tokio-retry 0.3+ (retry logic)
- chrono 0.4+ (time handling)
- anyhow 1.0+ (error utilities)
- once_cell 1.0+ (lazy statics)

[Unreleased]: https://github.com/matsuro-hadouken/blocks-production-lib/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/matsuro-hadouken/blocks-production-lib/releases/tag/v0.1.0