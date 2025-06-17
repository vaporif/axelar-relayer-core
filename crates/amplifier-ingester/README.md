# Amplifier Ingester

The Amplifier Ingester is a component that consumes events from message queue and forwards them to the Amplifier API. It acts as a bridge between blockchain networks and the Axelar Amplifier protocol.

## Overview

The ingester is designed to:
- Consume events from a message queue (NATS or GCP Pub/Sub)
- Process events concurrently for optimal throughput
- Forward events to the Amplifier API with proper authentication
- Handle retries and error scenarios gracefully

## Architecture

The ingester follows a modular architecture where the core ingestion logic is separated from the infrastructure components. This allows for easy integration with different message queue systems.

```
Blockchain Subscriber → Events → Message Queue → Ingester → Amplifier API
```

## Usage

### For Blockchain Integration

**Important**: When integrating a new blockchain, you don't need to modify the amplifier-ingester crate. Instead:

1. Implement a blockchain-specific subscriber that:
   - Monitors blockchain events (smart contract events, transactions, etc.)
   - Transforms blockchain data into Amplifier event format
   - Publishes events to the configured message queue (NATS/GCP) for amplifier ingester to process

2. The amplifier-ingester will automatically:
   - Consume your published events from the queue
   - Forward them to the Amplifier API
   - Handle authentication, retries, and error scenarios

### Configuration

The ingester requires a configuration file with the following sections:

```toml
# General ingester configuration
concurrent_queue_items = 100

[amplifier]
url = "https://amplifier-api.example.com"
chain = "ethereum"

[amplifier.identity]
# TLS identity configuration

# For NATS
[nats]
urls = ["nats://localhost:4222"]
stream_name = "amplifier_events"
stream_subject = "events.*"
stream_description = "Amplifier event stream"
consumer_description = "Amplifier ingester consumer"
deliver_group = "ingester_group"

# For GCP Pub/Sub
[gcp]
project_id = "your-project-id"
subscription_id = "amplifier-events-sub"
```

### Running

The ingester supports two message queue backends that are mutually exclusive:

```bash
# With GCP Pub/Sub (default)
cargo run --bin amplifier-ingester -- --config config.toml

# With NATS (requires disabling default features)
cargo run --bin amplifier-ingester --no-default-features --features nats -- --config config.toml
```

## Features

- **Concurrent Processing**: Configurable number of concurrent amplifier api clients pushing data
- **Multiple Queue Support**: NATS and GCP Pub/Sub implementations
- **TLS Authentication**: Secure communication with Amplifier API
- **Error Handling**: Automatic retries with exponential backoff
- **Observability**: Integrated metrics and tracing
- **BigInt Precision**: Supports forwarding BigInt features to amplifier-api for blockchain-specific numeric precision (see below)

### Blockchain-Specific BigInt Configuration

The ingester forwards BigInt feature flags to the `amplifier-api` crate, allowing you to optimize numeric precision for your specific blockchain:

- **Default (U256)**: For Ethereum and EVM-compatible chains
- **`bigint-u64`**: For Solana (uses 64-bit integers for all token amounts)
- **`bigint-u128`**: For chains requiring intermediate precision

To build with a specific BigInt type:

```bash
# For Solana (with GCP backend)
cargo build --bin amplifier-ingester --features bigint-u64

# For Solana (with NATS backend)
cargo build --bin amplifier-ingester --no-default-features --features "nats,bigint-u64"

# For chains using u128
cargo build --bin amplifier-ingester --features bigint-u128
```

This feature selection is passed through to the `amplifier-api` dependency, ensuring consistent numeric handling throughout the system.

## Development

### Adding Support for New Message Queues

To add support for a new message queue system:

1. Implement the `Consumer` trait from the infrastructure crate
2. Add a new feature flag in `Cargo.toml`
3. Create a new component module similar to `components/nats.rs`
4. Update the main binary to support the new feature

### Testing

Since GCP and NATS features are mutually exclusive, test each backend separately:

```bash
# Test with GCP (default)
cargo test

# Test with NATS
cargo test --no-default-features --features nats
```

## Related Components

- [amplifier-subscriber](../amplifier-subscriber/README.md): Fetches tasks from Amplifier API and publishes to queues
- [infrastructure](../infrastructure/README.md): Shared infrastructure components
