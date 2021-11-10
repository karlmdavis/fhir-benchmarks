//! Adds support for launching and interacting with the
//! [Spark](https://github.com/FirelyTeam/spark) FHIR server, which was originally created by
//! [Firely](https://fire.ly/) but is now community-maintained. Spark is written in C# and uses
//! MongoDB as its datastore.
use crate::AppState;
use crate::{
    sample_data::SampleResource,
    servers::{ServerHandle, ServerName, ServerPlugin},
};
use async_trait::async_trait;
use eyre::{eyre, Context, Result};
use std::path::{Path, PathBuf};
use std::{process::Command, sync::Arc};
use url::Url;

static SERVER_NAME: &str = "Spark FHIR R4 Server";

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
        launch_server(app_state).await
    }

    /// As detailed in this project's `doc/server-compliance.md` file, Spark non-compliantly rejects POSTs
    /// of resources with an ID. This method strips those IDs out, so that testing can proceed, as otherwise
    /// there's really not much we actually _can_ test.
    ///
    /// Parameters:
    /// * `sample_org`: a sample `Organization` JSON resource that has been generated to test this server
    fn fudge_sample_resource(&self, mut sample_resource: SampleResource) -> SampleResource {
        // Strip out the "id" element.
        sample_resource
            .resource_json
            .as_object_mut()
            .expect("JSON resource was empty")
            .remove("id");

        sample_resource
    }
}

/// Launches the server, producing a boxed [SparkFhirServerHandle].
///
/// Parameters:
/// * `app_state`: the application's [AppState]
#[tracing::instrument(level = "info", fields(server_name = SERVER_NAME), skip(app_state))]
async fn launch_server(app_state: &AppState) -> Result<Box<dyn ServerHandle>> {
    let server_work_dir = server_work_dir(&app_state.config.benchmark_dir()?);

    /*
     * (Re-)download the server's Docker Compose file from the project's GitHub. Note that this
     * always grabs and overwrites any pre-existing files with the latest.
     */
    let compose_url = "https://raw.githubusercontent.com/FirelyTeam/spark/r4/master/.docker/docker-compose.example.yml";
    let compose_response = reqwest::get(compose_url).await?;
    let compose_path = server_work_dir.join("docker-compose.yml");
    let mut compose_file = std::fs::File::create(compose_path)?;
    let compose_content = compose_response.text().await?;
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
        .current_dir(&server_work_dir)
        .output()
        .context("Failed to run 'docker-compose up'.")?;
    if !docker_up_output.status.success() {
        return Err(eyre!(crate::errors::AppError::ChildProcessFailure(
            docker_up_output.status,
            format!("Failed to launch {} via docker-compose.", SERVER_NAME),
            String::from_utf8_lossy(&docker_up_output.stdout).into(),
            String::from_utf8_lossy(&docker_up_output.stderr).into()
        )));
    }

    // The server containers have now been started, though they're not necessarily ready yet.
    let server_plugin = app_state
        .find_server_plugin(SERVER_NAME)
        .expect("Unable to find server plugin");
    let http_client = super::client_default()?;
    let server_handle = SparkFhirServerHandle {
        server_plugin,
        server_work_dir,
        http_client,
    };

    // Wait (up to a timeout) for the server to be ready.
    match wait_for_ready(app_state, &server_handle).await {
        Err(err) => {
            server_handle.emit_logs_info()?;
            Err(err)
        }
        Ok(_) => {
            let server_handle: Box<dyn ServerHandle> = Box::new(server_handle);
            Ok(server_handle)
        }
    }
}

/// Returns the work directory to use for the FHIR server.
fn server_work_dir(benchmark_dir: &Path) -> PathBuf {
    benchmark_dir.join("server_builds").join("firely_spark")
}

/// Checks the specified server repeatedly to see if it is ready, up to a hardcoded timeout.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the server to test
///
/// Returns an empty [Result], where an error indicates that the server was not ready.
#[tracing::instrument(level = "debug", fields(server_name = SERVER_NAME), skip(app_state, server_handle))]
async fn wait_for_ready(app_state: &AppState, server_handle: &dyn ServerHandle) -> Result<()> {
    tokio::time::timeout(std::time::Duration::from_secs(60), async {
        let mut ready = false;
        let mut probe = None;

        while !ready {
            probe = Some(
                crate::test_framework::metadata::check_metadata_operation(app_state, server_handle)
                    .await,
            );
            ready = probe.as_ref().expect("probe result missing").is_ok();

            if !ready {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
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

/// Represents a launched instance of the Spark FHIR server.
pub struct SparkFhirServerHandle {
    server_plugin: Arc<dyn ServerPlugin>,
    server_work_dir: PathBuf,
    http_client: reqwest::Client,
}

#[async_trait]
impl ServerHandle for SparkFhirServerHandle {
    fn plugin(&self) -> Arc<dyn ServerPlugin> {
        self.server_plugin.clone()
    }

    fn base_url(&self) -> url::Url {
        Url::parse("http://localhost:5555/fhir/").expect("Unable to parse URL.")
    }

    fn client(&self) -> Result<reqwest::Client> {
        Ok(self.http_client.clone())
    }

    fn emit_logs(&self) -> Result<String> {
        match Command::new("docker-compose")
            .args(&["logs", "--no-color"])
            .current_dir(&self.server_work_dir)
            .output()
            .context("Failed to run 'docker-compose logs'.")
        {
            Ok(output) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
            Err(err) => Err(err),
        }
    }

    #[tracing::instrument(level = "debug", fields(server_name = SERVER_NAME), skip(self, app_state))]
    async fn expunge_all_content(&self, app_state: &AppState) -> Result<()> {
        self.shutdown()?;
        launch_server(app_state).await?;
        Ok(())
    }

    #[tracing::instrument(level = "debug", fields(server_name = SERVER_NAME), skip(self))]
    fn shutdown(&self) -> Result<()> {
        let docker_down_output = Command::new("docker-compose")
            .args(&["down"])
            .current_dir(&self.server_work_dir)
            .output()
            .context("Failed to run 'docker-compose down'.")?;
        if !docker_down_output.status.success() {
            return Err(eyre!(crate::errors::AppError::ChildProcessFailure(
                docker_down_output.status,
                format!("Failed to shutdown {} via docker-compose.", SERVER_NAME),
                String::from_utf8_lossy(&docker_down_output.stdout).into(),
                String::from_utf8_lossy(&docker_down_output.stderr).into()
            )));
        }

        Ok(())
    }
}

/// Unit tests for the [crate::servers::firely_spark] module.
///
/// Note: These aren't exactly typical "unit" tests, as they will launch and shutdown the server via Docker,
/// which is a not-cheap operation.
#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, path::Path};

    use eyre::{eyre, Result};

    #[tokio::test]
    #[serial_test::serial(sample_data)]
    async fn verify_server_launch() -> Result<()> {
        let log_target = std::env::temp_dir().join(format!(
            "{}_verify_server_launch.log",
            Path::new(file!())
                .file_name()
                .unwrap_or(OsStr::new("server"))
                .to_string_lossy()
        ));

        let app_state = crate::tests_util::create_app_state_test()
            .await
            .expect("Unable to create test app state.");
        let server_plugin = app_state
            .find_server_plugin(super::SERVER_NAME)
            .expect("Unable to find server plugin");

        // This will launch the server and verify it's ready.
        // Note: we can't use the assert_* macros here, as panics would leave a dangling container.
        let launch_result = match server_plugin.launch(&app_state).await {
            Ok(server_handle) => server_handle.shutdown().map(|_| ()),
            Err(err) => Err(err),
        };

        // Clean up the log if things went well, otherwise return an error with the path to it.
        match launch_result {
            Ok(_) => {
                if log_target.exists() {
                    std::fs::remove_file(log_target)?;
                }
                Ok(())
            }
            Err(err) => Err(eyre!(
                "Server launch test failed due to error: {:?}. Log output: {:?}",
                err,
                log_target
            )),
        }
    }
}
