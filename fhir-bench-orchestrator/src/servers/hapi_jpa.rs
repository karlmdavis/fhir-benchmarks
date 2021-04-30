//! TODO
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use slog::{info, warn};
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
            .current_dir(
                app_state
                    .config
                    .benchmark_dir()?
                    .join("server_builds/hapi_fhir_jpaserver"),
            )
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
            .server_plugins
            .iter()
            .find(|p| p.server_name().0 == SERVER_NAME)
            .expect("Unable to find server plugin")
            .clone();
        let server_handle = HapiJpaFhirServerHandle { server_plugin };

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

/// Represents a launched instance of the HAPI FHIR JPA server.
pub struct HapiJpaFhirServerHandle {
    server_plugin: Arc<dyn ServerPlugin>,
}

#[async_trait]
impl ServerHandle for HapiJpaFhirServerHandle {
    fn plugin(&self) -> Arc<dyn ServerPlugin> {
        self.server_plugin.clone()
    }

    fn base_url(&self) -> url::Url {
        Url::parse("http://localhost:8080/hapi-fhir-jpaserver/fhir/").expect("Unable to parse URL.")
    }

    fn emit_logs_info(&self, logger: &slog::Logger) {
        let docker_logs_output = match Command::new("docker-compose")
            .args(&["logs", "--no-color"])
            .current_dir("server_builds/hapi_fhir_jpaserver")
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

    fn shutdown(&self) -> Result<()> {
        let docker_down_output = Command::new("docker-compose")
            .args(&["down"])
            .current_dir("server_builds/hapi_fhir_jpaserver")
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
