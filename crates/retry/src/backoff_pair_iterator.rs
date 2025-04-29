use core::num::NonZeroU64;
use core::time::Duration;

use rand::Rng as _;

/// A retry iterator driven by exponential back-off.
/// Retry returns alternating path of execution
/// i.e. you try two routes of execution with
/// independent backoff applied
/// Think of primary rpc endpoint and secondary as backup
/// The power corresponds to the number of past attempts.
/// Based on tokio retry + 2fn
#[derive(Debug, Clone)]
pub(crate) struct BackoffPairIterator {
    current_first_path_delay_ms: u64,
    current_second_path_delay_ms: u64,
    factor: NonZeroU64,
    max_delay_ms: u64,
    backoff_path: AlternationStep,
}

#[derive(Debug, Clone)]
pub(crate) enum AlternationStep {
    First,
    Second,
}

pub(crate) struct Iteration {
    pub duration: Duration,
    pub alteration_step: AlternationStep,
}

impl BackoffPairIterator {
    pub(crate) fn new(initial_delay: Duration, factor: NonZeroU64, max_delay: Duration) -> Self {
        let initial_delay_ms = u128_to_u64_saturating(initial_delay.as_millis());

        let max_delay_ms = u128_to_u64_saturating(max_delay.as_millis());

        Self {
            current_first_path_delay_ms: initial_delay_ms,
            current_second_path_delay_ms: 0,
            factor,
            max_delay_ms,
            backoff_path: AlternationStep::First,
        }
    }
}

impl Iterator for BackoffPairIterator {
    type Item = Iteration;

    fn next(&mut self) -> Option<Self::Item> {
        let current_path = self.backoff_path.clone();

        // flip path for next execution
        let mut duration_ms = match self.backoff_path {
            AlternationStep::First => {
                let duration_ms = self.current_first_path_delay_ms;
                self.backoff_path = AlternationStep::Second;

                self.current_first_path_delay_ms = jitter(
                    self.current_first_path_delay_ms
                        .saturating_mul(self.factor.into()),
                );

                duration_ms
            }
            AlternationStep::Second => {
                let duration_ms = self.current_second_path_delay_ms;
                self.backoff_path = AlternationStep::First;

                self.current_second_path_delay_ms = jitter(
                    self.current_first_path_delay_ms
                        .saturating_mul(self.factor.into()),
                );

                duration_ms
            }
        };

        if duration_ms > self.max_delay_ms {
            duration_ms = self.max_delay_ms;
        }

        Some(Iteration {
            duration: Duration::from_millis(duration_ms),
            alteration_step: current_path,
        })
    }
}

#[allow(clippy::cast_possible_truncation, reason = "truncate is ok")]
#[allow(clippy::float_arithmetic, reason = "Necessary for jitter calculation")]
#[allow(
    clippy::cast_precision_loss,
    reason = "Precision loss is acceptable for jitter"
)]
#[allow(clippy::as_conversions, reason = "Conversion is checked for safety")]
#[allow(clippy::cast_sign_loss, reason = "We check the sign before casting")]
fn jitter(duration_ms: u64) -> u64 {
    let jitter_factor: f64 = rand::rng().random_range(0.5_f64..1.5_f64);
    let jittered: f64 = duration_ms as f64 * jitter_factor;
    if jittered.is_finite() && jittered.is_sign_positive() && jittered < f64::MAX / 2.0 {
        jittered as u64
    } else {
        duration_ms
    }
}

fn u128_to_u64_saturating(value: u128) -> u64 {
    if value > u128::from(u64::MAX) {
        u64::MAX
    } else {
        u64::try_from(value).unwrap_or(u64::MAX)
    }
}
