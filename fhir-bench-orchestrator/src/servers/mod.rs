//! TODO

use crate::AppState;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use url::Url;

mod firely_spark;
mod hapi_jpa;

/// Represents the unique name of a FHIR server implementation.
///
/// Instances should generally be constructed from `&' static str`s, like this:
///
/// ```
/// # use fhir_bench_orchestrator::servers::ServerName;
/// static SERVER_NAME: &str = "Very Awesome Server";
/// let server_name: ServerName = SERVER_NAME.into();
/// ```
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

/// [ServerHandle] trait objects represent an instance of a FHIR server implementation that has been
/// started. The trait's methods provide the support required to actually access/use that FHIR server.
///
/// Implementations are required to be [Sync](core::marker::Sync), so that they may be used in `async`
/// contexts and otherwise borrowed across threads.
#[async_trait]
pub trait ServerHandle: Sync {
    /// Return the base URL for the running FHIR server, e.g. `http://localhost:8080/foo/`.
    fn base_url(&self) -> Url;

    /// Write the full log content from the running FHIR server and its dependencies to the
    /// specified [slog::Logger] at the info level.
    ///
    /// Note: This method should not panic. If unable to retrieve the logs, a warning about that
    /// failure should be logged, instead.
    fn emit_logs_info(&self, logger: &slog::Logger);

    /// Clear all content from the server, as if had just been launched with an empty database.
    ///
    /// Parameters:
    /// * `app_state`: the application's [AppState]
    async fn expunge_all_content(&self, app_state: &AppState) -> Result<()>;

    /// TODO
    fn shutdown(&self) -> Result<()>;
}

/// [ServerPlugin] implementations each represent a supported FHIR server implementation that can be started
/// and tested.
///
/// Implementations are required to be [Sync](core::marker::Sync), so that they may be used in `async`
/// contexts and otherwise borrowed across threads.
#[async_trait]
pub trait ServerPlugin: Sync {
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
    ///
    /// Parameters:
    /// * `app_state`: the application's [AppState]
    async fn launch(&self, app_state: &AppState) -> Result<Box<dyn ServerHandle>>;
}

/// Declares (and provides instances of) all of the `ServerPlugin` impls that are available to the
/// application.
pub fn create_server_plugins() -> Result<Vec<Box<dyn ServerPlugin>>> {
    let mut servers: Vec<Box<dyn ServerPlugin>> = vec![];

    servers.push(Box::new(hapi_jpa::HapiJpaFhirServerPlugin::new()));
    servers.push(Box::new(firely_spark::SparkFhirServerPlugin::new()));

    Ok(servers)
}
