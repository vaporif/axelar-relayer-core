//! Transaction relayer for Solana-Axelar integration

use std::path::PathBuf;

use relayer_engine::RelayerEngine;

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
    let config = toml::from_str::<relayer_engine::config::Config>(&config_file)
        .expect("invalid config file");

    RelayerEngine::new(config)
        .start_and_wait_for_shutdown()
        .await;
}

pub(crate) const fn get_service_version() -> &'static str {
    env!("GIT_HASH")
}

pub(crate) const fn get_service_name() -> &'static str {
    concat!(env!("CARGO_PKG_NAME"), env!("GIT_HASH"))
}

#[cfg(test)]
#[expect(
    clippy::panic_in_result_fn,
    reason = "the tests use result type to propagate errors that cannot fail"
)]
mod tests {
    use core::net::SocketAddr;
    use core::str::FromStr;
    use core::time::Duration;
    use std::sync::Arc;

    use relayer_engine::config::{
        AxelarApprover, AxelarToSolana, AxelarVerifier, Config, Database, HealthCheck,
        SolanaIncluder, SolanaSentinel, SolanaToAxelar, TransactionScanner,
    };
    use relayer_engine::solana_sdk::pubkey::Pubkey;
    use relayer_engine::solana_sdk::signature::Keypair;
    use relayer_engine::url::Url;

    #[test]
    fn parse_toml() -> eyre::Result<()> {
        let gateway_address = Pubkey::new_unique();
        let gateway_config_address = Pubkey::new_unique();
        let relayer_keypair = Keypair::new();
        let database_url = "postgres://user:password@localhost:5432/dbname";
        let healthcheck_bind_addr = "127.0.0.1:8000";

        let input = format!(
            r#"[axelar_to_solana.approver]
            rpc = "http://0.0.0.1/"

            [axelar_to_solana.includer]
            rpc = "http://0.0.0.2/"
            keypair = "{relayer_keypair_b58}"
            gateway_address = "{gateway_address}"
            gateway_config_address = "{gateway_config_address}"

            [solana_to_axelar.sentinel]
            gateway_address = "{gateway_address}"
            rpc = "http://0.0.0.3/"

            [solana_to_axelar.verifier]
            rpc = "http://0.0.0.4/"

            [database]
            url = "{database_url}"

            [health_check]
            bind_addr = "{healthcheck_bind_addr}""#,
            relayer_keypair_b58 = relayer_keypair.to_base58_string()
        );

        let parsed: Config = toml::from_str(&input)?;
        let expected = Config {
            axelar_to_solana: Some(AxelarToSolana {
                approver: AxelarApprover {
                    solana_chain_name: "solana".into(),
                    rpc: Url::parse("http://0.0.0.1/")?,
                },
                includer: SolanaIncluder {
                    rpc: Url::parse("http://0.0.0.2/")?,
                    keypair: Arc::new(relayer_keypair),
                    gateway_address,
                    gateway_config_address,
                },
            }),
            solana_to_axelar: Some(SolanaToAxelar {
                sentinel: SolanaSentinel {
                    solana_chain_name: "solana".into(),
                    gateway_address,
                    rpc: Url::parse("http://0.0.0.3/")?,
                    transaction_scanner: TransactionScanner::default(),
                },
                verifier: AxelarVerifier {
                    rpc: Url::parse("http://0.0.0.4/")?,
                },
            }),
            database: Database {
                url: Url::parse(database_url)?,
            },
            health_check: HealthCheck {
                bind_addr: SocketAddr::from_str(healthcheck_bind_addr)?,
            },
            cancellation_timeout: Duration::from_secs(30),
        };
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn partial_axelar_to_solana() -> eyre::Result<()> {
        let gateway_address = Pubkey::new_unique();
        let gateway_config_address = Pubkey::new_unique();
        let relayer_keypair = Keypair::new();
        let database_url = "postgres://user:password@localhost:5432/dbname";
        let healthcheck_bind_addr = "127.0.0.1:8000";

        let input = format!(
            r#"[axelar_to_solana.approver]
            rpc = "http://0.0.0.1/"

            [axelar_to_solana.includer]
            rpc = "http://0.0.0.2/"
            keypair = "{relayer_keypair_b58}"
            gateway_address = "{gateway_address}"
            gateway_config_address = "{gateway_config_address}"

            [database]
            url = "{database_url}"

            [health_check]
            bind_addr = "{healthcheck_bind_addr}""#,
            relayer_keypair_b58 = relayer_keypair.to_base58_string()
        );

        let parsed: Config = toml::from_str(&input)?;
        let expected = Config {
            axelar_to_solana: Some(AxelarToSolana {
                approver: AxelarApprover {
                    rpc: Url::parse("http://0.0.0.1/")?,
                    solana_chain_name: "solana".into(),
                },
                includer: SolanaIncluder {
                    rpc: Url::parse("http://0.0.0.2/")?,
                    keypair: Arc::new(relayer_keypair),
                    gateway_address,
                    gateway_config_address,
                },
            }),
            solana_to_axelar: None,
            database: Database {
                url: Url::parse(database_url)?,
            },
            health_check: HealthCheck {
                bind_addr: SocketAddr::from_str(healthcheck_bind_addr)?,
            },
            cancellation_timeout: Duration::from_secs(30),
        };
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn partial_solana_to_axelar() -> eyre::Result<()> {
        let gateway_address = Pubkey::new_unique();
        let database_url = "postgres://user:password@localhost:5432/dbname";
        let healthcheck_bind_addr = "127.0.0.1:8000";

        let input = format!(
            r#"[solana_to_axelar.sentinel]
            gateway_address = "{gateway_address}"
            rpc = "http://0.0.0.3/"

            [solana_to_axelar.verifier]
            rpc = "http://0.0.0.4/"

            [database]
            url = "{database_url}"

            [health_check]
            bind_addr = "{healthcheck_bind_addr}""#
        );

        let parsed: Config = toml::from_str(&input)?;
        let expected = Config {
            axelar_to_solana: None,
            solana_to_axelar: Some(SolanaToAxelar {
                sentinel: SolanaSentinel {
                    solana_chain_name: "solana".into(),
                    gateway_address,
                    rpc: Url::parse("http://0.0.0.3/")?,
                    transaction_scanner: TransactionScanner::default(),
                },
                verifier: AxelarVerifier {
                    rpc: Url::parse("http://0.0.0.4/")?,
                },
            }),
            database: Database {
                url: Url::parse(database_url)?,
            },
            health_check: HealthCheck {
                bind_addr: SocketAddr::from_str(healthcheck_bind_addr)?,
            },
            cancellation_timeout: Duration::from_secs(30),
        };
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn validation_fails_if_configured_without_transports() -> eyre::Result<()> {
        let database_url = "postgres://user:password@localhost:5432/dbname";
        let healthcheck_bind_addr = "127.0.0.1:8000";
        let input = format!(
            r#"[database]
            url = "{database_url}"

            [health_check]
            bind_addr = "{healthcheck_bind_addr}""#
        );
        let parsed = toml::from_str::<Config>(&input)?;
        assert!(parsed.axelar_to_solana.is_none());
        assert!(parsed.solana_to_axelar.is_none());
        Ok(())
    }

    #[test]
    fn deserialize_secret_key_from_env() -> eyre::Result<()> {
        let gateway_address = Pubkey::new_unique();
        let gateway_config_address = Pubkey::new_unique();
        let keypair = Keypair::new();
        let input = format!(
            r#"rpc = "http://0.0.0.1/"
            keypair = "$SECRET_KEY"
            gateway_address = "{gateway_address}"
            gateway_config_address = "{gateway_config_address}""#
        );
        let parsed = temp_env::with_var("SECRET_KEY", Some(keypair.to_base58_string()), || {
            toml::from_str::<SolanaIncluder>(&input)
        })?;
        let expected = SolanaIncluder {
            rpc: Url::parse("http://0.0.0.1/")?,
            keypair: Arc::new(keypair),
            gateway_address,
            gateway_config_address,
        };
        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn deserialize_all_from_env() -> eyre::Result<()> {
        let gateway_address = Pubkey::new_unique();
        let gateway_config_address = Pubkey::new_unique();
        let relayer_keypair = Keypair::new();
        let database_url = "postgres://user:password@localhost:5432/dbname";
        let healthcheck_bind_addr = "127.0.0.1:8000";

        let approver_rpc = "http://0.0.0.1/";
        let includer_rpc = "http://0.0.0.2/";
        let sentinel_rpc = "http://0.0.0.3/";
        let verifier_rpc = "http://0.0.0.4/";

        let input = r#"[axelar_to_solana.approver]
                rpc = "$APPROVER_RPC"

                [axelar_to_solana.includer]
                rpc = "$INCLUDER_RPC"
                keypair = "$SECRET_KEY"
                gateway_address = "$GATEWAY_ADDRESS"
                gateway_config_address = "$GATEWAY_CONFIG_ADDRESS"

                [solana_to_axelar.sentinel]
                gateway_address = "$GATEWAY_ADDRESS"
                rpc = "$SENTINEL_RPC"

                [solana_to_axelar.verifier]
                rpc = "$VERIFIER_RPC"

                [database]
                url = "$DATABASE_URL"

                [health_check]
                bind_addr = "$HEALTHCHECK_BIND_ADDRESS""#;

        let parsed = temp_env::with_vars(
            [
                ("APPROVER_RPC", Some(approver_rpc)),
                ("INCLUDER_RPC", Some(includer_rpc)),
                ("SENTINEL_RPC", Some(sentinel_rpc)),
                ("VERIFIER_RPC", Some(verifier_rpc)),
                ("SECRET_KEY", Some(&relayer_keypair.to_base58_string())),
                ("GATEWAY_ADDRESS", Some(&gateway_address.to_string())),
                (
                    "GATEWAY_CONFIG_ADDRESS",
                    Some(&gateway_config_address.to_string()),
                ),
                ("DATABASE_URL", Some(database_url)),
                ("HEALTHCHECK_BIND_ADDRESS", Some(healthcheck_bind_addr)),
            ],
            || toml::from_str::<Config>(input),
        )?;

        let expected = Config {
            axelar_to_solana: Some(AxelarToSolana {
                approver: AxelarApprover {
                    rpc: Url::parse(approver_rpc)?,
                    solana_chain_name: "solana".into(),
                },
                includer: SolanaIncluder {
                    rpc: Url::parse(includer_rpc)?,
                    keypair: Arc::new(relayer_keypair),
                    gateway_address,
                    gateway_config_address,
                },
            }),
            solana_to_axelar: Some(SolanaToAxelar {
                sentinel: SolanaSentinel {
                    solana_chain_name: "solana".into(),
                    gateway_address,
                    rpc: Url::parse(sentinel_rpc)?,
                    transaction_scanner: TransactionScanner::default(),
                },
                verifier: AxelarVerifier {
                    rpc: Url::parse(verifier_rpc)?,
                },
            }),
            database: Database {
                url: Url::parse(database_url)?,
            },
            health_check: HealthCheck {
                bind_addr: SocketAddr::from_str(healthcheck_bind_addr)?,
            },
            cancellation_timeout: Duration::from_secs(30),
        };
        assert_eq!(parsed, expected);
        Ok(())
    }
}
