# Amplifier Subscriber

The Amplifier Subscriber is a component that fetches tasks from the Amplifier API and publishes them to message queues for processing by blockchain-specific ingester. It enables the distribution of cross-chain tasks to the appropriate blockchain handlers.

## Overview

The subscriber is designed to:
- Poll the Amplifier API for new tasks
- Fetch tasks in configurable batches
- Publish tasks to message queues (NATS or GCP Pub/Sub)
- Maintain reliable task distribution with proper error handling

## Architecture

The subscriber acts as a task distributor in the Axelar Amplifier system:

```
Amplifier API → Subscriber → Tasks → Message Queue → Blockchain Ingester
```

## Usage

### For Blockchain Integration

**Important**: When integrating a new blockchain, you don't need to modify the amplifier-subscriber crate. Instead:

1. Implement a blockchain-specific ingester that:
   - Consumes tasks from the message queue pushed by amplifier subscriber
   - Processes tasks specific to your blockchain
   - Executes transactions or operations on the target blockchain

2. The amplifier-subscriber will automatically:
   - Fetch tasks from the Amplifier API
   - Publish them to the configured message queue
   - Handle authentication and error scenarios

### Configuration

The subscriber requires a configuration file with the following sections:

```toml
# General subscriber configuration
limit_per_request = 50

[amplifier_component]
url = "https://amplifier-api.example.com"
chain = "ethereum"

[amplifier_component.identity]
# TLS identity configuration

# For NATS
[nats]
urls = ["nats://localhost:4222"]
stream_name = "amplifier_tasks"
stream_subject = "tasks.*"
stream_description = "Amplifier task stream"

# For GCP Pub/Sub
[gcp]
project_id = "your-project-id"
topic_id = "amplifier-tasks"
```

### Running

```bash
# With NATS
cargo run --bin amplifier-subscriber --features nats -- --config config.toml

# With GCP Pub/Sub
cargo run --bin amplifier-subscriber --features gcp -- --config config.toml
```

## Features

- **Batch Processing**: Configurable batch size for task fetching
- **Multiple Queue Support**: NATS and GCP Pub/Sub implementations
- **TLS Authentication**: Secure communication with Amplifier API
- **Reliable Publishing**: Ensures tasks are successfully published to queues
- **Observability**: Integrated metrics and tracing

## Development

### Adding Support for New Message Queues

To add support for a new message queue system:

1. Implement the `Publisher` trait from the infrastructure crate
2. Add a new feature flag in `Cargo.toml`
3. Create a new component module similar to `components/nats.rs`
4. Update the main binary to support the new feature

### Testing

```bash
cargo test --features nats
cargo test --features gcp
```

## Performance Considerations

- **Polling Interval**: The subscriber polls continuously; ensure your `limit_per_request` is appropriate for your load
- **Queue Capacity**: Ensure your message queue can handle the task throughput
- **Network Latency**: Consider network latency when configuring batch sizes

## Related Components

- [amplifier-ingester](../amplifier-ingester/README.md): Consumes events and forwards to Amplifier API
- [infrastructure](../infrastructure/README.md): Shared infrastructure components
