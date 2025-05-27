//! # Default values
//!
//! Default values used in configs

const MAX_BUNDLE_SIZE: usize = 100;

/// Default max bundle size
#[must_use]
pub const fn default_max_bundle_size() -> usize {
    MAX_BUNDLE_SIZE
}

const CONCURRENT_QUEUE_ITEMS: usize = 50;

/// Default concurrent items
#[must_use]
pub const fn default_concurrent_queue_items() -> usize {
    CONCURRENT_QUEUE_ITEMS
}

const WORKER_COUNT: usize = 50;

/// Default worker count
#[must_use]
pub const fn default_worker_count() -> usize {
    WORKER_COUNT
}

const MAX_ERRORS: u32 = 20;

/// Default max errors
#[must_use]
pub const fn default_max_errors() -> u32 {
    MAX_ERRORS
}

const LIMIT_ITEMS: u8 = 100;

/// Default limit per request
#[must_use]
pub const fn default_limit_per_request() -> u8 {
    LIMIT_ITEMS
}
