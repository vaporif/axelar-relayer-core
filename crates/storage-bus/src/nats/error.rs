use async_nats::ConnectError;
use async_nats::jetstream::consumer::StreamError;
use async_nats::jetstream::consumer::push::MessagesError;
use async_nats::jetstream::context::{CreateKeyValueError, CreateStreamError, PublishError};
use async_nats::jetstream::kv::{EntryError, PutError, UpdateError};
use async_nats::jetstream::stream::{ConsumerError, DirectGetError, InfoError};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("connect to nats error: {0}")]
    Connect(#[from] ConnectError),
    #[error("create nats kvstore error: {0}")]
    CreateKeyValue(#[from] CreateKeyValueError),
    #[error("put nats kvstore val error: {0}")]
    Put(#[from] PutError),
    #[error("failed to deserialize val error: {0}")]
    Deserialize(std::io::Error),
    #[error("failed to serialize val, error: {0}")]
    Serialize(std::io::Error),
    #[error("update nats kvstore val error: {0}")]
    Update(#[from] UpdateError),
    #[error("get nats kvstore val error: {0}")]
    Entry(EntryError),
    #[error("create nats stream error: {0}")]
    CreateStream(#[from] CreateStreamError),
    #[error("publish nats message error: {0}")]
    Publish(#[from] PublishError),
    #[error("create nats consumer error: {0}")]
    CreateConsumer(ConsumerError),
    #[error("read nats stream message serror: {0}")]
    MessagesStream(StreamError),
    #[error("read nats consumer message error: {0}")]
    Messages(#[from] MessagesError),
    #[error("nats info error: {0}")]
    Info(#[from] InfoError),
    #[error("get direct nats message error: {0}")]
    DirectGet(#[from] DirectGetError),
    #[error("nats ack error: {0}")]
    Ack(async_nats::Error),
}
