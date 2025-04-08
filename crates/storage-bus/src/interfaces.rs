#[cfg(feature = "consumer-interfaces")]
pub mod consumer {
    use std::error::Error;
    use std::fmt::Debug;
    use std::future::Future;
    use std::time::Duration;

    pub trait QueueMessage<T>: Debug
    where
        T: Debug,
    {
        fn decoded(&self) -> &T;
        fn ack(
            &self,
            ack_kind: AckKind,
        ) -> impl Future<Output = Result<(), impl Error + Send + Sync + 'static>>;
    }

    pub trait Consumer<T: Debug> {
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

    #[derive(Debug)]
    pub enum AckKind {
        /// Acknowledges a message was completely handled.
        Ack,
        /// Signals that the message will not be processed now
        /// and processing can move onto the next message, NAK'd
        /// message will be retried.
        Nak(Option<Duration>),
        /// When sent before the AckWait period indicates that
        /// work is ongoing and the period should be extended by
        /// another equal to AckWait.
        Progress,
        /// Acknowledges the message was handled and requests
        /// delivery of the next message to the reply subject.
        /// Only applies to Pull-mode.
        Next,
        /// Instructs the server to stop redelivery of a message
        /// without acknowledging it as successfully processed.
        Term,
    }
}

#[cfg(feature = "publisher-interfaces")]
pub mod publisher {
    use std::error::Error;
    use std::future::{Future, IntoFuture};

    pub trait PeekMessage<T> {
        fn peek_last(
            &mut self,
        ) -> impl Future<Output = Result<Option<T>, impl Error + Send + Sync + 'static>>;
    }

    pub trait Publisher<T: Send + Sync> {
        type AckFuture: IntoFuture;
        fn publish(
            &self,
            deduplication_id: impl Into<String>,
            data: T,
        ) -> impl Future<Output = Result<Self::AckFuture, impl Error + Send + Sync + 'static>>;
    }
}

#[cfg(feature = "storage-interfaces")]
pub mod kv_store {
    use std::error::Error;
    use std::fmt::Debug;

    #[derive(Debug)]
    pub struct WithRevision<T> {
        pub value: T,
        pub revision: u64,
    }

    pub trait KvStore<T> {
        fn update(
            &self,
            data: WithRevision<T>,
        ) -> impl Future<Output = Result<WithRevision<T>, impl Error + Send + Sync + 'static>>;

        fn put(
            &self,
            value: T,
        ) -> impl Future<Output = Result<WithRevision<T>, impl Error + Send + Sync + 'static>>;

        fn get(
            &self,
        ) -> impl Future<Output = Result<Option<WithRevision<T>>, impl Error + Send + Sync + 'static>>;
    }
}
