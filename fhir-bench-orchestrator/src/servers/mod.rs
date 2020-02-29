//! TODO

use crate::errors::Result;

mod hapi_jpa;

/// TODO
/// Note: Once the handle goes out of scope, the drop will be invoked and this will need to shut down and clean up the server.
pub trait ServerHandle: Drop {}

/// TODO
pub trait ServerPlugin {
    /// TODO
    fn launch(&self) -> Result<Box<dyn ServerHandle>>;
}

/// Declares (and provides instances of) all of the `ServerPlugin` impls that are available to the
/// application.
pub fn create_server_plugins() -> Result<Vec<Box<dyn ServerPlugin>>> {
    let mut servers: Vec<Box<dyn ServerPlugin>> = vec![];

    servers.push(Box::new(hapi_jpa::HapiJpaFhirServerPlugin::new()));

    Ok(servers)
}
