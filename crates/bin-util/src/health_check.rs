//! # Health Check
//!
//! A lightweight HTTP server for implementing health and readiness probes.
//!
//! This crate provides functionality to create an HTTP server that responds to
//! health check requests on `/healthz` and readiness check requests on `/readyz`.
//! It allows registration of custom health check functions that determine the
//! server's health status.
//!
//! ## Example
//!
//! ```rust
//! use health_check::HealthCheckServer;
//! use tokio_util::sync::CancellationToken;
//! use eyre::Result;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let cancel_token = CancellationToken::new();
//!     let token_clone = cancel_token.clone();
//!
//!     let server = HealthCheckServer::new(8080)
//!         .add_health_check(|| async { Ok(()) });
//!
//!     // Run the server in a separate tokio task
//!     tokio::spawn(async move {
//!         server.run(token_clone).await;
//!     });
//!
//!     // Your application logic here
//!     // When ready to shut down:
//!     cancel_token.cancel();
//!
//!     Ok(())
//! }
//! ```
use axum::{
    Router,
    extract::Extension,
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::get,
};
use eyre::Result;
use serde_json::json;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// A type alias representing a health check function.
///
/// This type encapsulates an async function that returns a `Result<()>`,
/// where `Ok(())` indicates a successful health check and `Err(_)` indicates a failure.
pub type HealthCheck =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = Result<()>> + Send>> + Send + Sync>;

/// A server that handles health check and readiness probe requests.
///
/// The server responds to HTTP requests on two endpoints:
/// - `/healthz`: For health checks (liveness probes)
/// - `/readyz`: For readiness probes
///
/// Both endpoints return 200 OK when all registered health checks pass,
/// or 503 Service Unavailable when any check fails.
pub struct HealthCheckServer {
    port: u16,
    health_checks: Vec<HealthCheck>,
}

/// Creates a new `HealthCheckServer` bound to the specified port.
///
/// # Arguments
///
/// * `port` - The TCP port on which the server will listen for HTTP requests
///
/// # Returns
///
/// A new `HealthCheckServer` instance with no health checks registered.
pub fn new(port: u16) -> HealthCheckServer {
    HealthCheckServer {
        port,
        health_checks: Vec::new(),
    }
}

impl HealthCheckServer {
    /// Creates a new `HealthCheckServer` bound to the specified port.
    ///
    /// # Arguments
    ///
    /// * `port` - The TCP port on which the server will listen for HTTP requests
    ///
    /// # Returns
    ///
    /// A new `HealthCheckServer` instance with no health checks registered.
    pub fn new(port: u16) -> Self {
        Self {
            port,
            health_checks: Vec::new(),
        }
    }

    /// Registers a health check function with the server.
    ///
    /// Multiple health checks can be registered by chaining this method.
    /// All health checks are executed in parallel when a health check request is received.
    ///
    /// # Arguments
    ///
    /// * `f` - A function that returns a future resolving to a `Result<()>`.
    ///         An `Ok(())` result indicates the health check passed, while
    ///         an `Err(_)` indicates it failed.
    ///
    /// # Returns
    ///
    /// `Self` with the health check added, allowing for method chaining.
    pub fn add_health_check<F, Fut>(mut self, f: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        self.health_checks.push(Box::new(move || Box::pin(f())));
        self
    }

    /// Starts the HTTP server and runs until the cancellation token is triggered.
    ///
    /// The server will bind to `0.0.0.0` on the configured port and respond to
    /// health check and readiness requests. When the cancellation token is triggered,
    /// the server will perform a graceful shutdown.
    ///
    /// # Arguments
    ///
    /// * `cancel_token` - A cancellation token that will trigger server shutdown when canceled
    pub async fn run(self, cancel_token: CancellationToken) {
        let health_checks = Arc::new(self.health_checks);
        let state = health_checks.clone();

        let app = Router::new()
            .route("/healthz", get(handle_healthz))
            .route("/readyz", get(handle_readyz))
            .layer(Extension(state));

        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", self.port))
            .await
            .expect("Failed to bind to address");

        axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                cancel_token.cancelled().await;
            })
            .await
            .expect("Server error");
    }
}

async fn handle_healthz(
    Extension(health_checks): Extension<Arc<Vec<HealthCheck>>>,
) -> impl IntoResponse {
    check_and_respond(
        &health_checks,
        StatusCode::OK,
        StatusCode::SERVICE_UNAVAILABLE,
        "HEALTHY",
        "UNHEALTHY",
    )
    .await
}

async fn handle_readyz(
    Extension(health_checks): Extension<Arc<Vec<HealthCheck>>>,
) -> impl IntoResponse {
    check_and_respond(
        &health_checks,
        StatusCode::OK,
        StatusCode::SERVICE_UNAVAILABLE,
        "READY",
        "UNREADY",
    )
    .await
}

async fn check_and_respond(
    health_checks: &Vec<HealthCheck>,
    ok_status: StatusCode,
    err_status: StatusCode,
    ok_str: &str,
    err_str: &str,
) -> (StatusCode, Json<serde_json::Value>) {
    if health_checks.is_empty() {
        return (ok_status, Json(json!({ "status": ok_str })));
    }

    // Run all health checks in parallel
    let results = futures::future::join_all(health_checks.iter().map(|check| check())).await;

    let has_error = results.iter().any(|r| r.is_err());
    if has_error {
        tracing::debug!("Health check failed");
        (err_status, Json(json!({ "status": err_str })))
    } else {
        tracing::debug!("Health check succeeded");
        (ok_status, Json(json!({ "status": ok_str })))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicBool, Ordering},
        time::Duration,
    };
    use tokio::time::sleep;

    async fn run_server<F, Fut>(port: u16, health_check: F) -> CancellationToken
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let cancel_token = CancellationToken::new();
        let token_clone = cancel_token.clone();

        let server = HealthCheckServer::new(port).add_health_check(health_check);
        tokio::spawn(async move {
            server.run(token_clone).await;
        });

        // Give the server time to start
        sleep(Duration::from_millis(100)).await;

        cancel_token
    }

    fn get_free_port() -> u16 {
        std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }

    #[tokio::test]
    async fn test_health_check_success() {
        let port = get_free_port();
        run_server(port, || async { Ok(()) }).await;

        let url = format!("http://127.0.0.1:{}/healthz", port);
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.text().await.unwrap(), r#"{"status":"HEALTHY"}"#);
    }

    #[tokio::test]
    async fn test_health_check_failure() {
        let port = get_free_port();
        run_server(port, || async { Err(eyre::eyre!("Health check failed")) }).await;

        let url = format!("http://127.0.0.1:{}/healthz", port);
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 503);
        assert_eq!(resp.text().await.unwrap(), r#"{"status":"UNHEALTHY"}"#);
    }

    #[tokio::test]
    async fn test_multiple_health_checks() {
        let port = get_free_port();
        let flag = Arc::new(AtomicBool::new(false));
        let flag2 = flag.clone();
        let cancel_token = CancellationToken::new();
        let token_clone = cancel_token.clone();

        let server = HealthCheckServer::new(port)
            .add_health_check(|| async { Ok(()) })
            .add_health_check(move || {
                let ok = !flag2.load(Ordering::SeqCst);
                async move {
                    if ok {
                        Ok(())
                    } else {
                        Err(eyre::eyre!("Intentional failure"))
                    }
                }
            });

        tokio::spawn(async move {
            server.run(token_clone).await;
        });

        sleep(Duration::from_millis(100)).await;

        let url = format!("http://127.0.0.1:{}/healthz", port);
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.text().await.unwrap(), r#"{"status":"HEALTHY"}"#);

        flag.store(true, Ordering::SeqCst);
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 503);
        assert_eq!(resp.text().await.unwrap(), r#"{"status":"UNHEALTHY"}"#);
    }

    #[tokio::test]
    async fn test_readyz_endpoint() {
        let port = get_free_port();
        let cancel_token = run_server(port, || async { Ok(()) }).await;

        let url = format!("http://127.0.0.1:{}/readyz", port);
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);
        assert_eq!(resp.text().await.unwrap(), r#"{"status":"READY"}"#);

        cancel_token.cancel();

        let port2 = get_free_port();
        let cancel_token2 =
            run_server(port2, || async { Err(eyre::eyre!("Readiness failed")) }).await;

        let url2 = format!("http://127.0.0.1:{}/readyz", port2);
        let resp2 = reqwest::get(&url2).await.unwrap();
        assert_eq!(resp2.status(), 503);
        assert_eq!(resp2.text().await.unwrap(), r#"{"status":"UNREADY"}"#);

        cancel_token2.cancel();
    }

    #[tokio::test]
    async fn test_not_found() {
        let port = get_free_port();
        let cancel_token = run_server(port, || async { Ok(()) }).await;

        let url = format!("http://127.0.0.1:{}/notfound", port);
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 404);

        cancel_token.cancel();
    }

    #[tokio::test]
    async fn test_cancellation() {
        let port = get_free_port();
        let cancel_token = run_server(port, || async { Ok(()) }).await;

        // Verify server is running
        let url = format!("http://127.0.0.1:{}/healthz", port);
        let resp = reqwest::get(&url).await.unwrap();
        assert_eq!(resp.status(), 200);

        // Cancel the server
        cancel_token.cancel();
        sleep(Duration::from_millis(100)).await;

        // After cancellation, the server should no longer respond
        let result = reqwest::get(&url).await;
        assert!(
            result.is_err(),
            "Server should no longer respond after cancellation"
        );
    }
}
