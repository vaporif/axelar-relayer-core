use core::future::Future;
use core::pin::Pin;

use amplifier_api::types::PublishEventsRequest;
use futures_concurrency::future::FutureExt;
use quanta::Upkeep;
use tracing::{info_span, Instrument};

use crate::{config, healthcheck, listener, subscriber};

/// A valid command that the Amplifier component can act upon
#[derive(Debug)]
pub enum AmplifierCommand {
    /// Publish events to the Amplifier API
    PublishEvents(PublishEventsRequest),
}

pub(crate) type CommandReceiver = futures::channel::mpsc::UnboundedReceiver<AmplifierCommand>;

/// The core Amplifier API abstraction.
///
/// Internally, it spawns processes for:
/// - monitoring the liveliness of the Amplifier API (via helathcheck)
/// - listening for new tasks coming form Amplifeir API (Listener subprocess)
/// - sending events to the Amplifier API (Subscriber subprocess)
#[derive(Debug)]
pub struct Amplifier {
    config: config::Config,
    receiver: CommandReceiver,
}

/// Utility client used for communicating with the `Amplifier` instance
#[derive(Debug, Clone)]
pub struct AmplifierClient {
    /// send commands to the `Amplifier` instance
    pub sender: futures::channel::mpsc::UnboundedSender<AmplifierCommand>,
}

impl relayer_engine::RelayerComponent for Amplifier {
    fn process(self: Box<Self>) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + Send>> {
        use futures::FutureExt;

        self.process_internal().boxed()
    }
}

impl Amplifier {
    /// Instantiate a new Amplifier using the pre-configured configuration.
    ///
    /// The returned variable also returns a helper client that encompasses ways to communicate with
    /// the underlying Amplifier instance.
    #[must_use]
    pub fn new(config: config::Config) -> (Self, AmplifierClient) {
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let this = Self {
            config,
            receiver: rx,
        };
        let client = AmplifierClient { sender: tx };
        (this, client)
    }

    #[tracing::instrument(skip_all, name = "Amplifier")]
    pub(crate) async fn process_internal(self) -> eyre::Result<()> {
        let client =
            amplifier_api::AmplifierApiClient::new(self.config.url.clone(), &self.config.identity)?;
        let clock = get_clock()?;

        // spawn tasks
        let healthcheck =
            healthcheck::process_healthcheck(self.config.clone(), clock, client.clone())
                .instrument(info_span!("healthcheck"))
                .in_current_span();
        let to_amplifier_msgs = subscriber::process_msgs_to_amplifier(
            self.config.clone(),
            self.receiver,
            client.clone(),
        )
        .instrument(info_span!("subscriber"))
        .in_current_span();
        let from_amplifier_msgs =
            listener::process_msgs_from_amplifier(self.config.clone(), client.clone())
                .instrument(info_span!("listener"))
                .in_current_span();

        // await tasks until one of them exits (fatal)
        healthcheck
            .race(to_amplifier_msgs)
            .race(from_amplifier_msgs)
            .await?;
        eyre::bail!("listener crashed");
    }
}

pub(crate) fn get_clock() -> eyre::Result<quanta::Clock> {
    static CLOCK_UPKEEP_HANDLE: std::sync::OnceLock<quanta::Handle> =
        std::sync::OnceLock::<quanta::Handle>::new();
    let tickrate = core::time::Duration::from_millis(10);
    if let Err(quanta::Error::FailedToSpawnUpkeepThread(_)) =
        Upkeep::new(tickrate).start().map(|x| {
            CLOCK_UPKEEP_HANDLE
                .set(x)
                .expect("could not set upkeep handle");
        })
    {
        eyre::bail!("could not spawn clock upkeep thread");
    }
    let clock = quanta::Clock::new();
    Ok(clock)
}
