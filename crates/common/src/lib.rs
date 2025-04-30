//! Crate with amplifier component connectors.

use tokio_util::sync::CancellationToken;

/// Ensure backtrace is enabled
pub fn register_backtrace() {
    #[cfg(debug_assertions)]
    {
        // SAFETY: called from single thread i.e. is safe
        unsafe {
            std::env::set_var("RUST_BACKTRACE", "full");
        }
    }
}

/// Run from main thread once
#[must_use]
#[allow(clippy::print_stdout, reason = "runs before tracing")]
#[expect(clippy::missing_panics_doc, reason = "infallible")]
pub fn register_ctrlc_handler() -> CancellationToken {
    let cancel_token = CancellationToken::new();
    let ctrlc_token = cancel_token.clone();

    ctrlc::set_handler(move || {
        if ctrlc_token.is_cancelled() {
            #[expect(clippy::exit, reason = "ctrl+c")]
            std::process::exit(1);
        } else {
            println!("\nGraceful shutdown initiated. Press Ctrl+C again for immediate exit...");
            ctrlc_token.cancel();
        }
    })
    .expect("Failed to register ctrl+c handler");
    cancel_token
}

/// Generic trait for Id on a type
pub trait Id {
    /// type of message id
    type MessageId;
    /// return id
    fn id(&self) -> Self::MessageId;
}
