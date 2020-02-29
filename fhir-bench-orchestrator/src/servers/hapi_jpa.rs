//! TODO
use super::{ServerHandle, ServerPlugin};
use crate::errors::Result;
use std::process::Command;

/// The trait object for the `ServerPlugin` implementation for the HAPI FHIR JPA server.
pub struct HapiJpaFhirServerPlugin {}

impl HapiJpaFhirServerPlugin {
    /// Constructs a new `HapiJpaFhirServerPlugin` instance.
    pub fn new() -> HapiJpaFhirServerPlugin {
        HapiJpaFhirServerPlugin {}
    }
}

impl ServerPlugin for HapiJpaFhirServerPlugin {
    fn launch(&self) -> Result<Box<dyn ServerHandle>> {
        // Build and launch our submodule'd fork of the sample JPA server.
        let mut docker_up_output = Command::new("docker-compose")
            .args(&["up", "-d", "--build"])
            .current_dir("server_builds/hapi_fhir_jpaserver")
            .output()?;
        if !docker_up_output.status.success() {
            return Err(crate::errors::AppError::ChildProcessFailure(
                docker_up_output.status,
                "Failed to launch HAPI FHIR JPA Server via docker-compose.".to_owned(),
            ));
        }

        let hello = output.stdout;
    }
}

/// Represents a launched instance of the HAPI FHIR JPA server.
pub struct HapiJpaFhirServerHandle {}

impl ServerHandle for HapiJpaFhirServerHandle {}

impl Drop for HapiJpaFhirServerHandle {
    fn drop(&mut self) {
        unimplemented!()
    }
}
