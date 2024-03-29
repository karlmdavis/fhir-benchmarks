//! The main binary crate for the application, which is just a thin wrapper around the project's library
//! crate.

use eyre::Result;

#[tokio::main]
async fn main() -> Result<()> {
    fhir_bench_orchestrator::run_bench_orchestrator().await
}
