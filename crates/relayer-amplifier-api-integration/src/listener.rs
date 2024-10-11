use core::task::Poll;

use amplifier_api::requests::{self, WithTrailingSlash};
use amplifier_api::types::{ErrorResponse, GetTasksResult};
use amplifier_api::AmplifierRequest;
use futures::stream::StreamExt;
use tokio::task::JoinSet;
use tokio_stream::wrappers::IntervalStream;

use crate::config::Config;

// process incoming messages (aka `tasks`) coming in form Amplifier API
// 1. periodically check if we have new tasks for processing
// 2. if we do, try to act on them; spawning handlers concurrently
pub(crate) async fn process_msgs_from_amplifier(
    config: Config,
    client: amplifier_api::AmplifierApiClient,
) -> eyre::Result<()> {
    tracing::info!(poll_interval =? config.get_chains_poll_interval, "spawned");

    // Trailing slash is significant when making the API calls!
    let chain_with_trailing_slash = WithTrailingSlash::new(config.chain.clone());
    let mut join_set = JoinSet::<eyre::Result<()>>::new();

    let mut interval_stream =
        IntervalStream::new(tokio::time::interval(config.get_chains_poll_interval));

    let mut task_stream = futures::stream::poll_fn(move |cx| {
        // periodically query new tasks
        match interval_stream.poll_next_unpin(cx) {
            Poll::Ready(Some(_res)) => {
                let res = internal(&config, &chain_with_trailing_slash, &client, &mut join_set);
                // in case we were awoken by join_set being ready, let's re-run this function, while
                // returning the result of `internal`.
                cx.waker().wake_by_ref();
                return Poll::Ready(Some(Ok(res)));
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
            Poll::Ready(None) | Poll::Pending => Poll::Pending,
        }
    });

    while let Some(task_result) = task_stream.next().await {
        let Ok(res) = task_result else {
            tracing::error!(?task_result, "background task panicked");
            continue;
        };
        let Err(err) = res else {
            continue;
        };

        tracing::error!(?err, "background task returned an error");
    }

    eyre::bail!("fatal error when processing messages from amplifier")
}

pub(crate) fn internal(
    config: &Config,
    chain_with_trailing_slash: &WithTrailingSlash,
    client: &amplifier_api::AmplifierApiClient,
    to_join_set: &mut JoinSet<eyre::Result<()>>,
) -> eyre::Result<()> {
    let request = requests::GetChains::builder()
        .chain(chain_with_trailing_slash)
        .limit(config.get_chains_limit)
        .build();
    let request = client.build_request(&request)?;
    to_join_set.spawn(process_task_request(request));

    Ok(())
}

#[expect(clippy::unimplemented, reason = "will be added in the future")]
async fn process_task_request(
    request: AmplifierRequest<GetTasksResult, ErrorResponse>,
) -> eyre::Result<()> {
    let res = request.execute().await?;
    let res = res.json().await??;
    tracing::info!(task_count = ?res.tasks.len(), "received new tasks");
    for task_item in res.tasks {
        use amplifier_api::types::Task::{Execute, GatewayTx, Refund, Verify};
        match task_item.task {
            Verify(_) => unimplemented!("this will be supported in the future"),
            GatewayTx(gateway_tx_task) => {
                tracing::info!(incoming_gateway_tx_len = ?gateway_tx_task.execute_data.len(), "handle gateway task");
                // TODO: Send this to a pre-registered handler
            }
            Execute(_) => unimplemented!("this will be supported in the future"),
            Refund(_) => unimplemented!("this will be supported in the future"),
        }
    }
    Ok(())
}
