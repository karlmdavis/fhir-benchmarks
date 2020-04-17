//! Contains the code to run `/metadata` server operations.

use crate::AppState;
use crate::servers::ServerHandle;
use crate::test_framework::{ServerOperationLog, ServerOperationName};
use chrono::prelude::*;
use slog::warn;

static SERVER_OP_NAME_METADATA_TEXT: &str = "metadata";
static SERVER_OP_NAME_METADATA: ServerOperationName =
    ServerOperationName(SERVER_OP_NAME_METADATA_TEXT);

/// Verifies and benchmarks FHIR `/metadata` operations.
pub fn run_operation_metadata(app_state: &AppState, server_handle: &dyn ServerHandle) -> ServerOperationLog {
    let mut server_op_log = ServerOperationLog {
        operation: &SERVER_OP_NAME_METADATA,
        started: Utc::now(),
        iterations: app_state.config.iterations,
        completed: None,
        failures: None,
        metrics: None,
    };

    let url = server_handle
        .base_url()
        .join("metadata")
        .expect("Error parsing URL.");

    let mut failures = 0;
    for _ in 0..server_op_log.iterations {
        let response = reqwest::blocking::get(url.clone());
        match response {
            Ok(response) => {
                if !response.status().is_success() {
                    failures += 1;
                    warn!(app_state.logger, "request failed"; "url" => url.as_str(), "status" => response.status().as_str());
                }
                // TODO more checks needed
            }
            Err(err) => {
                failures += 1;
                warn!(app_state.logger, "request failed"; "url" => url.as_str(), "err" => format!("{:?}", err));
            }
        };
    }

    server_op_log.failures = Some(failures);
    server_op_log.completed = Some(Utc::now());

    server_op_log
}
