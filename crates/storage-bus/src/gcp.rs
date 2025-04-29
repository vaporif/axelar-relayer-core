/// google cloud platform implementation
use error::Error;
use google_cloud_pubsub::client::{Client, ClientConfig};

/// connectors to queue
pub mod connectors;
/// consumer
pub mod consumer;
/// error
pub mod error;
/// redis keyvalue strore
pub mod kv_store;
/// publisher
pub mod publisher;
pub(crate) mod util;

pub struct PubSubBuilder {
    client: Client,
}

impl PubSubBuilder {
    /// connect to pubsub
    pub async fn connect() -> Result<Self, Error> {
        let config = ClientConfig::default().with_auth().await?;
        let client = Client::new(config).await?;

        Ok(Self { client })
    }
}
