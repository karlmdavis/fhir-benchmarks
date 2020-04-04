//! TODO
use super::{ServerHandle, ServerName, ServerPlugin};
use crate::errors::Result;
use std::process::Command;
use url::Url;

static SERVER_NAME_TEXT: &str = "HAPI FHIR JPA Server";
static SERVER_NAME: ServerName = ServerName(SERVER_NAME_TEXT);

/// The trait object for the `ServerPlugin` implementation for the HAPI FHIR JPA server.
pub struct HapiJpaFhirServerPlugin {}

impl HapiJpaFhirServerPlugin {
    /// Constructs a new `HapiJpaFhirServerPlugin` instance.
    pub fn new() -> HapiJpaFhirServerPlugin {
        HapiJpaFhirServerPlugin {}
    }
}

impl ServerPlugin for HapiJpaFhirServerPlugin {
    fn server_name(&self) -> &'static ServerName {
        &SERVER_NAME
    }

    fn launch(&self) -> Result<Box<dyn ServerHandle>> {
        // Build and launch our submodule'd fork of the sample JPA server.
        let docker_up_output = Command::new("docker-compose")
            .args(&["up", "--detach", "--build"])
            .current_dir("server_builds/hapi_fhir_jpaserver")
            .output()?;
        if !docker_up_output.status.success() {
            return Err(crate::errors::AppError::ChildProcessFailure(
                docker_up_output.status,
                "Failed to launch HAPI FHIR JPA Server via docker-compose.".to_owned(),
            ));
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
            .output()?;
        if !docker_down_output.status.success() {
            return Err(crate::errors::AppError::ChildProcessFailure(
                docker_down_output.status,
                "Failed to shutdown HAPI FHIR JPA Server via docker-compose.".to_owned(),
            ));
        }

        Ok(())
    }

    fn base_url(&self) -> url::Url {
        Url::parse("http://localhost:8080/hapi-fhir-jpaserver/fhir/").expect("Unable to parse URL.")
    }
}
