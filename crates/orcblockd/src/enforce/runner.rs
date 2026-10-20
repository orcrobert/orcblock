use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

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

/// How long a program may run before it is killed. PowerShell's NRPT and
/// firewall cmdlets go through WMI, which can hang, and a hung call would
/// hold the service lock and stop the block's clock.
const TIMEOUT: Duration = Duration::from_secs(60);

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
        run_with_timeout(program, args, stdin, TIMEOUT)
    }
}

fn run_with_timeout(
    program: &str,
    args: &[&str],
    stdin: Option<&str>,
    timeout: Duration,
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
        // Only the system's modules, never ones from a user's
        // Documents folder, load into an elevated PowerShell.
        if let Some(root) = std::env::var_os("SystemRoot") {
            command.env(
                "PSModulePath",
                std::path::Path::new(&root).join(r"System32\WindowsPowerShell\v1.0\Modules"),
            );
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
    // Every pipe on its own thread, so neither a chatty program nor one that
    // never reads its input can hold the runner past the deadline.
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let feeder = child.stdin.take().zip(stdin).map(|(mut pipe, input)| {
        let input = input.to_owned();
        thread::spawn(move || pipe.write_all(input.as_bytes()))
    });
    let deadline = Instant::now() + timeout;
    let mut pause = Duration::from_millis(1);
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|err| fail(format!("wait: {err}")))?
        {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(fail(format!(
                "did not finish within {} s",
                timeout.as_secs_f32()
            )));
        }
        thread::sleep(pause);
        pause = (pause * 2).min(Duration::from_millis(50));
    };
    if let Some(feeder) = feeder {
        match feeder.join() {
            Ok(Ok(())) => {}
            Ok(Err(err)) => return Err(fail(format!("write stdin: {err}"))),
            Err(_) => return Err(fail("write stdin: the writer panicked".to_owned())),
        }
    }
    let text = |reader: thread::JoinHandle<Vec<u8>>| {
        String::from_utf8_lossy(&reader.join().unwrap_or_default()).into_owned()
    };
    let out = Output {
        stdout: text(stdout),
        stderr: text(stderr),
    };
    if status.success() {
        Ok(out)
    } else {
        Err(fail(format!("{status}: {}", out.stderr.trim())))
    }
}

/// Reads `pipe` to the end on its own thread.
fn drain(pipe: Option<impl Read + Send + 'static>) -> thread::JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    })
}

#[cfg(all(test, unix))]
mod tests;

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
