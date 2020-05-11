//! TODO
use super::{ServerHandle, ServerName, ServerPlugin};
use anyhow::{anyhow, Context, Result};
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

impl ServerPlugin for HapiJpaFhirServerPlugin {
    fn server_name(&self) -> &ServerName {
        &self.server_name
    }

    fn launch(&self) -> Result<Box<dyn ServerHandle>> {
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

        Ok(Box::new(HapiJpaFhirServerHandle {}))
    }
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
