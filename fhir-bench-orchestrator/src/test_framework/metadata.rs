//! Contains the code to run `/metadata` server operations.

use super::{
    ServerOperationIterationFailed, ServerOperationIterationStarting,
    ServerOperationIterationState, ServerOperationIterationSucceeded, ServerOperationMetrics,
};
use crate::servers::ServerHandle;
use crate::test_framework::{ServerOperationLog, ServerOperationMeasurement};
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use async_std::future::timeout;
use async_std::net::TcpStream;
use chrono::prelude::*;
use futures::prelude::*;
use hdrhistogram::Histogram;
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
/// * `operation_state`: the initial state machine for this operation iteration
/// * `url`: the full [Url] to the endpoint to test
///
/// Returns the final [ServerOperationIterationState] containing information about the operation's
/// success or failure.
async fn run_operation_metadata(
    app_state: &AppState,
    operation_state: ServerOperationIterationState<ServerOperationIterationStarting>,
    url: Url,
) -> std::result::Result<
    ServerOperationIterationState<ServerOperationIterationSucceeded>,
    ServerOperationIterationState<ServerOperationIterationFailed>,
> {
    trace!(app_state.logger, "GET '{}': starting...", url);
    let stream = match TcpStream::connect(&*url.socket_addrs(|| None).unwrap()).await {
        Ok(stream) => stream,
        Err(err) => {
            return Err(operation_state
                .completed()
                .failed(anyhow!(format!("{}", err))));
        }
    };
    let request = Request::new(Method::Get, url.clone());

    let response = async_h1::connect(stream.clone(), request).await;
    let operation_state = operation_state.completed();
    trace!(app_state.logger, "GET '{}': complete.", url);

    match response {
        Ok(mut response) => {
            let response_status = response.status();
            if !response_status.is_success() {
                let response_body = match response.body_string().await {
                    Ok(response_body) => response_body,
                    Err(err) => format!("Unable to retrieve response body due to error: '{}'", err),
                };

                let error = anyhow!(
                    "The GET /metadata to '{}' failed, with status '{}' and body: '{}'",
                    &url,
                    response_status,
                    response_body
                );
                let state = operation_state.failed(error);
                return Err(state);
            }

            // TODO more checks needed
            Ok(operation_state.succeeded())
        }
        Err(err) => Err(operation_state.failed(anyhow!(format!("{}", err)))),
    }
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
        let operation_state = ServerOperationIterationState::new();
        let operation = run_operation_metadata(app_state, operation_state.clone(), url.clone());
        let operation = timeout(
            app_state
                .config
                .operation_timeout
                .to_std()
                .expect("unable to convert Duration"),
            operation,
        );

        // Having the timeout gives us a wrapped Result<Result ...>>. Un-nest them.
        let result = operation.await;
        let result = result.map_err(|err| {
            operation_state
                .completed()
                .failed(anyhow!("Operation timed out: '{}'", err))
        });
        result.and_then(|wrapped_result| wrapped_result)
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
    let mut histogram = Histogram::<u64>::new(3).expect("Unable to construct histogram.");
    info!(
        app_state.logger,
        "Benchmarking GET /metadata: '{}' concurrent users: starting...", concurrent_users
    );
    let started = Utc::now();
    let mut iterations_failed: u32 = 0;
    while let Some(operation_result) = operations.next().await {
        match operation_result {
            Ok(operation_success) => {
                let duration = operation_success.duration();
                let duration_millis = duration.num_milliseconds();
                histogram
                    .record(duration_millis as u64)
                    .expect("Histogram recording failed.");
            }
            Err(err) => {
                warn!(
                    app_state.logger,
                    "Operation '{}' failed: '{:?}", SERVER_OP_NAME_METADATA, err
                );
                iterations_failed += 1;
            }
        }
    }
    let completed = Utc::now();
    info!(
        app_state.logger,
        "Benchmarking GET /metadata: '{}' concurrent users: completed.", concurrent_users
    );

    let iterations_succeeded = app_state.config.iterations - iterations_failed;
    let execution_duration = completed - started;
    ServerOperationMeasurement {
        concurrent_users,
        started,
        completed,
        execution_duration,
        iterations_failed,
        iterations_skipped: 0,
        metrics: ServerOperationMetrics::new(execution_duration, iterations_succeeded, histogram),
    }
}
