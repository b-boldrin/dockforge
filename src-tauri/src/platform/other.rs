//! Stand-in for non-Windows systems: fake apps so the dock can be styled anywhere.

use crate::model::{AppEntry, WindowInfo};

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

// The Windows taskbar only exists on Windows.
pub fn taskbar_state() -> u32 {
    0
}
pub fn hide_taskbar() {}
pub fn show_taskbar(_state: u32) {}

// Reserving a strip of the screen is a Windows feature.
pub fn reserve_edge(_window: isize, _edge: &str, _thickness: i32, _monitor: (i32, i32, i32, i32), _yield_to_others: bool) -> Option<(i32, i32, i32, i32)> {
    None
}
pub fn release_edge() {}

pub fn process_path(_pid: u32) -> Option<String> {
    None
}
pub fn display_name(exe: &str) -> String {
    exe.to_string()
}

pub fn fullscreen_active() -> bool {
    false
}

pub fn maximized_in_front() -> bool {
    false
}
