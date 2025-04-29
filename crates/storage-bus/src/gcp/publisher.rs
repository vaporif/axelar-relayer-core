use std::fmt::Debug;
use std::future::{Ready, ready};
use std::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::publisher::Publisher;
use interfaces::kv_store::KvStore;
use serde::{Deserialize, Serialize};

use super::error::Error;
use super::kv_store::GcpRedis;
use super::util::get_topic;
use crate::interfaces;

impl super::PubSubBuilder {
    pub async fn publisher<T: Send + Sync>(&self, topic: &str) -> Result<GcpPublisher<T>, Error> {
        let topic = get_topic(&self.client, topic).await?;

        let publisher = topic.new_publisher(None);

        Ok(GcpPublisher::<T> {
            publisher,
            _phantom: PhantomData,
        })
    }

    pub async fn peekable_publisher<T: Send + Sync>(
        &self,
        topic: &str,
        kv_store: GcpRedis<T>,
    ) -> Result<PeekableGcpPublisher<T>, Error> {
        let publisher = self.publisher::<T>(topic).await?;

        Ok(PeekableGcpPublisher::<T> {
            publisher,
            last_message_kv_store: kv_store,
        })
    }
}

pub struct GcpPublisher<T> {
    publisher: Publisher,
    _phantom: PhantomData<T>,
}

impl<T> interfaces::publisher::Publisher<T> for GcpPublisher<T>
where
    T: BorshSerialize + Serialize + for<'de> Deserialize<'de> + Debug + Send + Sync,
{
    // NOTE: Ack future is always finished since only after it finishes we can
    // update last pushed message
    type AckFuture = Ready<String>;

    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip_all)]
    async fn publish(
        &self,
        _deduplication_id: impl Into<String>,
        data: &T,
    ) -> Result<Self::AckFuture, Error> {
        let encoded = borsh::to_vec(&data).map_err(Error::Serialize)?;
        let message = PubsubMessage {
            data: encoded,
            ..Default::default()
        };
        let awaiter = self.publisher.publish(message.clone()).await;
        let result = awaiter.get().await.map_err(Error::Publish)?;
        let ready_future: Ready<String> = ready(result);
        Ok(ready_future)
    }
}

pub struct PeekableGcpPublisher<T> {
    publisher: GcpPublisher<T>,
    last_message_kv_store: GcpRedis<T>,
}

impl<T> interfaces::publisher::Publisher<T> for PeekableGcpPublisher<T>
where
    T: BorshSerialize + Serialize + for<'de> Deserialize<'de> + Debug + Send + Clone + Sync,
{
    // NOTE: Ack future is always finished since only after it finishes we can
    // update last pushed message
    type AckFuture = Ready<String>;

    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip_all)]
    async fn publish(
        &self,
        deduplication_id: impl Into<String>,
        data: &T,
    ) -> Result<Self::AckFuture, Error> {
        let future = self.publisher.publish(deduplication_id, data).await?;
        self.last_message_kv_store.upsert(data).await?;
        Ok(future)
    }
}

impl<T> interfaces::publisher::PeekMessage<T> for PeekableGcpPublisher<T>
where
    T: BorshDeserialize + Serialize + for<'de> Deserialize<'de> + Clone + Send + Sync + Debug,
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
