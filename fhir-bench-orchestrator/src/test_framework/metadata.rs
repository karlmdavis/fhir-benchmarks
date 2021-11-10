//! Contains the code to run `/metadata` server operations.

use super::{
    ServerOperationIterationFailed, ServerOperationIterationStarting,
    ServerOperationIterationState, ServerOperationIterationSucceeded, ServerOperationMetrics,
};
use crate::servers::ServerHandle;
use crate::test_framework::{ServerOperationLog, ServerOperationMeasurement};
use crate::AppState;
use chrono::prelude::*;
use eyre::{eyre, Result};
use futures::prelude::*;
use hdrhistogram::Histogram;
use std::convert::TryFrom;
use tracing::{trace_span, warn, Instrument};
use url::Url;

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
/// * `server_handle`: the [ServerHandle] for the server to test
/// * `operation_state`: the initial state machine for this operation iteration
///
/// Returns the final [ServerOperationIterationState] containing information about the operation's
/// success or failure.
async fn run_operation_metadata(
    server_handle: &dyn ServerHandle,
    operation_state: ServerOperationIterationState<ServerOperationIterationStarting>,
) -> std::result::Result<
    ServerOperationIterationState<ServerOperationIterationSucceeded>,
    ServerOperationIterationState<ServerOperationIterationFailed>,
> {
    let url = create_metadata_url(server_handle);

    let client = match server_handle.client() {
        Ok(client) => client,
        Err(err) => {
            return Err(operation_state
                .completed()
                .failed(eyre!(format!("Unable to create client: '{}'", err))));
        }
    };

    let request_builder = server_handle.request_builder(client, http::Method::GET, url.clone());
    let response = request_builder
        .send()
        .instrument(trace_span!("GET request", %url))
        .await;

    let operation_state = operation_state.completed();

    match response {
        Ok(response) => {
            let response_status = response.status();

            // Always pull response body, to drain stream and release connection.
            let response_body = match response.text().await {
                Ok(response_body) => response_body,
                Err(err) => format!("Unable to retrieve response body due to error: '{}'", err),
            };

            if !response_status.is_success() {
                let error = eyre!(
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
        Err(err) => Err(operation_state.failed(eyre!(format!("HTTP request failed: '{}'", err)))),
    }
}

/// Makes a `/metadata` request and verifies that it works as expected.
///
/// Intended for use as an "is the server running?" probe.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the [ServerHandle] for the server to test
///
/// Returns [Result::Ok] if the operation worked as expected, or [Result::Err] if it didn't.
pub async fn check_metadata_operation(
    app_state: &AppState,
    server_handle: &dyn ServerHandle,
) -> Result<()> {
    let operation_state = ServerOperationIterationState::new();
    let operation = crate::test_framework::metadata::run_operation_metadata(
        server_handle,
        operation_state.clone(),
    );
    let operation = tokio::time::timeout(
        app_state
            .config
            .operation_timeout
            .to_std()
            .expect("unable to convert Duration"),
        operation,
    );

    // Having the timeout gives us a wrapped Result<Result ...>>. Un-nest them.
    let result = operation.await?;
    result
        .map(|_| ())
        .map_err(|err| eyre!("Metadata check failed: '{:?}'", err))
}

/// Verifies and benchmarks the FHIR `/metadata` operations for the specified number of concurrent users.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the [ServerHandle] for the server implementation instance being tested
/// * `concurrent_users`: the number of users to try and test with concurrently
///
/// Returns a [ServerOperationMeasurement] with the results.
#[tracing::instrument(level = "info", skip(app_state, server_handle))]
async fn benchmark_operation_metadata_for_users(
    app_state: &AppState,
    server_handle: &dyn ServerHandle,
    concurrent_users: u32,
) -> ServerOperationMeasurement {
    /*
     * Build an iterator: One element for each iteration to run, run the operation for each iteration, and
     * count the iterations that failed.
     */
    let operations = (0..app_state.config.iterations).map(|_| async {
        let operation_state = ServerOperationIterationState::new();
        let operation = run_operation_metadata(server_handle, operation_state.clone());
        let operation = tokio::time::timeout(
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
                .failed(eyre!("Operation timed out: '{}'", err))
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
                warn!("Operation '{}' failed: '{:?}", SERVER_OP_NAME_METADATA, err);
                iterations_failed += 1;
            }
        }
    }
    let completed = Utc::now();

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
