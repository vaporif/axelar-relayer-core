use google_cloud_pubsub::client::Client;
use google_cloud_pubsub::subscription::Subscription;
use google_cloud_pubsub::topic::Topic;

use super::error::Error;

/// get & check if topic exists
pub(crate) async fn get_topic(client: &Client, topic_name: &str) -> Result<Topic, Error> {
    let topic = client.topic(topic_name);

    if !topic.exists(None).await.map_err(Error::TopicExistsCheck)? {
        return Err(Error::TopicNotFound {
            topic: topic_name.to_string(),
        });
    }

    Ok(topic)
}

/// get & check if subscription exists
pub(crate) async fn get_subscription(
    client: &Client,
    subscription_name: &str,
) -> Result<Subscription, Error> {
    let subscription = client.subscription(subscription_name);
    if !subscription
        .exists(None)
        .await
        .map_err(Error::SubscriptionExistsCheck)?
    {
        return Err(Error::SubscriptionNotFound {
            subscription: subscription_name.to_string(),
        });
    }

    Ok(subscription)
}
