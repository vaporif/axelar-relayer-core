# Bin Util

This crate provides common binary utilities for Axelar relayer components (ingesters and subscribers).

## Overview

The `bin-util` crate offers a set of shared utilities that simplify the development of relayer binaries by providing:

- **Configuration Management**: Loading and merging configurations from files and environment variables
- **Health Check Server**: Kubernetes-compatible health and readiness endpoints
- **Telemetry**: OpenTelemetry integration for distributed tracing and metrics
- **Logging**: Structured logging with color-eyre error handling
- **Metrics**: Pre-built metric collectors for tracking errors, operations, and blockchain-specific activities

## Features

### Configuration Management

The crate provides utilities to load configuration from TOML files and merge with environment variables:

```rust
use bin_util::config::Config;
use serde::Deserialize;

#[derive(Deserialize)]
struct MyConfig {
    api_url: String,
    timeout_secs: u64,
}

let config: MyConfig = Config::try_from_path("config.toml")?;
```

Environment variables with the `RELAYER_` prefix override configuration file values.

### Health Check Server

Implements a lightweight HTTP server for Kubernetes health probes:

```rust
use bin_util::health_check::{health_check_server, CheckHealth};

#[derive(Clone)]
struct MyHealthChecker;

impl CheckHealth for MyHealthChecker {
    async fn check(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Your health check logic here
        Ok(())
    }
}

let checker = MyHealthChecker;
let server = health_check_server(8080, checker);
```

Endpoints:
- `/healthz` - Liveness probe
- `/readyz` - Readiness probe

### Telemetry

OpenTelemetry setup for tracing and metrics:

```rust
use bin_util::telemetry::init_telemetry;

// Initialize telemetry with your service name
init_telemetry("my-relayer-component")?;
```

Supports both HTTP and gRPC protocols for OTLP export.

### Logging

Integrated logging with tracing and color-eyre:

```rust
use bin_util::init_logger;

// Initialize logging (automatically detects production vs debug)
init_logger()?;
```

- JSON format in production (`PRODUCTION=true`)
- Pretty format with colors in development

### Metrics

Pre-built metric collectors:

#### SimpleMetrics
Basic error and skip counters:

```rust
use bin_util::SimpleMetrics;

let metrics = SimpleMetrics::new();
metrics.error(); // Increment error counter
metrics.skip();  // Increment skip counter
```

#### BlockChainIngesterMetrics
Specialized metrics for blockchain ingesters:

```rust
use bin_util::BlockChainIngesterMetrics;

let metrics = BlockChainIngesterMetrics::new();
metrics.gateway_tx();      // Track gateway transactions
metrics.execute();         // Track executions
metrics.verify();          // Track verifications
metrics.refund();          // Track refunds
metrics.construct_proof(); // Track proof constructions
```

## Configuration Defaults

The crate provides sensible defaults for common configuration parameters:

- `BUNDLE_SIZE`: 50
- `ERROR_BUNDLE_SIZE`: 10
- `WORKER_COUNT`: 1
- `ERROR_COUNTER_LIMIT`: 100

These can be imported from `bin_util::config_defaults`.

## Usage Example

```rust
use bin_util::{init_logger, health_check::health_check_server, SimpleMetrics};
use bin_util::config::Config;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    init_logger()?;
    
    // Load configuration
    let config: MyConfig = Config::try_from_path("config.toml")?;
    
    // Initialize metrics
    let metrics = SimpleMetrics::new();
    
    // Start health check server
    let health_server = health_check_server(8080, MyHealthChecker);
    tokio::spawn(health_server);
    
    // Your application logic here
    
    Ok(())
}
```

## Environment Variables

All configuration can be overridden using environment variables with the `RELAYER_` prefix:

- `RELAYER_API_URL` - Override `api_url` in config
- `RELAYER_TIMEOUT_SECS` - Override `timeout_secs` in config
- `PRODUCTION` - Set to `true` for production logging format
- `OTEL_*` - Standard OpenTelemetry environment variables

## Dependencies

This crate is designed to be lightweight and includes only essential dependencies for binary utilities. It uses tokio for async runtime and integrates with standard observability tools.