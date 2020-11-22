//! Contains the integration tests for this project, which run the benchmarks in various configurations and
//! verify the results.
//!
//! Fun fact about Cargo tests: they capture all STDOUT and STDERR output. If and only if a test case fails,
//! the STDOUT & STDERR will be written out along with the failure.

use assert_cmd::Command;
use fhir_bench_orchestrator::config::{
    ENV_KEY_CONCURRENCY_LEVELS, ENV_KEY_ITERATIONS, ENV_KEY_POPULATION_SIZE,
};
use fhir_bench_orchestrator::test_framework::FrameworkResults;

/// Runs a small version of the benchmarks and verifies the results.
#[test]
fn benchmark_small() {
    // Launch the benchmark suite, just as it would be from a `cargo run` in the project's top directory.
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd
        .current_dir("..")
        .env("RUST_BACKTRACE", "1")
        .env(ENV_KEY_ITERATIONS, "2")
        .env(ENV_KEY_CONCURRENCY_LEVELS, "1,2")
        .env(ENV_KEY_POPULATION_SIZE, "10")
        .timeout(std::time::Duration::from_secs(60 * 5))
        .ok();
    assert!(output.is_ok(), "Failed to run benchmark: '{}'", output.unwrap_err());
    let output = output.unwrap();

    // We want to validate the output from STDOUT and STDERR, so we capture them to `str`s, here.
    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("STDERR:\n{}", stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT:\n{}", stdout);

    // Verify that the bechmarks ran to completion.
    assert_eq!(
        true,
        output.status.success(),
        "benchmark process exited with '{}'",
        output.status
    );
    let framework_results: FrameworkResults = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        framework_results.completed.is_some(),
        "benchmark results not marked completed"
    );

    // Verify the results from each FHIR server that was tested.
    for server_result in framework_results.servers {
        // Verify that the server launched successfully.
        assert!(
            server_result.launch.is_some(),
            "server '{}' launch did not run",
            server_result.server
        );
        if let Some(launch) = server_result.launch {
            assert!(
                launch.outcome.is_ok(),
                "server '{}' launch failed: '{:?}'",
                server_result.server,
                launch.outcome
            )
        }

        // Verify that the server's operations were tested as expected.
        assert!(
            server_result.operations.is_some(),
            "server '{}' operations did not run",
            server_result.server
        );
        if let Some(operations) = server_result.operations {
            for operation in operations {
                assert!(
                    operation.errors.is_empty(),
                    "server '{}' operation '{}' had errors",
                    server_result.server,
                    operation.operation
                );

                for measurement in operation.measurements {
                    // FIXME Remove this check once the framework is more solid. It's not tenable long-term as
                    // some servers will be unstable some of the time and we can't control that.
                    assert_eq!(
                        0, measurement.iterations_failed,
                        "server '{}' operation '{}' had failures",
                        server_result.server, operation.operation
                    );
                }
            }
        }
    }
}
