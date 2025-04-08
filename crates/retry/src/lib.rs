pub(crate) mod backoff_pair_iterator;
pub mod builder;
pub(crate) mod retry_fn;
pub(crate) mod retry_pair_fn;

use std::fmt::Display;
use std::num::NonZeroU64;
use std::time::Duration;

pub use builder::BackoffRetryBuilder;

#[derive(Debug, thiserror::Error)]
pub enum RetryError<Err: Display + Abortable> {
    #[error("aborted {0}")]
    Aborted(Err),
    // TODO: concat or store last error
    #[error("max retry attempts reached")]
    MaxAttempts,
    #[error("shuting down...")]
    Shutdown,
}

#[derive(Debug, Clone)]
pub struct BackoffParrams {
    initial_delay: Duration,
    backoff_factor: NonZeroU64,
    max_delay: Duration,
}

pub trait Abortable {
    fn abortable(&self) -> bool;
}
