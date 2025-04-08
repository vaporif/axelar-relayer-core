//! Crate with storage & message bus
pub mod interfaces;
#[cfg(feature = "nats")]
pub mod nats;
