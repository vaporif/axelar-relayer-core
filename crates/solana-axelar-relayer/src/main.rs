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

    let event_forwarder_config = solana_event_forwarder::Config::new(
        &config.solana_listener_component,
        &config.amplifier_component,
    );
    let (amplifier_component, amplifier_client) = Amplifier::new(config.amplifier_component);
    let (solana_listener_component, solana_listener_client) =
        solana_listener::SolanaListener::new(config.solana_listener_component);
    let solana_event_forwarder_component = solana_event_forwarder::SolanaEventForwarder::new(
        event_forwarder_config,
        solana_listener_client,
        amplifier_client,
    );
    let components: Vec<Box<dyn RelayerComponent>> = vec![
        Box::new(amplifier_component),
        Box::new(solana_listener_component),
        Box::new(solana_event_forwarder_component),
    ];
    RelayerEngine::new(config.relayer_engine, components)
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
    /// Configuration for the Solana transaction listener processor
    pub solana_listener_component: solana_listener::Config,
    /// Meta-configuration on the engine
    pub relayer_engine: relayer_engine::Config,
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "assertions in tests that return Result is fine"
)]
#[cfg(test)]
mod tests {
    use core::net::SocketAddr;
    use core::str::FromStr;
    use core::time::Duration;

    use amplifier_api::identity::Identity;
    use pretty_assertions::assert_eq;
    use solana_listener::solana_sdk::pubkey::Pubkey;

    use crate::Config;

    #[test]
    fn parse_toml() -> eyre::Result<()> {
        let amplifier_url = "https://examlple.com".parse()?;
        let healthcheck_bind_addr = "127.0.0.1:8000";
        let chain = "solana-devnet";
        let gateway_program_address = Pubkey::new_unique();
        let gateway_program_address_as_str = gateway_program_address.to_string();
        let solana_rpc = "https://solana-devnet.com".parse()?;
        let solana_tx_scan_poll_period = Duration::from_millis(42);
        let solana_tx_scan_poll_period_ms = solana_tx_scan_poll_period.as_millis();
        let max_concurrent_rpc_requests = 100;
        let identity = identity_fixture();
        let input = indoc::formatdoc! {r#"
            [amplifier_component]
            identity = '''
            {identity}
            '''
            url = "{amplifier_url}"
            chain = "{chain}"

            [relayer_engine]
            [relayer_engine.health_check]
            bind_addr = "{healthcheck_bind_addr}"

            [solana_listener_component]
            gateway_program_address = "{gateway_program_address_as_str}"
            solana_rpc = "{solana_rpc}"
            tx_scan_poll_period_in_milliseconds = {solana_tx_scan_poll_period_ms}
            max_concurrent_rpc_requests = {max_concurrent_rpc_requests}
        "#};

        let parsed: Config = toml::from_str(&input)?;
        let expected = Config {
            amplifier_component: relayer_amplifier_api_integration::Config::builder()
                .identity(Identity::new_from_pem_bytes(identity_fixture().as_bytes())?)
                .url(amplifier_url)
                .chain(chain.to_owned())
                .build(),
            relayer_engine: relayer_engine::Config {
                health_check: relayer_engine::HealthCheckConfig {
                    bind_addr: SocketAddr::from_str(healthcheck_bind_addr)?,
                },
            },
            solana_listener_component: solana_listener::Config {
                gateway_program_address,
                solana_rpc,
                tx_scan_poll_period: solana_tx_scan_poll_period,
                max_concurrent_rpc_requests,
            },
        };
        assert_eq!(parsed, expected);
        Ok(())
    }

    fn identity_fixture() -> String {
        include_str!("../../amplifier-api/fixtures/example_cert.pem").to_owned()
    }
}
