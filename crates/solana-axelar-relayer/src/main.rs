//! Transaction relayer for Solana-Axelar integration

use std::path::PathBuf;

use relayer_amplifier_api_integration::Amplifier;
use relayer_engine::{RelayerComponent, RelayerEngine};
use serde::Deserialize;

mod telemetry;

#[tokio::main]
async fn main() {
    // Load configuration
    let tracing_endpoint = std::env::var("TRACING_ENDPOINT").ok();
    let config_file = std::env::var("CONFIG")
        .unwrap_or_else(|_| "config.toml".to_owned())
        .parse::<PathBuf>()
        .expect("invalid file path");

    // Initialize tracing
    telemetry::init_telemetry(tracing_endpoint).expect("could not init telemetry");
    color_eyre::install().expect("color eyre could not be installed");

    let config_file = std::fs::read_to_string(config_file).expect("cannot read config file");
    let config = toml::from_str::<Config>(&config_file).expect("invalid config file");

    let (amplifier_component, _amplifier_client) = Amplifier::new(config.amplifier_component);
    let components: Vec<Box<dyn RelayerComponent>> = vec![Box::new(amplifier_component)];
    RelayerEngine::new(config.relayer_engine_config, components)
        .start_and_wait_for_shutdown()
        .await;
}

pub(crate) const fn get_service_version() -> &'static str {
    env!("GIT_HASH")
}

pub(crate) const fn get_service_name() -> &'static str {
    concat!(env!("CARGO_PKG_NAME"), env!("GIT_HASH"))
}

/// Top-level configuration for the relayer.
#[derive(Debug, Deserialize, PartialEq)]
pub struct Config {
    /// Configuration for the Amplifier API processor
    pub amplifier_component: relayer_amplifier_api_integration::Config,
    /// Health check server configuration.
    pub relayer_engine_config: relayer_engine::Config,
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "assertions in tests that return Result is fine"
)]
#[cfg(test)]
mod tests {
    use core::net::SocketAddr;
    use core::str::FromStr;

    use amplifier_api::identity::Identity;
    use pretty_assertions::assert_eq;

    use crate::Config;

    #[test]
    fn parse_toml() -> eyre::Result<()> {
        let amplifier_url = "https://examlple.com".parse()?;
        let healthcheck_bind_addr = "127.0.0.1:8000";
        let chain = "solana-devnet";
        let identity = identity_fixture();
        let input = format!(
            r#"
[amplifier_component]
identity = '''
{identity}
'''
url = "{amplifier_url}"
chain = "{chain}"

[relayer_engine_config]
[relayer_engine_config.health_check]
bind_addr = "{healthcheck_bind_addr}"
"#,
        );

        let parsed: Config = toml::from_str(&input)?;
        let expected = Config {
            amplifier_component: relayer_amplifier_api_integration::Config::builder()
                .identity(Identity::new_from_pem_bytes(identity_fixture().as_bytes())?)
                .url(amplifier_url)
                .chain(chain.to_owned())
                .build(),
            relayer_engine_config: relayer_engine::Config {
                health_check: relayer_engine::HealthCheckConfig {
                    bind_addr: SocketAddr::from_str(healthcheck_bind_addr)?,
                },
            },
        };
        assert_eq!(parsed, expected);
        Ok(())
    }

    fn identity_fixture() -> String {
        include_str!("../../amplifier-api/fixtures/example_cert.pem").to_owned()
    }
}
