//! Small things that touch the operating system but are not about windows or icons.

/// Turns "start with Windows" on or off for the current user (the standard Run key).
/// Uses the built-in `reg` tool, so no extra dependency is needed.
#[cfg(windows)]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    const KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

    let mut cmd = Command::new("reg");
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        cmd.args(["add", KEY, "/v", "DockForge", "/t", "REG_SZ", "/d"])
            .arg(format!("\"{}\"", exe.display()))
            .arg("/f");
    } else {
        cmd.args(["delete", KEY, "/v", "DockForge", "/f"]);
    }
    let output = cmd.creation_flags(CREATE_NO_WINDOW).output().map_err(|e| e.to_string())?;
    // Deleting a value that does not exist is fine.
    if output.status.success() || !enabled {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[cfg(not(windows))]
pub fn set_autostart(_enabled: bool) -> Result<(), String> {
    Ok(())
}
