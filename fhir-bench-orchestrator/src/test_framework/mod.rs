use crate::AppState;
use crate::errors::Result;
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use chrono::prelude::*;
use rust_decimal::Decimal;
use serde::Serialize;
use slog_derive::SerdeValue;

mod metadata;

/// Stores the complete set of results from a run of the framework.
#[derive(Clone, SerdeValue, Serialize)]
pub struct FrameworkResults {
    pub started: DateTime<Utc>,
    pub completed: Option<DateTime<Utc>>,
    pub servers: Vec<ServerResult>,
}

impl FrameworkResults {
    /// Constructs a new `FrameworkResults` instance.
    pub fn new(server_plugins: &Vec<Box<dyn ServerPlugin>>) -> FrameworkResults {
        FrameworkResults {
            started: Utc::now(),
            completed: None,
            servers: server_plugins
                .into_iter()
                .map(|p| p.server_name())
                .map(|n| ServerResult::new(n))
                .collect(),
        }
    }

    /// Returns the `ServerResult` for the specified `ServerName`.
    pub fn get_mut(&mut self, server_name: &'static ServerName) -> Option<&mut ServerResult> {
        self.servers
            .iter_mut()
            .filter(|s| s.server == server_name)
            .next()
    }
}

/// Stores the set of test results from a single server implementation.
#[derive(SerdeValue, Clone, Serialize)]
pub struct ServerResult {
    pub server: &'static ServerName,
    pub launch: Option<FrameworkOperationLog>,
    pub operations: Option<Vec<ServerOperationLog>>,
    pub shutdown: Option<FrameworkOperationLog>,
}

impl ServerResult {
    /// Constructs a new `ServerResult` instance.
    pub fn new(server: &'static ServerName) -> ServerResult {
        ServerResult {
            server,
            launch: None,
            operations: None,
            shutdown: None,
        }
    }
}

/// Records the outcomes of a framework operation.
#[derive(SerdeValue, Clone, Serialize)]
pub struct FrameworkOperationLog {
    pub started: DateTime<Utc>,
    pub completed: DateTime<Utc>,
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
#[derive(SerdeValue, Clone, Serialize)]
pub enum FrameworkOperationResult {
    /// Indicates that, as best as can be told, the framwork operation succeeded.
    Ok(),

    /// Indicates that a framework operation failed, and includes the related error messages.
    Errs(Vec<String>),
}

/// TODO
#[derive(SerdeValue, Clone, Serialize)]
pub struct ServerOperationLog {
    pub operation: &'static ServerOperationName,
    pub started: DateTime<Utc>,
    pub iterations: u32,
    pub completed: Option<DateTime<Utc>>,
    pub failures: Option<u32>,
    pub metrics: Option<ServerOperationMetrics>,
}

/// Represents the unique name of a FHIR server operation that this framework tests.
#[derive(SerdeValue, Clone, Serialize)]
pub struct ServerOperationName(pub &'static str);

impl std::fmt::Display for ServerOperationName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Details the performance of a single server operation, across all iterations (including any failures).
#[derive(SerdeValue, Clone, Serialize)]
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

/// TODO
pub fn run_operations(app_state: &AppState, server_handle: &dyn ServerHandle) -> Result<Vec<ServerOperationLog>> {
    let mut results = vec![];

    results.push(metadata::run_operation_metadata(app_state, server_handle));

    Ok(results)
}

/// Unit tests for the test case structures, etc.
///
/// Note: these tests will all fail unless the `serde_json` crate has the `preserve_order` feature enabled,
/// as otherwise serde serialization does not preserve field order.
#[cfg(test)]
mod tests {
    use crate::servers::ServerName;
    use crate::test_framework::{
        FrameworkOperationLog, FrameworkOperationResult, FrameworkResults, ServerOperationLog,
        ServerOperationMetrics, ServerOperationName, ServerResult,
    };
    use chrono::prelude::*;
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::str::FromStr;

    static SERVER_NAME_FAKE_TEXT: &str = "Fake HAPI";
    static SERVER_NAME_FAKE: ServerName = ServerName(SERVER_NAME_FAKE_TEXT);
    static SERVER_OP_NAME_FAKE_TEXT: &str = "Operation A";
    static SERVER_OP_NAME_FAKE: ServerOperationName = ServerOperationName(SERVER_OP_NAME_FAKE_TEXT);

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
            "started": "2020-01-01T15:00:00Z",
            "iterations": 10,
            "completed": "2020-01-01T16:00:00Z",
            "failures": 1,
            "metrics": null,
        });
        let expected = serde_json::to_string(&expected).unwrap();
        let actual = ServerOperationLog {
            operation: &SERVER_OP_NAME_FAKE,
            started: Utc.ymd(2020, 1, 1).and_hms(15, 0, 0),
            iterations: 10,
            completed: Some(Utc.ymd(2020, 1, 1).and_hms(16, 0, 0)),
            failures: Some(1),
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
                        "started": "2020-01-01T15:00:00Z",
                        "iterations": 10,
                        "completed": "2020-01-01T16:00:00Z",
                        "failures": 1,
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
            servers: vec![ServerResult {
                server: &SERVER_NAME_FAKE,
                launch: Some(FrameworkOperationLog {
                    started: Utc.ymd(2020, 1, 1).and_hms(13, 0, 0),
                    completed: Utc.ymd(2020, 1, 1).and_hms(14, 0, 0),
                    outcome: FrameworkOperationResult::Ok(),
                }),
                operations: Some(vec![ServerOperationLog {
                    operation: &SERVER_OP_NAME_FAKE,
                    started: Utc.ymd(2020, 1, 1).and_hms(15, 0, 0),
                    iterations: 10,
                    completed: Some(Utc.ymd(2020, 1, 1).and_hms(16, 0, 0)),
                    failures: Some(1),
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
