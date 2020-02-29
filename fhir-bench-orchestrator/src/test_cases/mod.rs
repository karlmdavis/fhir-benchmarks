use crate::errors::Result;

/// TODO
pub struct TestResult {
    metadata: TestCaseResult,
}

/// TODO
pub struct TestCaseResult {
    name: String,
    problems: Vec<String>,
}

/// TODO
pub fn run_tests() -> Result<TestResult> {
    unimplemented!()
}
