use std::fmt::Display;
use std::sync::atomic::AtomicBool;

use tokio_retry::strategy::ExponentialBackoff;

use super::{Abortable, RetryError};

pub struct RetryFn<Fn, T, Err>
where
    Fn: AsyncFn() -> Result<T, Err>,
    Err: Display,
{
    backoff: ExponentialBackoff,
    function: Fn,
    max_attempts: usize,
}

impl<Fn, T, Err> RetryFn<Fn, T, Err>
where
    Fn: AsyncFn() -> Result<T, Err>,
    Err: Display + Abortable,
{
    pub(crate) fn new(backoff: ExponentialBackoff, function: Fn) -> Self {
        Self {
            backoff,
            function,
            // TODO: config
            max_attempts: 20,
        }
    }

    // you can use a macro to reduce copy-paste
    // but this will reduce readability
    #[tracing::instrument(skip_all)]
    pub async fn retry(self, shutdown: &AtomicBool) -> Result<T, RetryError<Err>> {
        if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
            return Err(RetryError::Shutdown);
        }

        match (self.function)().await {
            Ok(res) => return Ok(res),
            Err(err) => {
                if err.abortable() {
                    tracing::error!(%err, "aborted");
                    return Err(RetryError::Aborted(err));
                }

                tracing::error!(%err, "will retry");
            }
        }
        for (retry_attempt, duration) in self.backoff.enumerate().take(self.max_attempts) {
            if shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                return Err(RetryError::Shutdown);
            }
            tokio::time::sleep(duration).await;
            match (self.function)().await {
                Ok(res) => return Ok(res),
                Err(err) => {
                    if err.abortable() {
                        tracing::error!(%err, "aborted");
                        return Err(RetryError::Aborted(err));
                    }

                    tracing::error!(%err, retry_attempt, "retryable lambda err");
                }
            }
        }

        return Err(RetryError::MaxAttempts);
    }
}
