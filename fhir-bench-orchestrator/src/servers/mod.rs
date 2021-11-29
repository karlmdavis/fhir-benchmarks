//! TODO

use crate::{
    config::AppConfig, sample_data::SampleResource,
    servers::docker_compose::DockerComposeServerPlugin, AppState,
};
use async_trait::async_trait;
use eyre::Result;
use serde::{Deserialize, Serialize};
use tracing::info;
use url::Url;

mod docker_compose;

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

impl ServerName {
    /// Converts the [ServerName] to a `&str` representation of it.
    fn as_str(&self) -> &str {
        &self.0
    }
}

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
    /// Return the [ServerPlugin] that this [ServerHandle] is an instance of.
    fn plugin(&self) -> &ServerPluginWrapper;

    /// Return the base URL for the running FHIR server, e.g. `http://localhost:8080/foo/`, which must have a
    /// trailing `/`.
    fn base_url(&self) -> Url;

    /// Returns a [reqwest::Client] that is properly configured for making HTTP(S)] requests to the server,
    /// e.g. it's set to accept self-signed certificates, etc.
    ///
    /// Note: It is strongly suggested that [ServerHandle] implementations cache and re-use the same object
    /// for every call to this method, as this will allow the use of HTTP connection pooling.
    fn client(&self) -> Result<reqwest::Client>;

    /// Creates a new [reqwest::RequestBuilder] that is properly configured for making HTTP(S)]
    /// requests to the server, e.g. authentication headers are set, etc.
    ///
    /// Parameters:
    /// * `client`: the [reqwest::Client] to use
    /// * `method`: the [http::Method] to call
    /// * `url`: the full/absolute [Url] to call
    fn request_builder(
        &self,
        client: reqwest::Client,
        method: http::Method,
        url: Url,
    ) -> reqwest::RequestBuilder {
        request_builder_default(client, method, url)
    }

    /// Returns the full log content from the running FHIR server and its dependencies.
    fn emit_logs(&self) -> Result<String>;

    /// Log the full log content from the running FHIR server and its dependencies at the info level.
    fn emit_logs_info(&self) -> Result<()> {
        info!(
            "Full docker-compose logs for '{}' server:\n{}",
            self.plugin().server_name(),
            self.emit_logs()?
        );

        Ok(())
    }

    /// Clear all content from the server, as if had just been launched with an empty database.
    ///
    /// Parameters:
    /// * `app_state`: the application's [AppState]
    async fn expunge_all_content(&self, app_state: &AppState) -> Result<()>;

    /// TODO
    fn shutdown(&self) -> Result<()>;
}

/// Creates a new [reqwest::Client] that is properly configured for making HTTP(S)] requests to the server,
/// e.g. it's set to accept self-signed certificates, etc.
///
/// Note: this is intended for use in [ServerHandle] implementations; other code should not use it directly,
/// and should instead use [ServerHandle::request_builder()].
pub fn client_default() -> Result<reqwest::Client> {
    let client_builder = reqwest::ClientBuilder::new();

    // Any server using HTTPS will be using a self-signed cert.
    let client_builder = client_builder.danger_accept_invalid_certs(true);

    Ok(client_builder.build()?)
}

/// Creates a new [reqwest::RequestBuilder] that is properly configured for making HTTP(S)]
/// requests to the server, e.g. authentication headers are set, etc.
///
/// Note: this is intended for use in [ServerHandle] implementations; other code should not use it directly,
/// and should instead use [ServerHandle::request_builder()].
///
/// Parameters:
/// * `client`: the [reqwest::Client] to use
/// * `method`: the [http::Method] to call
/// * `url`: the full/absolute [Url] to call
fn request_builder_default(
    client: reqwest::Client,
    method: http::Method,
    url: Url,
) -> reqwest::RequestBuilder {
    client.request(method, url)
}

/// Creates a new [reqwest::RequestBuilder] that is properly configured for making HTTP(S)]
/// requests to the server, e.g. authentication headers are set, etc.
///
/// Note: this is intended for use in [ServerHandle] implementations; other code should not use it directly,
/// and should instead use [ServerHandle::request_builder()].
///
/// Parameters:
/// * `client`: the [reqwest::Client] to use
/// * `method`: the [http::Method] to call
/// * `url`: the full/absolute [Url] to call
pub fn request_builder_ibm_fhir(
    client: reqwest::Client,
    method: http::Method,
    url: Url,
) -> reqwest::RequestBuilder {
    request_builder_default(client, method, url).basic_auth("fhiruser", Some("change-password"))
}

/// [ServerPlugin] implementations each represent a supported FHIR server implementation that can be started
/// and tested.
///
/// Implementations are required to be [Sync](core::marker::Sync), so that they may be used in `async`
/// contexts and otherwise borrowed across threads.
#[async_trait]
pub trait ServerPlugin: Clone {
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

    /// This is an escape hatch of sorts for non-compliant servers, which allows them to fudge/hack sample
    /// data a bit, such that it can be successfully POST'd or whatnot.
    ///
    /// Parameters:
    /// * `sample_org`: a sample `Organization` JSON resource that has been generated to test this server
    fn fudge_sample_resource(&self, sample_resource: SampleResource) -> SampleResource {
        /*
         * Design thoughts:
         * * In general, I'm not a fan of "fixing" noncompliant servers: I'd rather let them fail and have
         *   that reflected in their benchmark results. Sometimes, though, I'm MORE interested in seeing
         *   their performance. This is a tricky balance to strike, and so all such hacks need to be
         *   documented in the project's `doc/server-compliance.md` file.
         */

        /*
         * Most servers are compliant, thankfully, so we provide this default no-op implementation.
         */
        sample_resource
    }
}

/// An enum that wraps all supported [ServerPlugin] trait implementations.
///
/// This design pattern makes it much easier to downcast a given trait object via `let` binding, e.g.:
///
/// ```rust
/// # use fhir_bench_orchestrator::config::AppConfig;
/// # use fhir_bench_orchestrator::servers::{ServerPlugin, ServerPluginWrapper};
/// let config = AppConfig::new().unwrap();
/// let plugins = fhir_bench_orchestrator::servers::create_server_plugins(&config).unwrap();
/// let some_plugin = plugins.first();
///
/// if let Some(ServerPluginWrapper::DockerComposeServerPlugin(hapi_plugin)) = some_plugin {
///   println!("Server plugin name is {}.", hapi_plugin.server_name());
/// }
/// ```
///
/// Personal note: while this feels like a very dumb hack, it makes the app code **so much** nicer. Many,
/// many kudos to <https://bennetthardwick.com/dont-use-boxed-trait-objects-for-struct-internals/> for
/// the idea.
#[derive(Clone, Debug)]
pub enum ServerPluginWrapper {
    DockerComposeServerPlugin(DockerComposeServerPlugin),
}

#[async_trait]
impl ServerPlugin for ServerPluginWrapper {
    fn server_name(&self) -> &ServerName {
        match self {
            ServerPluginWrapper::DockerComposeServerPlugin(server_plugin) => {
                server_plugin.server_name()
            }
        }
    }

    async fn launch(&self, app_state: &AppState) -> Result<Box<dyn ServerHandle>> {
        match self {
            ServerPluginWrapper::DockerComposeServerPlugin(server_plugin) => {
                server_plugin.launch(app_state).await
            }
        }
    }
}

/// Declares (and provides instances of) all of the [ServerPlugin]s that are available to the application.
pub fn create_server_plugins(config: &AppConfig) -> Result<Vec<ServerPluginWrapper>> {
    /*
     * Design note: Why are these wrapped in Arcs? Great question! Each ServerHandle needs an owned copy of
     * them and we can't have the trait extend Copy or Clone, as that would make it not object safe.
     */
    Ok(vec![
        ServerPluginWrapper::DockerComposeServerPlugin(DockerComposeServerPlugin::new(
            "firely_spark".into(),
            config
                .benchmark_dir()?
                .join("server_builds")
                .join("firely_spark")
                .join("docker_compose_firely_spark.sh"),
            Url::parse("http://localhost:5555/fhir/").expect("Unable to parse URL."),
            request_builder_default,
        )),
        ServerPluginWrapper::DockerComposeServerPlugin(DockerComposeServerPlugin::new(
            "hapi_jpaserver_starter".into(),
            config
                .benchmark_dir()?
                .join("server_builds")
                .join("hapi_jpaserver_starter")
                .join("docker_compose_hapi_jpaserver_starter.sh"),
            Url::parse("http://localhost:8080/fhir/").expect("Unable to parse URL."),
            request_builder_default,
        )),
        ServerPluginWrapper::DockerComposeServerPlugin(DockerComposeServerPlugin::new(
            "ibm_fhir".into(),
            config
                .benchmark_dir()?
                .join("server_builds")
                .join("ibm_fhir")
                .join("docker_compose_ibm_fhir.sh"),
            Url::parse("https://localhost:9443/fhir-server/api/v4/").expect("Unable to parse URL."),
            request_builder_ibm_fhir,
        )),
    ])
}
