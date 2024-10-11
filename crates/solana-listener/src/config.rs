//! Configuration structures and primitives for the [`crate::RelayerEngine`]

use core::time::Duration;

use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;
use typed_builder::TypedBuilder;

/// Top-level configuration for the solana component.
#[derive(Debug, Deserialize, Clone, PartialEq, Eq, TypedBuilder)]
pub struct Config {
    /// Gateway program id
    #[serde(deserialize_with = "serde_utils::pubkey_decode")]
    #[builder(default = config_defaults::gateway_program_address())]
    #[serde(default = "config_defaults::gateway_program_address")]
    pub gateway_program_address: Pubkey,

    /// The rpc of the solana node
    pub solana_rpc: url::Url,

    /// How often we want to poll the network for new signatures
    #[builder(default = config_defaults::tx_scan_poll_period())]
    #[serde(
        rename = "tx_scan_poll_period_in_milliseconds",
        default = "config_defaults::tx_scan_poll_period",
        deserialize_with = "common_serde_utils::duration_ms_decode"
    )]
    pub tx_scan_poll_period: Duration,

    /// How many rpc requests we process at the same time to get data attached to a signature
    #[builder(default = config_defaults::max_concurrent_rpc_requests())]
    #[serde(
        rename = "max_concurrent_rpc_requests",
        default = "config_defaults::max_concurrent_rpc_requests"
    )]
    pub max_concurrent_rpc_requests: usize,
}

pub(crate) mod config_defaults {
    use core::time::Duration;

    use solana_sdk::pubkey::Pubkey;

    pub(crate) const fn tx_scan_poll_period() -> Duration {
        Duration::from_millis(1000)
    }

    pub(crate) const fn gateway_program_address() -> Pubkey {
        gmp_gateway::ID
    }

    pub(crate) const fn max_concurrent_rpc_requests() -> usize {
        5
    }
}

mod serde_utils {
    use core::str::FromStr;

    use serde::{Deserialize, Deserializer};
    use solana_sdk::pubkey::Pubkey;

    pub(crate) fn pubkey_decode<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw_string = String::deserialize(deserializer)?;
        let pubkey = Pubkey::from_str(raw_string.as_str())
            .inspect_err(|err| {
                tracing::error!(?err, "cannot parse base64 data");
            })
            .map_err(serde::de::Error::custom)?;
        Ok(pubkey)
    }
}
