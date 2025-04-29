output "topic_names" {
  value = [
    google_pubsub_topic.amplifier_events.name,
    google_pubsub_topic.amplifier_tasks.name,
    google_pubsub_topic.amplifier_events_dlq.name,
    google_pubsub_topic.amplifier_tasks_dlq.name,
  ]
  description = "Names of the created Pub/Sub topics"
}

output "subscription_names" {
  value = [
    google_pubsub_subscription.amplifier_events_sub.name,
    google_pubsub_subscription.amplifier_tasks_sub.name,
  ]
  description = "Names of the created Pub/Sub subscriptions"
}
