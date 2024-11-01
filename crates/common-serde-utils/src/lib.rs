//! Utilities for deserializing some common structures

use core::time::Duration;

use serde::{Deserialize as _, Deserializer};

/// Decode [`Duratoin`] assuming that the underlying number is representation of duration in
/// milliseconds
///
/// # Errors
/// When the provided number cannot be deserialized into an `u64`
pub fn duration_ms_decode<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    // Deserialize the raw number as a u64
    let raw_number = u64::deserialize(deserializer)?;
    // Convert it to a Duration
    let duration = Duration::from_millis(raw_number);
    Ok(duration)
}
