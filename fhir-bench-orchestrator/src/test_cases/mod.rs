use crate::errors::Result;
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use std::collections::HashMap;

/// Stores the set of test results from all server implementations.
pub struct CombinedResults {
    servers_result: HashMap<&'static ServerName, ServerResult>,
}

impl CombinedResults {
    /// Constructs a new `CombinedResults` instance with blank/default entries for the specified set of
    /// `ServerPlugin`s.
    pub fn new(server_plugins: &Vec<Box<dyn ServerPlugin>>) -> CombinedResults {
        let mut combined_results = CombinedResults {
            servers_result: HashMap::new(),
        };
        for server_plugin in server_plugins {
            // Extract the ServerPlugin's name.
            // Note: I only kinda sorta understand what's happening here, so baby-stepping it.
            let server_plugin: &Box<dyn ServerPlugin> = server_plugin;
            let server_plugin: &dyn ServerPlugin = &**server_plugin;
            let server_name = server_plugin.server_name();

            // Add an entry to `CombinedResults` for this plugin.
            combined_results
                .servers_result
                .insert(server_name, ServerResult::new(&server_name));
        }

        combined_results
    }

    /// Returns a mutable reference to the `ServerResult` for the specified `ServerName`.
    pub fn get_mut(&mut self, server_name: &ServerName) -> &mut ServerResult {
        // Note: we unwrap the `Option` automatically as all `ServerName`s should be static constants.
        self.servers_result
            .get_mut(server_name)
            .expect("Unknown server implementation.")
    }

    /// Returns an `Iterator` over the `ServerResult`s.
    pub fn iter(&self) -> impl std::iter::Iterator<Item = &ServerResult> {
        self.servers_result.iter().map(|(_, val)| val)
    }
}

/// Stores the set of test results from a single server implementation.
pub struct ServerResult {
    pub server_name: &'static ServerName,
    pub launch_result: Option<Result<()>>,
    pub test_cases_result: Option<Result<Vec<TestCaseResult>>>,
    pub shutdown_result: Option<Result<()>>,
}

impl ServerResult {
    /// Constructs a new `ServerResult` instance.
    pub fn new(server_name: &'static ServerName) -> ServerResult {
        ServerResult {
            server_name,
            launch_result: None,
            test_cases_result: None,
            shutdown_result: None,
        }
    }
}

/// TODO
pub struct TestCaseResult {
    pub name: String,
    pub problems: Vec<String>,
}

/// TODO
pub fn run_tests(server_handle: &dyn ServerHandle) -> Result<Vec<TestCaseResult>> {
    let mut results = vec![];

    results.push(run_test_metadata(server_handle));

    Ok(results)
}

/// TODO
pub fn run_test_metadata(server_handle: &dyn ServerHandle) -> TestCaseResult {
    let mut test_case_result = TestCaseResult {
        name: "metadata".to_string(),
        problems: vec![],
    };

    let url = server_handle
        .base_url()
        .join("metadata")
        .expect("Error parsing URL.");

    let response = reqwest::blocking::get(url);
    match response {
        Ok(response) => {
            if !response.status().is_success() {
                test_case_result
                    .problems
                    .push(format!("Unexpected response status: {}", response.status()));
            }
            // TODO more checks needed
        }
        Err(err) => {
            test_case_result
                .problems
                .push(format!("Unable to make HTTP request: {}", err));
        }
    };

    test_case_result
}
