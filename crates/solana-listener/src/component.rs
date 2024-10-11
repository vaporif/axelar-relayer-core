use core::future::Future;
use core::pin::Pin;
use core::str::FromStr;
use std::sync::Arc;

use futures_concurrency::future::FutureExt;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::config;

#[derive(Debug, Clone)]
pub struct SolanaTransaction {
    pub signature: Signature,
    pub logs: Vec<String>,
    pub block_time: Option<i64>,
    pub slot: u64,
}

pub enum TransactionScannerMessage {
    /// Typical message with the produced work.
    /// Contains the handle to a task that resolves into a
    /// [`SolanaTransaction`].
    Message(SolanaTransaction),
}

pub(crate) type MessageSender = futures::channel::mpsc::UnboundedSender<TransactionScannerMessage>;

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
    pub messages_sender: futures::channel::mpsc::UnboundedReceiver<TransactionScannerMessage>,
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
            messages_sender: rx_outgoing,
        };
        (this, client)
    }

    #[tracing::instrument(skip_all, name = "Solana Listener")]
    pub(crate) async fn process_internal(self) -> eyre::Result<()> {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrent_rpc_requests));
        let (tx, rx) = futures::channel::mpsc::unbounded();
        let scanner = signature_scanner::run(
            self.config.gateway_program_address,
            self.config.solana_rpc.clone(),
            tx,
            self.config.tx_scan_poll_period,
            Arc::clone(&semaphore),
            self.config.max_concurrent_rpc_requests,
        );

        let tx_retriever =
            transaction_retriever::run(self.config.solana_rpc, rx, self.sender, semaphore);

        scanner.race(tx_retriever).await?;
        eyre::bail!("listener crashed");
    }
}

/// Functions to obtain transaction signatures from Solana RPC.
pub(crate) mod signature_scanner {

    use core::time::Duration;
    use std::sync::Arc;

    use eyre::Context;
    use futures::channel::mpsc::UnboundedSender;
    use futures::SinkExt;
    use tokio::sync::Semaphore;
    use tracing::trace;
    use url::Url;

    use super::*;

    /// Continuously fetches signatures from RPC and pipe them over a channel to
    /// further processing.
    ///
    /// # Cancelation Safety
    ///
    /// This function is cancel safe. All lost work can be recovered as the
    /// task's savepoint is sourced from the persistence layer, which
    /// remains unchanged in this context.
    #[tracing::instrument(name = "signature scanner", skip_all, err)]
    pub async fn run(
        address: Pubkey,
        url: Url,
        signature_sender: UnboundedSender<Signature>,
        period: Duration,
        semaphore: Arc<Semaphore>,
        max_concurrent_rpc_requests: usize,
    ) -> eyre::Result<()> {
        let mut interval = tokio::time::interval(period);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let rpc_client = Arc::new(RpcClient::new(url.to_string()));
        let max_concurrent_rpc_requests = u32::try_from(max_concurrent_rpc_requests)?;

        loop {
            // Greedly ask for all available permits from this semaphore, to ensure this
            // task will not concur with `transaction_retriever::fetch` tasks.
            let all_permits = Arc::clone(&semaphore)
                .acquire_many_owned(max_concurrent_rpc_requests)
                .await?;

            trace!("acquired all semaphore permits (exclusive access)");

            // Now that we have all permits, scan for signatures while waiting for the
            // cancelation signal.
            collect_and_process_signatures(
                address,
                Arc::clone(&rpc_client),
                signature_sender.clone(),
            )
            .await?;

            // Give all permits back to the semaphore.
            drop(all_permits);

            // Give some time for downstream futures to acquire permits from this semaphore.
            trace!("sleeping");
            interval.tick().await;
        }
    }

    /// Calls Solana RPC after relevant transaction signatures and send results
    /// over a channel.
    #[tracing::instrument(skip_all, err)]
    async fn collect_and_process_signatures(
        address: Pubkey,
        rpc_client: Arc<RpcClient>,
        mut signature_sender: UnboundedSender<Signature>,
    ) -> eyre::Result<()> {
        let collected_signatures =
            fetch_signatures_until_exhaustion(Arc::clone(&rpc_client), address, None).await?;

        // Iterate backwards so oldest signatures are picked up first on the other end.
        for signature in collected_signatures.into_iter().rev() {
            signature_sender
                .send(signature)
                .await
                .wrap_err("signature sender dropped")?;
        }
        Ok(())
    }

    /// Fetches all Solana transaction signatures for an address until a
    /// specified signature is reached or no more transactions are
    /// available.
    #[tracing::instrument(skip(rpc_client, address), err)]
    async fn fetch_signatures_until_exhaustion(
        rpc_client: Arc<RpcClient>,
        address: Pubkey,
        until: Option<Signature>,
    ) -> eyre::Result<Vec<Signature>> {
        /// This is the max number of signatures returned by the Solana RPC. It
        /// is used as an indicator to tell if we need to continue
        /// querying the RPC for more signatures.
        const LIMIT: usize = 1_000;

        // Helper function to setup the configuration at each loop
        let config = |before: Option<Signature>| GetConfirmedSignaturesForAddress2Config {
            before,
            until,
            limit: Some(LIMIT),
            commitment: Some(CommitmentConfig::finalized()),
        };

        let mut collected_signatures = vec![];
        let mut last_visited: Option<Signature> = None;
        loop {
            let batch = rpc_client
                .get_signatures_for_address_with_config(&address, config(last_visited))
                .await?;

            // Get the last (oldest) signature on this batch or break if it is empty
            let Some(oldest) = batch.last() else { break };

            // Set up following calls to start from the point this one had left
            last_visited = Some(Signature::from_str(&oldest.signature)?);

            let batch_size = batch.len();
            collected_signatures.extend(batch.into_iter());

            // If the results are less than the limit, then it means we have all the
            // signatures we need.
            if batch_size < LIMIT {
                break;
            };
        }

        Ok(collected_signatures
            .into_iter()
            .map(|status| Signature::from_str(&status.signature))
            .collect::<Result<Vec<_>, _>>()?)
    }
}

/// Functions to resolve transaction signatures into full transactions, with
/// metadata.
pub(crate) mod transaction_retriever {
    use core::task::Poll;
    use std::sync::Arc;

    use eyre::Context;
    use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
    use futures::stream::FusedStream;
    use futures::{SinkExt, StreamExt};
    use solana_client::client_error::ClientError;
    use solana_client::rpc_client::RpcClientConfig;
    use solana_client::rpc_config::RpcTransactionConfig;
    use solana_transaction_status::option_serializer::OptionSerializer;
    use solana_transaction_status::{
        EncodedConfirmedTransactionWithStatusMeta, UiTransactionEncoding,
    };
    use thiserror::Error;
    use tokio::sync::{AcquireError, Semaphore};
    use tracing::debug;
    use url::Url;

    use super::*;
    use crate::retrying_http_sender::RetryingHttpSender;

    #[derive(Error, Debug)]
    pub(crate) enum TransactionRetrieverError {
        #[error("Failed to decode solana transaction: {signature}")]
        TransactionDecode { signature: Signature },
        #[error(transparent)]
        NonFatal(#[from] NonFatalError),
        /// This variant's value needs to be boxed to prevent a recursive type
        /// definition, since this error is also part of
        /// [`TransactionScannerMessage`].
        #[error("Failed to send processed transaction for event analysis: {0}")]
        SendTransactionError(#[from] Box<futures::channel::mpsc::SendError>),
        #[error(transparent)]
        SolanaClient(#[from] ClientError),
        #[error("Failed to acquire a semaphore permit")]
        SemaphoreClosed(#[from] AcquireError),
    }

    /// Errors that shouldn't halt the Sentinel.
    #[derive(Error, Debug)]
    pub(crate) enum NonFatalError {
        #[error("Got wrong signature from RPC. Expected: {expected}, received: {received}")]
        WrongTransactionReceived {
            expected: Signature,
            received: Signature,
        },
        #[error("Got a transaction without meta attribute: {signature}")]
        TransactionWithoutMeta { signature: Signature },
        #[error("Got a transaction without logs: {signature}")]
        TransactionWithoutLogs { signature: Signature },
    }

    /// Asynchronously processes incoming transaction signatures by spawning
    /// Tokio tasks to retrieve full transaction details.
    ///
    /// Tasks wait acquiring a semaphore permit before reaching the Solana RPC
    /// endpoint.
    ///
    /// Successfully fetched transactions are sent through a channel for
    /// further processing.
    ///
    /// # Cancellation Safety
    ///
    /// This function is cancel safe. All lost work can be recovered as the
    /// task's savepoint is sourced from the persistence layer, which
    /// remains unchanged in this context.
    #[tracing::instrument(name = "transaction-retriever", skip_all)]
    pub async fn run(
        url: Url,
        signature_receiver: UnboundedReceiver<Signature>,
        transaction_sender: UnboundedSender<TransactionScannerMessage>,
        semaphore: Arc<Semaphore>,
    ) -> eyre::Result<()> {
        let rpc_client = {
            let sender = RetryingHttpSender::new(url.to_string());
            let config = RpcClientConfig::with_commitment(CommitmentConfig::confirmed());
            let client = RpcClient::new_sender(sender, config);
            Arc::new(client)
        };

        let mut join_set = JoinSet::<eyre::Result<()>>::new();
        let mut signature_receiver = signature_receiver.fuse();
        let mut task = futures::stream::poll_fn(move |cx| {
            match signature_receiver.poll_next_unpin(cx) {
                core::task::Poll::Ready(Some(signature)) => {
                    join_set.spawn({
                        let semaphore = Arc::clone(&semaphore);
                        let rpc_client = Arc::clone(&rpc_client);
                        let mut transaction_sender = transaction_sender.clone();
                        async move {
                            let tx_result =
                                fetch_with_permit(signature, rpc_client, semaphore).await;

                            match tx_result {
                                Ok(tx) => {
                                    tracing::info!(?tx, "solana tx retrieved");
                                    transaction_sender
                                        .send(TransactionScannerMessage::Message(tx))
                                        .await
                                        .wrap_err("transaction sender failed")?;
                                }
                                Err(err) => match err {
                                    TransactionRetrieverError::NonFatal(non_fatal_error) => {
                                        tracing::debug!(
                                            ?non_fatal_error,
                                            "tx scanner returned non-fatal error"
                                        );
                                    }
                                    fatal_error @
                                        (TransactionRetrieverError::TransactionDecode { .. } |
                                        TransactionRetrieverError::SendTransactionError(_) |
                                        TransactionRetrieverError::SolanaClient(_) |
                                        TransactionRetrieverError::SemaphoreClosed(_)) => {
                                        return Err(fatal_error).wrap_err("tx retriever error")
                                    }
                                },
                            }

                            Ok(())
                        }
                    });
                }
                Poll::Pending => (),
                Poll::Ready(None) => {
                    tracing::error!("interval stream closed");
                    join_set.abort_all();
                }
            }

            // check if any background tasks are done
            match join_set.poll_join_next(cx) {
                Poll::Ready(Some(res)) => Poll::Ready(Some(res)),
                // join set returns `Poll::Ready(None)` when it's empty
                Poll::Ready(None) => {
                    if signature_receiver.is_terminated() {
                        return Poll::Ready(None)
                    }
                    Poll::Pending
                }
                Poll::Pending => Poll::Pending,
            }
        });

        while let Some(task_result) = task.next().await {
            let Ok(res) = task_result else {
                tracing::error!(?task_result, "background task panicked");
                continue;
            };
            let Err(err) = res else {
                continue;
            };

            tracing::error!(?err, "background task returned an error");
        }
        eyre::bail!("signature receiver closed")
    }

    /// Fetches a Solana transaction by calling the `getTransactionWithConfig`
    /// RPC method with its signature and decoding the result.
    #[tracing::instrument(skip(rpc_client))]
    async fn fetch(
        signature: Signature,
        rpc_client: Arc<RpcClient>,
    ) -> Result<SolanaTransaction, TransactionRetrieverError> {
        let config = RpcTransactionConfig {
            encoding: Some(UiTransactionEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            max_supported_transaction_version: Some(0),
        };

        let EncodedConfirmedTransactionWithStatusMeta {
            block_time,
            slot,
            transaction: transaction_with_meta,
        } = rpc_client
            .get_transaction_with_config(&signature, config)
            .await?;

        let decoded_transaction = transaction_with_meta
            .transaction
            .decode()
            .ok_or_else(|| TransactionRetrieverError::TransactionDecode { signature })?;

        // Check: This is the transaction we asked
        if !decoded_transaction.signatures.contains(&signature) {
            return Err(NonFatalError::WrongTransactionReceived {
                expected: signature,
                received: *decoded_transaction
                    .signatures
                    .first()
                    .expect("Solana transaction should have at least one signature"),
            }
            .into());
        }

        let meta = transaction_with_meta
            .meta
            .ok_or(NonFatalError::TransactionWithoutMeta { signature })?;

        let OptionSerializer::Some(logs) = meta.log_messages else {
            return Err(NonFatalError::TransactionWithoutLogs { signature }.into())
        };

        let transaction = SolanaTransaction {
            signature,
            logs,
            block_time,
            slot,
        };

        debug!(
            block_time = ?transaction.block_time,
            slot = %transaction.slot,
            "found solana transaction"
        );

        Ok(transaction)
    }

    /// Fetches a Solana transaction for the given signature once a semaphore
    /// permit is acquired.
    ///
    /// # Cancellation Safety
    ///
    /// This function is cancel safe. It will return without reaching the Solana
    /// RPC endpoint if a cancellation signal is received while waiting for
    /// a semaphore permit.
    async fn fetch_with_permit(
        signature: Signature,
        rpc_client: Arc<RpcClient>,
        semaphore: Arc<Semaphore>,
    ) -> Result<SolanaTransaction, TransactionRetrieverError> {
        let _permit = semaphore.acquire_owned().await?;
        fetch(signature, rpc_client).await
    }
}
