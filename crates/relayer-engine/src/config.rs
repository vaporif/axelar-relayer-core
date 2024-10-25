//! Configuration structures and primitives for the [`crate::RelayerEngine`]

use core::future::Future;
use core::net::SocketAddr;
use core::pin::Pin;

use serde::Deserialize;

/// Generic async component that the Relyer Engine can spawn and execute.
///
/// It's expected that that the Amplifeir API, Solana, Starknet and other integrators implement this
/// trait on their components if they want them integrated with the relayer engine.
pub trait RelayerComponent {
    /// Start processing of the specified component (run all async tasks)
    /// If the component returns an error, the engine will shut down.
    ///
    /// Meaning of the returned reult:
    /// - Ok(()) -- the component is shutting down but it does not warrant the shutdown of the whole
    ///   engine / other processes
    /// - `Err(eyre::Report)` -- the component is shutting down with a fatal error and it requires
    ///   the shutdown of the whole engine
    fn process(self: Box<Self>) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + Send>>;
}

/// Top-level configuration for the relayer engine.
/// Agnostic to the underlying components.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct Config {
    /// Health check server configuration.
    pub health_check: HealthCheckConfig,
}

/// Health check server configuration.
#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct HealthCheckConfig {
    /// Address to bind the health check server.
    pub bind_addr: SocketAddr,
}
