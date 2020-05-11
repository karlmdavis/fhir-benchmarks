//! The main binary crate for the application, which is just a thin wrapper around the project's library
//! crate.

use anyhow::Result;

#[async_std::main]
async fn main() -> Result<()> {
    fhir_bench_orchestrator::run_bench_orchestrator().await
}
