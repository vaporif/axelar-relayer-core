# Amplifier API

Rust client library for interacting with the Axelar Amplifier API. This crate provides types and utilities for relayers supporting the Axelar infrastructure.

## Features

### BigInt Type Configuration

The `BigInt` type used for token amounts can be configured at compile-time to match your blockchain's native precision requirements:

- **Default (U256)**: 256-bit unsigned integer - suitable for Ethereum and EVM-compatible chains
- **`bigint-u64`**: 64-bit unsigned integer - suitable for Solana and similar chains
- **`bigint-u128`**: 128-bit unsigned integer - suitable for chains with intermediate precision requirements

This feature allows you to optimize for your specific blockchain without handling unnecessary large number parsing. For example, Solana uses u64 for all token amounts, so enabling the `bigint-u64` feature avoids the overhead of parsing U256 values.

#### Usage Examples

```toml
# Default - uses U256 for Ethereum/EVM chains
amplifier-api = "0.1"

# For Solana integration
amplifier-api = { version = "0.1", features = ["bigint-u64"] }

# For chains requiring u128 precision
amplifier-api = { version = "0.1", features = ["bigint-u128"] }
```

## API Types

The crate provides comprehensive type definitions for:

- Events (MessageApproved, MessageExecuted, GasCredit, etc.)
- Tasks (Execute, Refund, Verify)
- Utilities for serialization/deserialization with both JSON and Borsh

## Examples

See the `examples/` directory for usage examples, including health check functionality.

## Dependencies

- `bnum` - For U256 support (default BigInt type)
- `borsh` - Binary serialization
- `serde` - JSON serialization
- `reqwest` - HTTP client with middleware support
- `chrono` - DateTime handling