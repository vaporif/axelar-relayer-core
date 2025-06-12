//! Crate with amplifier subscriber component
use amplifier_api::requests::WithTrailingSlash;
use amplifier_api::{AmplifierApiClient, requests};
use bin_util::SimpleMetrics;
use bin_util::health_check::CheckHealth;
use eyre::Context as _;
use infrastructure::interfaces::publisher::{PeekMessage, PublishMessage, Publisher};
use tokio::sync::Mutex;

/// subscribes to tasks from amplifier and sends them to queue
pub struct Subscriber<TaskQueuePublisher> {
    amplifier_client: AmplifierApiClient,
    task_queue_publisher: Mutex<TaskQueuePublisher>,
    chain: String,
    limit_items: u8,
    metrics: SimpleMetrics,
}

impl<TaskQueuePublisher> Subscriber<TaskQueuePublisher>
where
    TaskQueuePublisher:
        Publisher<amplifier_api::types::TaskItem> + PeekMessage<amplifier_api::types::TaskItem>,
{
    /// create subscriber
    pub fn new(
        amplifier_client: AmplifierApiClient,
        task_queue_publisher: TaskQueuePublisher,
        limit_items: u8,
        chain: String,
    ) -> Self {
        let metrics = SimpleMetrics::new("amplifier-subscriber", vec![]);
        Self {
            amplifier_client,
            task_queue_publisher: Mutex::new(task_queue_publisher),
            chain,
            limit_items,
            metrics,
        }
    }

    /// subscribe and process
    #[tracing::instrument(skip_all)]
    pub async fn subscribe(&self) -> eyre::Result<()> {
        let chain_with_trailing_slash = WithTrailingSlash::new(self.chain.clone());

        let res: eyre::Result<()> = {
            let mut publisher = self.task_queue_publisher.lock().await;
            let last_task_id = publisher
                .peek_last()
                .await
                .wrap_err("could not get last retrieved task id")?;
            drop(publisher);

            tracing::trace!(?last_task_id, "last retrieved task");

            let request = requests::GetChains::builder()
                .chain(&chain_with_trailing_slash)
                .limit(self.limit_items)
                .after(last_task_id)
                .build();

            tracing::trace!(?request, "request for amplifier api created");

            let request = self
                .amplifier_client
                .build_request(&request)
                .wrap_err("could not build amplifier request")?;

            tracing::trace!(?request, "sending");

            let response = request
                .execute()
                .await
                .wrap_err("could not sent amplifier api request")?;

            let response = response
                .json()
                .await
                .map_err(|err| eyre::Report::new(err).wrap_err("amplifier api failed"))?
                .map_err(|err| eyre::Report::new(err).wrap_err("failed to decode response"))?;

            tracing::trace!(?response, "amplifier response");

            let mut tasks = response.tasks;
            tasks.sort_unstable_by_key(|task| task.timestamp);

            if tasks.is_empty() {
                tracing::trace!("no amplifier tasks");
                return Ok(());
            }

            tracing::info!(count = tasks.len(), "got amplifier tasks");

            let batch = tasks.into_iter().map(PublishMessage::from).collect();

            tracing::trace!("sending to queue");
            self.task_queue_publisher
                .lock()
                .await
                .publish_batch(batch)
                .await
                .wrap_err("could not publish tasks to queue")?;
            tracing::info!("sent to queue");
            Ok(())
        };

        if res.is_err() {
            self.metrics.record_error();
        }

        res
    }
}

impl<TaskQueuePublisher> CheckHealth for Subscriber<TaskQueuePublisher>
where
    TaskQueuePublisher: Publisher<amplifier_api::types::TaskItem>
        + PeekMessage<amplifier_api::types::TaskItem>
        + Send
        + Sync
        + 'static,
{
    async fn check_health(&self) -> eyre::Result<()> {
        // Check if the task queue publisher is healthy
        let publisher_health = self.task_queue_publisher.lock().await.check_health().await;
        if let Err(err) = publisher_health {
            tracing::warn!(%err, "task queue publisher health check failed");
            self.metrics.record_error();
            return Err(err.into());
        }

        if let Err(err) = self
            .amplifier_client
            .build_request(&requests::HealthCheck)
            .wrap_err("could not build health check request")?
            .execute()
            .await
        {
            self.metrics.record_error();
            tracing::warn!(%err, "amplifier client health check failed");
            return Err(err.into());
        }

        Ok(())
    }
}

#[cfg(feature = "supervisor")]
impl<TaskQueuePublisher> supervisor::Worker for Subscriber<TaskQueuePublisher>
where
    TaskQueuePublisher: Publisher<amplifier_api::types::TaskItem>
        + PeekMessage<amplifier_api::types::TaskItem>
        + Send
        + Sync,
{
    fn do_work<'s>(
        &'s mut self,
    ) -> core::pin::Pin<Box<dyn Future<Output = eyre::Result<()>> + 's>> {
        Box::pin(async { self.subscribe().await })
    }
}
