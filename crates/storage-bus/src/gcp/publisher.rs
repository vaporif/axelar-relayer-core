use core::fmt::{Debug, Display};
use core::marker::PhantomData;
use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use google_cloud_googleapis::pubsub::v1::PubsubMessage;
use google_cloud_pubsub::client::Client;
use google_cloud_pubsub::publisher::Publisher;
use interfaces::kv_store::KvStore as _;

use super::GcpError;
use super::kv_store::RedisClient;
use super::util::get_topic;
use crate::interfaces;
use crate::interfaces::publisher::QueueMsgId;

const MSG_ID: &str = "Msg-Id";

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
    type Return = String;

    // TODO: remake to publish batch so there's no way they will be run concurrent
    #[allow(refining_impl_trait, reason = "simplification")]
    #[tracing::instrument(skip_all)]
    async fn publish(
        &self,
        // Deduplication is automatic
        deduplication_id: impl Into<String>,
        data: &T,
    ) -> Result<Self::Return, GcpError> {
        let encoded = borsh::to_vec(&data).map_err(GcpError::Serialize)?;
        let mut attributes = HashMap::new();
        attributes.insert(MSG_ID.to_owned(), deduplication_id.into());
        let message = PubsubMessage {
            data: encoded,
            attributes,
            ..Default::default()
        };
        let awaiter = self.publisher.publish(message.clone()).await;

        // NOTE: We always await since messages should be sent sequentially
        let result = awaiter
            .get()
            .await
            .map_err(|err| GcpError::Publish(Box::new(err)))?;
        Ok(result)
    }
}

/// Queue publisher with ability to get last message (without consuming)
#[allow(clippy::module_name_repetitions, reason = "Descriptive name")]
pub struct PeekableGcpPublisher<T: QueueMsgId> {
    publisher: GcpPublisher<T>,
    last_message_id_store: RedisClient<T::MessageId>,
}

impl<T> PeekableGcpPublisher<T>
where
    T: Send + Sync + QueueMsgId,
    T::MessageId: BorshSerialize + BorshDeserialize + Display,
{
    pub(crate) async fn new(
        client: &Client,
        topic: &str,
        kv_store: RedisClient<T::MessageId>,
    ) -> Result<Self, GcpError> {
        let publisher = GcpPublisher::new(client, topic).await?;

        Ok(Self {
            publisher,
            last_message_id_store: kv_store,
        })
    }
}

impl<T> interfaces::publisher::Publisher<T> for PeekableGcpPublisher<T>
where
    T: QueueMsgId + BorshSerialize + Debug + Send + Clone + Sync,
    T::MessageId: BorshSerialize + BorshDeserialize + AsRef<[u8]> + Send + Sync + Debug + Display,
{
    type Return = String;
    #[allow(refining_impl_trait, reason = "simplification")]
    #[tracing::instrument(skip_all)]
    async fn publish(
        &self,
        deduplication_id: impl Into<String>,
        data: &T,
    ) -> Result<Self::Return, GcpError> {
        let res = self.publisher.publish(deduplication_id, data).await?;
        self.last_message_id_store.upsert(&data.id()).await?;
        Ok(res)
    }
}

impl<T> interfaces::publisher::PeekMessage<T> for PeekableGcpPublisher<T>
where
    T: QueueMsgId,
    T::MessageId: BorshSerialize + BorshDeserialize + AsRef<[u8]> + Send + Sync + Debug + Display,
{
    #[allow(refining_impl_trait, reason = "simplification")]
    #[tracing::instrument(skip_all)]
    async fn peek_last(&mut self) -> Result<Option<T::MessageId>, GcpError> {
        self.last_message_id_store
            .get()
            .await?
            .map(|data| Ok(data.value))
            .transpose()
    }
}
