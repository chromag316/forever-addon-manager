//! Per-OS paths and process helpers.

use std::path::PathBuf;
use std::process::Command;

/// Where WoW Forever's AddOns folder usually is (changeable in Settings).
pub fn default_addons_path() -> String {
    #[cfg(target_os = "macos")]
    return "/Applications/World of Warcraft/_forever_/Interface/AddOns".into();
    #[cfg(target_os = "windows")]
    return r"C:\Program Files (x86)\World of Warcraft\_forever_\Interface\AddOns".into();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    return dirs::home_dir()
        .unwrap_or_default()
        .join("Games/battlenet/drive_c/Program Files (x86)/World of Warcraft/_forever_/Interface/AddOns")
        .to_string_lossy()
        .into();
}

/// App data folder: ~/Library/Application Support/ForeverAddons, %APPDATA%\ForeverAddons,
/// or ~/.local/share/ForeverAddons. (Same folder the earlier macOS app used.)
pub fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("ForeverAddons")
}

/// A `Command` that never pops up a console window on Windows.
pub fn command(program: &str) -> Command {
    #[allow(unused_mut)]
    let mut cmd = Command::new(program);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd
}
