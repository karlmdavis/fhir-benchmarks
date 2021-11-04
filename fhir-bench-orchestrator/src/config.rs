//! Application configuration.

use crate::util::serde_duration_millis;
use chrono::Duration;
use eyre::{eyre, Context, Result};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

/// The environment variable key for the [AppConfig.iterations] setting.
pub const ENV_KEY_ITERATIONS: &str = "FHIR_BENCH_ITERATIONS";

/// The environment variable key for the [AppConfig.operation_timeout] setting (in milliseconds).
pub const ENV_KEY_OPERATION_TIMEOUT: &str = "FHIR_BENCH_OPERATION_TIMEOUT_MS";

/// The environment variable key for the [AppConfig.concurrency_levels] setting.
pub const ENV_KEY_CONCURRENCY_LEVELS: &str = "FHIR_BENCH_CONCURRENCY_LEVELS";

/// The environment variable key for the [AppConfig.population_size] setting.
pub const ENV_KEY_POPULATION_SIZE: &str = "FHIR_BENCH_POPULATION_SIZE";

/// Represents the application's configuration.
#[derive(Clone, Deserialize, Serialize)]
pub struct AppConfig {
    /// The maximum number of iterations to exercise each operation for, during a benchmark run.
    pub iterations: u32,

    /// The maximum amount of time to let any individual operation being benchmarked run for.
    #[serde(with = "serde_duration_millis")]
    pub operation_timeout: Duration,

    /// The concurrency level(s) to test at. Each operation will be tested with an attempt to model each
    /// specified number of concurrent users.
    pub concurrency_levels: Vec<u32>,

    /// The maximum synthetic patient population size to benchmark with.
    pub population_size: u32,
}

impl AppConfig {
    pub fn new() -> Result<AppConfig> {
        // If present, load environment variables from a `.env` file in the working directory.
        dotenv::dotenv().ok();

        // Parse iterations.
        let iterations: std::result::Result<String, std::env::VarError> =
            env::var(ENV_KEY_ITERATIONS).or_else(|_| Ok(String::from("1000")));
        let iterations: u32 = iterations
            .context(format!("Unable to read {}.", ENV_KEY_ITERATIONS))?
            .parse()
            .context(format!("Unable to parse {}.", ENV_KEY_ITERATIONS))?;

        // Parse operation_timeout.
        let operation_timeout: std::result::Result<String, std::env::VarError> =
            env::var(ENV_KEY_OPERATION_TIMEOUT).or_else(|_| Ok(String::from("10000")));
        let operation_timeout: u32 = operation_timeout
            .context(format!("Unable to read {}.", ENV_KEY_OPERATION_TIMEOUT))?
            .parse()
            .context(format!("Unable to parse {}.", ENV_KEY_OPERATION_TIMEOUT))?;
        let operation_timeout = Duration::milliseconds(operation_timeout as i64);

        // Parse concurrency_levels.
        let concurrency_levels: std::result::Result<String, std::env::VarError> =
            env::var(ENV_KEY_CONCURRENCY_LEVELS).or_else(|_| Ok(String::from("1,10")));
        let concurrency_levels: std::result::Result<Vec<u32>, _> = concurrency_levels
            .context(format!("Unable to read {}.", ENV_KEY_CONCURRENCY_LEVELS))?
            .split(',')
            .map(str::parse::<u32>)
            .collect();
        let concurrency_levels = concurrency_levels
            .context(format!("Unable to parse {}.", ENV_KEY_CONCURRENCY_LEVELS))?;

        // Parse population_size.
        let population_size: std::result::Result<String, std::env::VarError> =
            env::var(ENV_KEY_POPULATION_SIZE).or_else(|_| Ok(String::from("100")));
        let population_size: u32 = population_size
            .context(format!("Unable to read {}.", ENV_KEY_POPULATION_SIZE))?
            .parse()
            .context(format!("Unable to parse {}.", ENV_KEY_POPULATION_SIZE))?;

        Ok(AppConfig {
            iterations,
            operation_timeout,
            concurrency_levels,
            population_size,
        })
    }

    /// Returns the root directory for the benchmarks project; the Git repo's top-level directory.
    pub fn benchmark_dir(&self) -> Result<PathBuf> {
        benchmark_dir()
    }
}

/// Returns the root directory for the benchmarks project; the Git repo's top-level directory.
pub fn benchmark_dir() -> Result<PathBuf> {
    // For now, this is hard-coded to check a couple of likely scenarios:
    //
    // 1. Someone is running from the `fhir-bench-orchestrator` module, like might happen if they are
    //    running a specific test from their IDE.
    // 2. Someone is running from the Git project's root directory, like they might from a terminal.
    let current_dir = std::env::current_dir().context("unable to retrieve current directory")?;

    if current_dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        == Some("fhir-bench-orchestrator".to_string())
    {
        Ok(current_dir
            .parent()
            .expect("Unable to get module parent directory.")
            .into())
    } else if current_dir
        .read_dir()?
        .any(|e| e.is_ok() && e.as_ref().unwrap().file_name() == ".git")
    {
        Ok(current_dir)
    } else {
        Err(eyre!(
            "Unable to find benchmark directory from current directory: '{:?}'",
            current_dir
        ))
    }
}
