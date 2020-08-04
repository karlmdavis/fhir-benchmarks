//! Contains the code to run `/metadata` server operations.

use crate::servers::ServerHandle;
use crate::test_framework::{ServerOperationLog, ServerOperationMeasurement};
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use chrono::prelude::*;
use futures::prelude::*;
use slog::warn;
use std::convert::TryFrom;
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
/// * `app_state`: the application's [AppState]
/// * `url`: the full [Url] to the endpoint to test
///
/// Returns an empty [Result], indicating whether or not the operation succeeded or failed.
pub async fn run_operation_metadata(app_state: &AppState, url: Url) -> Result<()> {
    // FIXME probably want to switch to something that supports async_std here
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
    let operations = (0..app_state.config.iterations).into_iter().map(|_| async {
        match run_operation_metadata(app_state, url.clone()).await {
            Ok(_) => 0u32,
            Err(_) => 1u32,
        }
    });

    /*
     * Convert that iterator to a parallel stream, and use use `buffer_unordered(...)` to set it to run it only up to
     * `concurrent_users`, at once.
     */
    let mut operations = futures::stream::iter(operations)
        .buffer_unordered(usize::try_from(concurrent_users).unwrap());

    /*
     * FIXME Remove this commented-out code in next commit.
     *
     * I like the `parallel_stream` API more, but had lifetime issues using it (with `app_state`) that I
     * don't know how to resolve.
     */
    /*
     * We want a stream of async operations, executed concurrently (up to a limit), where we'll sum up the failures.
     */
    // let operations = (0..app_state.config.iterations).collect::<Vec<u32>>()
    //     .into_par_stream().limit(usize::try_from(concurrent_users).unwrap())
    //     .map(|_| async {
    //         match run_operation_metadata_iteration(app_state, url.clone()).await {
    //             Ok(_) => 0u32,
    //             Err(_) => 1u32,
    //         }
    //         });
    // let mut iterations_failed: u32 = 0;
    // // Note: could have trouble here as this should have higher priority than async ops but doesn't
    // while let Some(n) = operations.next().await {
    //     iterations_failed += n;
    // }
    //let foo = parallel_stream::from_stream(stream::from_iter(0..app_state.config.iterations)).limit(2);

    /*
     * Kick off the execution of the stream, summing up all of the failures that are encountered.
     */
    let started = Utc::now();
    let mut iterations_failed: u32 = 0;
    while let Some(n) = operations.next().await {
        iterations_failed += n;
    }
    let completed = Utc::now();

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
