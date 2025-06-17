//! Crate for interacting with the Amplifier API.
//! Intended to be used by Relayers supporting the Axelar infrastructure

mod client;
pub use client::*;
mod error;
pub use error::AmplifierApiError;
pub mod types;
pub mod util;
/// Big integer type with configurable underlying type via features
pub mod bigint;
pub use chrono;
