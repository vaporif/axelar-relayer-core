resource "google_redis_instance" "kvstore" {
  name           = "kvstore"
  memory_size_gb = 1

  persistence_config {
    persistence_mode    = "RDB"
    rdb_snapshot_period = "TWELVE_HOURS"
  }
}


# TODO: Complete iam
# resource "google_redis_instance_iam_binding" "redis_instance_user" {
#   instance = google_redis_instance.kvstore.name
#   role     = "roles/redis.editor"  # Basic access role
#   members = [
#     "serviceAccount:${var.tasks_subscriber_service_account_email}",
#     "serviceAccount:${var.events_subscriber_service_account_email}",
#   ]
# }
