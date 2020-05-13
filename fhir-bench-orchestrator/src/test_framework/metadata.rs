//! Contains the code to run `/metadata` server operations.

use crate::servers::ServerHandle;
use crate::test_framework::ServerOperationLog;
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use chrono::prelude::*;
use slog::warn;
use url::Url;

static SERVER_OP_NAME_METADATA: &str = "metadata";

/// Creates the URL to access a server's `/metadata` endpoint.
pub fn create_metadata_url(server_handle: &dyn ServerHandle) -> Url {
    server_handle
        .base_url()
        .join("metadata")
        .expect("Error parsing URL.")
}

/// Runs a single iteration of the `/metadata` operation and verifies its result, logging out any faults that
/// were found.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `url`: the full [Url] to the endpoint to test
///
/// Returns an empty [Result], indicating whether or not the operation succeeded or failed.
pub async fn run_operation_metadata_iteration(app_state: &AppState, url: Url) -> Result<()> {
    // FIXME probably want to switch to something that supports std_async here
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

/// Verifies and benchmarks FHIR `/metadata` operations.
pub async fn run_operation_metadata(
    app_state: &AppState,
    server_handle: &dyn ServerHandle,
) -> ServerOperationLog {
    let mut server_op_log = ServerOperationLog {
        operation: SERVER_OP_NAME_METADATA.into(),
        started: Utc::now(),
        iterations: app_state.config.iterations,
        completed: None,
        failures: None,
        metrics: None,
    };

    let url = create_metadata_url(server_handle);

    let mut failures = 0;
    for _ in 0..server_op_log.iterations {
        let iteration_result = run_operation_metadata_iteration(app_state, url.clone()).await;
        if iteration_result.is_err() {
            failures += 1;
        }
    }

    server_op_log.failures = Some(failures);
    server_op_log.completed = Some(Utc::now());

    server_op_log
}
