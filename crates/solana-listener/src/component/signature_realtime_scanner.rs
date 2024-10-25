use core::str::FromStr as _;
use std::sync::Arc;

use futures::{SinkExt as _, StreamExt as _};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcTransactionLogsConfig, RpcTransactionLogsFilter};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Signature;
use tracing::{info_span, Instrument as _};

use super::MessageSender;
use crate::component::signature_batch_scanner;
use crate::SolanaTransaction;

#[tracing::instrument(skip_all, err, name = "realtime log ingestion")]
pub(crate) async fn process_realtime_logs(
    config: crate::Config,
    latest_processed_signature: Option<Signature>,
    rpc_client: Arc<RpcClient>,
    mut signature_sender: MessageSender,
) -> Result<(), eyre::Error> {
    let gateway_program_address = config.gateway_program_address;
    loop {
        tracing::info!(endpoint =? config.solana_ws.as_str(), ?gateway_program_address, "init new WS connection");
        let client =
            solana_client::nonblocking::pubsub_client::PubsubClient::new(config.solana_ws.as_str())
                .await?;
        let (ws_stream, _unsubscribe) = client
            .logs_subscribe(
                RpcTransactionLogsFilter::Mentions(vec![gateway_program_address.to_string()]),
                RpcTransactionLogsConfig {
                    commitment: Some(CommitmentConfig::finalized()),
                },
            )
            .await?;
        let mut ws_stream = ws_stream
            .filter(|item| {
                // only keep non-error items
                core::future::ready(item.value.err.is_none())
            })
            .filter_map(|item| {
                // parse the returned data into a format we can forward to other components
                core::future::ready({
                    Signature::from_str(&item.value.signature)
                        .map(|signature| {
                            SolanaTransaction {
                                // timestamp not available via the the WS API
                                timestamp: None,
                                signature,
                                logs: item.value.logs,
                                slot: item.context.slot,
                            }
                        })
                        .ok()
                })
            })
            .inspect(|item| {
                tracing::info!(item = ?item.signature, "found tx");
            })
            .boxed();

        // It takes a few seconds for the Solana node to accept the WS connection.
        // During this time we might have already missed a few signatures.
        // We attempt to fetch the diff here.
        // This will only trigger upon the very first WS returned signature
        let next = ws_stream.next().await;
        let Some(t2_signature) = next else {
            // reconnect if connection dropped
            continue;
        };

        signature_batch_scanner::fetch_batches_in_range(
            &config,
            Arc::clone(&rpc_client),
            &signature_sender,
            Some(t2_signature.signature),
            latest_processed_signature,
        )
        .instrument(info_span!("fetching missed signatures"))
        .await?;
        // forward the tx data to be processed
        signature_sender.send(t2_signature).await?;

        // start processing the rest of the messages
        tracing::info!("waiting realtime logs");
        while let Some(item) = ws_stream.next().await {
            signature_sender.send(item).await?;
        }
        tracing::warn!("websocket stream exited");
    }
}
