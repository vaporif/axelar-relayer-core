use std::fmt::Display;
use std::num::NonZeroU64;
use std::time::Duration;

use tokio_retry::strategy::ExponentialBackoff;

use crate::backoff_pair_iterator::BackoffPairIterator;
use crate::retry_fn::RetryFn;
use crate::retry_pair_fn::RetryPairFn;
use crate::{Abortable, BackoffParrams};

#[derive(Debug, Clone)]
pub struct BackoffRetryBuilder {
    backoff: BackoffParrams,
}

impl BackoffRetryBuilder {
    pub fn new(initial_delay: Duration, backoff_factor: NonZeroU64, max_delay: Duration) -> Self {
        let backoff = BackoffParrams {
            initial_delay,
            backoff_factor,
            max_delay,
        };

        BackoffRetryBuilder { backoff }
    }

    #[must_use]
    pub fn with_retry<Fn, T, Err>(&self, function: Fn) -> RetryFn<Fn, T, Err>
    where
        Fn: AsyncFn() -> Result<T, Err>,
        Err: Display + Abortable,
    {
        let backoff =
            ExponentialBackoff::from_millis(self.backoff.initial_delay.as_millis() as u64)
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
