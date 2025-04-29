provider "google" {
  project = "amplifier-relayer-mainnet"
  region  = "europe-north2"
}

terraform {
  backend "gcs" {
    bucket = "amplifier-relayer-mainnet-tf-state"
    prefix = "mainnet/terraform.tfstate"
  }
}

module "iam" {
  source = "../../modules/iam"
}

module "kms" {
  source                                 = "../../modules/kms"
  protection_level                       = "HARDWARE"
  events_publisher_service_account_email = module.iam.events_publisher_service_account_email
}

module "pubsub" {
  source                                  = "../../modules/pubsub"
  tasks_ingester_service_account_email    = module.iam.tasks_ingester_service_account_email
  tasks_subscriber_service_account_email  = module.iam.tasks_subscriber_service_account_email
  events_ingester_service_account_email   = module.iam.events_ingester_service_account_email
  events_subscriber_service_account_email = module.iam.events_subscriber_service_account_email
}

module "memorystore" {
  source                               = "../../modules/memorystore"
  tasks_ingester_service_account_email = module.iam.tasks_ingester_service_account_email
}
