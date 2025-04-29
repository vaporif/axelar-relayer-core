/// Consumer interfaces
#[cfg(feature = "consumer-interfaces")]
pub mod consumer {
    use core::error::Error;
    use core::fmt::Debug;

    /// queue message trait
    pub trait QueueMessage<T>: Debug
    where
        T: Debug,
    {
        /// Decoded message
        fn decoded(&self) -> &T;
        /// Ack response
        fn ack(
            &self,
            ack_kind: AckKind,
        ) -> impl Future<Output = Result<(), impl Error + Send + Sync + 'static>>;
    }

    /// consumer
    pub trait Consumer<T: Debug> {
        /// messages stream
        fn messages(
            &self,
        ) -> impl Future<
            Output = Result<
                impl futures::Stream<
                    Item = Result<impl QueueMessage<T>, impl Error + Send + Sync + 'static>,
                >,
                impl Error + Send + Sync + 'static,
            >,
        >;
    }

    /// Ack responses
    #[derive(Debug)]
    pub enum AckKind {
        /// Acknowledges a message was completely handled.
        Ack,
        /// Signals that the message will not be processed now
        /// and processing can move onto the next message, NAK'd
        /// message will be retried.
        Nak,
        /// When sent before the `AckWait` period indicates that
        /// work is ongoing and the period should be extended by
        /// another equal to `AckWait`.
        Progress,
    }
}

/// Publish interfaces
#[cfg(feature = "publisher-interfaces")]
pub mod publisher {
    use core::error::Error;

    // TODO: peek should return not the full message but just message id
    /// peekable
    pub trait PeekMessage<T> {
        /// Get last message from queue without consuming
        fn peek_last(
            &mut self,
        ) -> impl Future<Output = Result<Option<T>, impl Error + Send + Sync + 'static>>;
    }

    /// publisher
    #[allow(clippy::impl_trait_in_params, reason = "improves readability")]
    pub trait Publisher<T: Send + Sync> {
        /// ack future type
        type AckFuture: IntoFuture;
        /// Publish message to queue
        fn publish(
            &self,
            deduplication_id: impl Into<String>,
            data: &T,
        ) -> impl Future<Output = Result<Self::AckFuture, impl Error + Send + Sync + 'static>>;
    }
}

#[cfg(feature = "storage-interfaces")]
pub mod kv_store {
    use std::error::Error;
    use std::fmt::Debug;

    /// Value with revision
    #[derive(Debug)]
    pub struct WithRevision<T> {
        pub value: T,
        pub revision: u64,
    }

    pub trait KvStore<T> {
        /// Update value in kvstore
        fn update(
            &self,
            data: &WithRevision<T>,
        ) -> impl Future<Output = Result<u64, impl Error + Send + Sync + 'static>>;

        /// Create value in kvstore
        fn put(
            &self,
            value: &T,
        ) -> impl Future<Output = Result<u64, impl Error + Send + Sync + 'static>>;

        /// Get value from kvstore
        fn get(
            &self,
        ) -> impl Future<Output = Result<Option<WithRevision<T>>, impl Error + Send + Sync + 'static>>;
    }
}
