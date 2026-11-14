//! Stand-in for non-Windows systems: fake apps so the dock can be styled anywhere.

use crate::model::{AppEntry, MediaInfo, WindowInfo};

fn sample(key: &str, name: &str, windows: usize, active: bool) -> AppEntry {
    AppEntry {
        key: key.to_string(),
        name: name.to_string(),
        exe: None,
        windows: (0..windows)
            .map(|i| WindowInfo { hwnd: (key.len() * 100 + i) as i64, title: format!("{name} {}", i + 1) })
            .collect(),
        active,
        pinned: false,
    }
}

pub fn snapshot() -> Vec<AppEntry> {
    vec![
        sample("files", "File Explorer", 1, false),
        sample("browser", "Browser", 2, true),
        sample("editor", "Code Editor", 1, false),
        sample("music", "Music", 1, false),
        sample("chat", "Chat", 1, false),
    ]
}

pub fn activate(_hwnds: &[i64], _action: &str) -> Result<(), String> {
    Ok(())
}

pub fn close_windows(_hwnds: &[i64]) {}

pub fn icon_data_url(_exe: &str) -> Option<String> {
    None
}

pub fn start_watcher(_notify: impl Fn() + Send + Sync + 'static) {}

/// A fixed sample so the "now playing" widget can be styled on any system.
pub fn media_now() -> Option<MediaInfo> {
    Some(MediaInfo {
        title: "Dozing Off Again".into(),
        artist: "Sample Artist".into(),
        source: "Spotify".into(),
        playing: true,
        thumbnail: None,
    })
}

pub fn media_command(_command: &str) -> Result<(), String> {
    Ok(())
}
