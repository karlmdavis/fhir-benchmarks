//! TODO

mod errors;
mod servers;
mod test_cases;

use crate::errors::Result;
use crate::servers::{ServerHandle, ServerPlugin};
use crate::test_cases::CombinedResults;

#[async_std::main]
async fn main() -> Result<()> {
    // Parse command line args.
    // TODO

    // Find all FHIR server implementations that can be tested.
    let server_plugins: Vec<Box<dyn ServerPlugin>> = servers::create_server_plugins()?;

    // Setup all global/shared resources.
    let shared_resources = create_shared_resources();

    // Test each selected FHIR server implementation.
    let mut combined_results = CombinedResults::new(&server_plugins);
    for server_plugin in &server_plugins {
        // Extract a reference to the ServerPlugin.
        // Note: I only kinda sorta understand what's happening here, so baby-stepping it.
        let server_plugin: &Box<dyn ServerPlugin> = server_plugin;
        let server_plugin: &dyn ServerPlugin = &**server_plugin;

        // Store results for the test here.
        let mut server_result = combined_results.get_mut(server_plugin.server_name());

        // Launch the implementation's server, etc. This will likely take a while.
        let launch_result = server_plugin.launch();

        // Destructure the launch result into success and failure objects, so they have separate ownership.
        let (server_handle, launch_error) = match launch_result {
            Ok(server_handle) => (Some(server_handle), None),
            Err(launch_error) => (None, Some(launch_error)),
        };

        // Store the launch result's success/error for the records.
        server_result.launch_result = match launch_error {
            Some(launch_error) => Some(Err(launch_error)),
            None => Some(Ok(())),
        };

        // If the server launched successfully, move on to testing it and then shutting it down.
        if server_result.launch_result.as_ref().unwrap().is_ok() {
            let server_handle: &dyn ServerHandle = &*server_handle.unwrap();

            // Run the tests against the server.
            let test_cases_result = test_cases::run_tests(server_handle);
            server_result.test_cases_result = Some(test_cases_result);

            // Shutdown and cleanup the server and its resources.
            let shutdown_result = server_handle.shutdown();
            server_result.shutdown_result = Some(shutdown_result);
        }
    }

    // Print results.
    print_results(&combined_results);

    Ok(())
}

/// Initialize the application resources (e.g. test data) that will be used across projects.
fn create_shared_resources() -> () {
    // TODO
}

/// Print all of the results to STDOUT.
fn print_results(combined_results: &CombinedResults) -> () {
    for server_result in combined_results.iter() {
        println!("{}", server_result.server_name);
    }
}
