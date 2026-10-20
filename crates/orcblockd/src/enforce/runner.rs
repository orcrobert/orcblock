use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use nix::sys::signal::{Signal, killpg};
use nix::unistd::Pid;

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
        stdin: Option<&[u8]>,
    ) -> Result<Output, EnforceError>;
}

/// How long a program may run before it is killed. The daemon runs
/// programs while it holds its state, and some read paths a user controls,
/// so one that hangs must not stall every block.
const DEADLINE: Duration = Duration::from_secs(10);

const POLL: Duration = Duration::from_millis(5);

/// The real runner; kills a program that runs longer than 10 seconds.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(
        &self,
        program: &str,
        args: &[&str],
        stdin: Option<&[u8]>,
    ) -> Result<Output, EnforceError> {
        run_within(program, args, stdin, DEADLINE)
    }
}

fn run_within(
    program: &str,
    args: &[&str],
    stdin: Option<&[u8]>,
    deadline: Duration,
) -> Result<Output, EnforceError> {
    let fail = |reason: String| EnforceError::Program {
        program: program.to_owned(),
        reason,
    };
    let started = Instant::now();
    let mut child = Command::new(program)
        .args(args)
        .env_clear()
        // Its own group, so a timeout also ends what it started (`sudo`
        // starts `open`).
        .process_group(0)
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| fail(format!("could not start: {err}")))?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    if let (Some(input), Some(mut pipe)) = (stdin, child.stdin.take()) {
        // On its own thread: a program that never reads its input must not
        // block past the deadline. A failed write shows in the exit status.
        let input = input.to_vec();
        thread::spawn(move || {
            let _ = pipe.write_all(&input);
        });
    }
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= deadline => {
                stop(child, true);
                return Err(fail(format!("killed after {}s", deadline.as_secs_f32())));
            }
            Ok(None) => thread::sleep(POLL),
            Err(err) => {
                // It may be reaped already, and its group reused.
                stop(child, false);
                return Err(fail(format!("wait: {err}")));
            }
        }
    };
    let collect = |reader: JoinHandle<Vec<u8>>| {
        String::from_utf8_lossy(&reader.join().unwrap_or_default()).into_owned()
    };
    let out = Output {
        stdout: collect(stdout),
        stderr: collect(stderr),
    };
    if status.success() {
        Ok(out)
    } else {
        Err(fail(format!("{status}: {}", out.stderr.trim())))
    }
}

/// Kills a program, and its group when `group` says the child is known to
/// be unreaped (so the group id is still its own), then reaps it on a thread
/// of its own: an unreaped child stays a zombie of the root daemon, and one
/// stuck in the kernel must not hold up the caller. The pipe readers are
/// left behind for the same reason.
fn stop(mut child: Child, group: bool) {
    if group && let Ok(pid) = i32::try_from(child.id()) {
        let _ = killpg(Pid::from_raw(pid), Signal::SIGKILL);
    }
    let _ = child.kill();
    thread::spawn(move || {
        let _ = child.wait();
    });
}

/// Reads a pipe to the end on its own thread, so a program that fills one
/// pipe while nobody reads cannot block.
fn drain(pipe: Option<impl Read + Send + 'static>) -> JoinHandle<Vec<u8>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_end(&mut bytes);
        }
        bytes
    })
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
        /// The input, read as UTF-8 with replacement.
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

    type Respond = dyn Fn(&Call) -> Result<Output, String> + Send + Sync;

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
        /// Answers from the command line alone.
        pub(crate) fn new(
            respond: impl Fn(&str) -> Result<Output, String> + Send + Sync + 'static,
        ) -> Self {
            Self::by_call(move |call| respond(&call.line()))
        }

        /// Answers from the whole call, input included.
        pub(crate) fn by_call(
            respond: impl Fn(&Call) -> Result<Output, String> + Send + Sync + 'static,
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
            stdin: Option<&[u8]>,
        ) -> Result<Output, EnforceError> {
            let call = Call {
                program: program.to_owned(),
                args: args.iter().map(|&a| a.to_owned()).collect(),
                stdin: stdin.map(|bytes| String::from_utf8_lossy(bytes).into_owned()),
            };
            let answer = (self.respond)(&call);
            self.calls
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(call);
            answer.map_err(|reason| EnforceError::Program {
                program: program.to_owned(),
                reason,
            })
        }
    }
}

#[cfg(test)]
mod tests;
