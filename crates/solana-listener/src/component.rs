use core::future::Future;
use core::pin::Pin;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::RpcClientConfig;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;

use crate::config;
use crate::retrying_http_sender::RetryingHttpSender;

mod log_processor;
mod signature_batch_scanner;
mod signature_realtime_scanner;

/// Typical message with the produced work.
/// Contains the handle to a task that resolves into a
/// [`SolanaTransaction`].
#[derive(Debug, Clone)]
pub struct SolanaTransaction {
    /// signature of the transaction (id)
    pub signature: Signature,
    /// optional timespamp
    pub timestamp: Option<DateTime<Utc>>,
    /// The raw transaction logs
    pub logs: Vec<String>,
    /// the slot number of the tx
    pub slot: u64,
}

pub(crate) type MessageSender = futures::channel::mpsc::UnboundedSender<SolanaTransaction>;

/// The listener component that has the core functionality:
/// - monitor (poll) the solana blockchain for new signatures coming from the gateway program
/// - fetch the actual event data from the provided signature
/// - forward the tx event data to the `SolanaListenerClient`
#[derive(Debug)]
pub struct SolanaListener {
    config: config::Config,
    sender: MessageSender,
}

/// Utility client used for communicating with the `SolanaListener` instance
#[derive(Debug)]
pub struct SolanaListenerClient {
    /// Receive transaction messagese from `SolanaListener` instance
    pub log_receiver: futures::channel::mpsc::UnboundedReceiver<SolanaTransaction>,
}

impl relayer_engine::RelayerComponent for SolanaListener {
    fn process(self: Box<Self>) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + Send>> {
        use futures::FutureExt;

        self.process_internal().boxed()
    }
}

impl SolanaListener {
    /// Instantiate a new `SolanaListener` using the pre-configured configuration.
    ///
    /// The returned variable also returns a helper client that encompasses ways to communicate with
    /// the underlying `SolanaListener` instance.
    #[must_use]
    pub fn new(config: config::Config) -> (Self, SolanaListenerClient) {
        let (tx_outgoing, rx_outgoing) = futures::channel::mpsc::unbounded();
        let this = Self {
            config,
            sender: tx_outgoing,
        };
        let client = SolanaListenerClient {
            log_receiver: rx_outgoing,
        };
        (this, client)
    }

    #[tracing::instrument(skip_all, name = "Solana Listener")]
    pub(crate) async fn process_internal(self) -> eyre::Result<()> {
        let rpc_client = {
            let sender = RetryingHttpSender::new(
                self.config.solana_http_rpc.to_string(),
                self.config.max_concurrent_rpc_requests,
            );
            let config = RpcClientConfig::with_commitment(CommitmentConfig::finalized());
            let client = RpcClient::new_sender(sender, config);
            Arc::new(client)
        };

        // we fetch potentially missed signatures based on the provided the config
        let latest =
            signature_batch_scanner::scan_old_signatures(&self.config, &self.sender, &rpc_client)
                .await?;

        // we start processing realtime logs
        signature_realtime_scanner::process_realtime_logs(
            self.config,
            latest,
            rpc_client,
            self.sender,
        )
        .await?;

        eyre::bail!("listener crashed");
    }
}
