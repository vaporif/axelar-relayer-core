use std::fmt::Debug;
use std::future::{Ready, ready};
use std::time::Duration;

use borsh::{BorshDeserialize, BorshSerialize};
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::publisher::Publisher;
use interfaces::kv_store::KvStore;
use serde::{Deserialize, Serialize};

use super::error::Error;
use super::kv_store::GcpKvStore;
use super::util::get_or_create_topic;
use crate::interfaces;

impl super::PubSubBuilder {
    pub async fn publisher<T: Send + Sync>(
        self,
        topic: &str,
        allowed_persistence_regions: Vec<String>,
        message_retention: Duration,
        kv_store: GcpKvStore<T>,
    ) -> Result<GcpPublisher<T>, Error> {
        let topic = get_or_create_topic(
            &self.client,
            topic,
            allowed_persistence_regions,
            message_retention,
        )
        .await?;

        let publisher = topic.new_publisher(None);

        Ok(GcpPublisher::<T> {
            publisher,
            last_message_kv_store: kv_store,
        })
    }
}

pub struct GcpPublisher<T> {
    publisher: Publisher,
    last_message_kv_store: GcpKvStore<T>,
}

impl<T> interfaces::publisher::Publisher<T> for GcpPublisher<T>
where
    T: BorshSerialize + Serialize + for<'de> Deserialize<'de> + Send + Sync,
{
    // NOTE: Ack future is always finished since only after it finishes we can
    // update last pushed message
    type AckFuture = Ready<String>;

    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip_all)]
    async fn publish(
        &self,
        _deduplication_id: impl Into<String>,
        data: T,
    ) -> Result<Self::AckFuture, Error> {
        let encoded = borsh::to_vec(&data).map_err(Error::Serialize)?;
        let message = PubsubMessage {
            data: encoded,
            ..Default::default()
        };
        let awaiter = self.publisher.publish(message.clone()).await;
        let result = awaiter.get().await.map_err(Error::Publish)?;
        self.last_message_kv_store.upsert(data).await?;
        let ready_future: Ready<String> = ready(result);
        Ok(ready_future)
    }
}

impl<T> interfaces::publisher::PeekMessage<T> for GcpPublisher<T>
where
    T: BorshDeserialize + Serialize + for<'de> Deserialize<'de> + Send + Sync + Debug,
{
    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip_all)]
    async fn peek_last(&mut self) -> Result<Option<T>, Error> {
        self.last_message_kv_store
            .get()
            .await?
            .map(|data| Ok(data.value))
            .transpose()
    }
}
