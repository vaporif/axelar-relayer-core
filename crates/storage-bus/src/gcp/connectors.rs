use std::fmt::Debug;

use borsh::BorshDeserialize;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::PubSubBuilder;
use super::consumer::GcpConsumer;
use super::error::Error;
use super::kv_store::GcpRedis;
use super::publisher::{GcpPublisher, PeekableGcpPublisher};

pub async fn connect_consumer<T>(
    subscription: &str,
    message_buffer_size: usize,
    nak_deadline_secs: i32,
    cancel_token: CancellationToken,
) -> Result<GcpConsumer<T>, Error>
where
    T: Debug + Send + Sync + BorshDeserialize + 'static,
{
    PubSubBuilder::connect()
        .await?
        .consumer(
            subscription,
            message_buffer_size,
            nak_deadline_secs,
            cancel_token,
        )
        .await
}

pub async fn connect_publisher<T>(topic: &str) -> Result<GcpPublisher<T>, Error>
where
    T: Send + Sync,
{
    PubSubBuilder::connect().await?.publisher(topic).await
}

pub async fn connect_peekable_publisher<T>(
    topic: &str,
    redis_connection: String,
    redis_key: String,
) -> Result<PeekableGcpPublisher<T>, Error>
where
    T: Send + Sync + Serialize + for<'de> Deserialize<'de>,
{
    let kv_store = GcpRedis::connect(redis_key, redis_connection).await?;

    PubSubBuilder::connect()
        .await?
        .peekable_publisher(topic, kv_store)
        .await
}
