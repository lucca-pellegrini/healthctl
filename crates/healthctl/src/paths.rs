use anyhow::Result;
use std::path::PathBuf;

/// Find a companion binary (daemon or dashboard) that was bundled with healthctl.
/// Search order:
/// 1. Inside AppImage at $APPDIR/usr/bin/ (when running from AppImage)
/// 2. Next to the current executable (normal install)
pub fn find_companion_binary(name: &str) -> Result<PathBuf> {
    // Check if running inside an AppImage (APPDIR env var is set)
    if let Ok(appdir) = std::env::var("APPDIR") {
        let appimage_path = PathBuf::from(&appdir).join("usr/bin").join(name);
        if appimage_path.exists() {
            return Ok(appimage_path);
        }
    }

    // Fall back to looking next to the current executable
    let self_exe = std::env::current_exe()?;
    let sibling_path = self_exe.parent().expect("binary has parent dir").join(name);

    if sibling_path.exists() {
        return Ok(sibling_path);
    }

    anyhow::bail!(
        "{} binary not found (checked APPDIR and {})",
        name,
        sibling_path.display()
    )
}
