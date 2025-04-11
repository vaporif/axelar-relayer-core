use core::fmt::Debug;
use core::marker::PhantomData;
use core::time::Duration;

use async_nats::jetstream;
use borsh::BorshDeserialize;
use futures::StreamExt as _;
use uuid::Uuid;

use super::NatsStream;
use super::error::Error;
use crate::interfaces;

impl NatsStream {
    pub async fn consumer<T: Debug>(
        self,
        description: impl Into<String>,
        deliver_group: impl Into<String>,
    ) -> Result<NatsConsumer<T>, Error> {
        // TODO: Revisit config
        let config = jetstream::consumer::push::Config {
            // TODO: generate proper name,
            deliver_subject: self.inbox.clone(),
            name: Some(Uuid::new_v4().to_string()),
            deliver_group: Some(deliver_group.into()),
            description: Some(description.into()),
            deliver_policy: jetstream::consumer::DeliverPolicy::New,
            ack_policy: jetstream::consumer::AckPolicy::Explicit,
            ack_wait: Duration::from_secs(10),
            max_deliver: 10,
            replay_policy: async_nats::jetstream::consumer::ReplayPolicy::Instant,
            sample_frequency: 10,
            max_ack_pending: 100,
            flow_control: true,
            idle_heartbeat: Duration::from_secs(10),
            inactive_threshold: Duration::from_secs(15),
            ..Default::default()
        };

        tracing::debug!(?config, "create or get consumer with config");
        let consumer_inner = self
            .stream
            .create_consumer(config)
            .await
            .map_err(Error::CreateConsumer)?;
        tracing::debug!("consumer ready");
        Ok(NatsConsumer::<T> {
            consumer_inner,
            _phantom: PhantomData,
        })
    }
}

impl From<interfaces::consumer::AckKind> for jetstream::AckKind {
    fn from(val: interfaces::consumer::AckKind) -> Self {
        match val {
            interfaces::consumer::AckKind::Ack => Self::Ack,
            interfaces::consumer::AckKind::Nak => Self::Nak(None),
            interfaces::consumer::AckKind::Progress => Self::Progress,
        }
    }
}
#[derive(Debug)]
pub struct NatsMessage<T> {
    decoded: T,
    msg: jetstream::Message,
}

impl<T: BorshDeserialize + Debug> NatsMessage<T> {
    fn decode(msg: jetstream::Message) -> Result<Self, Error> {
        tracing::debug!(?msg, "decoding msg");
        let decoded = T::deserialize(&mut msg.payload.as_ref()).map_err(Error::Deserialize)?;
        tracing::debug!(?decoded, "decoded msg");
        Ok(Self { decoded, msg })
    }
}

impl<T: Debug> interfaces::consumer::QueueMessage<T> for NatsMessage<T> {
    #[expect(refining_impl_trait)]
    #[tracing::instrument(skip_all)]
    async fn ack(&self, ack_kind: interfaces::consumer::AckKind) -> Result<(), Error> {
        tracing::debug!(?ack_kind, "sending ack");
        self.msg
            .ack_with(ack_kind.into())
            .await
            .map_err(Error::Ack)?;
        tracing::debug!("ack sent");
        Ok(())
    }

    fn decoded(&self) -> &T {
        &self.decoded
    }
}

pub struct NatsConsumer<T> {
    consumer_inner: jetstream::consumer::Consumer<jetstream::consumer::push::Config>,
    _phantom: PhantomData<T>,
}

impl<T> interfaces::consumer::Consumer<T> for NatsConsumer<T>
where
    T: BorshDeserialize + Debug,
{
    #[expect(refining_impl_trait)]
    #[tracing::instrument(skip_all)]
    async fn messages(
        &self,
    ) -> Result<
        impl futures::Stream<Item = Result<impl interfaces::consumer::QueueMessage<T>, Error>>,
        Error,
    > {
        tracing::debug!("getting message stream");
        let stream = self
            .consumer_inner
            .messages()
            .await
            .map_err(Error::MessagesStream)?;

        let decoded_stream = stream.then(|msg_result| async {
            match msg_result {
                Ok(msg) => NatsMessage::decode(msg),
                Err(err) => Err(Error::from(err)),
            }
        });

        Ok(decoded_stream)
    }
}
