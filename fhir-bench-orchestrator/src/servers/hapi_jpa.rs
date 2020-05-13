//! TODO
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use crate::AppState;
use anyhow::{anyhow, Context, Result};
use async_trait::async_trait;
use std::process::Command;
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
            .current_dir("server_builds/hapi_fhir_jpaserver")
            .output()
            .context("Failed to run 'docker-compose up'.")?;
        if !docker_up_output.status.success() {
            return Err(anyhow!(crate::errors::AppError::ChildProcessFailure(
                docker_up_output.status,
                "Failed to launch HAPI FHIR JPA Server via docker-compose.".to_owned(),
            )));
        }

        // The server containers have now been started, though they're not necessarily ready yet.
        let server_handle = HapiJpaFhirServerHandle {};

        // Wait (up to a timeout) for the server to be ready.
        wait_for_ready(app_state, &server_handle).await?;

        Ok(Box::new(server_handle))
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
    async_std::future::timeout(std::time::Duration::from_secs(10), async {
        let mut ready = false;
        let mut probe = None;

        while !ready {
            probe = Some(probe_for_ready(app_state, server_handle).await);
            ready = match probe.as_ref().expect("probe result missing") {
                Ok(_) => true,
                Err(_) => false,
            };
        }

        probe.expect("probe results missing")
    })
    .await?
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
    Ok(
        crate::test_framework::metadata::run_operation_metadata_iteration(app_state, probe_url)
            .await?,
    )
}

/// Represents a launched instance of the HAPI FHIR JPA server.
pub struct HapiJpaFhirServerHandle {}

impl ServerHandle for HapiJpaFhirServerHandle {
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
            )));
        }

        Ok(())
    }

    fn base_url(&self) -> url::Url {
        Url::parse("http://localhost:8080/hapi-fhir-jpaserver/fhir/").expect("Unable to parse URL.")
    }
}
