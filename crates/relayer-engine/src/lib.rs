//! # Axelar Relayer engine
//! It's repsonible for relaying packets form the Amplifier API to the configured edge chain

mod config;
pub use config::{Config, HealthCheckConfig, RelayerComponent};
use tokio::task::JoinSet;
use tracing::{Instrument as _, info_span};
pub use url;

/// Generic array of components to be consumed by the engine
pub type Components = Vec<Box<dyn config::RelayerComponent>>;

/// The core engine that will route packets
pub struct RelayerEngine {
    #[expect(
        dead_code,
        reason = "it will be used when spawning the HTTP health check"
    )]
    config: Config,
    components: Components,
}

impl RelayerEngine {
    #[must_use]
    /// Initialise a new [`RelayerEngine`] based on the provided configuration
    pub const fn new(config: Config, components: Components) -> Self {
        Self { config, components }
    }

    /// Main entrypoint to spawn all the services according to the configuration
    ///
    /// it will only stop when one of the spawned sub-tasks exit with an error, or panics.
    #[tracing::instrument(skip_all)]
    pub async fn start_and_wait_for_shutdown(self) {
        let mut set = JoinSet::new();

        // Attempt to spawn tasks and handle errors immediately
        if let Err(err) = self.spawn_tasks(&mut set).in_current_span().await {
            tracing::error!(?err, "failed to spawn tasks");
            tracing::info!("shutdown");
            set.shutdown().await;
            return;
        }

        // Wait for the first task to exit and handle its outcome
        while let Some(task_result) = set.join_next().await {
            match task_result {
                Ok(Ok(())) => {
                    tracing::warn!("A task exited successfully");
                    continue;
                }
                Ok(Err(err)) => {
                    tracing::error!(?err, "A task returned an error, shutting down the system");
                    break;
                }
                Err(join_err) => {
                    tracing::error!(?join_err, "A task panicked, shutting down the system");
                    break;
                }
            }
        }

        // Shutdown the task set
        tracing::info!("shutdown");
        set.shutdown().await;
    }

    #[tracing::instrument(skip_all)]
    async fn spawn_tasks(self, set: &mut JoinSet<eyre::Result<()>>) -> eyre::Result<()> {
        // spawn the provided components
        for component in self.components {
            let process = component.process();
            set.spawn(process);
        }

        // -- internal utility tasks
        set.spawn(
            async {
                tracing::info!("spawning global shutdown signal processor");
                tokio::signal::ctrl_c().await?;
                eyre::bail!("Ctrl-c received, shutting down");
            }
            .instrument(info_span!("Ctrl-c signal handler")),
        );

        // todo spawn /health endpoint using axum

        Ok(())
    }
}
