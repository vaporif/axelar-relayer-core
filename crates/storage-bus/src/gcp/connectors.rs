pub(crate) const AMPLIFIER_KV_STORE_PROJECT_ID: &str = "amplifier";

pub mod events {
    use std::time::Duration;

    use google_cloud_googleapis::pubsub::v1::DeadLetterPolicy;
    use tokio_util::sync::CancellationToken;

    use super::{AMPLIFIER_KV_STORE_PROJECT_ID, dlq_topic};
    use crate::gcp::PubSubBuilder;
    use crate::gcp::consumer::{ConsumerConfig, GcpConsumer};
    use crate::gcp::error::Error;
    use crate::gcp::kv_store::GcpKvStore;
    use crate::gcp::publisher::GcpPublisher;

    const EVENTS_TOPIC: &str = "amplifier-events";
    const EVENTS_COLLECTION_ID: &str = "amplifier-events";
    const EVENTS_DOCUMENT_ID: &str = "amplifier-events";

    pub async fn connect_consumer(
        message_buffer_size: usize,
        ack_deadline_seconds: i32,
        nak_deadline_secs: i32,
        max_delivery_attempts: i32,
        dlq_retention: Duration,
        allowed_persistence_regions: Vec<String>,
        cancel_token: CancellationToken,
    ) -> Result<GcpConsumer<amplifier_api::types::Event>, Error> {
        let dead_letter_policy = DeadLetterPolicy {
            dead_letter_topic: dlq_topic(EVENTS_TOPIC),
            max_delivery_attempts,
        };

        let consumer_config = ConsumerConfig {
            ack_deadline_seconds,
            nak_deadline_secs,
            dead_letter_policy,
        };

        PubSubBuilder::connect()
            .await?
            .consumer(
                EVENTS_TOPIC,
                consumer_config,
                message_buffer_size,
                allowed_persistence_regions,
                dlq_retention,
                cancel_token,
            )
            .await
    }

    pub async fn connect_publisher(
        allowed_persistence_regions: Vec<String>,
        message_retention: Duration,
    ) -> Result<GcpPublisher<amplifier_api::types::Event>, Error> {
        let kv_store = GcpKvStore::connect(
            EVENTS_COLLECTION_ID.to_string(),
            EVENTS_DOCUMENT_ID.to_string(),
            AMPLIFIER_KV_STORE_PROJECT_ID,
        )
        .await?;

        PubSubBuilder::connect()
            .await?
            .publisher(
                EVENTS_TOPIC,
                allowed_persistence_regions,
                message_retention,
                kv_store,
            )
            .await
    }
}

pub mod tasks {
    use std::time::Duration;

    use google_cloud_googleapis::pubsub::v1::DeadLetterPolicy;
    use tokio_util::sync::CancellationToken;

    use super::{AMPLIFIER_KV_STORE_PROJECT_ID, dlq_topic};
    use crate::gcp::PubSubBuilder;
    use crate::gcp::consumer::{ConsumerConfig, GcpConsumer};
    use crate::gcp::error::Error;
    use crate::gcp::kv_store::GcpKvStore;
    use crate::gcp::publisher::GcpPublisher;

    const TASKS_TOPIC: &str = "amplifier-tasks";
    const TASKS_COLLECTION_ID: &str = "amplifier-events";
    const TASKS_DOCUMENT_ID: &str = "amplifier-events";

    pub async fn connect_consumer(
        message_buffer_size: usize,
        ack_deadline_seconds: i32,
        nak_deadline_secs: i32,
        max_delivery_attempts: i32,
        dlq_retention: Duration,
        allowed_persistence_regions: Vec<String>,
        cancel_token: CancellationToken,
    ) -> Result<GcpConsumer<amplifier_api::types::Event>, Error> {
        let dead_letter_policy = DeadLetterPolicy {
            dead_letter_topic: dlq_topic(TASKS_TOPIC),
            max_delivery_attempts,
        };

        let consumer_config = ConsumerConfig {
            ack_deadline_seconds,
            nak_deadline_secs,
            dead_letter_policy,
        };

        PubSubBuilder::connect()
            .await?
            .consumer(
                TASKS_TOPIC,
                consumer_config,
                message_buffer_size,
                allowed_persistence_regions,
                dlq_retention,
                cancel_token,
            )
            .await
    }

    pub async fn connect_publisher(
        allowed_persistence_regions: Vec<String>,
        message_retention: Duration,
    ) -> Result<GcpPublisher<amplifier_api::types::Event>, Error> {
        let kv_store = GcpKvStore::connect(
            TASKS_COLLECTION_ID.to_string(),
            TASKS_DOCUMENT_ID.to_string(),
            AMPLIFIER_KV_STORE_PROJECT_ID,
        )
        .await?;

        PubSubBuilder::connect()
            .await?
            .publisher(
                TASKS_TOPIC,
                allowed_persistence_regions,
                message_retention,
                kv_store,
            )
            .await
    }
}

pub(crate) fn dlq_topic(topic: &str) -> String {
    format!("{topic}-dlq")
}
