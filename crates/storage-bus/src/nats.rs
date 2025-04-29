/// nats implementation
use std::time::Duration;

use async_nats::{self, jetstream};
use error::Error;
use url::Url;

/// Nats clients builder
pub struct NatsBuilder {
    inbox: String,
    context: jetstream::context::Context,
}

pub mod connectors;
pub mod consumer;
pub mod error;
pub mod kv_store;
pub mod publisher;

impl NatsBuilder {
    pub async fn connect_to_nats(urls: &[Url]) -> Result<Self, Error> {
        let connect_options = async_nats::ConnectOptions::default().retry_on_initial_connect();
        let client = async_nats::connect_with_options(urls, connect_options).await?;
        let inbox = client.new_inbox();
        tracing::debug!("connected to nats");
        let context = jetstream::new(client);

        Ok(Self { inbox, context })
    }

    pub async fn stream(
        self,
        name: impl Into<String>,
        subject: impl Into<String>,
        description: impl Into<String>,
    ) -> Result<NatsStream, Error> {
        // TODO: Revisit config
        let config = jetstream::stream::Config {
            name: name.into(),
            discard: jetstream::stream::DiscardPolicy::Old,
            subjects: vec![subject.into()],
            retention: jetstream::stream::RetentionPolicy::Limits,
            duplicate_window: Duration::from_secs(60 * 5),
            description: Some(description.into()),
            allow_rollup: true,
            deny_delete: true,
            allow_direct: true,
            ..Default::default()
        };
        tracing::debug!(?config, "create or get stream with config");
        let stream = self.context.get_or_create_stream(config).await?;
        tracing::debug!("stream ready");

        Ok(NatsStream {
            inbox: self.inbox,
            context: self.context,
            stream,
        })
    }
}

pub struct NatsStream {
    inbox: String,
    context: jetstream::context::Context,
    stream: jetstream::stream::Stream,
}
