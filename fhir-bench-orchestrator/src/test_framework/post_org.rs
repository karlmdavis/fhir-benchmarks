//! Provides the `benchmark_post_org(...)` method for benchmarking FHIR `POST /Organization`
//! operations.

use crate::servers::ServerHandle;
use crate::test_framework::{ServerOperationLog, ServerOperationMeasurement};
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use chrono::prelude::*;
use chrono::Duration;
use futures::prelude::*;
use slog::{trace, warn};
use std::convert::TryFrom;
use url::Url;

static SERVER_OP_NAME_POST_ORG: &str = "POST /Organization";

/// Attempts to verify and benchmark FHIR `POST /Organization` operations for the specified FHIR
/// server.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the [ServerHandle] for the server implementation instance being tested
///
/// Returns a [ServerOperationLog] detailing the results of the benchmark attempt.
pub async fn benchmark_post_org(
    app_state: &AppState,
    server_handle: &dyn ServerHandle,
) -> ServerOperationLog {
    trace!(
        app_state.logger,
        "Benchmarking POST /Organization: starting..."
    );
    let mut server_op_log = ServerOperationLog::new(SERVER_OP_NAME_POST_ORG.into());

    for concurrent_users in app_state.config.concurrency_levels.clone() {
        let measurement =
            benchmark_post_org_for_users(app_state, server_handle, concurrent_users).await;
        server_op_log.measurements.push(measurement);
    }

    trace!(
        app_state.logger,
        "Benchmarking POST /Organization: completed."
    );
    server_op_log
}

/// Verifies and benchmarks FHIR `POST /Organization` operations for the specified number of concurrent users.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the [ServerHandle] for the server implementation instance being tested
/// * `concurrent_users`: the number of users to try and test with concurrently
///
/// Returns a [ServerOperationMeasurement] with the results.
async fn benchmark_post_org_for_users(
    app_state: &AppState,
    server_handle: &dyn ServerHandle,
    concurrent_users: u32,
) -> ServerOperationMeasurement {
    // Setup the results tracking state.
    trace!(
        app_state.logger,
        "Benchmarking POST /Organization: '{}' concurrent users: starting...",
        concurrent_users
    );
    let started = Utc::now();
    let mut execution_duration: Duration = Duration::seconds(0);
    let mut iterations_attempted: u32 = 0;
    let mut iterations_failed: u32 = 0;

    /* The iterations need to be split across groups, based on the resources (i.e. sample data) that each
     * iteration will consume. */
    let sample_orgs_count = match app_state.sample_data.load_sample_orgs() {
        Ok(orgs) => u32::try_from(orgs.len()).unwrap(),
        Err(err) => {
            warn!(app_state.logger, "Sample data: unable to load: {}", err);
            let completed = Utc::now();
            return ServerOperationMeasurement {
                concurrent_users,
                started,
                completed,
                execution_duration: completed - started,
                iterations_failed: 0,
                iterations_skipped: app_state.config.iterations,
                metrics: None,
            };
        }
    };
    let groups = (app_state.config.iterations - 1) / sample_orgs_count + 1;
    for group_index in 0..groups {
        // How many iterations should be run for this group?
        let iterations_remaining = app_state.config.iterations - iterations_attempted;
        let group_iterations = std::cmp::min(sample_orgs_count, iterations_remaining);

        // Wipe the server to start with a blank slate. Also allows for sample data to be re-used.
        match expunge_everything(app_state, server_handle) {
            Ok(_) => {}
            Err(err) => {
                warn!(app_state.logger, "FHIR server expunge: error: {}", err);
                let completed = Utc::now();
                return ServerOperationMeasurement {
                    concurrent_users,
                    started,
                    completed,
                    execution_duration: completed - started,
                    iterations_failed,
                    iterations_skipped: iterations_remaining,
                    // TODO implement metrics
                    metrics: None,
                };
            }
        };

        // Load the sample data that each iteration will consume an element of.
        let mut sample_data = match app_state.sample_data.load_sample_orgs() {
            Ok(sample_data) => sample_data,
            Err(err) => {
                warn!(app_state.logger, "Sample data: unable to load: {}", err);
                let completed = Utc::now();
                return ServerOperationMeasurement {
                    concurrent_users,
                    started,
                    completed,
                    execution_duration: completed - started,
                    iterations_failed,
                    iterations_skipped: iterations_remaining,
                    // TODO implement metrics
                    metrics: None,
                };
            }
        };
        if u32::try_from(sample_data.len()).unwrap() > group_iterations {
            sample_data.resize(
                usize::try_from(group_iterations).unwrap(),
                sample_data[0].clone(),
            );
        }
        trace!(
            app_state.logger,
            "Benchmarking POST /Organization: '{}' concurrent users: group '{}/{}' with '{}' iterations: starting...",
            concurrent_users, group_index + 1, groups, group_iterations
        );
        let group_started = Utc::now();

        // Run the iterations for this group.
        let group_iterations_failed = benchmark_post_org_for_users_and_data(
            app_state,
            server_handle,
            concurrent_users,
            sample_data,
        )
        .await;

        let group_completed = Utc::now();
        trace!(
            app_state.logger,
            "Benchmarking POST /Organization: '{}' concurrent users: group '{}/{}' with '{}' iterations: completed.",
            concurrent_users, group_index, groups, group_iterations
        );
        iterations_attempted += group_iterations;
        iterations_failed += group_iterations_failed;
        execution_duration = execution_duration + (group_completed - group_started);
    }

    let completed = Utc::now();
    trace!(
        app_state.logger,
        "Benchmarking POST /Organization: '{}' concurrent users: completed.",
        concurrent_users
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

/// Verifies and benchmarks FHIR `POST /Organization` operations for the specified number of concurrent users
/// using the specified sample data.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the [ServerHandle] for the server implementation instance being tested
/// * `concurrent_users`: the number of users to try and test with concurrently
/// * `sample_data`: the sample data to test against -- one iteration should be run for each element in it
///
/// Returns the number of iterations that failed.
async fn benchmark_post_org_for_users_and_data(
    app_state: &AppState,
    server_handle: &dyn ServerHandle,
    concurrent_users: u32,
    sample_data: Vec<serde_json::Value>,
) -> u32 {
    let url = create_org_url(server_handle);

    /*
     * Build an iterator: One element for each iteration to run, run the operation for each iteration, and
     * count the iterations that failed.
     */
    let operations = sample_data.into_iter().map(|org| async {
        match run_operation_post_org(app_state, url.clone(), org).await {
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
     * Kick off the execution of the stream, summing up all of the failures that are encountered.
     */
    let mut iterations_failed: u32 = 0;
    while let Some(n) = operations.next().await {
        iterations_failed += n;
    }

    iterations_failed
}

/// Creates the URL to access a server's `/Organization` endpoint.
///
/// Parameters:
/// * `server_handle`: the [ServerHandle] for the server implementation instance being tested
fn create_org_url(server_handle: &dyn ServerHandle) -> Url {
    server_handle
        .base_url()
        .join("Organization")
        .expect("Error parsing URL.")
}

/// Runs a single iteration of the `POST /Organization` operation and verifies its result, logging out any
/// faults that were found.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `url`: the full [Url] to the endpoint to test
/// * `org`: the sample `Organization` resource to test with
///
/// Returns an empty [Result], indicating whether or not the operation succeeded or failed.
async fn run_operation_post_org(
    app_state: &AppState,
    url: Url,
    org: serde_json::Value,
) -> Result<()> {
    let org_string = serde_json::to_string(&org)?;
    let org_id = org.get("id").expect("Organization missing ID.").to_string();

    // FIXME probably want to switch to something that supports std_async here
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.clone())
        .header("Content-Type", "application/fhir+json")
        .body(org_string)
        .send()
        .with_context(|| {
            format!(
                "The POST to '{}' failed for Organization '{}'.",
                url, org_id
            )
        })?;

    let response_status = response.status();
    if !response_status.is_success() {
        let response_body = response.text()?;
        warn!(app_state.logger, "POST failed"; "url" => url.as_str(), "resource_id" => &org_id, "status" => response_status.as_str(), "response_body" => response_body);
        return Err(anyhow!(
            "The POST to '{}' failed for Organization '{}', with status '{}'.",
            &url,
            &org_id,
            response_status
        ));
    }
    // TODO more checks needed
    Ok(())
}

/// Expunge all resources from the server.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the [ServerHandle] for the server implementation instance being tested
fn expunge_everything(app_state: &AppState, server_handle: &dyn ServerHandle) -> Result<()> {
    // FIXME probably want to switch to something that supports async_std here
    let url = server_handle
        .base_url()
        .join("$expunge")
        .expect("Error parsing URL.");
    let client = reqwest::blocking::Client::new();
    let response = client
        .post(url.clone())
        .query(&[("expungeEverything", "true")])
        .send()
        .with_context(|| format!("The POST to '{}' failed.", url))?;

    if !response.status().is_success() {
        warn!(app_state.logger, "POST failed"; "url" => url.as_str(), "status" => response.status().as_str());
        return Err(anyhow!(
            "The POST to '{}' failed, with status '{}'.",
            &url,
            &response.status()
        ));
    }
    // TODO more checks needed
    Ok(())
}