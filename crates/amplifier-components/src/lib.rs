//! Crate with amplifier component connectors.
use core::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use eyre::Context as _;
use relayer_amplifier_api_integration::Config;
use relayer_amplifier_api_integration::amplifier_api::AmplifierApiClient;

#[cfg(feature = "nats")]
pub mod nats;

pub fn register_backtrace() {
    #[cfg(debug_assertions)]
    {
        // Note: unsafe only in multithreaded
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "full");
        }
    }
}

/// Run from main thread once
#[must_use]
pub fn register_ctrlc_handler() -> Arc<AtomicBool> {
    let ctrl_c_shutdown = Arc::new(AtomicBool::new(false));
    let shutdown = ctrl_c_shutdown.clone();
    ctrlc::set_handler(move || {
        if ctrl_c_shutdown.load(Ordering::Relaxed) {
            std::process::exit(1);
        } else {
            println!("\nGraceful shutdown initiated. Press Ctrl+C again for immediate exit...");
            ctrl_c_shutdown.store(true, Ordering::Relaxed);
        }
    })
    .expect("Failed to register ctrl+c handler");

    shutdown
}

#[expect(dead_code)]
fn amplifier_client(config: &Config) -> eyre::Result<AmplifierApiClient> {
    AmplifierApiClient::new(config.url.clone(), &config.identity)
        .wrap_err("amplifier api client failed to create")
}
