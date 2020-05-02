//! Contains the integration tests for this project, which run the benchmarks in various configurations and
//! verify the results.

use assert_cmd::Command;
use fhir_bench_orchestrator::test_framework::FrameworkResults;

/// Runs the benchmarks in their default configuration and verifies the results.
#[test]
fn default_config() {
    let mut cmd = Command::cargo_bin(env!("CARGO_PKG_NAME")).unwrap();
    let output = cmd.unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    println!("STDERR:\n{}", stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("STDOUT:\n{}", stdout);

    assert_eq!(true, output.status.success());
    assert_eq!("", stderr);
    let framework_results: FrameworkResults = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(true, framework_results.completed.is_some());
}