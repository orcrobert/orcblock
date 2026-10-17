use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunError {
    /// The command ran and exited unsuccessfully; holds its combined output.
    Failed(String),
    TimedOut,
    Spawn(String),
}

/// Runs external programs. Hooks only ever run fixed git and cargo commands.
pub(crate) trait Runner {
    fn run(
        &self,
        dir: &Path,
        program: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<String, RunError>;
}

pub(crate) struct SystemRunner;

impl Runner for SystemRunner {
    fn run(
        &self,
        dir: &Path,
        program: &str,
        args: &[&str],
        timeout: Duration,
    ) -> Result<String, RunError> {
        let mut child = Command::new(program)
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| RunError::Spawn(format!("{program}: {err}")))?;

        let mut stdout = child.stdout.take();
        let mut stderr = child.stderr.take();
        let out_reader = thread::spawn(move || drain(stdout.as_mut()));
        let err_reader = thread::spawn(move || drain(stderr.as_mut()));

        let deadline = Instant::now() + timeout;
        let status = loop {
            match child.try_wait() {
                Ok(Some(status)) => break Some(status),
                Ok(None) if Instant::now() >= deadline => {
                    let _ = child.kill();
                    let _ = child.wait();
                    break None;
                }
                Ok(None) => thread::sleep(Duration::from_millis(20)),
                Err(err) => return Err(RunError::Spawn(format!("{program}: {err}"))),
            }
        };

        let mut output = out_reader.join().unwrap_or_default();
        output.push_str(&err_reader.join().unwrap_or_default());
        match status {
            None => Err(RunError::TimedOut),
            Some(status) if status.success() => Ok(output),
            Some(_) => Err(RunError::Failed(output)),
        }
    }
}

fn drain(pipe: Option<&mut impl Read>) -> String {
    let mut buf = String::new();
    if let Some(pipe) = pipe {
        let _ = pipe.read_to_string(&mut buf);
    }
    buf
}

#[cfg(test)]
pub(crate) mod fake {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use super::{RunError, Runner};

    type Responder = Box<dyn Fn(&str) -> Result<String, RunError>>;

    /// Answers commands by matching "program arg arg" against a responder and
    /// records every call.
    pub(crate) struct FakeRunner {
        respond: Responder,
        pub(crate) calls: RefCell<Vec<(PathBuf, String)>>,
    }

    impl FakeRunner {
        pub(crate) fn new(respond: impl Fn(&str) -> Result<String, RunError> + 'static) -> Self {
            Self {
                respond: Box::new(respond),
                calls: RefCell::new(Vec::new()),
            }
        }

        pub(crate) fn commands(&self) -> Vec<String> {
            self.calls.borrow().iter().map(|(_, c)| c.clone()).collect()
        }
    }

    impl Runner for FakeRunner {
        fn run(
            &self,
            dir: &Path,
            program: &str,
            args: &[&str],
            _timeout: Duration,
        ) -> Result<String, RunError> {
            let line = std::iter::once(program)
                .chain(args.iter().copied())
                .collect::<Vec<_>>()
                .join(" ");
            self.calls
                .borrow_mut()
                .push((dir.to_path_buf(), line.clone()));
            (self.respond)(&line)
        }
    }
}

#[cfg(test)]
mod tests;
