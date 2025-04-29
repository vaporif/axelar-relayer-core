output "amplifier_api_keyring" {
  description = "ID of the amplifier api keyring"
  value       = google_kms_key_ring.amplifier_api.id
}
