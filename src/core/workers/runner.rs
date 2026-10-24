//! Command runner for runa.
//!
//! Runs editor commands that do not need the terminal, so the TUI keeps running.

use std::ffi::OsString;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::thread;

use crossbeam_channel::{Receiver, Sender};

use crate::core::workers::{CommandTask, WorkerResponse};

/// Bytes of stderr kept for the error message.
const STDERR_KEEP_BYTES: u64 = 4096;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub(super) fn start(task_rx: Receiver<CommandTask>, res_tx: Sender<WorkerResponse>) {
    thread::spawn(move || {
        while let Ok(CommandTask {
            program,
            args,
            dir,
            check_status,
        }) = task_rx.recv()
        {
            let name = program
                .file_stem()
                .unwrap_or(program.as_os_str())
                .to_string_lossy()
                .into_owned();

            match spawn(&program, &args, dir.as_deref(), check_status) {
                Ok(child) => {
                    let res_tx = res_tx.clone();
                    thread::spawn(move || wait_for(child, &name, check_status, &res_tx));
                }
                Err(e) => {
                    let _ = res_tx.send(WorkerResponse::Error(format!("{name}: {e}")));
                }
            }
        }
    });
}

/// Spawns the command without any access to the terminal.
fn spawn(
    program: &Path,
    args: &[OsString],
    dir: Option<&Path>,
    check_status: bool,
) -> io::Result<Child> {
    let stderr = if check_status {
        Stdio::piped()
    } else {
        Stdio::null()
    };

    let mut command = Command::new(program);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(stderr);

    if let Some(dir) = dir {
        command.current_dir(dir);
    }

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command.spawn()
}

/// Waits for the command and reports a failure with the first line of its stderr.
fn wait_for(mut child: Child, name: &str, check_status: bool, res_tx: &Sender<WorkerResponse>) {
    let mut head = Vec::new();
    if let Some(mut stderr) = child.stderr.take() {
        let _ = stderr
            .by_ref()
            .take(STDERR_KEEP_BYTES)
            .read_to_end(&mut head);

        // Drain the rest
        let _ = io::copy(&mut stderr, &mut io::sink());
    }

    let message = match child.wait() {
        Ok(_) if !check_status => return,
        Ok(status) if status.success() => return,
        Ok(status) => {
            let stderr = String::from_utf8_lossy(&head);
            match stderr.lines().map(str::trim).find(|line| !line.is_empty()) {
                Some(line) => format!("{name}: {status}: {line}"),
                None => format!("{name}: {status}"),
            }
        }
        Err(e) => format!("{name}: {e}"),
    };
    let _ = res_tx.send(WorkerResponse::Error(message));
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;
    use std::time::Duration;

    use crossbeam_channel::{RecvTimeoutError, unbounded};
    use tempfile::{TempDir, tempdir};

    const TIMEOUT: Duration = Duration::from_secs(5);

    fn run(
        program: &str,
        args: &[&str],
        check_status: bool,
    ) -> (Receiver<WorkerResponse>, TempDir) {
        let dir = tempdir().expect("temp dir is created");
        let (task_tx, task_rx) = unbounded();
        let (res_tx, res_rx) = unbounded();
        start(task_rx, res_tx);
        let task = CommandTask {
            program: PathBuf::from(program),
            args: args.iter().map(OsString::from).collect(),
            dir: Some(dir.path().to_path_buf()),
            check_status,
        };
        task_tx.send(task).expect("runner thread is running");
        (res_rx, dir)
    }

    fn error_message(res_rx: &Receiver<WorkerResponse>) -> String {
        match res_rx.recv_timeout(TIMEOUT) {
            Ok(WorkerResponse::Error(message)) => message,
            other => panic!("expected an error, got {other:?}"),
        }
    }

    // Disconnected means every sender is gone without sending anything
    fn assert_nothing_sent(res_rx: &Receiver<WorkerResponse>) {
        assert_eq!(
            res_rx.recv_timeout(TIMEOUT).err(),
            Some(RecvTimeoutError::Disconnected)
        );
    }

    #[cfg(unix)]
    #[test]
    fn failure_reports_the_first_stderr_line() {
        let (res_rx, _dir) = run(
            "sh",
            &["-c", "echo first >&2; echo second >&2; exit 3"],
            true,
        );
        assert_eq!(error_message(&res_rx), "sh: exit status: 3: first");
    }

    #[cfg(windows)]
    #[test]
    fn failure_reports_the_first_stderr_line() {
        let (res_rx, _dir) = run(
            "cmd",
            &["/C", "echo first 1>&2 & echo second 1>&2 & exit /B 3"],
            true,
        );
        assert_eq!(error_message(&res_rx), "cmd: exit code: 3: first");
    }

    #[cfg(unix)]
    #[test]
    fn success_sends_nothing() {
        let (res_rx, _dir) = run("sh", &["-c", "echo ignored; exit 0"], true);
        assert_nothing_sent(&res_rx);
    }

    #[cfg(windows)]
    #[test]
    fn success_sends_nothing() {
        let (res_rx, _dir) = run("cmd", &["/C", "echo ignored & exit /B 0"], true);
        assert_nothing_sent(&res_rx);
    }

    #[cfg(unix)]
    #[test]
    fn unchecked_failure_sends_nothing() {
        let (res_rx, _dir) = run("sh", &["-c", "echo ignored >&2; exit 1"], false);
        assert_nothing_sent(&res_rx);
    }

    #[cfg(windows)]
    #[test]
    fn unchecked_failure_sends_nothing() {
        let (res_rx, _dir) = run("cmd", &["/C", "echo ignored 1>&2 & exit /B 1"], false);
        assert_nothing_sent(&res_rx);
    }

    #[cfg(unix)]
    #[test]
    fn chatty_stderr_does_not_kill_the_command() {
        let (res_rx, _dir) = run(
            "sh",
            &["-c", "yes runa | head -c 1000000 >&2 && exit 3"],
            true,
        );
        assert_eq!(error_message(&res_rx), "sh: exit status: 3: runa");
    }

    #[test]
    fn missing_program_reports_its_name() {
        let (res_rx, _dir) = run("runa-no-such-program", &[], false);
        assert!(error_message(&res_rx).starts_with("runa-no-such-program: "));
    }
}
