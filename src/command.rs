use anyhow::Result;
use std::process::{Command, Output};

/// Trait for running system commands - allows mocking in tests
pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[String]) -> Result<Output>;
}

/// Real command runner that executes actual system commands
pub struct SystemCommandRunner;

impl CommandRunner for SystemCommandRunner {
    fn run(&self, program: &str, args: &[String]) -> Result<Output> {
        Ok(Command::new(program).args(args).output()?)
    }
}

/// Mock command runner for testing
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::os::unix::process::ExitStatusExt;
    use std::sync::Mutex;

    pub struct MockCommandRunner {
        expectations: Mutex<HashMap<String, MockExpectation>>,
    }

    pub struct MockExpectation {
        pub args: Vec<String>,
        pub stdout: String,
        pub stderr: String,
        pub success: bool,
    }

    impl MockCommandRunner {
        pub fn new() -> Self {
            Self {
                expectations: Mutex::new(HashMap::new()),
            }
        }

        pub fn expect(&self, program: &str, args: &[&str], stdout: &str, success: bool) {
            let mut expectations = self.expectations.lock().unwrap();
            let key = format!("{} {}", program, args.join(" "));
            expectations.insert(
                key,
                MockExpectation {
                    args: args.iter().map(|s| s.to_string()).collect(),
                    stdout: stdout.to_string(),
                    stderr: String::new(),
                    success,
                },
            );
        }
    }

    impl CommandRunner for MockCommandRunner {
        fn run(&self, program: &str, args: &[String]) -> Result<Output> {
            let expectations = self.expectations.lock().unwrap();
            let key = format!("{} {}", program, args.join(" "));

            let expectation = expectations
                .get(&key)
                .ok_or_else(|| anyhow::anyhow!("Unexpected command: {}", key))?;

            Ok(Output {
                status: std::process::ExitStatus::from_raw(if expectation.success { 0 } else { 1 }),
                stdout: expectation.stdout.as_bytes().to_vec(),
                stderr: expectation.stderr.as_bytes().to_vec(),
            })
        }
    }
}
