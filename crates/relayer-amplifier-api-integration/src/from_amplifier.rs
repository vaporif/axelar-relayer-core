use core::task::Poll;
use std::sync::{Arc, Mutex};

use amplifier_api::requests::{self, WithTrailingSlash};
use amplifier_api::types::{ErrorResponse, GetTasksResult, TaskItemId};
use amplifier_api::AmplifierRequest;
use futures::stream::StreamExt as _;
use futures::SinkExt as _;
use tokio::task::JoinSet;
use tokio_stream::wrappers::IntervalStream;

use crate::component::AmplifierTaskSender;
use crate::config::Config;

// process incoming messages (aka `tasks`) coming in form Amplifier API
// 1. periodically check if we have new tasks for processing
// 2. if we do, try to act on them; spawning handlers concurrently
pub(crate) async fn process(
    config: Config,
    client: amplifier_api::AmplifierApiClient,
    fan_out_sender: AmplifierTaskSender,
) -> eyre::Result<()> {
    tracing::info!(poll_interval =? config.get_chains_poll_interval, "spawned");

    // Trailing slash is significant when making the API calls!
    let chain_with_trailing_slash = WithTrailingSlash::new(config.chain.clone());
    let mut join_set = JoinSet::<eyre::Result<()>>::new();

    let mut interval_stream = IntervalStream::new({
        let mut interval = tokio::time::interval(config.get_chains_poll_interval);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        interval
    });
    let latest_task = Arc::new(Mutex::new(Option::<TaskItemId>::None));

    let mut task_stream = futures::stream::poll_fn(move |cx| {
        // periodically query the API for new tasks but only if the downstream processor is ready to
        // accept
        match interval_stream.poll_next_unpin(cx) {
            Poll::Ready(Some(_res)) => {
                let res = internal(
                    &config,
                    Arc::clone(&latest_task),
                    &chain_with_trailing_slash,
                    &client,
                    fan_out_sender.clone(),
                    &mut join_set,
                );
                // in case we were awoken by join_set being ready, let's re-run this function,
                // while returning the result of `internal`.
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
    tasks_after: Arc<Mutex<Option<TaskItemId>>>,
    chain_with_trailing_slash: &WithTrailingSlash,
    client: &amplifier_api::AmplifierApiClient,
    fan_out_sender: AmplifierTaskSender,
    to_join_set: &mut JoinSet<eyre::Result<()>>,
) -> eyre::Result<()> {
    if !fan_out_sender.is_empty() {
        // the downstream client is still processing the events, don't send any new ones
        return Ok(())
    }
    let tasks_after_internal = tasks_after.lock().expect("lock poisoned").clone();
    let request = requests::GetChains::builder()
        .chain(chain_with_trailing_slash)
        .limit(config.get_chains_limit)
        .after(tasks_after_internal)
        .build();
    let request = client.build_request(&request)?;
    to_join_set.spawn(process_task_request(request, tasks_after, fan_out_sender));

    Ok(())
}

async fn process_task_request(
    request: AmplifierRequest<GetTasksResult, ErrorResponse>,
    tasks_after: Arc<Mutex<Option<TaskItemId>>>,
    mut fan_out_sender: AmplifierTaskSender,
) -> eyre::Result<()> {
    let res = request.execute().await?;
    let res = res.json().await??;
    let Some(last_task) = res.tasks.last().map(|x| x.id.clone()) else {
        return Ok(());
    };
    tracing::info!(task_count = ?res.tasks.len(), "received new tasks");
    let mut iter = futures::stream::iter(res.tasks.into_iter().map(Ok));
    fan_out_sender.send_all(&mut iter).await?;
    {
        let mut lock = tasks_after.lock().expect("lock poisoned");
        lock.replace(last_task);
    };
    Ok(())
}
