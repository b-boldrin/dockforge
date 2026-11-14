//! Data sent to the web side. Mirrors src/lib/types.ts.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WindowInfo {
    /// Native window handle, as a number (fits in a JS number on x64).
    pub hwnd: i64,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AppEntry {
    pub key: String,
    pub name: String,
    pub exe: Option<String>,
    pub windows: Vec<WindowInfo>,
    pub active: bool,
    /// True if the user pinned this app: it stays in the dock even when closed.
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemePayload {
    pub id: String,
    pub name: String,
    pub css: String,
}

/// What the store shows for each installed theme.
#[derive(Debug, Clone, Serialize)]
pub struct ThemeSummary {
    pub id: String,
    pub name: String,
    pub author: String,
    pub tags: Vec<String>,
    pub built_in: bool,
}

/// What is playing right now, as reported by Windows' media controls (the same source as
/// the volume flyout): Spotify, YouTube Music in a browser, any player that integrates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MediaInfo {
    pub title: String,
    pub artist: String,
    /// The app that is playing, e.g. "Spotify" or "Chrome".
    pub source: String,
    pub playing: bool,
    /// Cover art as a data: URL, when the player provides one.
    pub thumbnail: Option<String>,
}

/// Numbers for the system widget. All percentages are 0–100.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct SystemStats {
    pub cpu: u8,
    pub memory: u8,
    /// `None` on a computer without a battery.
    pub battery: Option<u8>,
    pub charging: bool,
}
