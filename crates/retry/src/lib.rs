//! Crate with retry logic
pub(crate) mod backoff_pair_iterator;
pub mod builder;
pub(crate) mod retry_fn;
pub(crate) mod retry_pair_fn;

use core::fmt::Display;
use core::num::NonZeroU64;
use std::time::Duration;

pub use builder::BackoffRetryBuilder;

#[derive(Debug, thiserror::Error)]
pub enum RetryError<Err: Display + Abortable> {
    #[error("aborted {0}")]
    Aborted(Err),
    // TODO: add last err
    #[error("max retry attempts reached")]
    MaxAttempts,
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

#[cfg(test)]
mod tests {
    use std::num::NonZeroU64;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct TestError(bool); // true = abortable
    impl std::fmt::Display for TestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "TestError abortable={}", self.0)
        }
    }
    impl Abortable for TestError {
        fn abortable(&self) -> bool {
            self.0
        }
    }

    #[tokio::test]
    async fn builder_with_retry_success() {
        let builder = BackoffRetryBuilder::new(
            Duration::from_millis(1),
            NonZeroU64::new(2).unwrap(),
            Duration::from_millis(10),
        );
        let called = Arc::new(Mutex::new(0));
        let called2 = called.clone();
        let func = move || {
            let called = called2.clone();
            async move {
                let mut lock = called.lock().unwrap();
                *lock += 1;
                if *lock < 2 {
                    Err(TestError(false))
                } else {
                    Ok::<_, TestError>("ok")
                }
            }
        };
        let retry = builder.with_retry(func);
        let result = retry.retry().await;
        assert_eq!(result.unwrap(), "ok");
        assert_eq!(*called.lock().unwrap(), 2);
    }

    #[tokio::test]
    async fn builder_with_retry_pair_success() {
        let builder = BackoffRetryBuilder::new(
            Duration::from_millis(1),
            NonZeroU64::new(2).unwrap(),
            Duration::from_millis(10),
        );
        let func1 = || async { Err::<&'static str, _>(TestError(false)) };
        let func2 = || async { Ok::<_, TestError>("ok2") };
        let retry = builder.with_retry_pair(func1, func2);
        let result = retry.retry().await;
        assert_eq!(result.unwrap(), "ok2");
    }

    #[tokio::test]
    async fn builder_with_retry_aborts_on_abortable() {
        let builder = BackoffRetryBuilder::new(
            Duration::from_millis(1),
            NonZeroU64::new(2).unwrap(),
            Duration::from_millis(10),
        );
        let func = || async { Err::<(), _>(TestError(true)) };
        let retry = builder.with_retry(func);
        let result = retry.retry().await;
        assert!(matches!(result, Err(RetryError::Aborted(_))));
    }
}
