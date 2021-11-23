//! Contains the `run_operations(...)` method and result types for the benchmark test framework.

use crate::config::AppConfig;
use crate::servers::{ServerHandle, ServerName, ServerPlugin};
use crate::util::{serde_duration_iso8601, serde_histogram};
use crate::AppState;
use chrono::prelude::*;
use chrono::Duration;
use eyre::Result;
use hdrhistogram::Histogram;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub mod metadata;
mod post_org;

/// Stores the complete set of results from a run of the framework.
#[derive(Clone, Deserialize, Serialize)]
pub struct FrameworkResults {
    /// When the benchmark framework run started, in wall clock time.
    pub started: DateTime<Utc>,

    /// When the benchmark framework run started, in wall clock time.
    pub completed: Option<DateTime<Utc>>,

    /// The configuration that the benchmark framework was run with.
    pub config: AppConfig,

    /// Details on the system used to run the benchmarks.
    pub benchmark_metadata: FrameworkMetadata,

    /// The [ServerResult] for each of the supported FHIR servers that a benchmark was attempted for.
    pub servers: Vec<ServerResult>,
}

impl FrameworkResults {
    /// Constructs a new `FrameworkResults` instance.
    ///
    /// Params:
    /// * `config`: the application's configuration
    /// * `server_plugins`: the set of [ServerPlugin]s representing the supported FHIR server implementations
    pub fn new(config: &AppConfig, server_plugins: &[Arc<dyn ServerPlugin>]) -> FrameworkResults {
        FrameworkResults {
            started: Utc::now(),
            completed: None,
            config: config.clone(),
            benchmark_metadata: FrameworkMetadata::default(),
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

/// Stores details on the system used to run the benchmarks.
#[derive(Deserialize, Clone, Serialize)]
pub struct FrameworkMetadata {
    /// Whether or not the framework was compiled in debug or release mode.
    pub cargo_profile: String,

    /// The Git branch that was built.
    pub git_branch: String,

    /// The Git version that was built.
    pub git_semver: String,

    /// The specific Git commit ID that was built.
    pub git_sha: String,

    /// The number of CPU cores available to the system (which may include virtual/hyperthread cores).
    pub cpu_core_count: u16,

    /// The brand name of the CPUs.
    pub cpu_brand_name: String,

    // The CPU frequency.
    pub cpu_frequency: u16,
}

impl Default for FrameworkMetadata {
    /// Constructs a new [FrameworkMetadata] instance.
    fn default() -> Self {
        FrameworkMetadata {
            cargo_profile: env!("VERGEN_CARGO_PROFILE").to_string(),
            git_branch: env!("VERGEN_GIT_BRANCH").to_string(),
            git_semver: env!("VERGEN_GIT_SEMVER").to_string(),
            git_sha: env!("VERGEN_GIT_SHA").to_string(),
            cpu_core_count: env!("VERGEN_SYSINFO_CPU_CORE_COUNT")
                .parse::<u16>()
                .expect("Unable to parse CPU core count."),
            cpu_brand_name: env!("VERGEN_SYSINFO_CPU_BRAND").to_string(),
            cpu_frequency: env!("VERGEN_SYSINFO_CPU_FREQUENCY")
                .parse::<u16>()
                .expect("Unable to parse CPU frequency."),
        }
    }
}

/// Stores the set of test results from a single server implementation.
#[derive(Deserialize, Clone, Serialize)]
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
#[derive(Debug, Deserialize, Clone, Serialize)]
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
#[derive(Debug, Deserialize, Clone, Serialize)]
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
#[derive(Deserialize, Clone, Serialize)]
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
#[derive(Deserialize, Clone, Serialize)]
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
    #[serde(with = "serde_duration_iso8601")]
    pub execution_duration: Duration,

    /// The number of iterations that failed to produce the expected result.
    pub iterations_failed: u32,

    /// The number of iterations that were skipped due to problems that halte the benchmark attempt early.
    pub iterations_skipped: u32,

    /// The [ServerOperationMetrics] for the measurement attempt.
    pub metrics: ServerOperationMetrics,
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
#[derive(Deserialize, Clone, Serialize)]
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
#[derive(Deserialize, Clone, Serialize)]
pub struct ServerOperationMetrics {
    pub throughput_per_second: f64,
    pub latency_millis_mean: f64,
    pub latency_millis_p50: u64,
    pub latency_millis_p90: u64,
    pub latency_millis_p99: u64,
    pub latency_millis_p999: u64,
    pub latency_millis_p100: u64,
    #[serde(with = "serde_histogram")]
    pub latency_histogram: Histogram<u64>,
    pub latency_histogram_hgrm_gzip: String,
}

impl ServerOperationMetrics {
    pub fn new(
        duration: Duration,
        iterations_succeeded: u32,
        histogram: Histogram<u64>,
    ) -> ServerOperationMetrics {
        let duration_millis: f64 = duration.num_milliseconds() as f64;
        let throughput_per_millis: f64 = Into::<f64>::into(iterations_succeeded) / duration_millis;
        let throughput_per_second: f64 = throughput_per_millis * 1000f64;
        let latency_histogram_hgrm_gzip =
            crate::util::histogram_hgrm_export::export_to_hgrm_gzip(&histogram)
                .expect("Unable to export histogram.");

        ServerOperationMetrics {
            throughput_per_second,
            latency_millis_mean: histogram.mean(),
            latency_millis_p50: histogram.value_at_quantile(0.5),
            latency_millis_p90: histogram.value_at_quantile(0.9),
            latency_millis_p99: histogram.value_at_quantile(0.99),
            latency_millis_p999: histogram.value_at_quantile(0.999),
            latency_millis_p100: histogram.max(),
            latency_histogram: histogram,
            latency_histogram_hgrm_gzip,
        }
    }
}

/// A state machine for tracking the progress and results of a single iteration for a server
/// operation being benchmarked.
#[derive(Clone, Debug)]
struct ServerOperationIterationState<S> {
    _inner: S,
}

/// This [ServerOperationIterationState] state node models an operation that is starting.
#[derive(Clone, Debug)]
struct ServerOperationIterationStarting {
    /// When this operation iteration started, in wall-clock time.
    started: DateTime<Utc>,
}

/// This [ServerOperationIterationState] state node models an operation that has completed, but
/// before success or failure has been determined.
#[derive(Debug)]
struct ServerOperationIterationCompleted {
    /*
     * Design note: if the benchmark runner is under memory pressure, the state here could instead
     * be collapsed to a single Duration instance.
     */
    /// The state from the operation's start.
    start: ServerOperationIterationStarting,

    /// When this operation iteration completed, in wall-clock time.
    completed: DateTime<Utc>,
}

/// This [ServerOperationIterationState] state node models an operation that has completed
/// successfully.
#[derive(Debug)]
struct ServerOperationIterationSucceeded {
    /// The state from the operation's completion.
    completed: ServerOperationIterationCompleted,
}

/// This [ServerOperationIterationState] state node models an operation that failed to complete
/// successfully.
#[derive(Debug)]
struct ServerOperationIterationFailed {
    /// The state from the operation's completion.
    completed: ServerOperationIterationCompleted,

    /// The [anyhow::Error] detailing how/why the operation iteraion failed.
    error: eyre::Error,
}

impl ServerOperationIterationState<ServerOperationIterationStarting> {
    /// Creates a new [ServerOperationIterationState] state machine instance, as the operation
    /// iteration is being started.
    pub fn new() -> ServerOperationIterationState<ServerOperationIterationStarting> {
        ServerOperationIterationState {
            _inner: ServerOperationIterationStarting {
                started: Utc::now(),
            },
        }
    }

    /// Transitions this [ServerOperationIterationState] state machine instance after the operation
    /// iteration completes, but before its success or failure has been determined.
    pub fn completed(self) -> ServerOperationIterationState<ServerOperationIterationCompleted> {
        ServerOperationIterationState {
            _inner: ServerOperationIterationCompleted {
                start: self._inner,
                completed: Utc::now(),
            },
        }
    }
}

impl ServerOperationIterationState<ServerOperationIterationCompleted> {
    /// Transitions this [ServerOperationIterationState] state machine instance after the operation
    /// iteration has been deemed a success.
    pub fn succeeded(self) -> ServerOperationIterationState<ServerOperationIterationSucceeded> {
        ServerOperationIterationState {
            _inner: ServerOperationIterationSucceeded {
                completed: self._inner,
            },
        }
    }

    /// Transitions this [ServerOperationIterationState] state machine instance after the operation
    /// iteration has been deemed a failure.
    ///
    /// Parameters:
    /// * `error`: the [anyhow::Error] detailing how/why the operation iteraion failed
    pub fn failed(
        self,
        error: eyre::Error,
    ) -> ServerOperationIterationState<ServerOperationIterationFailed> {
        ServerOperationIterationState {
            _inner: ServerOperationIterationFailed {
                completed: self._inner,
                error,
            },
        }
    }
}

impl ServerOperationIterationState<ServerOperationIterationSucceeded> {
    /// Returns the [Duration] that the operation iteration ran for.
    pub fn duration(&self) -> Duration {
        self._inner.completed.completed - self._inner.completed.start.started
    }
}

/// Runs the benchmark framework to test the supported operations for the specified FHIR server.
///
/// Parameters:
/// * `app_state`: the application's [AppState]
/// * `server_handle`: the [ServerHandle] for the server implementation to be tested
///
/// Returns the [ServerOperationLog]s for each operation that was tested.
#[allow(clippy::vec_init_then_push)]
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
    use crate::test_framework::{
        FrameworkOperationLog, FrameworkOperationResult, FrameworkResults, ServerOperationLog,
        ServerOperationMeasurement, ServerOperationMetrics, ServerResult,
    };
    use crate::util::serde_duration_iso8601;
    use crate::{config::AppConfig, test_framework::FrameworkMetadata};
    use chrono::prelude::*;
    use chrono::Duration;
    use hdrhistogram::Histogram;
    use serde_json::json;

    static SERVER_NAME_FAKE: &str = "Fake HAPI";
    static SERVER_OP_NAME_FAKE: &str = "Operation A";

    /// Verifies that `FrameworkOperationLog` serializes as expected.
    #[tracing::instrument(level = "info")]
    #[test_env_log::test(tokio::test)]
    async fn serialize_framework_operation_log() {
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
    #[tracing::instrument(level = "info")]
    #[test_env_log::test(tokio::test)]
    async fn serialize_server_operation_metrics() {
        let expected = json!({
            "throughput_per_second": 42.0,
            "latency_millis_mean": 1.0,
            "latency_millis_p50": 1,
            "latency_millis_p90": 1,
            "latency_millis_p99": 1,
            "latency_millis_p999": 1,
            "latency_millis_p100": 1,
            "latency_histogram": "HISTFAAAABx4nJNpmSzMwMDAyAABzFAaxmey/wBlAQA8yQJ9",
            "latency_histogram_hgrm_gzip": "foo",
        });
        let expected = serde_json::to_string(&expected).unwrap();
        let actual = ServerOperationMetrics {
            throughput_per_second: 42.0,
            latency_millis_mean: 1.0,
            latency_millis_p50: 1,
            latency_millis_p90: 1,
            latency_millis_p99: 1,
            latency_millis_p999: 1,
            latency_millis_p100: 1,
            latency_histogram: Histogram::<u64>::new(3).expect("Error creating histogram."),
            latency_histogram_hgrm_gzip: "foo".into(),
        };
        let actual = serde_json::to_string(&actual).unwrap();
        assert_eq!(expected, actual);
    }

    /// Verifies that `ServerOperationLog` serializes as expected.
    #[tracing::instrument(level = "info")]
    #[test_env_log::test(tokio::test)]
    async fn serialize_server_operation_log() {
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
    #[tracing::instrument(level = "info")]
    #[test_env_log::test(tokio::test)]
    async fn serialize_server_operation_measurement() {
        let expected = json!({
            "concurrent_users": 10,
            "started": "2020-01-01T15:00:00Z",
            "completed": "2020-01-01T16:00:00Z",
            "execution_duration": "PT1.234S",
            "iterations_failed": 1,
            "iterations_skipped": 0,
            "metrics": {
                "throughput_per_second": 42.0,
                "latency_millis_mean": 1.0,
                "latency_millis_p50": 1,
                "latency_millis_p90": 1,
                "latency_millis_p99": 1,
                "latency_millis_p999": 1,
                "latency_millis_p100": 1,
                "latency_histogram": "HISTFAAAABx4nJNpmSzMwMDAyAABzFAaxmey/wBlAQA8yQJ9",
                "latency_histogram_hgrm_gzip": "foo",
            }
        });
        let expected = serde_json::to_string(&expected).unwrap();
        let actual = ServerOperationMeasurement {
            concurrent_users: 10,
            started: Utc.ymd(2020, 1, 1).and_hms(15, 0, 0),
            completed: Utc.ymd(2020, 1, 1).and_hms(16, 0, 0),
            execution_duration: Duration::nanoseconds(serde_duration_iso8601::NANOS_PER_SEC + 234),
            iterations_failed: 1,
            iterations_skipped: 0,
            metrics: ServerOperationMetrics {
                throughput_per_second: 42.0,
                latency_millis_mean: 1.0,
                latency_millis_p50: 1,
                latency_millis_p90: 1,
                latency_millis_p99: 1,
                latency_millis_p999: 1,
                latency_millis_p100: 1,
                latency_histogram: Histogram::<u64>::new(3).expect("Error creating histogram."),
                latency_histogram_hgrm_gzip: "foo".into(),
            },
        };
        let actual = serde_json::to_string(&actual).unwrap();
        assert_eq!(expected, actual);
    }

    /// Verifies that `FrameworkResults` serializes as expected.
    #[tracing::instrument(level = "info")]
    #[test_env_log::test(tokio::test)]
    async fn serialize_framework_results() {
        let expected = json!({
            "started": "2020-01-01T12:00:00Z",
            "completed": "2020-01-01T19:00:00Z",
            "config": {
                "iterations": 1,
                "operation_timeout": 1000,
                "concurrency_levels": [1, 10],
                "population_size": 1,
            },
            "benchmark_metadata": {
                "cargo_profile": "release",
                "git_branch": "main",
                "git_semver": "1.0-foo",
                "git_sha": "foo",
                "cpu_core_count": 64,
                "cpu_brand_name": "Very Awesome CPU",
                "cpu_frequency": 42,
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
                                "throughput_per_second": 42.0,
                                "latency_millis_mean": 1.0,
                                "latency_millis_p50": 1,
                                "latency_millis_p90": 1,
                                "latency_millis_p99": 1,
                                "latency_millis_p999": 1,
                                "latency_millis_p100": 1,
                                "latency_histogram": "HISTFAAAABx4nJNpmSzMwMDAyAABzFAaxmey/wBlAQA8yQJ9",
                                "latency_histogram_hgrm_gzip": "foo",
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
                operation_timeout: Duration::milliseconds(1000),
                concurrency_levels: vec![1, 10],
                population_size: 1,
            },
            benchmark_metadata: FrameworkMetadata {
                cargo_profile: "release".into(),
                git_branch: "main".into(),
                git_semver: "1.0-foo".into(),
                git_sha: "foo".into(),
                cpu_core_count: 64,
                cpu_brand_name: "Very Awesome CPU".into(),
                cpu_frequency: 42,
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
                            serde_duration_iso8601::NANOS_PER_SEC + 234,
                        ),
                        iterations_failed: 1,
                        iterations_skipped: 0,
                        metrics: ServerOperationMetrics {
                            throughput_per_second: 42.0,
                            latency_millis_mean: 1.0,
                            latency_millis_p50: 1,
                            latency_millis_p90: 1,
                            latency_millis_p99: 1,
                            latency_millis_p999: 1,
                            latency_millis_p100: 1,
                            latency_histogram: Histogram::<u64>::new(3)
                                .expect("Error creating histogram."),
                            latency_histogram_hgrm_gzip: "foo".into(),
                        },
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
