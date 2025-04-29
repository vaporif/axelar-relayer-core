#![allow(
    clippy::missing_docs,
    reason = "the error macro already is descriptive enough"
)]

use google_cloud_pubsub::client::{self, google_cloud_auth};
use redis::RedisError;

/// Errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("auth error {0}")]
    Auth(#[from] google_cloud_auth::error::Error),
    #[error("client error {0}")]
    Client(#[from] client::Error),
    #[error("topic exists error {0}")]
    TopicExistsCheck(tonic::Status),
    #[error("topic not found: {topic}")]
    TopicNotFound { topic: String },
    #[error("topic exists check error {0}")]
    SubscriptionExistsCheck(tonic::Status),
    #[error("subscription not found: {subscription}")]
    SubscriptionNotFound { subscription: String },
    #[error("publish failure, error: {0}")]
    Publish(tonic::Status),
    #[error("ack error {0}")]
    Ack(tonic::Status),
    #[error("nack error {0}")]
    Nak(tonic::Status),
    #[error("modify ack deadline error {0}")]
    ModifyAckDeadline(tonic::Status),
    #[error("failed to deserialize val error: {0}")]
    Deserialize(std::io::Error),
    #[error("messages receiver task error {0}")]
    ReceiverTaskCrash(tonic::Status),
    #[error("failed to serialize val, error: {0}")]
    Serialize(std::io::Error),
    #[error("error connecting to redis {0}")]
    Connection(RedisError),
    #[error("error serializing data to redis {0}")]
    RedisSerialize(serde_json::Error),
    #[error("error saving data to redis {0}")]
    RedisSave(RedisError),
    #[error("error getting data from redis {0}")]
    RedisGet(RedisError),
    #[error("error deserializing data from redis {0}")]
    RedisDeserialize(serde_json::Error),
}
