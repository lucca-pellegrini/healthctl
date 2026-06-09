use anyhow::{Context, Result};
use healthctl_lib::ipc;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::Command;

use crate::cli::{DaemonAction, DaemonCommand};
use crate::paths::find_companion_binary;

pub fn handle(cmd: DaemonCommand) -> Result<()> {
    match cmd.action {
        Some(DaemonAction::Stop) => stop_daemon(),
        Some(DaemonAction::Restart) => {
            stop_daemon().ok(); // Ignore error if not running.
            std::thread::sleep(std::time::Duration::from_millis(300));
            spawn_daemon()?;
            println!("daemon restarted");
            Ok(())
        }
        Some(DaemonAction::Status) => check_status(),
        // `start` and the bare `daemon` both start the daemon if not running.
        Some(DaemonAction::Start) | None => start_daemon(),
    }
}

/// Start the daemon if it is not already running.
fn start_daemon() -> Result<()> {
    let socket_path = ipc::socket_path();
    if UnixStream::connect(&socket_path).is_ok() {
        println!("daemon is already running");
    } else {
        spawn_daemon()?;
        println!("daemon started");
    }
    Ok(())
}

/// Spawn the daemon binary in the background with stdio detached.
pub fn spawn_daemon() -> Result<()> {
    let daemon_exe = find_companion_binary("healthctl-daemon")?;

    Command::new(&daemon_exe)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to spawn healthctl-daemon")?;

    tracing::info!("spawned daemon: {}", daemon_exe.display());
    Ok(())
}

fn stop_daemon() -> Result<()> {
    let socket_path = ipc::socket_path();
    let stream = UnixStream::connect(&socket_path).context("daemon is not running")?;
    let mut stream = stream;

    let payload = serde_json::to_string(&ipc::Request::Shutdown)?;
    writeln!(stream, "{payload}")?;
    stream.flush()?;

    let mut reader = BufReader::new(&stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).ok();

    println!("daemon stopped");
    Ok(())
}

fn check_status() -> Result<()> {
    let socket_path = ipc::socket_path();
    match UnixStream::connect(&socket_path) {
        Ok(stream) => {
            let mut stream = stream;
            let payload = serde_json::to_string(&ipc::Request::Ping)?;
            writeln!(stream, "{payload}")?;
            stream.flush()?;

            let mut reader = BufReader::new(&stream);
            let mut response_line = String::new();
            reader.read_line(&mut response_line)?;

            println!("daemon is running (socket: {})", socket_path.display());
        }
        Err(_) => {
            println!("daemon is not running");
        }
    }
    Ok(())
}
