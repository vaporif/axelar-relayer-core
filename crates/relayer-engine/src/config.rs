//! Configuration structures and primitives for the [`crate::RelayerEngine`]

use core::net::SocketAddr;
use core::time::Duration;
use std::sync::Arc;

use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use url::Url;

/// Top-level configuration for the relayer.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    /// Configuration for relaying from Axelar to Solana.
    pub axelar_to_solana: Option<AxelarToSolana>,
    /// Configuration for relaying from Solana to Axelar.
    pub solana_to_axelar: Option<SolanaToAxelar>,
    /// Database configuration.
    pub database: Database,
    /// Health check server configuration.
    pub health_check: HealthCheck,
    /// Cancellation timeout duration, in seconds.
    #[serde(
        rename = "cancellation_timeout_in_seconds",
        default = "config_defaults::cancellation_timeout"
    )]
    pub cancellation_timeout: Duration,
}

/// Configuration for relaying from Axelar to Solana.
#[derive(Debug, Deserialize, PartialEq)]
pub struct AxelarToSolana {
    /// Configuration for the Axelar approver.
    pub approver: AxelarApprover,
    /// Configuration for including transactions on Solana.
    pub includer: SolanaIncluder,
}

/// Configuration for relaying from Solana to Axelar.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct SolanaToAxelar {
    /// Configuration for monitoring Solana transactions.
    pub sentinel: SolanaSentinel,
    /// Configuration for the Axelar verifier.
    pub verifier: AxelarVerifier,
}

/// Database configuration.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Database {
    /// Database connection URL.
    #[serde(deserialize_with = "serde_utils::deserialize_url")]
    pub url: Url,
}

/// Health check server configuration.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct HealthCheck {
    /// Address to bind the health check server.
    #[serde(deserialize_with = "serde_utils::deserialize_socket_addr")]
    pub bind_addr: SocketAddr,
}

/// Configuration for the Axelar approver.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct AxelarApprover {
    /// Axelar RPC endpoint URL.
    #[serde(deserialize_with = "serde_utils::deserialize_url")]
    pub rpc: Url,
    /// Name of the Solana chain as known to Axelar.
    #[serde(default = "config_defaults::solana_chain_name")]
    pub solana_chain_name: String,
}

/// Configuration for including transactions on Solana.
#[derive(Debug, Deserialize, PartialEq)]
pub struct SolanaIncluder {
    /// Solana RPC endpoint URL.
    #[serde(deserialize_with = "serde_utils::deserialize_url")]
    pub rpc: Url,
    /// Keypair for signing transactions.
    #[serde(deserialize_with = "serde_utils::deserialize_keypair")]
    pub keypair: Arc<Keypair>,
    /// Address of the Solana gateway program.
    #[serde(deserialize_with = "serde_utils::deserialize_pubkey")]
    pub gateway_address: Pubkey,
    /// Address of the Solana gateway configuration account.
    #[serde(deserialize_with = "serde_utils::deserialize_pubkey")]
    pub gateway_config_address: Pubkey,
}

/// Configuration for monitoring Solana transactions.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct SolanaSentinel {
    /// Address of the Solana gateway program.
    #[serde(deserialize_with = "serde_utils::deserialize_pubkey")]
    pub gateway_address: Pubkey,
    /// Solana RPC endpoint URL.
    #[serde(deserialize_with = "serde_utils::deserialize_url")]
    pub rpc: Url,
    /// Name of the Solana chain that's pre-configured on Axelar.
    #[serde(default = "config_defaults::solana_chain_name")]
    pub solana_chain_name: String,
    /// Configuration for the transaction scanner.
    #[serde(default)]
    pub transaction_scanner: TransactionScanner,
}

/// Configuration for the transaction scanner.
#[derive(Debug, Deserialize, PartialEq, Eq, Clone, Copy)]
pub struct TransactionScanner {
    /// Interval for fetching signatures, in seconds.
    #[serde(
        rename = "fetch_signatures_interval_in_seconds",
        default = "config_defaults::sentinel_fetch_signatures_interval"
    )]
    pub fetch_signatures_interval: Duration,
    /// Maximum number of concurrent RPC requests.
    #[serde(default = "config_defaults::sentinel_max_concurrent_rpc_requests")]
    pub max_concurrent_rpc_requests: usize,
    /// Capacity of the transaction queue.
    #[serde(default = "config_defaults::sentinel_queue_capacity")]
    pub queue_capacity: usize,
}

impl Default for TransactionScanner {
    fn default() -> Self {
        Self {
            fetch_signatures_interval: config_defaults::sentinel_fetch_signatures_interval(),
            max_concurrent_rpc_requests: config_defaults::sentinel_max_concurrent_rpc_requests(),
            queue_capacity: config_defaults::sentinel_queue_capacity(),
        }
    }
}

/// Configuration for the Axelar verifier.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct AxelarVerifier {
    /// Axelar RPC endpoint URL.
    #[serde(deserialize_with = "serde_utils::deserialize_uri")]
    pub rpc: Url,
}

/// Utility functions for custom deserialization with serde.
mod serde_utils {
    use core::str::FromStr;

    use serde::Deserializer;

    use super::*;

    /// Deserializes a base58-encoded keypair from a string or environment variable.
    pub(crate) fn deserialize_keypair<'de, D>(deserializer: D) -> Result<Arc<Keypair>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let env_var = from_env(deserializer)?;
        let bytes = solana_sdk::bs58::decode(env_var)
            .into_vec()
            .map_err(serde::de::Error::custom)?;
        Keypair::from_bytes(&bytes)
            .map(Arc::new)
            .map_err(serde::de::Error::custom)
    }

    /// Deserializes a public key from a string or environment variable.
    pub(crate) fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let env_var = from_env(deserializer)?;
        Pubkey::from_str(&env_var).map_err(serde::de::Error::custom)
    }

    /// Deserializes a socket address from a string or environment variable.
    pub(crate) fn deserialize_socket_addr<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error>
    where
        D: Deserializer<'de>,
    {
        let env_var = from_env(deserializer)?;
        SocketAddr::from_str(&env_var).map_err(serde::de::Error::custom)
    }

    /// Deserializes a URI from a string or environment variable.
    pub(crate) fn deserialize_uri<'de, D>(deserializer: D) -> Result<Url, D::Error>
    where
        D: Deserializer<'de>,
    {
        let env_var = from_env(deserializer)?;
        Url::from_str(&env_var).map_err(serde::de::Error::custom)
    }

    /// Deserializes a URL from a string or environment variable.
    pub(crate) fn deserialize_url<'de, D>(deserializer: D) -> Result<Url, D::Error>
    where
        D: Deserializer<'de>,
    {
        let env_var = from_env(deserializer)?;
        Url::from_str(&env_var).map_err(serde::de::Error::custom)
    }

    /// Deserializes a string and resolves it as an environment variable if prefixed with `$`.
    fn from_env<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_string = String::deserialize(deserializer)?;
        if let Some(env_var) = raw_string.strip_prefix('$') {
            std::env::var(env_var).map_err(serde::de::Error::custom)
        } else {
            Ok(raw_string)
        }
    }
}

mod config_defaults {
    use super::*;
    pub(crate) const fn cancellation_timeout() -> Duration {
        Duration::from_secs(30)
    }
    pub(crate) const fn sentinel_fetch_signatures_interval() -> Duration {
        Duration::from_secs(5)
    }
    pub(crate) const fn sentinel_max_concurrent_rpc_requests() -> usize {
        20
    }
    pub(crate) const fn sentinel_queue_capacity() -> usize {
        1_000
    }

    pub(crate) fn solana_chain_name() -> String {
        "solana".into()
    }
}
