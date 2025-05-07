use core::fmt::Debug;
use core::marker::PhantomData;

use async_nats::jetstream;
use async_nats::jetstream::publish::PublishAck;
use borsh::{BorshDeserialize, BorshSerialize};

use super::NatsError;
use crate::interfaces;
use crate::interfaces::publisher::QueueMsgId;

const NATS_MSG_ID: &str = "Nats-Msg-Id";

/// Queue publisher
#[allow(clippy::module_name_repetitions, reason = "Descriptive name")]
pub struct NatsPublisher<T> {
    context: jetstream::Context,
    stream: jetstream::stream::Stream,
    subject: String,
    _phantom: PhantomData<T>,
}

impl<T> NatsPublisher<T> {
    pub(crate) const fn new(
        context: jetstream::Context,
        stream: jetstream::stream::Stream,
        subject: String,
    ) -> Self {
        Self {
            context,
            stream,
            subject,
            _phantom: PhantomData,
        }
    }
}

impl<T: BorshSerialize + Debug + Send + Sync> interfaces::publisher::Publisher<T>
    for NatsPublisher<T>
{
    type Return = PublishAck;

    #[allow(refining_impl_trait, reason = "simplify")]
    #[tracing::instrument(skip_all)]
    async fn publish(
        &self,
        deduplication_id: impl Into<String>,
        data: &T,
    ) -> Result<Self::Return, NatsError> {
        let mut headers = async_nats::HeaderMap::new();
        let deduplication_id: String = deduplication_id.into();
        tracing::debug!(?deduplication_id, ?data, "got message");
        headers.append(NATS_MSG_ID.to_owned(), deduplication_id);
        let data = borsh::to_vec(&data).map_err(NatsError::Serialize)?;
        tracing::debug!("message encoded");
        // NOTE: We always await since messages should be sent sequentially
        let publish_ack = self
            .context
            .publish_with_headers(self.subject.clone(), headers, data.into())
            .await?
            .await?;

        tracing::debug!("message published");

        Ok(publish_ack)
    }

    #[allow(refining_impl_trait)]
    async fn check_health(&self) -> Result<(), NatsError> {
        tracing::debug!("checking health");
        self.stream.get_info().await?;
        Ok(())
    }
}

impl<T> interfaces::publisher::PeekMessage<T> for NatsPublisher<T>
where
    T: BorshDeserialize + QueueMsgId,
{
    // TODO: make sure you don't remove message from
    // main stream if moving out to dlq
    #[allow(refining_impl_trait, reason = "simplify")]
    #[tracing::instrument(skip_all)]
    async fn peek_last(&mut self) -> Result<Option<T::MessageId>, NatsError> {
        let last_sequence = self.stream.info().await?.state.last_sequence;
        if last_sequence == 0 {
            tracing::debug!("no messages");
            return Ok(None);
        }
        tracing::debug!(last_sequence, "last sequence is");
        let msg = self.stream.direct_get(last_sequence).await?;

        tracing::debug!(?msg, "found message");
        let msg = T::deserialize(&mut msg.payload.as_ref()).map_err(NatsError::Deserialize)?;
        Ok(Some(msg.id()))
    }
}
