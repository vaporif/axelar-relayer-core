//! Crate with supervisor and worker trait
use std::collections::HashMap;
use std::future::Future;
use std::panic;
use std::panic::AssertUnwindSafe;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use eyre::Context;

const SHUTDOWN_SIGNAL_CHECK_INTERVAL_MS: u64 = 500;
const WORKER_GRACEFUL_SHUTDOWN_MS: u64 = 2000;
const WORKER_CRASH_CHECK_MS: u64 = 2000;

type WorkerName = String;

// Fn that will create worker
pub type WorkerBuildFn =
    Box<dyn Fn() -> Pin<Box<dyn Future<Output = eyre::Result<Box<dyn Worker>>>>> + Send>;

// i.e. ingester/subscriber
pub trait Worker: Send {
    fn do_work<'s>(
        &'s mut self,
        shutdown: &'s AtomicBool,
    ) -> Pin<Box<dyn Future<Output = eyre::Result<()>> + 's>>;
}

#[tracing::instrument(name = "supervisor", skip_all)]
pub fn run(
    worker_builders: HashMap<WorkerName, WorkerBuildFn>,
    shutdown: &AtomicBool,
    tickrate: Duration,
) -> eyre::Result<()> {
    _ = std::thread::scope(|scope| {
        let (worker_crashed_tx, worker_crashed_rx) = std::sync::mpsc::channel();
        let mut active_workers = HashMap::new();

        let supervisor_handle = scope.spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .wrap_err("failed to create tokio runtime for supervisor")?;

            runtime.block_on(async move {
                for (worker_name, builder) in worker_builders.iter() {
                    let worker = match builder().await {
                        Ok(worker) => worker,
                        Err(err) => {
                            tracing::error!(%worker_name, ?err, "error during building worker");
                            let _ = worker_crashed_tx.send(worker_name.clone());
                            continue;
                        },
                    };

                    let worker_handle = spawn_worker(
                        worker_name.clone(),
                        worker,
                        worker_crashed_tx.clone(),
                        shutdown,
                        scope,
                        tickrate
                    );

                    active_workers.insert(worker_name.clone(), Some(worker_handle));
                }

                tracing::info!("started");

                let mut interval = tokio::time::interval(Duration::from_millis(SHUTDOWN_SIGNAL_CHECK_INTERVAL_MS));
                loop {
                    tokio::select! {
                        _ = interval.tick() => {
                            if shutdown.load(Ordering::Relaxed) {
                                tracing::debug!("supervisor detected shutdown signal");
                                tokio::time::sleep(Duration::from_millis(WORKER_GRACEFUL_SHUTDOWN_MS)).await;
                                break;
                            }
                        }
                        worker_result = async {
                            match worker_crashed_rx.try_recv() {
                                Ok(name) => Some(name),
                                Err(std::sync::mpsc::TryRecvError::Empty) => {
                                    tokio::time::sleep(Duration::from_millis(WORKER_CRASH_CHECK_MS)).await;
                                    None
                                }
                                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                                    tracing::debug!("worker channel disconnected");
                                    None
                                }
                            }
                        } => {
                            if let Some(worker_name) = worker_result {
                                if shutdown.load(Ordering::Relaxed) {
                                    tracing::debug!("worker crashed during shutdown, no restarting");
                                    continue;
                                }
                                tracing::info!(worker_name, "Restarting worker");
                                let builder = &worker_builders[&worker_name];

                                // TODO: rate limiter?
                                let worker = match builder().await {
                                    Ok(worker) => worker,
                                    Err(err) => {
                                        tracing::error!(%worker_name, ?err, "error during building worker");
                                        let _ = worker_crashed_tx.send(worker_name.clone());
                                        continue;
                                    },
                                };
                                let worker_handle = spawn_worker(
                                    worker_name.clone(),
                                    worker,
                                    worker_crashed_tx.clone(),
                                    shutdown,
                                    scope,
                                    tickrate
                                );

                                if let Some(Some(old_handle)) = active_workers.remove(&worker_name) {
                                    let _ = old_handle.join();
                                    tracing::debug!(worker_name, "Replaced existing worker handle");
                                }
                                active_workers.insert(worker_name.clone(), Some(worker_handle));
                            }
                        }
                    }
                }
            });

            eyre::Ok(())
        });

        let _ = supervisor_handle
            .join()
            .map_err(|err| eyre::eyre!(format!("Supervisor thread panicked: {err:?}")))?;

        tracing::info!("Supervisor shutting down");
        eyre::Ok(())
    });

    eyre::Ok(())
}

// TODO: add heartbeat to relayer
fn spawn_worker<'scope>(
    worker_name: WorkerName,
    mut worker: Box<dyn Worker>,
    worker_crashed_tx: std::sync::mpsc::Sender<String>,
    shutdown: &'scope AtomicBool,
    scope: &'scope std::thread::Scope<'scope, '_>,
    tickrate: Duration,
) -> std::thread::ScopedJoinHandle<'scope, ()> {
    tracing::debug!(worker_name, "starting worker");
    let worker_name = worker_name.clone();

    scope.spawn(move || {
        let runtime = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(rt) => rt,
            Err(err) => {
                tracing::error!(?err, worker_name, "failed to create tokio runtime");
                worker_crashed_tx.send(worker_name).expect("shutting down");
                return;
            }
        };

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            runtime.block_on(async {
                let mut interval = tokio::time::interval(tickrate);
                while !shutdown.load(std::sync::atomic::Ordering::Relaxed) {
                    interval.tick().await;
                    worker.do_work(shutdown).await?;
                }

                eyre::Ok(())
            })
        }));

        match result {
            Ok(Ok(_)) => {
                tracing::info!(worker_name, "worker exited, shutting down");
            }
            Ok(Err(err)) => {
                tracing::error!(worker_name, ?err, "worker error");
            }
            Err(panic_err) => {
                let panic_msg = if let Some(s) = panic_err.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_err.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    format!("Unknown panic: {:?}", panic_err)
                };
                tracing::error!(worker_name, %panic_msg, "worker panicked");
            }
        }

        if !shutdown.load(Ordering::Relaxed) {
            let _ = worker_crashed_tx.send(worker_name.clone());
            tracing::debug!(worker_name, "Reported worker crash for restart");
        }

        tracing::debug!("worker exit post crash report")
    })
}
