terraform {
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "6.31.0"
    }
  }
  required_version = ">=1.9.0"
}

provider "google" {
  project = "amplifier-relayer-testnet"
  region  = "europe-north2"
}

terraform {
  backend "gcs" {
    bucket = "amplifier-relayer-testnet-tf-state"
    prefix = "testnet/terraform.tfstate"
  }
}

module "iam" {
  source = "../../modules/iam"
}

module "kms" {
  source                                 = "../../modules/kms"
  protection_level                       = "SOFTWARE"
  events_publisher_service_account_email = module.iam.events_publisher_service_account_email
}

module "pubsub" {
  source                                  = "../../modules/pubsub"
  tasks_publisher_service_account_email   = module.iam.tasks_publisher_service_account_email
  tasks_subscriber_service_account_email  = module.iam.tasks_subscriber_service_account_email
  events_publisher_service_account_email  = module.iam.events_publisher_service_account_email
  events_subscriber_service_account_email = module.iam.events_subscriber_service_account_email
}

module "memorystore" {
  source                                 = "../../modules/memorystore"
  tasks_publisher_service_account_email  = module.iam.tasks_publisher_service_account_email
  events_publisher_service_account_email = module.iam.events_publisher_service_account_email
}
