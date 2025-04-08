use std::fmt::Display;
use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{Abortable, RetryError};
use crate::backoff_pair_iterator::{AlternationStep, BackoffPairIterator};

pub struct RetryPairFn<Fn1, Fn2, T, Err> {
    backoff: BackoffPairIterator,
    first_function: Fn1,
    second_function: Fn2,
    max_attempts: usize,
    _phantom: PhantomData<(T, Err)>,
}

impl<Fn1, Fn2, T, Err> RetryPairFn<Fn1, Fn2, T, Err>
where
    Fn1: AsyncFn() -> Result<T, Err>,
    Fn2: AsyncFn() -> Result<T, Err>,
    Err: Display + Abortable,
{
    pub(crate) fn new(
        backoff: BackoffPairIterator,
        first_function: Fn1,
        second_function: Fn2,
    ) -> Self {
        Self {
            backoff,
            first_function,
            second_function,
            // TODO: config
            max_attempts: 20,
            _phantom: PhantomData,
        }
    }

    // you can use a macro to reduce copy-paste
    // but this will reduce readability
    #[tracing::instrument(skip_all)]
    pub async fn retry(self, shutdown: &AtomicBool) -> Result<T, RetryError<Err>> {
        if shutdown.load(Ordering::Relaxed) {
            return Err(RetryError::Shutdown);
        }

        let mut primary_fn_aborted = false;
        let mut secondary_fn_aborted = false;
        match (self.first_function)().await {
            Ok(res) => return Ok(res),
            Err(err) => {
                if err.abortable() {
                    tracing::error!(%err, "primary lambda aborted");
                    primary_fn_aborted = true;
                } else {
                    tracing::error!(%err, "primary lambda failed, will retry");
                }
            }
        }

        if shutdown.load(Ordering::Relaxed) {
            return Err(RetryError::Shutdown);
        }

        match (self.second_function)().await {
            Ok(res) => return Ok(res),
            Err(err) => {
                if err.abortable() {
                    tracing::error!(%err, "secondary lambda aborted");
                    secondary_fn_aborted = true;
                } else {
                    tracing::error!(%err, "secondary lambda failed, will retry");
                }
            }
        }

        for (retry_count, iteration) in self.backoff.enumerate().take(self.max_attempts) {
            if shutdown.load(Ordering::Relaxed) {
                return Err(RetryError::Shutdown);
            }
            match iteration.alteration_step {
                AlternationStep::First => {
                    if primary_fn_aborted {
                        continue;
                    }

                    tokio::time::sleep(iteration.duration).await;
                    match (self.first_function)().await {
                        Ok(res) => return Ok(res),
                        Err(err) => {
                            if !err.abortable() {
                                tracing::error!(%err, retry_count, "retry attempt with primary lambda failed");
                                continue;
                            }

                            primary_fn_aborted = true;
                            tracing::error!(%err, retry_count, "primary lambda aborted");

                            if secondary_fn_aborted {
                                tracing::error!("both lambdas aborted");
                                return Err(RetryError::Aborted(err));
                            } else {
                                continue;
                            }
                        }
                    }
                }
                AlternationStep::Second => {
                    if secondary_fn_aborted {
                        continue;
                    }
                    tokio::time::sleep(iteration.duration).await;
                    match (self.second_function)().await {
                        Ok(res) => return Ok(res),
                        Err(err) => {
                            if !err.abortable() {
                                tracing::error!(%err, retry_count, "retry attempt with secondary lambda failed");
                                continue;
                            }

                            secondary_fn_aborted = true;
                            tracing::error!(%err, retry_count, "secondary lambda aborted");

                            if primary_fn_aborted {
                                tracing::error!("both lambdas aborted");
                                return Err(RetryError::Aborted(err));
                            } else {
                                continue;
                            }
                        }
                    }
                }
            }
        }

        return Err(RetryError::MaxAttempts);
    }
}
