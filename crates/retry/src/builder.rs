use core::fmt::Display;
use core::num::NonZeroU64;
use core::time::Duration;

use tokio_retry::strategy::ExponentialBackoff;

use super::backoff_pair_iterator::BackoffPairIterator;
use super::retry_fn::RetryFn;
use super::retry_pair_fn::RetryPairFn;
use super::{Abortable, BackoffParrams};

#[derive(Debug, Clone)]
pub struct Builder {
    backoff: BackoffParrams,
}

impl Builder {
    #[must_use]
    pub const fn new(
        initial_delay: Duration,
        backoff_factor: NonZeroU64,
        max_delay: Duration,
    ) -> Self {
        let backoff = BackoffParrams {
            initial_delay,
            backoff_factor,
            max_delay,
        };

        Self { backoff }
    }

    #[must_use]
    pub fn with_retry<Fn, T, Err>(&self, function: Fn) -> RetryFn<Fn, T, Err>
    where
        Fn: AsyncFn() -> Result<T, Err>,
        Err: Display + Abortable,
    {
        let backoff = ExponentialBackoff::from_millis(
            u64::try_from(self.backoff.initial_delay.as_millis()).unwrap_or(u64::MAX),
        )
        .factor(self.backoff.backoff_factor.into())
        .max_delay(self.backoff.max_delay);

        RetryFn::new(backoff, function)
    }

    #[must_use]
    pub fn with_retry_pair<Fn1, Fn2, T, Err>(
        &self,
        first_function: Fn1,
        second_function: Fn2,
    ) -> RetryPairFn<Fn1, Fn2, T, Err>
    where
        Fn1: AsyncFn() -> Result<T, Err>,
        Fn2: AsyncFn() -> Result<T, Err>,
        Err: Display + Abortable,
    {
        let backoff = BackoffPairIterator::new(
            self.backoff.initial_delay,
            self.backoff.backoff_factor,
            self.backoff.max_delay,
        );
        RetryPairFn::new(backoff, first_function, second_function)
    }
}
