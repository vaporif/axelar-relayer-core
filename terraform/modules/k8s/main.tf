// TODO: finish it and import in env
resource "google_container_cluster" "primary" {
  name               = var.cluster_name
  location           = var.cluster_location
  initial_node_count = 4

  node_config {
    machine_type = var.machine_type
    oauth_scopes = [
      "https://www.googleapis.com/auth/compute",
      "https://www.googleapis.com/auth/devstorage.read_only",
      "https://www.googleapis.com/auth/logging.write",
      "https://www.googleapis.com/auth/monitoring.write",
    ]
  }
}

resource "kubernetes_namespace" "primary_namespace" {
  metadata {
    name = var.namespace_name
  }
  depends_on = [google_container_cluster.primary]
}
