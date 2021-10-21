//! Contains the [ServerPlugin] and [ServerHandle] implementations for the
//! [IBM FHIR](https://github.com/IBM/FHIR) server.
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::{process::Command, sync::Arc};
use url::Url;

static SERVER_NAME: &str = "IBM FHIR Server";

/// The username to use for HTTP operations against the FHIR server.
static FHIR_USERNAME: &str = "fhiruser";

/// The password to use for HTTP operations against the FHIR server.
static FHIR_PASSWORD: Option<&'static str> = Some("change-password");

/// The trait object for the `ServerPlugin` implementation for the IBM FHIR server.
pub struct IbmFhirServerPlugin {
    server_name: ServerName,
}

impl IbmFhirServerPlugin {
    /// Constructs a new `HapiJpaFhirServerPlugin` instance.
    pub fn new() -> IbmFhirServerPlugin {
        IbmFhirServerPlugin {
            server_name: SERVER_NAME.into(),
        }
    }
}

#[async_trait]
impl ServerPlugin for IbmFhirServerPlugin {
    fn server_name(&self) -> &ServerName {
        &self.server_name
    }

    async fn launch(&self, app_state: &AppState) -> Result<Box<dyn ServerHandle>> {
        launch_server(app_state).await
    }
}

async fn launch_server(app_state: &AppState) -> Result<Box<dyn ServerHandle>> {
    let server_work_dir = server_work_dir(&app_state.config.benchmark_dir()?);

    /*
     * Build and launch the server.
     *
     * Note: The environment variables used here are required to get build caching working correctly,
     * particularly for CI machines where the cache would otherwise be cold.
     */
    let docker_up_output = Command::new("./ibm_fhir_launch.sh")
        .current_dir(&server_work_dir)
        .output()
        .context("Failed to run 'ibm_fhir_launch.sh'.")?;
    if !docker_up_output.status.success() {
        return Err(anyhow!(crate::errors::AppError::ChildProcessFailure(
            docker_up_output.status,
            "Failed to launch IBM FHIR Server.".to_owned(),
            String::from_utf8_lossy(&docker_up_output.stdout).into(),
            String::from_utf8_lossy(&docker_up_output.stderr).into()
        )));
    }

    // The server containers have now been started, though they're not necessarily ready yet.
    let server_plugin = app_state
        .find_server_plugin(SERVER_NAME)
        .expect("Unable to find server plugin");
    let http_client = super::client_default()?;
    let server_handle = IbmFhirServerHandle {
        server_plugin,
        server_work_dir,
        http_client,
    };

    // Wait (up to a timeout) for the server to be ready.
    match wait_for_ready(app_state, &server_handle).await {
        Err(err) => {
            server_handle.emit_logs_info(&app_state.logger)?;
            Err(err)
        }
        Ok(_) => Ok(Box::new(server_handle)),
    }
}

/// Returns the work directory to use for the FHIR server.
fn server_work_dir(benchmark_dir: &Path) -> PathBuf {
    benchmark_dir.join("server_builds").join("ibm_fhir")
}

/// Checks the specified server repeatedly to see if it is ready, up to a hardcoded timeout.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the server to test
///
/// Returns an empty [Result], where an error indicates that the server was not ready.
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

/// Represents a launched instance of the IBM FHIR server.
pub struct IbmFhirServerHandle {
    server_plugin: Arc<dyn ServerPlugin>,
    server_work_dir: PathBuf,
    http_client: reqwest::Client,
}

#[async_trait]
impl ServerHandle for IbmFhirServerHandle {
    fn plugin(&self) -> Arc<dyn ServerPlugin> {
        self.server_plugin.clone()
    }

    fn base_url(&self) -> url::Url {
        Url::parse("https://localhost:9443/fhir-server/api/v4/").expect("Unable to parse URL.")
    }

    fn client(&self) -> Result<reqwest::Client> {
        Ok(self.http_client.clone())
    }

    fn request_builder(
        &self,
        client: reqwest::Client,
        method: http::Method,
        url: Url,
    ) -> reqwest::RequestBuilder {
        super::request_builder_default(client, method, url).basic_auth(FHIR_USERNAME, FHIR_PASSWORD)
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

    async fn expunge_all_content(&self, app_state: &AppState) -> Result<()> {
        self.shutdown()?;
        launch_server(app_state).await?;
        Ok(())
    }

    fn shutdown(&self) -> Result<()> {
        let docker_down_output = Command::new("docker-compose")
            .args(&["down"])
            .current_dir(&self.server_work_dir)
            .output()
            .context("Failed to run 'docker-compose down'.")?;
        if !docker_down_output.status.success() {
            return Err(anyhow!(crate::errors::AppError::ChildProcessFailure(
                docker_down_output.status,
                "Failed to shutdown IBM FHIR Server via docker-compose.".to_owned(),
                String::from_utf8_lossy(&docker_down_output.stdout).into(),
                String::from_utf8_lossy(&docker_down_output.stderr).into()
            )));
        }

        Ok(())
    }
}

/// Unit tests for the [crate::servers::ibm_fhir] module.
///
/// Note: These aren't exactly typical "unit" tests, as they will launch and shutdown the server via Docker,
/// which is a not-cheap operation.
#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, path::Path};

    use anyhow::{anyhow, Result};

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

        let app_state = crate::tests_util::create_app_state_test(&log_target.clone())
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
            Err(err) => Err(anyhow!(
                "Server launch test failed due to error: {:?}. Log output: {:?}",
                err,
                log_target
            )),
        }
    }
}
