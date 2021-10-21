//! TODO
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use slog::{info, warn};
use std::path::{Path, PathBuf};
use std::{process::Command, sync::Arc};
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

    async fn launch(&self, app_state: &AppState) -> Result<Box<dyn ServerHandle>> {
        let server_work_dir = server_work_dir(&app_state.config.benchmark_dir()?);

        /*
         * Build and launch our submodule'd fork of the sample JPA server.
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
            return Err(anyhow!(crate::errors::AppError::ChildProcessFailure(
                docker_up_output.status,
                "Failed to launch HAPI FHIR JPA Server via docker-compose.".to_owned(),
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
                server_handle.emit_logs_info(&app_state.logger);
                Err(err)
            }
            Ok(_) => Ok(Box::new(server_handle)),
        }
    }
}

/// Returns the work directory to use for the FHIR server.
fn server_work_dir(benchmark_dir: &Path) -> PathBuf {
    benchmark_dir
        .join("server_builds")
        .join("hapi_fhir_jpaserver")
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
        Url::parse("http://localhost:8080/hapi-fhir-jpaserver/fhir/").expect("Unable to parse URL.")
    }

    fn client(&self) -> Result<reqwest::Client> {
        Ok(self.http_client.clone())
    }

    fn emit_logs_info(&self, logger: &slog::Logger) {
        let docker_logs_output = match Command::new("docker-compose")
            .args(&["logs", "--no-color"])
            .current_dir(&self.server_work_dir)
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

    /// Expunge all resources from the server.
    ///
    /// See <https://smilecdr.com/docs/fhir_repository/deleting_data.html#expunge> for details.
    ///
    /// Parameters:
    /// * `app_state`: the application's [AppState]
    /// * `server_handle`: the [ServerHandle] for the server implementation instance being tested
    async fn expunge_all_content(&self, app_state: &AppState) -> Result<()> {
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
            .await
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

    fn shutdown(&self) -> Result<()> {
        let docker_down_output = Command::new("docker-compose")
            .args(&["down"])
            .current_dir(&self.server_work_dir)
            .output()
            .context("Failed to run 'docker-compose down'.")?;
        if !docker_down_output.status.success() {
            return Err(anyhow!(crate::errors::AppError::ChildProcessFailure(
                docker_down_output.status,
                "Failed to shutdown HAPI FHIR JPA Server via docker-compose.".to_owned(),
                String::from_utf8_lossy(&docker_down_output.stdout).into(),
                String::from_utf8_lossy(&docker_down_output.stderr).into()
            )));
        }

        Ok(())
    }
}
