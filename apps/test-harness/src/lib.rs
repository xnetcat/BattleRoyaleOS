//! Test Harness Application
//!
//! Automated testing framework for kernel components.
//! Communicates via serial port for integration with external test runners.

#![no_std]

/// Test result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestResult {
    Pass,
    Fail,
    Skip,
    Timeout,
}

/// Test case
pub struct TestCase {
    pub name: &'static str,
    pub category: &'static str,
    pub run: fn() -> TestResult,
}

/// Test suite
pub struct TestSuite {
    name: &'static str,
    tests: &'static [TestCase],
    current_index: usize,
    results: TestSuiteResults,
}

/// Test suite results
#[derive(Debug, Clone, Default)]
pub struct TestSuiteResults {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub timed_out: usize,
}

impl TestSuite {
    pub const fn new(name: &'static str, tests: &'static [TestCase]) -> Self {
        Self {
            name,
            tests,
            current_index: 0,
            results: TestSuiteResults {
                total: 0,
                passed: 0,
                failed: 0,
                skipped: 0,
                timed_out: 0,
            },
        }
    }

    /// Get suite name
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Get total test count
    pub fn test_count(&self) -> usize {
        self.tests.len()
    }

    /// Run next test
    pub fn run_next(&mut self) -> Option<(&'static str, TestResult)> {
        if self.current_index >= self.tests.len() {
            return None;
        }

        let test = &self.tests[self.current_index];
        self.current_index += 1;
        self.results.total += 1;

        let result = (test.run)();

        match result {
            TestResult::Pass => self.results.passed += 1,
            TestResult::Fail => self.results.failed += 1,
            TestResult::Skip => self.results.skipped += 1,
            TestResult::Timeout => self.results.timed_out += 1,
        }

        Some((test.name, result))
    }

    /// Check if all tests have run
    pub fn is_complete(&self) -> bool {
        self.current_index >= self.tests.len()
    }

    /// Get results
    pub fn results(&self) -> &TestSuiteResults {
        &self.results
    }

    /// Reset suite for re-running
    pub fn reset(&mut self) {
        self.current_index = 0;
        self.results = TestSuiteResults::default();
    }
}

/// Test harness for running all suites
pub struct TestHarness {
    suites: &'static [TestSuite],
    current_suite: usize,
    overall_results: TestSuiteResults,
}

impl TestHarness {
    pub const fn new(suites: &'static [TestSuite]) -> Self {
        Self {
            suites,
            current_suite: 0,
            overall_results: TestSuiteResults {
                total: 0,
                passed: 0,
                failed: 0,
                skipped: 0,
                timed_out: 0,
            },
        }
    }

    /// Get suite count
    pub fn suite_count(&self) -> usize {
        self.suites.len()
    }

    /// Check if all suites have run
    pub fn is_complete(&self) -> bool {
        self.current_suite >= self.suites.len()
    }

    /// Get overall results
    pub fn results(&self) -> &TestSuiteResults {
        &self.overall_results
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.overall_results.failed == 0 && self.overall_results.timed_out == 0
    }
}

/// Format a test result as a serial protocol message
pub fn format_result(test_name: &str, result: TestResult) -> [u8; 64] {
    let mut buffer = [0u8; 64];
    let result_str = match result {
        TestResult::Pass => "pass",
        TestResult::Fail => "fail",
        TestResult::Skip => "skip",
        TestResult::Timeout => "timeout",
    };

    // Format: "RESULT:<test_name>:<result>\n"
    let prefix = b"RESULT:";
    let mut pos = 0;

    for &b in prefix {
        if pos < buffer.len() {
            buffer[pos] = b;
            pos += 1;
        }
    }

    for b in test_name.bytes() {
        if pos < buffer.len() {
            buffer[pos] = b;
            pos += 1;
        }
    }

    if pos < buffer.len() {
        buffer[pos] = b':';
        pos += 1;
    }

    for b in result_str.bytes() {
        if pos < buffer.len() {
            buffer[pos] = b;
            pos += 1;
        }
    }

    if pos < buffer.len() {
        buffer[pos] = b'\n';
    }

    buffer
}
