use anyhow::{Context, Result};
use std::process::Command;

/// Launch the Tauri dashboard binary in the background with stdio detached.
pub fn handle() -> Result<()> {
    spawn_dashboard()?;
    println!("dashboard started");
    Ok(())
}

/// Spawn the `healthctl-dashboard` binary in the background.
fn spawn_dashboard() -> Result<()> {
    // Find the dashboard binary next to ourselves.
    let self_exe = std::env::current_exe()?;
    let dashboard_exe = self_exe
        .parent()
        .expect("binary has parent dir")
        .join("healthctl-dashboard");

    if !dashboard_exe.exists() {
        anyhow::bail!("dashboard binary not found at {}", dashboard_exe.display());
    }

    Command::new(&dashboard_exe)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to spawn healthctl-dashboard")?;

    tracing::info!("spawned dashboard: {}", dashboard_exe.display());
    Ok(())
}
