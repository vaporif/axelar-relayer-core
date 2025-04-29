//! Crate with amplifier component connectors.

use tokio_util::sync::CancellationToken;

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
#[allow(clippy::print_stdout, reason = "runs before tracing")]
pub fn register_ctrlc_handler() -> CancellationToken {
    let cancel_token = CancellationToken::new();
    let ctrlc_token = cancel_token.clone();
    ctrlc::set_handler(move || {
        if ctrlc_token.is_cancelled() {
            std::process::exit(1);
        } else {
            println!("\nGraceful shutdown initiated. Press Ctrl+C again for immediate exit...");
            ctrlc_token.cancel();
        }
    })
    .expect("Failed to register ctrl+c handler");
    cancel_token
}
