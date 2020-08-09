//! Contains the code to run `/metadata` server operations.

use crate::servers::ServerHandle;
use crate::test_framework::{ServerOperationLog, ServerOperationMeasurement};
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use async_std::future::timeout;
use async_std::net::TcpStream;
use chrono::prelude::*;
use futures::prelude::*;
use http_types::{Method, Request, Url};
use slog::{info, trace, warn};
use std::convert::TryFrom;

static SERVER_OP_NAME_METADATA: &str = "metadata";

/// Verifies and benchmarks the FHIR `/metadata` operations.
pub async fn benchmark_operation_metadata(
    app_state: &AppState,
    server_handle: &dyn ServerHandle,
) -> ServerOperationLog {
    let mut server_op_log = ServerOperationLog::new(SERVER_OP_NAME_METADATA.into());
    for concurrent_users in app_state.config.concurrency_levels.clone() {
        let measurement =
            benchmark_operation_metadata_for_users(app_state, server_handle, concurrent_users)
                .await;
        server_op_log.measurements.push(measurement);
    }

    server_op_log
}

/// Creates the URL to access a server's `/metadata` endpoint.
pub fn create_metadata_url(server_handle: &dyn ServerHandle) -> Url {
    server_handle
        .base_url()
        .join("metadata")
        .expect("Error parsing URL.")
}

/// Runs a single iteration of the `/metadata` operation and verifies its result, logging out any faults that
/// were found (in addition to returning the error).
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `url`: the full [Url] to the endpoint to test
///
/// Returns an empty [Result], indicating whether or not the operation succeeded or failed.
async fn run_operation_metadata(app_state: &AppState, url: Url) -> Result<()> {
    trace!(app_state.logger, "GET '{}': starting...", url);
    let stream = TcpStream::connect(&*url.socket_addrs(|| None).unwrap()).await?;
    let request = Request::new(Method::Get, url.clone());

    let mut response = async_h1::connect(stream.clone(), request)
        .await
        .or_else(|err| Err(anyhow!(format!("{}", err))))?;
    trace!(app_state.logger, "GET '{}': complete.", url);

    let response_status = response.status();
    if !response_status.is_success() {
        let response_body = response
            .body_string()
            .await
            .or_else(|err| Err(anyhow!(format!("{}", err))))?;
        return Err(anyhow!(
            "The GET /metadata to '{}' failed, with status '{}' and body: '{}'",
            &url,
            response_status,
            response_body
        ));
    }

    // TODO more checks needed
    Ok(())
}

/// Runs a single iteration of the `/metadata` operation and verifies its result, logging out any faults that
/// were found (in addition to returning the error).
///
/// Note: Unlike `run_operation_metadata(...)`, this method uses the `reqwest library and won't panic if the
/// server isn't ready yet.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `url`: the full [Url] to the endpoint to test
///
/// Returns an empty [Result], indicating whether or not the operation succeeded or failed.
pub async fn run_operation_metadata_safe(app_state: &AppState, url: Url) -> Result<()> {
    let response = reqwest::blocking::get(url.clone())
        .with_context(|| format!("request for '{}' failed", url))?;

    if !response.status().is_success() {
        warn!(app_state.logger, "request failed"; "url" => url.as_str(), "status" => response.status().as_str());
        return Err(anyhow!(
            "request for '{}' failed with status '{}'",
            url,
            response.status()
        ));
    }
    // TODO more checks needed
    Ok(())
}

/// Verifies and benchmarks the FHIR `/metadata` operations for the specified number of concurrent users.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the [ServerHandle] for the server implementation instance being tested
/// * `concurrent_users`: the number of users to try and test with concurrently
///
/// Returns a [ServerOperationMeasurement] with the results.
async fn benchmark_operation_metadata_for_users(
    app_state: &AppState,
    server_handle: &dyn ServerHandle,
    concurrent_users: u32,
) -> ServerOperationMeasurement {
    let url = create_metadata_url(server_handle);

    /*
     * Build an iterator: One element for each iteration to run, run the operation for each iteration, and
     * count the iterations that failed.
     */
    let operations = (0..app_state.config.iterations).map(|_| async {
        let operation = run_operation_metadata(app_state, url.clone());
        let operation = timeout(
            app_state
                .config
                .operation_timeout
                .to_std()
                .expect("unable to convert Duration"),
            operation,
        );
        match operation.await {
            Ok(_) => 0u32,
            Err(err) => {
                warn!(app_state.logger, "GET /metadata failed"; "error" => format!("{:?}", err));
                1u32
            }
        }
    });

    /*
     * Convert that iterator to a parallel stream, and use use `buffer_unordered(...)` to set it to run it only up to
     * `concurrent_users`, at once.
     */
    let mut operations = futures::stream::iter(operations)
        .buffer_unordered(usize::try_from(concurrent_users).unwrap());

    /*
     * Kick off the execution of the stream, summing up all of the failures that are encountered.
     */
    info!(
        app_state.logger,
        "Benchmarking GET /metadata: '{}' concurrent users: starting...", concurrent_users
    );
    let started = Utc::now();
    let mut iterations_failed: u32 = 0;
    while let Some(n) = operations.next().await {
        iterations_failed += n;
    }
    let completed = Utc::now();
    info!(
        app_state.logger,
        "Benchmarking GET /metadata: '{}' concurrent users: completed.", concurrent_users
    );

    ServerOperationMeasurement {
        concurrent_users,
        started,
        completed,
        execution_duration: completed - started,
        iterations_failed,
        iterations_skipped: 0,
        // TODO need to implement metrics
        metrics: None,
    }
}
