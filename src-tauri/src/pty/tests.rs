//! End-to-end tests against real ptys (Unix only: they spawn sh, cat and sleep).

use std::path::PathBuf;
use std::process::Command;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use super::kill::KILL_GRACE;
use super::*;

const TIMEOUT: Duration = Duration::from_secs(10);

fn custom(command: &str, args: &[&str]) -> Launch {
    let args: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
    resolve_launch(PtyKind::Custom, Some(command), &args).expect("launch")
}

fn spawn(manager: &PtyManager, launch: &Launch) -> (SessionInfo, Receiver<PtyOutput>) {
    spawn_in(manager, launch, std::env::temp_dir())
}

fn spawn_in(
    manager: &PtyManager,
    launch: &Launch,
    cwd: PathBuf,
) -> (SessionInfo, Receiver<PtyOutput>) {
    let (tx, rx) = mpsc::channel();
    let sink: OutputSink = Box::new(move |out| {
        let _ = tx.send(out);
    });
    let info = manager
        .spawn(
            PtyKind::Custom,
            launch,
            &cwd,
            TermSize::clamped(80, 24),
            sink,
        )
        .expect("spawn");
    (info, rx)
}

/// Reads until the exit message; returns all output and the exit info.
fn read_to_exit(rx: &Receiver<PtyOutput>) -> (String, ExitInfo) {
    let mut output = Vec::new();
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left).expect("session did not exit in time") {
            PtyOutput::Data(bytes) => output.extend(bytes),
            PtyOutput::Exit(info) => return (String::from_utf8_lossy(&output).into_owned(), info),
        }
    }
}

/// Reads until `needle` shows up in the output (or panics on timeout).
fn read_until(rx: &Receiver<PtyOutput>, needle: &str) -> String {
    let mut output = Vec::new();
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx
            .recv_timeout(left)
            .expect("output did not arrive in time")
        {
            PtyOutput::Data(bytes) => output.extend(bytes),
            PtyOutput::Exit(info) => panic!("exited early with {info:?}"),
        }
        let text = String::from_utf8_lossy(&output).into_owned();
        if text.contains(needle) {
            return text;
        }
    }
}

fn process_exists(pid: u32) -> bool {
    // `kill -0` also succeeds for zombies, so this proves the child was reaped.
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

fn wait_until(mut condition: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + TIMEOUT;
    while Instant::now() < deadline {
        if condition() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

#[test]
fn echo_hello_reports_output_and_exit_code() {
    let manager = PtyManager::new();
    let (info, rx) = spawn(&manager, &custom("sh", &["-c", "echo hello"]));
    let (output, exit) = read_to_exit(&rx);
    assert!(output.contains("hello"), "output was {output:?}");
    assert_eq!(exit.code, Some(0));
    assert!(
        wait_until(|| manager.list().is_empty()),
        "session not removed"
    );
    assert!(!manager.kill(info.id), "already gone");
}

#[test]
fn non_zero_exit_codes_are_reported() {
    let manager = PtyManager::new();
    let (_, rx) = spawn(&manager, &custom("sh", &["-c", "exit 3"]));
    assert_eq!(read_to_exit(&rx).1.code, Some(3));
}

#[test]
fn terminal_env_and_cwd_are_set() {
    let manager = PtyManager::new();
    let dir = std::env::temp_dir().canonicalize().expect("temp dir");
    let launch = custom("sh", &["-c", "echo $TERM $COLORTERM; pwd -P"]);
    let (_, rx) = spawn_in(&manager, &launch, dir.clone());
    let (output, _) = read_to_exit(&rx);
    assert!(
        output.contains("xterm-256color truecolor"),
        "output was {output:?}"
    );
    assert!(
        output.contains(&*dir.to_string_lossy()),
        "output was {output:?}"
    );
}

#[test]
fn writes_reach_cat_and_come_back() {
    let manager = PtyManager::new();
    let (info, rx) = spawn(&manager, &custom("cat", &[]));
    manager.write(info.id, b"ping-from-test\n").expect("write");
    read_until(&rx, "ping-from-test");
    assert!(manager.kill(info.id));
    read_to_exit(&rx);
}

#[test]
fn resize_applies_to_the_pty() {
    let manager = PtyManager::new();
    let (info, rx) = spawn(&manager, &custom("sh", &["-c", "sleep 0.5; stty size"]));
    manager
        .resize(info.id, TermSize::clamped(101, 42))
        .expect("resize");
    let (output, _) = read_to_exit(&rx);
    assert!(output.contains("42 101"), "output was {output:?}");
}

#[test]
fn kill_reaps_the_child() {
    let manager = PtyManager::new();
    let (info, rx) = spawn(&manager, &custom("sleep", &["30"]));
    let pid = info.pid.expect("pid");
    assert!(process_exists(pid));
    assert!(manager.kill(info.id));
    read_to_exit(&rx);
    assert!(
        wait_until(|| !process_exists(pid)),
        "child {pid} not reaped"
    );
    assert!(manager.list().is_empty());
}

#[test]
fn kill_escalates_when_hangup_is_ignored() {
    let manager = PtyManager::new();
    let launch = custom("sh", &["-c", "trap '' HUP; while :; do sleep 1; done"]);
    let (info, rx) = spawn(&manager, &launch);
    let pid = info.pid.expect("pid");
    std::thread::sleep(Duration::from_millis(200)); // let the trap install
    let started = Instant::now();
    assert!(manager.kill(info.id));
    let (_, exit) = read_to_exit(&rx);
    assert!(started.elapsed() >= KILL_GRACE, "exited before escalation");
    assert!(
        exit.signal.is_some() || exit.code != Some(0),
        "exit was {exit:?}"
    );
    assert!(
        wait_until(|| !process_exists(pid)),
        "child {pid} not reaped"
    );
}

#[test]
fn shutdown_stops_every_session() {
    let manager = PtyManager::new();
    let (a, _rx_a) = spawn(&manager, &custom("sleep", &["30"]));
    let (b, _rx_b) = spawn(&manager, &custom("sleep", &["30"]));
    manager.shutdown();
    assert!(manager.list().is_empty());
    for pid in [a.pid, b.pid].into_iter().flatten() {
        assert!(wait_until(|| !process_exists(pid)), "child {pid} survived");
    }
}

#[test]
fn shutdown_waits_for_recently_killed_children() {
    let manager = PtyManager::new();
    let launch = custom("sh", &["-c", "trap '' HUP; while :; do sleep 1; done"]);
    let (info, _rx) = spawn(&manager, &launch);
    let pid = info.pid.expect("pid");
    std::thread::sleep(Duration::from_millis(200));
    assert!(manager.kill(info.id)); // SIGHUP is ignored; the 2 s watchdog has not fired yet
    manager.shutdown();
    assert!(
        wait_until(|| !process_exists(pid)),
        "child {pid} outlived shutdown"
    );
}

#[test]
fn kill_all_stops_every_session() {
    let manager = PtyManager::new();
    let (a, rx_a) = spawn(&manager, &custom("sleep", &["30"]));
    let (b, rx_b) = spawn(&manager, &custom("sleep", &["30"]));
    manager.kill_all();
    assert!(manager.list().is_empty());
    read_to_exit(&rx_a);
    read_to_exit(&rx_b);
    for pid in [a.pid, b.pid].into_iter().flatten() {
        assert!(wait_until(|| !process_exists(pid)), "child {pid} survived");
    }
}

#[test]
fn reader_pauses_until_output_is_acknowledged() {
    use super::flow::HIGH_WATER;
    const TOTAL: usize = 3 * 1024 * 1024;
    let manager = PtyManager::new();
    let script = format!("head -c {TOTAL} /dev/zero | tr '\\0' x");
    let (info, rx) = spawn(&manager, &custom("sh", &["-c", &script]));

    // Without acks the reader stops near the high-water mark and the child blocks.
    let mut received = 0usize;
    let deadline = Instant::now() + Duration::from_millis(800);
    while let Ok(out) = rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        match out {
            PtyOutput::Data(bytes) => received += bytes.len(),
            PtyOutput::Exit(info) => panic!("exited without acks: {info:?}"),
        }
    }
    assert!(received >= HIGH_WATER / 2, "too little output: {received}");
    assert!(
        received < HIGH_WATER + 64 * 1024,
        "flow control did not pause: {received}"
    );

    // Acknowledging lets everything through.
    manager.ack(info.id, received).expect("ack");
    let deadline = Instant::now() + TIMEOUT;
    loop {
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(PtyOutput::Data(bytes)) => {
                received += bytes.len();
                // The session may already have exited while these bytes were queued.
                let _ = manager.ack(info.id, bytes.len());
            }
            Ok(PtyOutput::Exit(exit)) => {
                assert_eq!(exit.code, Some(0));
                break;
            }
            Err(_) => panic!("stalled at {received} bytes"),
        }
    }
    assert_eq!(received, TOTAL);
}

#[test]
fn unknown_sessions_are_errors() {
    let manager = PtyManager::new();
    assert!(matches!(
        manager.write(999, b"x"),
        Err(crate::error::AppError::NotFound(_))
    ));
    assert!(manager.resize(999, TermSize::clamped(80, 24)).is_err());
    assert!(!manager.kill(999));
}

#[test]
fn default_shell_starts_interactively() {
    let manager = PtyManager::new();
    let launch = resolve_launch(PtyKind::Shell, None, &[]).expect("shell");
    assert!(launch.program.is_absolute());
    let (info, rx) = spawn(&manager, &launch);
    manager
        .write(info.id, b"echo shell-$((40+2))\nexit\n")
        .expect("write");
    let (output, exit) = read_to_exit(&rx);
    assert!(output.contains("shell-42"), "output was {output:?}");
    assert_eq!(exit.code, Some(0));
}
