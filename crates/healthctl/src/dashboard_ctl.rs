use anyhow::{Context, Result};
use std::process::Command;

use crate::paths::find_companion_binary;

/// Launch the Tauri dashboard binary in the background with stdio detached.
pub fn handle() -> Result<()> {
    spawn_dashboard()?;
    println!("dashboard started");
    Ok(())
}

/// Spawn the `healthctl-dashboard` binary in the background.
fn spawn_dashboard() -> Result<()> {
    let dashboard_exe = find_companion_binary("healthctl-dashboard")?;

    Command::new(&dashboard_exe)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .context("failed to spawn healthctl-dashboard")?;

    tracing::info!("spawned dashboard: {}", dashboard_exe.display());
    Ok(())
}
