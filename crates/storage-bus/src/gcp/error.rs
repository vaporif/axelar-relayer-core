use firestore::errors::FirestoreError;
use google_cloud_pubsub::client::{self, google_cloud_auth};
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("auth error {0}")]
    Auth(#[from] google_cloud_auth::error::Error),
    #[error("client error {0}")]
    Client(#[from] client::Error),
    #[error("topic exists error {0}")]
    TopicExists(tonic::Status),
    #[error("topic create error {0}")]
    TopicCreate(tonic::Status),
    #[error("topic exists error {0}")]
    SubscriptionExists(tonic::Status),
    #[error("topic create error {0}")]
    SubscriptionCreate(tonic::Status),
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
    #[error(transparent)]
    Firestore(#[from] FirestoreError),
}
