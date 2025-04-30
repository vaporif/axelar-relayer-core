use core::fmt::Debug;
use core::future::{Ready, ready};
use core::marker::PhantomData;

use borsh::{BorshDeserialize, BorshSerialize};
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::client::Client;
use google_cloud_pubsub::publisher::Publisher;
use interfaces::kv_store::KvStore as _;
use serde::{Deserialize, Serialize};

use super::GcpError;
use super::kv_store::RedisClient;
use super::util::get_topic;
use crate::interfaces;

/// Queue publisher
#[allow(clippy::module_name_repetitions, reason = "Descriptive name")]
pub struct GcpPublisher<T> {
    publisher: Publisher,
    _phantom: PhantomData<T>,
}

impl<T: Send + Sync> GcpPublisher<T> {
    pub(crate) async fn new(client: &Client, topic: &str) -> Result<Self, GcpError> {
        let topic = get_topic(client, topic).await?;

        let publisher = topic.new_publisher(None);

        Ok(Self {
            publisher,
            _phantom: PhantomData,
        })
    }
}

impl<T> interfaces::publisher::Publisher<T> for GcpPublisher<T>
where
    T: BorshSerialize + Debug + Send + Sync,
{
    // NOTE: Ack future is always finished since only after it finishes we can
    // update last pushed message
    type AckFuture = Ready<String>;

    #[allow(refining_impl_trait, reason = "simplification")]
    #[tracing::instrument(skip_all)]
    async fn publish(
        &self,
        // Deduplication is automatic
        _deduplication_id: impl Into<String>,
        data: &T,
    ) -> Result<Self::AckFuture, GcpError> {
        let encoded = borsh::to_vec(&data).map_err(GcpError::Serialize)?;
        let message = PubsubMessage {
            data: encoded,
            ..Default::default()
        };
        let awaiter = self.publisher.publish(message.clone()).await;
        let result = awaiter
            .get()
            .await
            .map_err(|err| GcpError::Publish(Box::new(err)))?;
        let ready_future: Ready<String> = ready(result);
        Ok(ready_future)
    }
}

/// Queue publisher with ability to get last message (without consuming)
#[allow(clippy::module_name_repetitions, reason = "Descriptive name")]
pub struct PeekableGcpPublisher<T: common::Id> {
    publisher: GcpPublisher<T>,
    last_message_kv_store: RedisClient<T::MessageId>,
}

impl<T> PeekableGcpPublisher<T>
where
    T: Send + Sync + common::Id,
    T::MessageId: Serialize + for<'de> Deserialize<'de>,
{
    pub(crate) async fn new(
        client: &Client,
        topic: &str,
        kv_store: RedisClient<T::MessageId>,
    ) -> Result<Self, GcpError> {
        let publisher = GcpPublisher::new(client, topic).await?;

        Ok(Self {
            publisher,
            last_message_kv_store: kv_store,
        })
    }
}

impl<T> interfaces::publisher::Publisher<T> for PeekableGcpPublisher<T>
where
    T: common::Id + BorshSerialize + Debug + Send + Clone + Sync,
    T::MessageId: Serialize + for<'de> Deserialize<'de>,
{
    // NOTE: Ack future is always finished since only after it finishes we can
    // update last pushed message
    type AckFuture = Ready<String>;

    #[allow(refining_impl_trait, reason = "simplification")]
    #[tracing::instrument(skip_all)]
    async fn publish(
        &self,
        deduplication_id: impl Into<String>,
        data: &T,
    ) -> Result<Self::AckFuture, GcpError> {
        let future = self.publisher.publish(deduplication_id, data).await?;
        self.last_message_kv_store.upsert(&data.id()).await?;
        Ok(future)
    }
}

impl<T> interfaces::publisher::PeekMessage<T> for PeekableGcpPublisher<T>
where
    T: common::Id
        + BorshDeserialize
        + Serialize
        + for<'de> Deserialize<'de>
        + Clone
        + Send
        + Sync
        + Debug,
    T::MessageId: Serialize + for<'de> Deserialize<'de> + Send + Sync + Debug,
{
    #[allow(refining_impl_trait, reason = "simplification")]
    #[tracing::instrument(skip_all)]
    async fn peek_last(&mut self) -> Result<Option<T::MessageId>, GcpError> {
        self.last_message_kv_store
            .get()
            .await?
            .map(|data| Ok(data.value))
            .transpose()
    }
}
