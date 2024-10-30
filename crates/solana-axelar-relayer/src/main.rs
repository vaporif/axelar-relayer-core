//! Transaction relayer for Solana-Axelar integration

use std::path::PathBuf;
use std::sync::Arc;

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

    let rpc_client = retrying_solana_http_sender::new_client(&config.solana_rpc);
    let event_forwarder_config = solana_event_forwarder::Config::new(
        &config.solana_listener_component,
        &config.amplifier_component,
    );
    let name_on_amplifier = config.amplifier_component.chain.clone();
    let (amplifier_component, amplifier_client, amplifier_task_receiver) =
        Amplifier::new(config.amplifier_component);
    let gateway_task_processor = solana_gateway_task_processor::SolanaTxPusher::new(
        config.solana_gateway_task_processor,
        name_on_amplifier.clone(),
        Arc::clone(&rpc_client),
        amplifier_task_receiver,
    );
    let (solana_listener_component, solana_listener_client) = solana_listener::SolanaListener::new(
        config.solana_listener_component,
        Arc::clone(&rpc_client),
    );
    let solana_event_forwarder_component = solana_event_forwarder::SolanaEventForwarder::new(
        event_forwarder_config,
        solana_listener_client,
        amplifier_client,
    );
    let components: Vec<Box<dyn RelayerComponent>> = vec![
        Box::new(amplifier_component),
        Box::new(solana_listener_component),
        Box::new(solana_event_forwarder_component),
        Box::new(gateway_task_processor),
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
    /// Configuration for the Solana transaction listener processor
    pub solana_gateway_task_processor: solana_gateway_task_processor::Config,
    /// Meta-configuration on the engine
    pub relayer_engine: relayer_engine::Config,
    /// Shared configuration for the Solana RPC client
    pub solana_rpc: retrying_solana_http_sender::Config,
}

#[expect(
    clippy::panic_in_result_fn,
    reason = "assertions in tests that return Result is fine"
)]
#[cfg(test)]
mod tests {
    use core::net::SocketAddr;
    use core::str::FromStr as _;
    use core::time::Duration;

    use amplifier_api::identity::Identity;
    use pretty_assertions::assert_eq;
    use solana_listener::solana_sdk::pubkey::Pubkey;
    use solana_listener::solana_sdk::signature::{Keypair, Signature};
    use solana_listener::MissedSignatureCatchupStrategy;

    use crate::Config;

    #[test]
    fn parse_toml() -> eyre::Result<()> {
        let amplifier_url = "https://examlple.com".parse()?;
        let healthcheck_bind_addr = "127.0.0.1:8000";
        let chain = "solana-devnet";
        let gateway_program_address = Pubkey::new_unique();
        let gateway_program_address_as_str = gateway_program_address.to_string();
        let solana_rpc = "https://api.solana-devnet.com".parse()?;
        let solana_ws = "wss://api.solana-devnet.com".parse()?;
        let solana_tx_scan_poll_period = Duration::from_millis(42);
        let solana_tx_scan_poll_period_ms = solana_tx_scan_poll_period.as_millis();
        let max_concurrent_rpc_requests = 100;
        let signing_keypair = Keypair::new();
        let signing_keypair_as_str = signing_keypair.to_base58_string();
        let latest_processed_signature = Signature::new_unique().to_string();
        let identity = identity_fixture();
        let missed_signature_catchup_strategy = "until_beginning";
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
            solana_ws = "{solana_ws}"
            tx_scan_poll_period_in_milliseconds = {solana_tx_scan_poll_period_ms}
            missed_signature_catchup_strategy = "{missed_signature_catchup_strategy}"
            latest_processed_signature = "{latest_processed_signature}"

            [solana_gateway_task_processor]
            signing_keypair = "{signing_keypair_as_str}"
            gateway_program_address = "{gateway_program_address_as_str}"

            [solana_rpc]            
            max_concurrent_rpc_requests = {max_concurrent_rpc_requests}
            solana_http_rpc = "{solana_rpc}"
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
                tx_scan_poll_period: solana_tx_scan_poll_period,
                solana_ws,
                missed_signature_catchup_strategy: MissedSignatureCatchupStrategy::UntilBeginning,
                latest_processed_signature: Some(Signature::from_str(&latest_processed_signature)?),
            },
            solana_gateway_task_processor: solana_gateway_task_processor::Config {
                gateway_program_address,
                signing_keypair,
            },
            solana_rpc: retrying_solana_http_sender::Config {
                max_concurrent_rpc_requests,
                solana_http_rpc: solana_rpc,
            },
        };
        assert_eq!(parsed, expected);
        Ok(())
    }

    fn identity_fixture() -> String {
        include_str!("../../amplifier-api/fixtures/example_cert.pem").to_owned()
    }
}
