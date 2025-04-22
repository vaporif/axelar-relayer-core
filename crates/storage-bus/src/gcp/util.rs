use std::time::Duration;

use google_cloud_googleapis::pubsub::v1::MessageStoragePolicy;
use google_cloud_pubsub::client::Client;
use google_cloud_pubsub::subscription::{Subscription, SubscriptionConfig};
use google_cloud_pubsub::topic::{Topic, TopicConfig};

use super::error::Error;

pub(crate) async fn get_or_create_topic(
    client: &Client,
    topic: &str,
    allowed_persistence_regions: Vec<String>,
    retention_duration: Duration,
) -> Result<Topic, Error> {
    let topic = client.topic(topic);

    if !topic.exists(None).await.map_err(Error::TopicExists)? {
        let config = TopicConfig {
            message_storage_policy: Some(MessageStoragePolicy {
                allowed_persistence_regions,
                enforce_in_transit: true,
            }),
            message_retention_duration: Some(retention_duration),
            ..Default::default()
        };
        topic
            .create(Some(config), None)
            .await
            .map_err(Error::TopicCreate)?;
    }

    Ok(topic)
}

pub(crate) async fn get_or_create_subscription(
    client: &Client,
    topic: &str,
    allowed_persistence_regions: Vec<String>,
    retention_duration: Duration,
    config: SubscriptionConfig,
) -> Result<Subscription, Error> {
    let subscription = format!("{topic}-subscription");
    let topic = get_or_create_topic(
        client,
        topic,
        allowed_persistence_regions,
        retention_duration,
    )
    .await?;

    let subscription = client.subscription(&subscription);
    if !subscription
        .exists(None)
        .await
        .map_err(Error::SubscriptionExists)?
    {
        subscription
            .create(topic.fully_qualified_name(), config, None)
            .await
            .map_err(Error::SubscriptionCreate)?;
    }

    Ok(subscription)
}
