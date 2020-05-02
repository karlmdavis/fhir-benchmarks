//! TODO

use anyhow::Result;
use serde::{Deserialize,Serialize};
use url::Url;

mod hapi_jpa;

/// Represents the unique name of a FHIR server implementation.
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct ServerName(pub String);

impl std::fmt::Display for ServerName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<&str> for ServerName {
    fn from(server_name: &str) -> Self {
        ServerName(server_name.to_owned())
    }
}

/// TODO
pub trait ServerHandle {
    /// Return the base URL for the running FHIR server, e.g. `http://localhost:8080/foo/`.
    fn base_url(&self) -> Url;

    /// TODO
    fn shutdown(&self) -> Result<()>;
}

/// TODO
pub trait ServerPlugin {
    /// Returns the unique `ServerName` for this `ServerPlugin`.
    fn server_name(&self) -> &ServerName;

    /// Implementations of this method must launch an instance of the FHIR server implementation, including
    /// all necessary configuration to get the server ready for use. Implementations of this method must
    /// **not** load any data; FHIR searches against newly-launched servers should return no results.
    ///
    /// If the launch operation fails for any reason, implementations **must** still return a `Result::Err` after
    /// terminating any server processes and cleaning up all resources used by the server, just as if the
    /// server had been launched and the `ServerHandle::shutdown()` method was called. This is essential in
    /// order to ensure that a failed launch of one server does not impair the launch and testing of other
    /// server implementations.
    fn launch(&self) -> Result<Box<dyn ServerHandle>>;
}

/// Declares (and provides instances of) all of the `ServerPlugin` impls that are available to the
/// application.
pub fn create_server_plugins() -> Result<Vec<Box<dyn ServerPlugin>>> {
    let mut servers: Vec<Box<dyn ServerPlugin>> = vec![];

    servers.push(Box::new(hapi_jpa::HapiJpaFhirServerPlugin::new()));

    Ok(servers)
}
