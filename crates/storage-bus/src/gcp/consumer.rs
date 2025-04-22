use std::fmt::Debug;
use std::marker::PhantomData;
use std::time::Duration;

use borsh::BorshDeserialize;
use google_cloud_googleapis::pubsub::v1::DeadLetterPolicy;
use google_cloud_pubsub::subscriber::ReceivedMessage;
use google_cloud_pubsub::subscription::{Subscription, SubscriptionConfig};
use tokio_util::sync::CancellationToken;

use super::Error;
use super::util::get_or_create_subscription;
use crate::interfaces;

pub struct ConsumerConfig {
    pub ack_deadline_seconds: i32,
    pub dead_letter_policy: DeadLetterPolicy,
    pub nak_deadline_secs: i32,
}

impl super::PubSubBuilder {
    pub async fn consumer<T: Debug + Send + Sync + BorshDeserialize + 'static>(
        self,
        topic: &str,
        consumer_config: ConsumerConfig,
        message_buffer_size: usize,
        allowed_persistence_regions: Vec<String>,
        dlq_retention_duration: Duration,
        cancel_token: CancellationToken,
    ) -> Result<GcpConsumer<T>, Error> {
        let dlq_topic = consumer_config.dead_letter_policy.dead_letter_topic.clone();
        let config = SubscriptionConfig {
            ack_deadline_seconds: consumer_config.ack_deadline_seconds,
            dead_letter_policy: Some(consumer_config.dead_letter_policy),
            ..Default::default()
        };

        // NOTE: Subscription is required, otherwise dlq messages will dissapear
        _ = get_or_create_subscription(
            &self.client,
            &dlq_topic,
            allowed_persistence_regions.clone(),
            dlq_retention_duration,
            SubscriptionConfig::default(),
        );

        let subscription = get_or_create_subscription(
            &self.client,
            topic,
            allowed_persistence_regions,
            dlq_retention_duration,
            config,
        )
        .await?;

        let (sender, receiver) = flume::bounded(message_buffer_size);

        let read_messages_handle = start_read_messages_task(
            subscription,
            sender,
            consumer_config.nak_deadline_secs,
            cancel_token.clone(),
        );

        let consumer = GcpConsumer::<T> {
            receiver,
            cancel_token,
            read_messages_handle,
            _phantom: PhantomData,
        };

        Ok(consumer)
    }
}

#[derive(Debug)]
pub struct GcpMessage<T: Send + Sync> {
    msg: ReceivedMessage,
    decoded: T,
    nak_deadline_secs: i32,
}

impl<T: BorshDeserialize + Send + Sync + Debug> GcpMessage<T> {
    fn decode(msg: ReceivedMessage, nak_deadline_secs: i32) -> Result<Self, Error> {
        tracing::debug!(?msg, "decoding msg");
        let decoded = T::deserialize(&mut msg.message.data.as_ref()).map_err(Error::Deserialize)?;
        tracing::debug!(?decoded, "decoded msg");
        Ok(Self {
            decoded,
            msg,
            nak_deadline_secs,
        })
    }
}

impl<T: Debug + Send + Sync> interfaces::consumer::QueueMessage<T> for GcpMessage<T> {
    fn decoded(&self) -> &T {
        &self.decoded
    }

    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip_all)]
    async fn ack(&self, ack_kind: interfaces::consumer::AckKind) -> Result<(), Error> {
        tracing::debug!(?ack_kind, "sending ack");

        match ack_kind {
            interfaces::consumer::AckKind::Ack => self.msg.ack().await.map_err(Error::Ack)?,
            interfaces::consumer::AckKind::Nak => self.msg.nack().await.map_err(Error::Nak)?,
            interfaces::consumer::AckKind::Progress => self
                .msg
                .modify_ack_deadline(self.nak_deadline_secs)
                .await
                .map_err(Error::ModifyAckDeadline)?,
        }
        tracing::debug!("ack sent");
        Ok(())
    }
}

pub struct GcpConsumer<T: Send + Sync> {
    receiver: flume::Receiver<Result<GcpMessage<T>, Error>>,
    cancel_token: CancellationToken,
    read_messages_handle: tokio::task::JoinHandle<Result<(), Error>>,
    _phantom: PhantomData<T>,
}

impl<T> interfaces::consumer::Consumer<T> for GcpConsumer<T>
where
    T: BorshDeserialize + Sync + Send + Debug,
{
    #[allow(refining_impl_trait)]
    #[tracing::instrument(skip_all)]
    async fn messages(
        &self,
    ) -> Result<
        impl futures::Stream<Item = Result<impl interfaces::consumer::QueueMessage<T>, Error>>,
        Error,
    > {
        if self.read_messages_handle.is_finished() {
            // TODO: process
        }
        tracing::debug!("getting message stream");

        Ok(self.receiver.stream())
    }
}

/// Starts tokio task to read from subscription to relay (messages) to
/// receiver channel
fn start_read_messages_task<T>(
    subscription: Subscription,
    sender: flume::Sender<Result<GcpMessage<T>, Error>>,
    nak_deadline_secs: i32,
    cancel_token: CancellationToken,
) -> tokio::task::JoinHandle<Result<(), Error>>
where
    T: BorshDeserialize + Send + Sync + Debug + 'static,
{
    tokio::spawn(async move {
        subscription
            .receive(
                move |message, cancel| {
                    let sender = sender.clone();
                    async move {
                        tracing::debug!(?message, "got message");
                        match GcpMessage::decode(message, nak_deadline_secs) {
                            Ok(decoded_message) => {
                                if let Err(err) = sender.send_async(Ok(decoded_message)).await {
                                    tracing::info!(?err, "shutting down");
                                    cancel.cancel();
                                }
                            }
                            Err(err) => {
                                if let Err(err) = sender.send_async(Err(err)).await {
                                    tracing::info!(?err, "shutting down");
                                    cancel.cancel();
                                }
                            }
                        }
                    }
                },
                cancel_token.clone(),
                None,
            )
            .await
            .map_err(Error::ReceiverTaskCrash)
    })
}

// TODO: create topic too?

impl<T: Send + Sync> Drop for GcpConsumer<T> {
    fn drop(&mut self) {
        self.cancel_token.cancel();
    }
}
