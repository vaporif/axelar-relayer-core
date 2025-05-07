use google_cloud_pubsub::client::Client;
use google_cloud_pubsub::subscription::Subscription;
use google_cloud_pubsub::topic::Topic;

use super::GcpError;

pub(crate) async fn get_topic(client: &Client, topic_name: &str) -> Result<Topic, GcpError> {
    let topic = client.topic(topic_name);

    if !topic
        .exists(None)
        .await
        .map_err(|err| GcpError::TopicExistsCheck(Box::new(err)))?
    {
        return Err(GcpError::TopicNotFound {
            topic: topic_name.to_owned(),
        });
    }

    Ok(topic)
}

pub(crate) async fn get_subscription(
    client: &Client,
    subscription_name: &str,
) -> Result<Subscription, GcpError> {
    let subscription = client.subscription(subscription_name);
    if !subscription
        .exists(None)
        .await
        .map_err(|err| GcpError::SubscriptionExistsCheck(Box::new(err)))?
    {
        return Err(GcpError::SubscriptionNotFound {
            subscription: subscription_name.to_owned(),
        });
    }

    Ok(subscription)
}
