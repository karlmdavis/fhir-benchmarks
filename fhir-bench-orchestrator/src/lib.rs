//! TODO

pub mod config;
pub mod errors;
pub mod servers;
pub mod test_framework;

use anyhow::{anyhow,Context,Result};
use crate::errors::AppError;
use crate::servers::{ServerHandle, ServerPlugin};
use crate::test_framework::{FrameworkOperationLog, FrameworkOperationResult, FrameworkResults};
use chrono::prelude::*;
use slog::{self, o, Drain};
use slog_json;
use crate::config::AppConfig;

/// Represents the application's context/state.
pub struct AppState {
    pub logger: slog::Logger,
    pub config: AppConfig,
    pub server_plugins: Vec<Box<dyn ServerPlugin>>,
    pub shared_resources: (),
}

/// The library crate's primary entry point: this does all the things.
pub async fn run_bench_orchestrator() -> Result<()> {
    // Initialize the app's state.
    let app_state = create_app_state()?;

    // Route all log crate usage (from our dependencies) to slog, instead.
    // Note: This has to stay in scope in order to keep working.
    let _scope_guard = slog_scope::set_global_logger(app_state.logger.clone());
    slog_stdlog::init_with_level(log::Level::Info)?;

    // Verify that pre-requisites are present.
    verify_prereqs()?;

    // Test each selected FHIR server implementation.
    let mut framework_results = FrameworkResults::new(&app_state.server_plugins);
    for server_plugin in &app_state.server_plugins {
        let server_plugin: &dyn ServerPlugin = &**server_plugin;

        // Store results for the test here.
        let mut server_result = framework_results
            .get_mut(server_plugin.server_name())
            .ok_or_else(|| AppError::UnknownServerError(server_plugin.server_name().clone()))?;

        // Launch the implementation's server, etc. This will likely take a while.
        let launch_started = Utc::now();
        let launch_result = server_plugin.launch();
        let launch_completed = Utc::now();

        // Destructure the launch result into success and failure objects, so they have separate ownership.
        let (server_handle, launch_error) = match launch_result {
            Ok(server_handle) => (Some(server_handle), None),
            Err(launch_error) => (None, Some(launch_error)),
        };

        // Store the launch result's success/error for the records.
        server_result.launch = Some(FrameworkOperationLog {
            started: launch_started,
            completed: launch_completed,
            outcome: match launch_error {
                None => FrameworkOperationResult::Ok(),
                Some(launch_error) => {
                    FrameworkOperationResult::Errs(vec![format!("{:?}", launch_error)])
                }
            },
        });

        // If the server launched successfully, move on to testing it and then shutting it down.
        if server_result.launch.as_ref().unwrap().is_ok() {
            let server_handle: &dyn ServerHandle = &*server_handle.unwrap();


            // Run the tests against the server.
            let operations = test_framework::run_operations(&app_state, server_handle)?;
            server_result.operations = Some(operations);

            // Optionally pause for manual debugging.
            // std::io::stdin().read_line(&mut String::new()).unwrap();

            // Shutdown and cleanup the server and its resources.
            let shutdown_started = Utc::now();
            let shutdown_result = server_handle.shutdown();
            let shutdown_completed = Utc::now();
            server_result.shutdown = Some(FrameworkOperationLog {
                started: shutdown_started,
                completed: shutdown_completed,
                outcome: match shutdown_result {
                    Ok(_) => FrameworkOperationResult::Ok(),
                    Err(err) => FrameworkOperationResult::Errs(vec![format!("{:?}", err)]),
                },
            });
        }
    }

    // Output results.
    framework_results.completed = Some(Utc::now());
    output_results(&framework_results);

    Ok(())
}

/// Initializes the [AppState].
#[allow(clippy::let_unit_value)]
fn create_app_state() -> Result<AppState> {
    // Create the root slog logger.
    let logger = create_logger_root();

    // Parse command line args.
    let config = AppConfig::new()?;

    // Find all FHIR server implementations that can be tested.
    let server_plugins: Vec<Box<dyn ServerPlugin>> = servers::create_server_plugins()?;

    // Setup all global/shared resources.
    // TODO: Remove `allow` on method once this actually does something.
    let shared_resources = create_shared_resources();

    Ok(AppState {
        logger, config, server_plugins, shared_resources
    })
}

/// Builds the root Logger for the application to use.
fn create_logger_root() -> slog::Logger {
    let drain = slog_json::Json::new(std::io::stderr())
        .set_pretty(true)
        .add_default_keys()
        .build()
        .fuse();
    let drain = slog_async::Async::new(drain).build().fuse();

    slog::Logger::root(drain, o!())
}

/// Verifies that the required tools are present on this system.
fn verify_prereqs() -> Result<()> {
    use std::process::Command;

    let docker_compose_output = Command::new("docker-compose")
        .args(&["--help"])
        .output()
        .context("Failed to run 'docker-compose --help'.")?;
    if !docker_compose_output.status.success() {
        return Err(anyhow!(crate::errors::AppError::ChildProcessFailure(
            docker_compose_output.status,
            "Missing pre-req: docker-compose.".to_owned(),
        )));
    }

    Ok(())
}

/// Initialize the application resources (e.g. test data) that will be used across projects.
fn create_shared_resources() {
    // TODO this'll be stuff like the synthetic data and all that
}

/// Output all of the results.
fn output_results(framework_results: &FrameworkResults) {
    let framework_results_pretty = serde_json::to_string_pretty(&framework_results).unwrap();
    println!("{}", framework_results_pretty);
}
