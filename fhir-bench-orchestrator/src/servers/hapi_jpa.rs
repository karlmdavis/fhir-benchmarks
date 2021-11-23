//! TODO
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use crate::AppState;
use async_trait::async_trait;
use eyre::{eyre, Context, Result};
use std::path::{Path, PathBuf};
use std::{process::Command, sync::Arc};
use tracing::{trace_span, warn, Instrument};
use url::Url;

static SERVER_NAME: &str = "HAPI FHIR JPA Server";

/// The trait object for the `ServerPlugin` implementation for the HAPI FHIR JPA server.
pub struct HapiJpaFhirServerPlugin {
    server_name: ServerName,
}

impl HapiJpaFhirServerPlugin {
    /// Constructs a new `HapiJpaFhirServerPlugin` instance.
    pub fn new() -> HapiJpaFhirServerPlugin {
        HapiJpaFhirServerPlugin {
            server_name: SERVER_NAME.into(),
        }
    }
}

#[async_trait]
impl ServerPlugin for HapiJpaFhirServerPlugin {
    fn server_name(&self) -> &ServerName {
        &self.server_name
    }

    #[tracing::instrument(level = "info", fields(server_name = SERVER_NAME), skip(self, app_state))]
    async fn launch(&self, app_state: &AppState) -> Result<Box<dyn ServerHandle>> {
        let server_work_dir = server_work_dir(&app_state.config.benchmark_dir()?);

        /*
         * Build and launch the server.
         */
        let docker_up_output = Command::new("./docker_compose_hapi_jpaserver_starter.sh")
            .args(&["up", "--detach"])
            .current_dir(&server_work_dir)
            .output()
            .context("Failed to run 'docker_compose_hapi_jpaserver_starter.sh'.")?;
        if !docker_up_output.status.success() {
            return Err(eyre!(crate::errors::AppError::ChildProcessFailure(
                docker_up_output.status,
                format!("Failed to launch {} via Docker Compose.", SERVER_NAME),
                String::from_utf8_lossy(&docker_up_output.stdout).into(),
                String::from_utf8_lossy(&docker_up_output.stderr).into()
            )));
        }

        // The server containers have now been started, though they're not necessarily ready yet.
        let server_plugin = app_state
            .find_server_plugin(SERVER_NAME)
            .expect("Unable to find server plugin");
        let http_client = super::client_default()?;
        let server_handle = HapiJpaFhirServerHandle {
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
}

/// Returns the work directory to use for the FHIR server.
fn server_work_dir(benchmark_dir: &Path) -> PathBuf {
    benchmark_dir
        .join("server_builds")
        .join("hapi_jpaserver_starter")
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
    tokio::time::timeout(std::time::Duration::from_secs(60 * 5), async {
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

/// Represents a launched instance of the HAPI FHIR JPA server.
pub struct HapiJpaFhirServerHandle {
    server_plugin: Arc<dyn ServerPlugin>,
    server_work_dir: PathBuf,
    http_client: reqwest::Client,
}

#[async_trait]
impl ServerHandle for HapiJpaFhirServerHandle {
    fn plugin(&self) -> Arc<dyn ServerPlugin> {
        self.server_plugin.clone()
    }

    fn base_url(&self) -> url::Url {
        Url::parse("http://localhost:8080/fhir/").expect("Unable to parse URL.")
    }

    fn client(&self) -> Result<reqwest::Client> {
        Ok(self.http_client.clone())
    }

    fn emit_logs(&self) -> Result<String> {
        match Command::new("./docker_compose_hapi_jpaserver_starter.sh")
            .args(&["logs", "--no-color"])
            .current_dir(&self.server_work_dir)
            .output()
            .context("Failed to run 'docker-compose logs'.")
        {
            Ok(output) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
            Err(err) => Err(err),
        }
    }

    /// Expunge all resources from the server.
    ///
    /// See <https://smilecdr.com/docs/fhir_repository/deleting_data.html#expunge> for details.
    ///
    /// Parameters:
    /// * `app_state`: the application's [AppState]
    #[tracing::instrument(level = "debug", fields(server_name = SERVER_NAME), skip(self, _app_state))]
    async fn expunge_all_content(&self, _app_state: &AppState) -> Result<()> {
        // FIXME probably want to switch to something that supports async_std here
        let url = self
            .base_url()
            .join("$expunge")
            .expect("Error parsing URL.");
        let client = self.client()?;
        let response = client
            .post(url.clone())
            .query(&[("expungeEverything", "true")])
            .send()
            .instrument(trace_span!("POST request", %url))
            .await
            .with_context(|| format!("The POST to '{}' failed.", url))?;

        if !response.status().is_success() {
            warn!(
                url = url.as_str(),
                status = response.status().as_str(),
                "POST failed"
            );
            return Err(eyre!(
                "The POST to '{}' failed, with status '{}'.",
                &url,
                &response.status()
            ));
        }
        // TODO more checks needed
        Ok(())
    }

    #[tracing::instrument(level = "debug", fields(server_name = SERVER_NAME), skip(self))]
    fn shutdown(&self) -> Result<()> {
        let docker_down_output = Command::new("./docker_compose_hapi_jpaserver_starter.sh")
            .args(&["down"])
            .current_dir(&self.server_work_dir)
            .output()
            .context("Failed to run 'docker-compose down'.")?;
        if !docker_down_output.status.success() {
            return Err(eyre!(crate::errors::AppError::ChildProcessFailure(
                docker_down_output.status,
                "Failed to shutdown HAPI FHIR JPA Server via docker-compose.".to_owned(),
                String::from_utf8_lossy(&docker_down_output.stdout).into(),
                String::from_utf8_lossy(&docker_down_output.stderr).into()
            )));
        }

        Ok(())
    }
}

/// Unit tests for the [crate::servers::hapi_jpa] module.
///
/// Note: These aren't exactly typical "unit" tests, as they will launch and shutdown the server via Docker,
/// which is a not-cheap operation.
#[cfg(test)]
mod tests {
    use std::{ffi::OsStr, path::Path};

    #[tracing::instrument(level = "info")]
    #[test_env_log::test(tokio::test)]
    #[serial_test::serial(sample_data)]
    async fn verify_server_launch() {
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
        assert!(
            launch_result.is_ok(),
            "Server launch test failed due to error: {:?}. Log output: {:?}",
            launch_result.unwrap_err(),
            log_target
        );
        if log_target.exists() {
            std::fs::remove_file(log_target).expect("Unable to remove temp file.");
        }
    }
}
