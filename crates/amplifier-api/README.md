# Amplifier API

Rust client library for interacting with the Axelar Amplifier API. This crate provides types and utilities for relayers supporting the Axelar infrastructure.

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
