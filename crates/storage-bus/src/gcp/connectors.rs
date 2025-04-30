use core::fmt::Debug;

use borsh::BorshDeserialize;
use google_cloud_pubsub::client::{Client, ClientConfig};
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use super::GcpError;
use super::consumer::GcpConsumer;
use super::kv_store::RedisClient;
use super::publisher::{GcpPublisher, PeekableGcpPublisher};

pub async fn connect_consumer<T>(
    subscription: &str,
    message_buffer_size: usize,
    nak_deadline_secs: i32,
    cancel_token: CancellationToken,
) -> Result<GcpConsumer<T>, GcpError>
where
    T: Debug + Send + Sync + BorshDeserialize + 'static,
{
    let client = connect_client().await?;
    let consumer = GcpConsumer::<T>::new(
        &client,
        subscription,
        message_buffer_size,
        nak_deadline_secs,
        cancel_token,
    )
    .await?;

    Ok(consumer)
}

/// connect publisher
pub async fn connect_publisher<T>(topic: &str) -> Result<GcpPublisher<T>, GcpError>
where
    T: Send + Sync,
{
    let client = connect_client().await?;
    let publisher = GcpPublisher::<T>::new(&client, topic).await?;
    Ok(publisher)
}

/// connect peekable publisher with ability to get last pushed message (without consuming it)
pub async fn connect_peekable_publisher<T>(
    topic: &str,
    redis_connection: String,
    redis_key: String,
) -> Result<PeekableGcpPublisher<T>, GcpError>
where
    T: common::Id + Send + Sync + Serialize + for<'de> Deserialize<'de>,
    T::MessageId: Serialize + for<'de> Deserialize<'de> + Debug,
{
    let kv_store = RedisClient::connect(redis_key, redis_connection).await?;
    let client = connect_client().await?;
    let publisher = PeekableGcpPublisher::new(&client, topic, kv_store).await?;
    Ok(publisher)
}

async fn connect_client() -> Result<Client, GcpError> {
    let config = ClientConfig::default().with_auth().await?;
    let client = Client::new(config).await?;
    Ok(client)
}
