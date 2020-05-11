//! Contains the integration tests for this project, which run the benchmarks in various configurations and
//! verify the results.
//!
//! Fun fact about Cargo tests: they capture all STDOUT and STDERR output. If and only if a test case fails,
//! the STDOUT & STDERR will be written out along with the failure.

use assert_cmd::Command;
use fhir_bench_orchestrator::test_framework::FrameworkResults;

/// Runs the benchmarks in their default configuration and verifies the results.
#[test]
fn default_config() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd.current_dir("..").unwrap();

    // We want to validate the output from STDOUT and STDERR, so we capture them to `str`s, here.
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("STDERR:\n{}", stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT:\n{}", stdout);

    // Verify that the bechmarks ran to completion.
    assert_eq!(true, output.status.success(), "benchmark process exited with '{}'", output.status);
    assert!(stderr.is_empty(), "benchmark process had STDERR output");
    let framework_results: FrameworkResults = serde_json::from_slice(&output.stdout).unwrap();
    assert!(framework_results.completed.is_some(), "benchmark results not marked completed");

    // Verify the results from each FHIR server that was tested.
    for server_result in framework_results.servers {
        // Verify that the server launched successfully.
        assert!(server_result.launch.is_some(), "server '{}' launch did not run", server_result.server);
        if let Some(launch) = server_result.launch {
            assert!(launch.outcome.is_ok(), "server '{}' launch failed: '{:?}'", server_result.server, launch.outcome)
        }

        // Verify that the server's operations were tested as expected.
        assert!(server_result.operations.is_some(), "server '{}' operations did not run", server_result.server);
        if let Some(operations) = server_result.operations {
            for operation in operations {
                // FIXME Remove this check once the framework is more solid. It's not tenable long-term as
                // some servers will be unstable some of the time and we can't control that.
                assert_eq!(Some(0), operation.failures, "server '{}' operation '{}' had failures",
                    server_result.server, operation.operation);
            }
        }
    }
}