//! TODO

pub mod config;
pub mod errors;
mod sample_data;
pub mod servers;
pub mod test_framework;
mod util;

use crate::config::AppConfig;
use crate::errors::AppError;
use crate::sample_data::SampleData;
use crate::servers::{ServerHandle, ServerPlugin};
use crate::test_framework::{FrameworkOperationLog, FrameworkOperationResult, FrameworkResults};
use chrono::prelude::*;
use eyre::{eyre, Result, WrapErr};
use servers::ServerPluginWrapper;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, fmt::format::FmtSpan, EnvFilter};

/// Represents the application's context/state.
pub struct AppState {
    pub config: AppConfig,
    pub server_plugins: Vec<ServerPluginWrapper>,
    pub sample_data: SampleData,
}

impl AppState {
    /// Returns the [ServerPlugin] matching the specified name.
    fn find_server_plugin(&self, server_name: &str) -> Option<&ServerPluginWrapper> {
        self.server_plugins
            .iter()
            .find(|p| p.server_name().0 == server_name)
    }
}

/// The library crate's primary entry point: this does all the things.
pub async fn run_bench_orchestrator() -> Result<()> {
    /* Initialize tracing & logging. Because the "tracing-log" feature from "tracing-subscriber" is active,
    this will also route all log crate usage (from our dependencies) to tracing, instead. */
    let fmt_layer = fmt::layer()
        .with_writer(std::io::stderr)
        .with_ansi(atty::is(atty::Stream::Stderr))
        /* NEW and CLOSE cover the start and end of every run through a span, respectively. Generally, at
        least NEW should be enabled here, so users can keep track of progress, as most progress logging is
        provided via info spans in the application. */
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        /* Set this to true to include code locations in each log message. They're disnabled by default as
        they add a lot of noise, though they're helpful if you're trying to debug library code. */
        .with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .with(tracing_error::ErrorLayer::default())
        .init();

    // Initialize the app's state.
    let app_state = create_app_state().await?;

    // Verify that pre-requisites are present.
    verify_prereqs()?;

    // Test each selected FHIR server implementation.
    let mut framework_results = FrameworkResults::new(&app_state.config, &app_state.server_plugins);
    for server_plugin in &app_state.server_plugins {
        // Store results for the test here.
        let mut server_result = framework_results
            .get_mut(server_plugin.server_name())
            .ok_or_else(|| AppError::UnknownServerError(server_plugin.server_name().clone()))?;

        // Launch the implementation's server, etc. This will likely take a while.
        let launch_started = Utc::now();
        let launch_result = server_plugin.launch(&app_state).await;
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
            let operations = test_framework::run_operations(&app_state, server_handle)
                .await
                .with_context(|| {
                    format!(
                        "Error when running operations for server '{}'.",
                        server_plugin.server_name()
                    )
                })?;
            server_result.operations = Some(operations);

            // Shutdown and cleanup the server and its resources.

            // Optionally pause for manual debugging.
            // std::io::stdin().read_line(&mut String::new()).unwrap();

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
async fn create_app_state() -> Result<AppState> {
    // Parse command line args.
    let config = AppConfig::new()?;

    // Find all FHIR server implementations that can be tested.
    let server_plugins: Vec<ServerPluginWrapper> = servers::create_server_plugins(&config)?;

    // Setup all global/shared resources.
    let sample_data = sample_data::generate_data_using_config(&config)
        .await
        .context("Error when generating sample data.")?;

    Ok(AppState {
        config,
        server_plugins,
        sample_data,
    })
}

/// Verifies that the required tools are present on this system.
fn verify_prereqs() -> Result<()> {
    use std::process::Command;

    let docker_compose_output = Command::new("docker-compose")
        .args(&["--help"])
        .output()
        .context("Failed to run 'docker-compose --help'.")?;
    if !docker_compose_output.status.success() {
        return Err(eyre!(crate::errors::AppError::ChildProcessFailure(
            docker_compose_output.status,
            "Missing pre-req: docker-compose.".to_owned(),
            String::from_utf8_lossy(&docker_compose_output.stdout).into(),
            String::from_utf8_lossy(&docker_compose_output.stderr).into()
        )));
    }

    Ok(())
}

/// Output all of the results.
fn output_results(framework_results: &FrameworkResults) {
    let framework_results_pretty = serde_json::to_string_pretty(&framework_results).unwrap();
    println!("{}", framework_results_pretty);
}
