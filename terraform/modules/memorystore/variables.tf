variable "tasks_publisher_service_account_email" {
  type        = string
  description = "The email address of the tasks publisher service account"
}

variable "events_publisher_service_account_email" {
  type        = string
  description = "The email address of the events publisher service account"
}

variable "default_labels" {
  description = "Default labels to apply to all resources"
  type        = map(string)
  default     = {}
}
