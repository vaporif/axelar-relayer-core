output "memorystore_id" {
  description = "ID of the memorystore database"
  value       = google_redis_instance.kvstore.id
}

