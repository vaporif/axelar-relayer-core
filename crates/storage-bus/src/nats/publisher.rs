use core::fmt::Debug;
use core::marker::PhantomData;

use async_nats::jetstream;
use borsh::{BorshDeserialize, BorshSerialize};

use super::error::Error;
use crate::interfaces;

const NATS_MSG_ID: &str = "Nats-Msg-Id";

impl super::NatsStream {
    pub fn publisher<T: Send + Sync>(
        self,
        subject: impl Into<String>,
    ) -> Result<NatsPublisher<T>, Error> {
        Ok(NatsPublisher::<T> {
            context: self.context,
            stream: self.stream,
            subject: subject.into(),
            _phantom: PhantomData,
        })
    }
}

pub struct NatsPublisher<T> {
    context: jetstream::Context,
    stream: jetstream::stream::Stream,
    subject: String,
    _phantom: PhantomData<T>,
}

impl<T: BorshSerialize + Send + Sync + Debug> interfaces::publisher::Publisher<T>
    for NatsPublisher<T>
{
    type AckFuture = jetstream::context::PublishAckFuture;

    #[expect(refining_impl_trait)]
    #[tracing::instrument(skip_all)]
    async fn publish(
        &self,
        deduplication_id: impl Into<String>,
        data: T,
    ) -> Result<Self::AckFuture, Error> {
        let mut headers = async_nats::HeaderMap::new();
        let deduplication_id: String = deduplication_id.into();
        tracing::debug!(?deduplication_id, ?data, "got message");
        headers.append(NATS_MSG_ID.to_owned(), deduplication_id);
        let data = borsh::to_vec(&data).map_err(Error::Serialize)?;
        tracing::debug!("message encoded");
        let publish_ack_future = self
            .context
            .publish_with_headers(self.subject.clone(), headers, data.into())
            .await?;

        tracing::debug!("message published");

        Ok(publish_ack_future)
    }
}

impl<T> interfaces::publisher::PeekMessage<T> for NatsPublisher<T>
where
    T: BorshDeserialize + Send + Sync,
{
    // TODO: make sure you don't remove message from
    // main stream if moving out to dlq
    #[expect(refining_impl_trait)]
    #[tracing::instrument(skip_all)]
    async fn peek_last(&mut self) -> Result<Option<T>, Error> {
        let last_sequence = self.stream.info().await?.state.last_sequence;
        if last_sequence == 0 {
            tracing::debug!("no messages");
            return Ok(None);
        }
        tracing::debug!(last_sequence, "last sequence is");
        let msg = self.stream.direct_get(last_sequence).await?;

        tracing::debug!(?msg, "found message");
        let msg = T::deserialize(&mut msg.payload.as_ref()).map_err(Error::Deserialize)?;
        Ok(Some(msg))
    }
}
