//! Contains the `run_operations(...)` method and result types for the benchmark test framework.

use crate::config::AppConfig;
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use crate::AppState;
use anyhow::Result;
use chrono::prelude::*;
use chrono::Duration;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use slog_derive::SerdeValue;

pub mod metadata;
mod post_org;
mod serde_duration;

/// Stores the complete set of results from a run of the framework.
#[derive(Clone, Deserialize, SerdeValue, Serialize)]
pub struct FrameworkResults {
    /// When the benchmark framework run started, in wall clock time.
    pub started: DateTime<Utc>,

    /// When the benchmark framework run started, in wall clock time.
    pub completed: Option<DateTime<Utc>>,

    /// The configuration that the benchmark framework was run with.
    pub config: AppConfig,

    /// The [ServerResult] for each of the supported FHIR servers that a benchmark was attempted for.
    pub servers: Vec<ServerResult>,
}

impl FrameworkResults {
    /// Constructs a new `FrameworkResults` instance.
    ///
    /// Params:
    /// * `config`: the application's configuration
    /// * `server_plugins`: the set of [ServerPlugin]s representing the supported FHIR server implementations
    pub fn new(config: &AppConfig, server_plugins: &[Box<dyn ServerPlugin>]) -> FrameworkResults {
        FrameworkResults {
            started: Utc::now(),
            completed: None,
            config: config.clone(),
            servers: server_plugins
                .iter()
                .map(|p| p.server_name())
                .map(|n| ServerResult::new(n.clone()))
                .collect(),
        }
    }

    /// Returns the `ServerResult` for the specified `ServerName`.
    pub fn get_mut(&mut self, server_name: &ServerName) -> Option<&mut ServerResult> {
        self.servers.iter_mut().find(|s| s.server == *server_name)
    }
}

/// Stores the set of test results from a single server implementation.
#[derive(Deserialize, SerdeValue, Clone, Serialize)]
pub struct ServerResult {
    /// The name of the FHIR server that this [ServerResult] is for.
    pub server: ServerName,

    /// A [FrameworkOperationLog] instance detailing the results of the attempt to launch the FHIR server.
    pub launch: Option<FrameworkOperationLog>,

    /// The [ServerOperationLog]s detailing the benchmarking results of each FHIR server operation that was
    /// tested.
    pub operations: Option<Vec<ServerOperationLog>>,

    /// A [FrameworkOperationLog] instance detailing the results of the attempt to launch the FHIR server.
    pub shutdown: Option<FrameworkOperationLog>,
}

impl ServerResult {
    /// Constructs a new `ServerResult` instance.
    pub fn new(server: ServerName) -> ServerResult {
        ServerResult {
            server,
            launch: None,
            operations: None,
            shutdown: None,
        }
    }
}

/// Records the outcomes of a framework operation.
#[derive(Debug, Deserialize, SerdeValue, Clone, Serialize)]
pub struct FrameworkOperationLog {
    /// When the operation was started, in wall clock time.
    pub started: DateTime<Utc>,

    /// When the operation completed, in wall clock time.
    pub completed: DateTime<Utc>,

    /// Details the success or failure of the operation.
    pub outcome: FrameworkOperationResult,
}

impl FrameworkOperationLog {
    /// Returns `true` if `FrameworkOperationLog.outcome` is `FrameworkOperationResult::Ok`, or `false`
    /// otherwise.
    pub fn is_ok(&self) -> bool {
        match self.outcome {
            FrameworkOperationResult::Ok() => true,
            FrameworkOperationResult::Errs(_) => false,
        }
    }
}

/// Eunmerates the success vs. failure outcomes of a framework operation.
#[derive(Debug, Deserialize, SerdeValue, Clone, Serialize)]
pub enum FrameworkOperationResult {
    /// Indicates that, as best as can be told, the framwork operation succeeded.
    Ok(),

    /// Indicates that a framework operation failed, and includes the related error messages.
    Errs(Vec<String>),
}

impl FrameworkOperationResult {
    /// Returns `true` if this `FrameworkOperationResult` is `FrameworkOperationResult::Ok`, or `false`
    /// otherwise.
    pub fn is_ok(&self) -> bool {
        match self {
            FrameworkOperationResult::Ok() => true,
            FrameworkOperationResult::Errs(_) => false,
        }
    }
}

/// Models the results of trying to benchmark a specific server implementation for the specified
/// [ServerOperationName].
#[derive(Deserialize, SerdeValue, Clone, Serialize)]
pub struct ServerOperationLog {
    /// The name of the operation that was benchmarked.
    pub operation: ServerOperationName,

    /// The error message of any problems that halted the benchmark attempt early. Generally, if this is not
    /// empty, then the number of skipped iterations in some/all of the measurements will be non-zero and some expected
    /// `measurements` entries may be missing.
    pub errors: Vec<String>,

    /// The benchmark runs/measurements made at various levels of concurrency.
    pub measurements: Vec<ServerOperationMeasurement>,
}

impl ServerOperationLog {
    /// Constructs a new [ServerOperationLog] for the specified [ServerOperationName].
    pub fn new(operation: ServerOperationName) -> ServerOperationLog {
        ServerOperationLog {
            operation,
            errors: vec![],
            measurements: vec![],
        }
    }
}

/// Models the measurement attempts made for a [ServerOperationLog] at a particular level of concurrency.
#[derive(Deserialize, SerdeValue, Clone, Serialize)]
pub struct ServerOperationMeasurement {
    /// The number of concurrent users' worth of load that the benchmark attempted to generate.
    pub concurrent_users: u32,

    /// When this measurement attempt started, in wall-clock time.
    pub started: DateTime<Utc>,

    /// When this measurement attempt completed, in wall-clock time.
    pub completed: DateTime<Utc>,

    /// How long this measurement attempt spent running iterations, which excludes more general setup work,
    /// such as pushing the data to test against to the server implementation. (It will still include some
    /// miscellaneous overhead from each iteration.)
    #[serde(with = "serde_duration")]
    pub execution_duration: Duration,

    /// The number of iterations that failed to produce the expected result.
    pub iterations_failed: u32,

    /// The number of iterations that were skipped due to problems that halte the benchmark attempt early.
    pub iterations_skipped: u32,

    /// The [ServerOperationMetrics] for the measurement attempt, which will be present if at least one
    /// iteration was successful.
    pub metrics: Option<ServerOperationMetrics>,
}

/// Represents the unique name of a FHIR server operation that this framework tests.
///
/// Instances should generally be constructed from `&' static str`s, like this:
///
/// ```
/// # use fhir_bench_orchestrator::test_framework::ServerOperationName;
/// static SERVER_OP_NAME: &str = "Very Awesome Server";
/// let server_op_name: ServerOperationName = SERVER_OP_NAME.into();
/// ```
#[derive(Deserialize, SerdeValue, Clone, Serialize)]
pub struct ServerOperationName(pub String);

impl From<&str> for ServerOperationName {
    fn from(server_operation_name: &str) -> Self {
        ServerOperationName(server_operation_name.to_owned())
    }
}

impl std::fmt::Display for ServerOperationName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Details the performance of a single server operation, across all iterations (including any failures).
#[derive(Deserialize, SerdeValue, Clone, Serialize)]
pub struct ServerOperationMetrics {
    pub throughput_rpm: Decimal,
    pub latency_mean: Decimal,
    pub latency_p50: Decimal,
    pub latency_p90: Decimal,
    pub latency_p99: Decimal,
    pub latency_p999: Decimal,
    pub latency_p100: Decimal,
    pub latency_hdr_histogram: String, // FIXME type
}

/// Runs the benchmark framework to test the supported operations for the specified FHIR server.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the [ServerHandle] for the server implementation to be tested
///
/// Returns the [ServerOperationLog]s for each operation that was tested.
pub async fn run_operations(
    app_state: &AppState,
    server_handle: &dyn ServerHandle,
) -> Result<Vec<ServerOperationLog>> {
    let mut results = vec![];

    results.push(metadata::benchmark_operation_metadata(app_state, server_handle).await);
    results.push(post_org::benchmark_post_org(app_state, server_handle).await);

    Ok(results)
}

/// Unit tests for the test case structures, etc.
///
/// Note: these tests will all fail unless the `serde_json` crate has the `preserve_order` feature enabled,
/// as otherwise serde serialization does not preserve field order.
#[cfg(test)]
mod tests {
    use crate::config::AppConfig;
    use crate::test_framework::{
        FrameworkOperationLog, FrameworkOperationResult, FrameworkResults, ServerOperationLog,
        ServerOperationMeasurement, ServerOperationMetrics, ServerResult,
    };
    use chrono::prelude::*;
    use chrono::Duration;
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::str::FromStr;

    static SERVER_NAME_FAKE: &str = "Fake HAPI";
    static SERVER_OP_NAME_FAKE: &str = "Operation A";

    /// Verifies that `FrameworkOperationLog` serializes as expected.
    #[test]
    fn serialize_framework_operation_log() {
        let expected = json!({
            "started": "2020-01-01T13:00:00Z",
            "completed": "2020-01-01T14:00:00Z",
            "outcome": {
                "Ok": []
            }
        });
        let expected = serde_json::to_string(&expected).unwrap();
        let actual: FrameworkOperationLog = FrameworkOperationLog {
            started: Utc.ymd(2020, 1, 1).and_hms(13, 0, 0),
            completed: Utc.ymd(2020, 1, 1).and_hms(14, 0, 0),
            outcome: FrameworkOperationResult::Ok(),
        };
        let actual = serde_json::to_string(&actual).unwrap();
        assert_eq!(expected, actual);
    }

    /// Verifies that `ServerOperationMetrics` serializes as expected.
    #[test]
    fn serialize_server_operation_metrics() {
        let expected = json!({
            "throughput_rpm": 42.0,
            "latency_mean": 1.0,
            "latency_p50": 1.0,
            "latency_p90": 1.0,
            "latency_p99": 1.0,
            "latency_p999": 1.0,
            "latency_p100": 1.0,
            // A Base64 encoding of the HDR Histogram data.
            "latency_hdr_histogram": "SGVsbG8sIFdvcmxk"
        });
        let expected = serde_json::to_string(&expected).unwrap();
        let actual = ServerOperationMetrics {
            throughput_rpm: Decimal::from_str("42.0").unwrap(),
            latency_mean: Decimal::from_str("1.0").unwrap(),
            latency_p50: Decimal::from_str("1.0").unwrap(),
            latency_p90: Decimal::from_str("1.0").unwrap(),
            latency_p99: Decimal::from_str("1.0").unwrap(),
            latency_p999: Decimal::from_str("1.0").unwrap(),
            latency_p100: Decimal::from_str("1.0").unwrap(),
            // FIXME This should be an actual Histogram struct.
            latency_hdr_histogram: "SGVsbG8sIFdvcmxk".into(),
        };
        let actual = serde_json::to_string(&actual).unwrap();
        assert_eq!(expected, actual);
    }

    /// Verifies that `ServerOperationLog` serializes as expected.
    #[test]
    fn serialize_server_operation_log() {
        let expected = json!({
            "operation": "Operation A",
            "errors": [],
            "measurements": [],
        });
        let expected = serde_json::to_string(&expected).unwrap();
        let actual = ServerOperationLog {
            operation: SERVER_OP_NAME_FAKE.into(),
            errors: vec![],
            measurements: vec![],
        };
        let actual = serde_json::to_string(&actual).unwrap();
        assert_eq!(expected, actual);
    }

    /// Verifies that [ServerOperationMeasurement] serializes as expected.
    #[test]
    fn serialize_server_operation_measurement() {
        let expected = json!({
            "concurrent_users": 10,
            "started": "2020-01-01T15:00:00Z",
            "completed": "2020-01-01T16:00:00Z",
            "execution_duration": "PT1.234S",
            "iterations_failed": 1,
            "iterations_skipped": 0,
            "metrics": null,
        });
        let expected = serde_json::to_string(&expected).unwrap();
        let actual = ServerOperationMeasurement {
            concurrent_users: 10,
            started: Utc.ymd(2020, 1, 1).and_hms(15, 0, 0),
            completed: Utc.ymd(2020, 1, 1).and_hms(16, 0, 0),
            execution_duration: Duration::nanoseconds(super::serde_duration::NANOS_PER_SEC + 234),
            iterations_failed: 1,
            iterations_skipped: 0,
            metrics: None,
        };
        let actual = serde_json::to_string(&actual).unwrap();
        assert_eq!(expected, actual);
    }

    /// Verifies that `FrameworkResults` serializes as expected.
    #[test]
    fn serialize_framework_results() {
        let expected = json!({
            "started": "2020-01-01T12:00:00Z",
            "completed": "2020-01-01T19:00:00Z",
            "config": {
                "iterations": 1,
                "concurrency_levels": [1, 10],
                "population_size": 1,
            },
            "servers": [{
                "server": "Fake HAPI",
                "launch": {
                    "started": "2020-01-01T13:00:00Z",
                    "completed": "2020-01-01T14:00:00Z",
                    "outcome": {
                        "Ok": []
                    }
                },
                "operations": [
                    {
                        "operation": "Operation A",
                        "errors": [],
                        "measurements": [{
                            "concurrent_users": 10,
                            "started": "2020-01-01T15:00:00Z",
                            "completed": "2020-01-01T16:00:00Z",
                            "execution_duration": "PT1.234S",
                            "iterations_failed": 1,
                            "iterations_skipped": 0,
                            "metrics": {
                                "throughput_rpm": 42.0,
                                "latency_mean": 1.0,
                                "latency_p50": 1.0,
                                "latency_p90": 1.0,
                                "latency_p99": 1.0,
                                "latency_p999": 1.0,
                                "latency_p100": 1.0,
                                // A Base64 encoding of the HDR Histogram data.
                                "latency_hdr_histogram": "SGVsbG8sIFdvcmxk"
                            }
                        }]
                    }
                ],
                "shutdown": {
                    "started": "2020-01-01T17:00:00Z",
                    "completed": "2020-01-01T18:00:00Z",
                    "outcome": {
                        "Ok": []
                    }
                }
            }
        ]});
        let expected = serde_json::to_string(&expected).unwrap();
        let actual = FrameworkResults {
            started: Utc.ymd(2020, 1, 1).and_hms(12, 0, 0),
            completed: Some(Utc.ymd(2020, 1, 1).and_hms(19, 0, 0)),
            config: AppConfig {
                iterations: 1,
                concurrency_levels: vec![1, 10],
                population_size: 1,
            },
            servers: vec![ServerResult {
                server: SERVER_NAME_FAKE.into(),
                launch: Some(FrameworkOperationLog {
                    started: Utc.ymd(2020, 1, 1).and_hms(13, 0, 0),
                    completed: Utc.ymd(2020, 1, 1).and_hms(14, 0, 0),
                    outcome: FrameworkOperationResult::Ok(),
                }),
                operations: Some(vec![ServerOperationLog {
                    operation: SERVER_OP_NAME_FAKE.into(),
                    errors: vec![],
                    measurements: vec![ServerOperationMeasurement {
                        concurrent_users: 10,
                        started: Utc.ymd(2020, 1, 1).and_hms(15, 0, 0),
                        completed: Utc.ymd(2020, 1, 1).and_hms(16, 0, 0),
                        execution_duration: Duration::nanoseconds(
                            super::serde_duration::NANOS_PER_SEC + 234,
                        ),
                        iterations_failed: 1,
                        iterations_skipped: 0,
                        metrics: Some(ServerOperationMetrics {
                            throughput_rpm: Decimal::from_str("42.0").unwrap(),
                            latency_mean: Decimal::from_str("1.0").unwrap(),
                            latency_p50: Decimal::from_str("1.0").unwrap(),
                            latency_p90: Decimal::from_str("1.0").unwrap(),
                            latency_p99: Decimal::from_str("1.0").unwrap(),
                            latency_p999: Decimal::from_str("1.0").unwrap(),
                            latency_p100: Decimal::from_str("1.0").unwrap(),
                            // FIXME This should be an actual Histogram struct.
                            latency_hdr_histogram: "SGVsbG8sIFdvcmxk".into(),
                        }),
                    }],
                }]),
                shutdown: Some(FrameworkOperationLog {
                    started: Utc.ymd(2020, 1, 1).and_hms(17, 0, 0),
                    completed: Utc.ymd(2020, 1, 1).and_hms(18, 0, 0),
                    outcome: FrameworkOperationResult::Ok(),
                }),
            }],
        };
        let actual = serde_json::to_string(&actual).unwrap();
        assert_eq!(expected, actual);
    }
}
