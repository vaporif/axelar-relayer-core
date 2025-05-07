output "tasks_publisher_service_account_email" {
  value       = google_service_account.tasks_publisher_iam.email
  description = "The email address of the tasks publisher service account"
}

output "tasks_subscriber_service_account_email" {
  value       = google_service_account.tasks_subscriber_iam.email
  description = "The email address of the tasks subscriber service account"
}

output "events_publisher_service_account_email" {
  value       = google_service_account.events_publisher_iam.email
  description = "The email address of the events publisher service account"
}

output "events_subscriber_service_account_email" {
  value       = google_service_account.events_subscriber_iam.email
  description = "The email address of the events subscriber service account"
}
