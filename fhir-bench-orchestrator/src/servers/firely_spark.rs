//! Adds support for launching and interacting with the
//! [Spark](https://github.com/FirelyTeam/spark) FHIR server, which was originally created by
//! [Firely](https://fire.ly/) but is now community-maintained. Spark is written in C# and uses
//! MongoDB as its datastore.
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use slog::{info, warn};
use std::process::Command;
use url::Url;

static SERVER_NAME: &str = "Spark FHIR R4 Server";
static SPARK_DIR: &str = "server_builds/firely_spark";

/// The trait object for the `ServerPlugin` implementation for the Spark FHIR server.
pub struct SparkFhirServerPlugin {
    server_name: ServerName,
}

impl SparkFhirServerPlugin {
    /// Constructs a new `HapiJpaFhirServerPlugin` instance.
    pub fn new() -> SparkFhirServerPlugin {
        SparkFhirServerPlugin {
            server_name: SERVER_NAME.into(),
        }
    }
}

#[async_trait]
impl ServerPlugin for SparkFhirServerPlugin {
    fn server_name(&self) -> &ServerName {
        &self.server_name
    }

    async fn launch(&self, app_state: &AppState) -> Result<Box<dyn ServerHandle>> {
        launch_server(&app_state).await
    }
}

/// Launches the server, producing a boxed [SparkFhirServerHandle].
///
/// Parameters:
/// * `app_state`: the application's [AppState]
async fn launch_server(app_state: &AppState) -> Result<Box<dyn ServerHandle>> {
    /*
     * (Re-)download the server's Docker Compose file from the project's GitHub. Note that this
     * always grabs and overwrites any pre-existing files with the latest.
     */
    let compose_url = "https://raw.githubusercontent.com/FirelyTeam/spark/r4/master/.docker/docker-compose.example.yml";
    let compose_response = reqwest::blocking::get(compose_url)?;
    let compose_path = app_state
        .config
        .benchmark_dir()?
        .join(SPARK_DIR)
        .join("docker-compose.yml");
    let mut compose_file = std::fs::File::create(compose_path)?;
    let compose_content = compose_response.text()?;
    std::io::copy(&mut compose_content.as_bytes(), &mut compose_file)?;

    /*
     * Build and launch the server.
     *
     * Note: The environment variables used here are required to get build caching working correctly,
     * particularly for CI machines where the cache would otherwise be cold.
     */

    let docker_up_output = Command::new("docker-compose")
        .args(&["up", "--detach"])
        .env("COMPOSE_DOCKER_CLI_BUILD", "1")
        .env("DOCKER_BUILDKIT", "1")
        .current_dir(app_state.config.benchmark_dir()?.join(SPARK_DIR))
        .output()
        .context("Failed to run 'docker-compose up'.")?;
    if !docker_up_output.status.success() {
        return Err(anyhow!(crate::errors::AppError::ChildProcessFailure(
            docker_up_output.status,
            format!("Failed to launch {} via docker-compose.", SERVER_NAME),
            String::from_utf8_lossy(&docker_up_output.stdout).into(),
            String::from_utf8_lossy(&docker_up_output.stderr).into()
        )));
    }

    // The server containers have now been started, though they're not necessarily ready yet.
    let server_handle = SparkFhirServerHandle {};

    // Wait (up to a timeout) for the server to be ready.
    match wait_for_ready(app_state, &server_handle).await {
        Err(err) => {
            server_handle.emit_logs_info(&app_state.logger);
            Err(err)
        }
        Ok(_) => Ok(Box::new(server_handle)),
    }
}

/// Checks the specified server repeatedly to see if it is ready, up to a hardcoded timeout.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the server to test
///
/// Returns an empty [Result], where an error indicates that the server was not ready.
async fn wait_for_ready(app_state: &AppState, server_handle: &dyn ServerHandle) -> Result<()> {
    async_std::future::timeout(std::time::Duration::from_secs(60), async {
        let mut ready = false;
        let mut probe = None;

        while !ready {
            probe = Some(probe_for_ready(app_state, server_handle).await);
            ready = probe.as_ref().expect("probe result missing").is_ok();

            if !ready {
                async_std::task::sleep(std::time::Duration::from_millis(500)).await;
            }
        }

        probe.expect("probe results missing")
    })
    .await
    .with_context(|| {
        format!(
            "Timed out while waiting for server '{}' to launch.",
            SERVER_NAME
        )
    })?
}

/// Checks the specified server one time to see if it is ready.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the server to test
///
/// Returns an empty [Result], where an error indicates that the server was not ready.
async fn probe_for_ready(app_state: &AppState, server_handle: &dyn ServerHandle) -> Result<()> {
    let probe_url = crate::test_framework::metadata::create_metadata_url(server_handle);
    Ok(crate::test_framework::metadata::run_operation_metadata_safe(app_state, probe_url).await?)
}

/// Represents a launched instance of the Spark FHIR server.
pub struct SparkFhirServerHandle {}

#[async_trait]
impl ServerHandle for SparkFhirServerHandle {
    fn base_url(&self) -> url::Url {
        Url::parse("http://localhost:5555/fhir/").expect("Unable to parse URL.")
    }

    fn emit_logs_info(&self, logger: &slog::Logger) {
        let docker_logs_output = match Command::new("docker-compose")
            .args(&["logs", "--no-color"])
            .current_dir(SPARK_DIR)
            .output()
            .context("Failed to run 'docker-compose logs'.")
        {
            Ok(output) => output,
            Err(err) => {
                warn!(
                    logger,
                    "Unable to retrieve docker-compose logs for '{}' server: {}", SERVER_NAME, err
                );
                return;
            }
        };
        info!(
            logger,
            "Full docker-compose logs for '{}' server:\n{}",
            SERVER_NAME,
            String::from_utf8_lossy(&docker_logs_output.stdout)
        );
    }

    async fn expunge_all_content(&self, app_state: &AppState) -> Result<()> {
        self.shutdown()?;
        launch_server(&app_state).await?;
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        let docker_down_output = Command::new("docker-compose")
            .args(&["down"])
            .current_dir(SPARK_DIR)
            .output()
            .context("Failed to run 'docker-compose down'.")?;
        if !docker_down_output.status.success() {
            return Err(anyhow!(crate::errors::AppError::ChildProcessFailure(
                docker_down_output.status,
                format!("Failed to shutdown {} via docker-compose.", SERVER_NAME),
                String::from_utf8_lossy(&docker_down_output.stdout).into(),
                String::from_utf8_lossy(&docker_down_output.stderr).into()
            )));
        }

        Ok(())
    }
}
