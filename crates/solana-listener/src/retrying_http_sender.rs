use core::time::Duration;
use std::sync::Arc;

use async_trait::async_trait;
use backoff::future::retry;
use backoff::ExponentialBackoffBuilder;
use serde_json::Value;
use solana_client::client_error::{ClientError, ClientErrorKind};
use solana_client::rpc_sender::RpcTransportStats;
use solana_rpc_client::http_sender::HttpSender;
use solana_rpc_client::rpc_sender::RpcSender;
use solana_rpc_client_api::client_error::Result as ClientResult;
use solana_rpc_client_api::request::RpcRequest;
use tokio::sync::Semaphore;
use tracing::error;

/// The maximum elapsed time for retrying failed requests.
const TWO_MINUTES: Duration = Duration::from_millis(2 * 60 * 1_000);

/// A wrapper around `HttpSender` that adds retry logic for sending RPC
/// requests.
pub(crate) struct RetryingHttpSender {
    http_client: HttpSender,
    request_permit: Arc<Semaphore>,
}

impl RetryingHttpSender {
    pub(crate) fn new(url: String, max_concurrent_requests: usize) -> Self {
        let http = HttpSender::new(url);
        let request_permit = Arc::new(Semaphore::new(max_concurrent_requests));
        Self {
            http_client: http,
            request_permit,
        }
    }

    async fn send_internal(
        &self,
        request: RpcRequest,
        params: &Value,
    ) -> Result<Value, backoff::Error<ClientError>> {
        use ClientErrorKind::{
            Custom, Io, Middleware, Reqwest, RpcError, SerdeJson, SigningError, TransactionError,
        };
        // get the permit to make the request
        let _permit = Arc::clone(&self.request_permit)
            .acquire_owned()
            .await
            .expect("the semaphore will never be closed");

        // make the actual request
        self.http_client
            .send(request, params.clone())
            .await
            .inspect_err(|error| error!(%error))
            .map_err(|error| match *error.kind() {
                // Retry on networking-io related errors
                Io(_) | Reqwest(_) => backoff::Error::transient(error),
                // Fail instantly on other errors
                SerdeJson(_) | RpcError(_) | SigningError(_) | TransactionError(_) | Custom(_) => {
                    backoff::Error::permanent(error)
                }
                Middleware(_) => backoff::Error::permanent(error),
            })
    }
}

#[async_trait]
impl RpcSender for RetryingHttpSender {
    #[tracing::instrument(skip(self), name = "retrying_http_sender")]
    async fn send(&self, request: RpcRequest, params: Value) -> ClientResult<Value> {
        let strategy = ExponentialBackoffBuilder::new()
            .with_max_elapsed_time(Some(TWO_MINUTES))
            .build();
        let operation = || self.send_internal(request, &params);
        retry(strategy, operation).await
    }

    fn get_transport_stats(&self) -> RpcTransportStats {
        self.http_client.get_transport_stats()
    }

    fn url(&self) -> String {
        self.http_client.url()
    }
}
