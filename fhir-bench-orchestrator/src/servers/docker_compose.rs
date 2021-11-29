//! Provides utility functions for [ServerPlugin] and [ServerHandle] implementations that use Docker Compose.
//!
//! These functions all assume that the server has a dedicated directory, which contains a custom shell
//! script that wraps `docker-compose` with any setup, environment variables, etc. needed to run things
//! correctly for that FHIR server.

use super::ServerPluginWrapper;
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use crate::AppState;
use async_trait::async_trait;
use eyre::{eyre, Context, Result};
use std::ffi::OsStr;
use std::fmt::Debug;
use std::path::PathBuf;
use std::process::Command;
use std::process::Output;
use url::Url;

/// Each instance of this struct represents a particular FHIR Server implementation, where the implementation
/// is launched and managed via Docker Compose.
#[derive(Clone, Debug)]
pub struct DockerComposeServerPlugin {
    server_name: ServerName,
    server_script: PathBuf,
    base_url: Url,
    request_builder_factory:
        fn(client: reqwest::Client, method: http::Method, url: Url) -> reqwest::RequestBuilder,
}

impl DockerComposeServerPlugin {
    /// Returns the [PathBuf] to the `docker compose` wrapper script for this server.
    fn server_script(&self) -> PathBuf {
        self.server_script.clone()
    }

    /// Returns the base [Url] that the server will use, once launched.
    fn base_url(&self) -> &Url {
        &self.base_url
    }
}

impl DockerComposeServerPlugin {
    /// Constructs a new `DockerComposeServerPlugin` instance that will represent a particular FHIR Server
    /// implementation.
    ///
    /// Parameters:
    /// * `server_name`: the [ServerName] that will uniquely identify the FHIR Server implemenation
    /// * `server_script`: a [PathBuf] to the shell script that wraps the `docker compose` command for this
    ///   particular FHIR Server implementation
    /// * `base_url`: the base [Url] that should be used for all requests to the FHIR Server, once launched
    /// * `request_builder_factory`: a function that can produce the [reqwest::RequestBuilder] to use when
    ///   querying the FHIR Server, once launched
    pub fn new(
        server_name: ServerName,
        server_script: PathBuf,
        base_url: Url,
        request_builder_factory: fn(
            client: reqwest::Client,
            method: http::Method,
            url: Url,
        ) -> reqwest::RequestBuilder,
    ) -> DockerComposeServerPlugin {
        DockerComposeServerPlugin {
            server_name,
            server_script,
            base_url,
            request_builder_factory,
        }
    }
}

#[async_trait]
impl ServerPlugin for DockerComposeServerPlugin {
    fn server_name(&self) -> &ServerName {
        &self.server_name
    }

    async fn launch(&self, app_state: &AppState) -> Result<Box<dyn ServerHandle>> {
        launch_server(app_state, self).await
    }
}

/// Runs the specified Docker Compose subcommand with the specified argument, for the specified FHIR Server
/// implementation.
///
/// Parameters:
/// * `server_plugin`: the [DockerComposeServerPlugin] that represents the FHIR Server implementation to run
///   the command for/against
/// * `args`: the Docker Compose subcommand and options to run, e.g. `["up", "--detach"]`
#[tracing::instrument(level = "info", skip(server_plugin))]
fn run_docker_compose<I, S>(server_plugin: &DockerComposeServerPlugin, args: I) -> Result<Output>
where
    I: IntoIterator<Item = S> + Debug,
    S: AsRef<OsStr>,
{
    /*
     * Build and launch the FHIR server.
     */
    let docker_compose_output = Command::new(server_plugin.server_script())
        .args(args)
        .output()
        .with_context(|| {
            format!(
                "Error returned by control command for the '{}' FHIR server.",
                server_plugin.server_name()
            )
        })?;
    if !docker_compose_output.status.success() {
        return Err(eyre!(crate::errors::AppError::ChildProcessFailure(
            docker_compose_output.status,
            format!(
                "Error returned by control command for the '{}' FHIR server.",
                server_plugin.server_name()
            ),
            String::from_utf8_lossy(&docker_compose_output.stdout).into(),
            String::from_utf8_lossy(&docker_compose_output.stderr).into()
        )));
    }

    Ok(docker_compose_output)
}

/// Launches the server, producing a boxed [SparkFhirServerHandle].
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_plugin`: the [DockerComposeServerPlugin] for the server to launch
async fn launch_server(
    app_state: &AppState,
    server_plugin: &DockerComposeServerPlugin,
) -> Result<Box<dyn ServerHandle>> {
    /*
     * Build and launch the server.
     */
    run_docker_compose(server_plugin, &["up", "--detach"]).with_context(|| {
        format!(
            "Running '{} up --detach' failed.",
            server_plugin
                .server_script()
                .file_name()
                .expect("Unable to get control script name.")
                .to_string_lossy()
        )
    })?;

    /*
     * The server containers have now been started, though they're not necessarily ready yet. Build a
     * handle for it, copying any fields from the plugin that will be needed (as we can't safely downcast
     * the plugin, so this is the only way to have access to those fields from the handle).
     */
    let server_plugin = app_state
        .find_server_plugin(server_plugin.server_name().as_str())
        .expect("Unable to find server plugin");
    let http_client = super::client_default()?;
    let server_handle = DockerComposeServerHandle {
        server_plugin: server_plugin.clone(),
        http_client,
    };

    // Wait (up to a timeout) for the server to be ready.
    match wait_for_ready(app_state, &server_handle).await {
        Err(err) => {
            server_handle.emit_logs_info()?;
            Err(err)
        }
        Ok(_) => {
            let server_handle: Box<dyn ServerHandle> = Box::new(server_handle);
            Ok(server_handle)
        }
    }
}

/// Checks the specified server repeatedly to see if it is ready, up to a hardcoded timeout.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the [DockerComposeServerPlugin] to test
///
/// Returns an empty [Result], where an error indicates that the server was not ready.
#[tracing::instrument(level = "debug", skip(app_state, server_handle))]
async fn wait_for_ready(
    app_state: &AppState,
    server_handle: &DockerComposeServerHandle,
) -> Result<()> {
    let probe_result = tokio::time::timeout(std::time::Duration::from_secs(60 * 5), async {
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
            server_handle.plugin().server_name()
        )
    })?;

    match probe_result {
        Err(err) => {
            server_handle.emit_logs_info()?;
            Err(err)
        }
        Ok(_) => Ok(()),
    }
}

/// Represents a running instance of a [DockerComposeServerPlugin] instance.
struct DockerComposeServerHandle {
    server_plugin: ServerPluginWrapper,
    http_client: reqwest::Client,
}

#[async_trait]
impl ServerHandle for DockerComposeServerHandle {
    fn plugin(&self) -> &ServerPluginWrapper {
        &self.server_plugin
    }

    fn base_url(&self) -> url::Url {
        let server_plugin = server_plugin_downcast(self);
        server_plugin.base_url().clone()
    }

    fn client(&self) -> Result<reqwest::Client> {
        Ok(self.http_client.clone())
    }

    fn request_builder(
        &self,
        client: reqwest::Client,
        method: http::Method,
        url: Url,
    ) -> reqwest::RequestBuilder {
        let server_plugin = server_plugin_downcast(self);
        (server_plugin.request_builder_factory)(client, method, url)
    }

    fn emit_logs(&self) -> Result<String> {
        let server_plugin = server_plugin_downcast(self);
        match run_docker_compose(server_plugin, &["logs", "--no-color"]).with_context(|| {
            format!(
                "Running '{} up --detach' failed.",
                server_plugin
                    .server_script()
                    .file_name()
                    .expect("Unable to get control script name.")
                    .to_string_lossy()
            )
        }) {
            Ok(output) => Ok(String::from_utf8_lossy(&output.stdout).to_string()),
            Err(err) => Err(err),
        }
    }

    #[tracing::instrument(level = "debug", skip(self, app_state))]
    async fn expunge_all_content(&self, app_state: &AppState) -> Result<()> {
        self.shutdown()?;
        let server_plugin = server_plugin_downcast(self);
        launch_server(app_state, server_plugin).await?;
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    fn shutdown(&self) -> Result<()> {
        let server_plugin = server_plugin_downcast(self);

        let docker_down_output =
            run_docker_compose(server_plugin, &["down"]).with_context(|| {
                format!(
                    "Running '{} down' failed.",
                    server_plugin
                        .server_script()
                        .file_name()
                        .expect("Unable to get control script name.")
                        .to_string_lossy()
                )
            })?;
        if !docker_down_output.status.success() {
            return Err(eyre!(crate::errors::AppError::ChildProcessFailure(
                docker_down_output.status,
                format!(
                    "Failed to shutdown '{}' via Docker Compose.",
                    server_plugin.server_name()
                ),
                String::from_utf8_lossy(&docker_down_output.stdout).into(),
                String::from_utf8_lossy(&docker_down_output.stderr).into()
            )));
        }

        Ok(())
    }
}

/// Extract the downcast [DockerComposeServerPlugin] from the specified [DockerComposeServerHandle].
fn server_plugin_downcast(server_handle: &DockerComposeServerHandle) -> &DockerComposeServerPlugin {
    match &server_handle.server_plugin {
        ServerPluginWrapper::DockerComposeServerPlugin(server_plugin) => server_plugin,
        #[allow(unreachable_patterns)]
        _ => panic!("Unsupported downcast attempt."),
    }
}
