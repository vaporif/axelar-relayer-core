resource "google_service_account" "tasks_publisher_iam" {
  account_id   = "tasks-publisher"
  display_name = "Service account for tasks publisher"
}

resource "google_service_account" "tasks_subscriber_iam" {
  account_id   = "tasks-subscriber"
  display_name = "Service account for tasks subscriber"
}

resource "google_service_account" "events_publisher_iam" {
  account_id   = "events-publisher"
  display_name = "Service account for events publisher"
}

resource "google_service_account" "events_subscriber_iam" {
  account_id   = "events-subscriber"
  display_name = "Service account for tasks subscriber"
}

