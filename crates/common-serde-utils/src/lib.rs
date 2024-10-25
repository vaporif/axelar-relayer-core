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

/// Decode [`solana_sdk::pubkey::Pubkey`] from a string in base58 format.
///
/// # Errors
/// Returns an error if the provided string is not a valid base58-encoded public key.
///
/// # Errors
/// This function will return an error if:
/// - The deserialized string is not valid base58 data.
/// - The deserialized string cannot be parsed into a [`solana_sdk::pubkey::Pubkey`].
#[cfg(feature = "solana-sdk")]
pub fn pubkey_decode<'de, D>(deserializer: D) -> Result<solana_sdk::pubkey::Pubkey, D::Error>
where
    D: Deserializer<'de>,
{
    use core::str::FromStr as _;

    let raw_string = String::deserialize(deserializer)?;
    let pubkey = solana_sdk::pubkey::Pubkey::from_str(raw_string.as_str())
        .inspect_err(|err| {
            tracing::error!(?err, "cannot parse base58 data");
        })
        .map_err(serde::de::Error::custom)?;
    Ok(pubkey)
}
