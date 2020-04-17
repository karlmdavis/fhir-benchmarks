//! Application configuration.

use crate::errors::Result;
use std::env;

/// Represents the application's configuration.
pub struct AppConfig {
    pub iterations: u32,
}

impl AppConfig {
    pub fn new() -> Result<AppConfig> {
        // If present, load environment variables from a `.env` file in the working directory.
        dotenv::dotenv().ok();

        // Parse the configurable entries.
        let iterations: std::result::Result<String, std::env::VarError> =
            env::var("FHIR_BENCH_ITERATIONS").or_else(|_| Ok(String::from("1000")));
        let iterations: u32 = iterations?.parse()?;
        Ok(AppConfig { iterations })
    }
}
