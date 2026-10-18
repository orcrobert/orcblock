use std::io::Write;
use std::process::{Command, Stdio};

use super::EnforceError;

/// What a program printed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Output {
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
}

/// Runs system programs by absolute path, never through a shell.
pub trait CommandRunner: Send {
    /// Runs `program` with `args`, feeding it `stdin`; a non-zero exit is an
    /// [`EnforceError::Program`].
    fn run(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&str>,
    ) -> Result<Output, EnforceError>;
}

/// System variables kept for programs on Windows.
#[cfg(windows)]
const WINDOWS_ENV: [&str; 7] = [
    "SystemRoot",
    "SystemDrive",
    "windir",
    "ProgramData",
    "ProgramFiles",
    "TEMP",
    "TMP",
];

/// Starts a console program without opening a console window.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The real runner. On Unix programs get an empty environment; on Windows
/// only a fixed set of system variables.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&str>,
    ) -> Result<Output, EnforceError> {
        let fail = |reason: String| EnforceError::Program {
            program: program.to_owned(),
            reason,
        };
        let mut command = Command::new(program);
        command.args(args).env_clear();
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // PowerShell and other system tools fail without these; nothing
            // else from the caller's environment is passed on.
            for key in WINDOWS_ENV {
                if let Some(value) = std::env::var_os(key) {
                    command.env(key, value);
                }
            }
            command.creation_flags(CREATE_NO_WINDOW);
        }
        let mut child = command
            .stdin(if stdin.is_some() {
                Stdio::piped()
            } else {
                Stdio::null()
            })
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| fail(format!("could not start: {err}")))?;
        if let (Some(input), Some(mut pipe)) = (stdin, child.stdin.take()) {
            pipe.write_all(input.as_bytes())
                .map_err(|err| fail(format!("write stdin: {err}")))?;
        }
        let output = child
            .wait_with_output()
            .map_err(|err| fail(format!("wait: {err}")))?;
        let out = Output {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        };
        if output.status.success() {
            Ok(out)
        } else {
            Err(fail(format!("{}: {}", output.status, out.stderr.trim())))
        }
    }
}

#[cfg(test)]
pub(crate) mod fake {
    use std::sync::{Arc, Mutex, PoisonError};

    use super::{CommandRunner, EnforceError, Output};

    /// One recorded call.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct Call {
        pub(crate) program: String,
        pub(crate) args: Vec<String>,
        pub(crate) stdin: Option<String>,
    }

    impl Call {
        /// The call as one line, like `/sbin/pfctl -E`.
        pub(crate) fn line(&self) -> String {
            std::iter::once(self.program.as_str())
                .chain(self.args.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(" ")
        }
    }

    type Respond = dyn Fn(&str) -> Result<Output, String> + Send + Sync;

    /// Records calls and answers from a function of the command line.
    #[derive(Clone)]
    pub(crate) struct FakeRunner {
        calls: Arc<Mutex<Vec<Call>>>,
        respond: Arc<Respond>,
    }

    impl std::fmt::Debug for FakeRunner {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.debug_struct("FakeRunner").finish_non_exhaustive()
        }
    }

    impl FakeRunner {
        pub(crate) fn new(
            respond: impl Fn(&str) -> Result<Output, String> + Send + Sync + 'static,
        ) -> Self {
            Self {
                calls: Arc::default(),
                respond: Arc::new(respond),
            }
        }

        /// The calls so far, each as one line.
        pub(crate) fn lines(&self) -> Vec<String> {
            self.calls().iter().map(Call::line).collect()
        }

        pub(crate) fn calls(&self) -> Vec<Call> {
            self.calls
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clone()
        }

        pub(crate) fn forget(&self) {
            self.calls
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .clear();
        }
    }

    impl CommandRunner for FakeRunner {
        fn run(
            &self,
            program: &str,
            args: &[&str],
            stdin: Option<&str>,
        ) -> Result<Output, EnforceError> {
            let call = Call {
                program: program.to_owned(),
                args: args.iter().map(|&a| a.to_owned()).collect(),
                stdin: stdin.map(str::to_owned),
            };
            let line = call.line();
            self.calls
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(call);
            (self.respond)(&line).map_err(|reason| EnforceError::Program {
                program: program.to_owned(),
                reason,
            })
        }
    }
}
