//! TODO

mod errors;
mod servers;
mod test_cases;

use crate::errors::Result;
use servers::ServerPlugin;
use std::cell::Cell;

fn main() -> Result<()> {
    // Parse command line args.
    // TODO

    // Find all FHIR server implementations that can be tested.
    let mut server_plugins: Vec<Box<dyn ServerPlugin>> = servers::create_server_plugins()?;

    // Setup all global/shared resources.
    let shared_resources = create_shared_resources();

    // Test each selected FHIR server implementation.
    let mut test_results = vec![];
    for server_plugin in server_plugins {
        // Extract a mutable reference to the ServerPlugin for the implementation.
        // Note: I only kinda sorta understand what's happening here, so baby-stepping it.
        let server_plugin: Box<dyn ServerPlugin> = server_plugin;
        let server_plugin: &dyn ServerPlugin = &*server_plugin;

        // Launch the implementation's server, etc. This will likely take a while.
        let server_handle = server_plugin.launch()?;
        // Yay! It worked.

        // Run the tests against the server.
        let server_test_result = test_cases::run_tests();
        test_results.push(server_test_result);

        // Note: As we exit this block, server_handle will go out of scope, get dropped, and so the server
        // resources will get shutdown and cleaned up.
    }

    // Print results.
    // TODO

    Ok(())
}

/// Initialize the application resources (e.g. test data) that will be used across projects.
fn create_shared_resources() -> () {
    // TODO
}
