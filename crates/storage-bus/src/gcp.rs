/// google cloud platform implementation
use error::Error;
use google_cloud_pubsub::client::{Client, ClientConfig};

pub mod connectors;
pub mod consumer;
pub mod error;
pub mod kv_store;
pub mod publisher;
pub(crate) mod util;

pub struct PubSubBuilder {
    client: Client,
}

impl PubSubBuilder {
    pub async fn connect() -> Result<Self, Error> {
        let config = ClientConfig::default().with_auth().await?;
        let client = Client::new(config).await?;

        Ok(Self { client })
    }
}
