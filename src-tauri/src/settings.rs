//! User settings, stored as JSON in %APPDATA%\DockForge\settings.json.
//!
//! Everything the settings window can change lives in [`Settings`]. Values coming from the
//! web side are never trusted: [`Settings::sanitized`] clamps numbers and rejects unknown
//! words before anything is saved or applied.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_THEME: &str = "forge-glass";
pub const MAX_PINS: usize = 40;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Pin {
    pub name: String,
    pub path: String,
    /// 0 = a favorite (always in group 1). 1..=3 = an app the user moved to group 2, 3 or 4.
    pub group: u8,
}


/// A dock always has at least `MIN_GROUPS` groups and at most `MAX_GROUPS` (Alt+1 … Alt+9).
pub const MIN_GROUPS: usize = 4;
pub const MAX_GROUPS: usize = 9;

/// The kinds of players the user can turn on and off one by one.
pub const MEDIA_KINDS: [&str; 3] = ["spotify", "browsers", "others"];

/// Which kind a player is, from the name Windows gives for it.
pub fn media_kind(source: &str) -> &'static str {
    const BROWSERS: [&str; 8] = ["chrome", "msedge", "edge", "firefox", "brave", "opera", "vivaldi", "arc"];
    let name = source.to_lowercase();
    if name.contains("spotify") {
        "spotify"
    } else if BROWSERS.iter().any(|b| name.contains(b)) {
        "browsers"
    } else {
        "others"
    }
}

/// Bumped when a saved settings file needs a one-time change on load (see `migrated`).
pub const VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Which `VERSION` wrote the file. Files from before versions existed load as 0.
    #[serde(default)]
    pub version: u32,
    pub theme: String,
    /// "bottom" | "top" | "left" | "right"
    pub position: String,
    pub icon_size: u32,
    pub magnify: f32,
    pub auto_hide: bool,
    /// A dock that is always visible keeps its strip of the screen for itself, so maximized
    /// windows end where the dock begins (like the Windows taskbar does).
    pub reserve_space: bool,
    /// Milliseconds to wait before an auto-hidden dock slides away.
    pub hide_delay: u32,
    /// "both" | "pinned" | "open"
    pub show_mode: String,
    /// "focus-min" | "cycle" | "focus"
    pub click_action: String,
    pub separator: bool,
    /// Show each open app's name next to its icon (horizontal docks only).
    pub labels: bool,
    /// Optional tweaks on top of the theme. `None` means "whatever the theme says".
    pub accent: Option<String>,
    pub radius: Option<u32>,
    pub opacity: Option<u32>,
    pub pins: Vec<Pin>,
    pub start_with_windows: bool,
    /// Hide the Windows taskbar while DockForge is running (on by default: the dock replaces
    /// it). It comes back when DockForge closes.
    pub hide_taskbar: bool,
    /// "pt" | "en"
    pub language: String,
    /// Animations: "on", "off", or "system" (follow the Windows "animation effects" switch).
    pub motion: String,
    /// "float": a dock sized to its icons. "bar": edge to edge, with the clock at the end.
    /// "island": a small pill (clock, music) that opens to show the apps when pointed at.
    pub layout: String,
    /// Where along its edge a float or island dock sits: "start" | "center" | "end".
    /// "start" is left for a horizontal dock and top for a vertical one.
    pub align: String,
    /// Show the favorites (pins of group 0). Off = the dock only shows what is open.
    pub favorites_enabled: bool,
    /// Four groups, switched with Alt+1 … Alt+4. Group 1 has the favorites and everything
    /// open; the others start empty and get the apps the user moves there.
    pub groups_enabled: bool,
    pub active_group: u8,
    pub group_names: Vec<String>,
    pub widget_media: bool,
    /// Kinds of players the "now playing" widget ignores: "spotify", "browsers", "others".
    pub media_blocked: Vec<String>,
    pub widget_clock: bool,
    /// Processor, memory and battery.
    pub widget_system: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: VERSION,
            theme: DEFAULT_THEME.to_string(),
            position: "bottom".into(),
            icon_size: 48,
            magnify: 1.3,
            auto_hide: false,
            reserve_space: true,
            hide_delay: 600,
            show_mode: "both".into(),
            click_action: "focus-min".into(),
            separator: true,
            labels: false,
            accent: None,
            radius: None,
            opacity: None,
            pins: Vec::new(),
            start_with_windows: false,
            hide_taskbar: true,
            language: "pt".into(),
            motion: "on".into(),
            layout: "float".into(),
            align: "center".into(),
            favorites_enabled: true,
            groups_enabled: false,
            active_group: 0,
            group_names: default_group_names(),
            widget_media: false,
media_blocked: Vec::new(),
                        widget_clock: true,
            widget_system: false,
        }
    }
}

fn default_group_names() -> Vec<String> {
    vec!["Principal".into(), "Estudo".into(), "Jogos".into(), "Criação".into()]
}

/// The name a group gets when the user has not typed one.
fn default_group_name(index: usize) -> String {
    default_group_names().get(index).cloned().unwrap_or_else(|| format!("Grupo {}", index + 1))
}

fn one_of(value: &str, allowed: &[&str], fallback: &str) -> String {
    if allowed.contains(&value) { value.to_string() } else { fallback.to_string() }
}

fn is_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

impl Settings {
    /// Returns a copy that is safe to save and apply.
    pub fn sanitized(&self) -> Settings {
        let defaults = Settings::default();
        // How many groups there are is how many names were saved.
        let groups = self.group_names.len().clamp(MIN_GROUPS, MAX_GROUPS);
        let mut pins: Vec<Pin> = Vec::new();
        for pin in &self.pins {
            let name = pin.name.trim();
            let path = pin.path.trim();
            // An app lives in one place only: a favorite, or a member of one group.
            let duplicate = pins.iter().any(|p: &Pin| p.path.eq_ignore_ascii_case(path));
            // An app whose group was removed goes back to being a plain open app.
            let orphan = pin.group as usize >= groups;
            if name.is_empty() || path.is_empty() || name.len() > 120 || path.len() > 520 || duplicate || orphan {
                continue;
            }
            pins.push(Pin { name: name.to_string(), path: path.to_string(), group: pin.group });
            if pins.len() >= MAX_PINS {
                break;
            }
        }
        let layout = one_of(&self.layout, &["float", "bar", "island"], "float");
        let mut position = one_of(&self.position, &["bottom", "top", "left", "right"], "bottom");
        // The island opens sideways, so it only lives on the top or bottom edge.
        if layout == "island" && matches!(position.as_str(), "left" | "right") {
            position = "top".into();
        }
        Settings {
            version: self.version.min(VERSION),
            theme: if self.theme.is_empty() || self.theme.len() > 40 { defaults.theme } else { self.theme.clone() },
            position,
            icon_size: self.icon_size.clamp(32, 72),
            magnify: if self.magnify.is_finite() { self.magnify.clamp(1.0, 1.6) } else { 1.3 },
            auto_hide: self.auto_hide,
            reserve_space: self.reserve_space,
            hide_delay: self.hide_delay.min(2000),
            show_mode: one_of(&self.show_mode, &["both", "pinned", "open"], "both"),
            click_action: one_of(&self.click_action, &["focus-min", "cycle", "focus"], "focus-min"),
            separator: self.separator,
            labels: self.labels,
            accent: self.accent.clone().filter(|c| is_hex_color(c)),
            radius: self.radius.map(|r| r.min(32)),
            opacity: self.opacity.map(|o| o.clamp(30, 100)),
            pins,
            start_with_windows: self.start_with_windows,
            hide_taskbar: self.hide_taskbar,
            language: one_of(&self.language, &["pt", "en"], "pt"),
            motion: one_of(&self.motion, &["on", "system", "off"], "on"),
            layout,
            align: one_of(&self.align, &["start", "center", "end"], "center"),
            favorites_enabled: self.favorites_enabled,
            groups_enabled: self.groups_enabled,
            active_group: self.active_group.min(groups as u8 - 1),
            group_names: (0..groups)
                .map(|i| {
                    let name: String = self.group_names.get(i).map(|n| n.trim().chars().take(24).collect()).unwrap_or_default();
                    if name.is_empty() { default_group_name(i) } else { name }
                })
                .collect(),
            widget_media: self.widget_media,
            media_blocked: MEDIA_KINDS.iter().filter(|k| self.media_blocked.iter().any(|b| b == *k)).map(|k| k.to_string()).collect(),
            widget_clock: self.widget_clock,
            widget_system: self.widget_system,
        }
    }
}

impl Settings {
    /// Puts an app in a group. Group 0 means "back to group 1": the app leaves whatever
    /// group it was in (a favorite stays a favorite). Returns false if nothing changed.
    pub fn move_to_group(&mut self, name: &str, path: &str, group: u8) -> bool {
        let before = self.pins.clone();
        if group == 0 {
            self.pins.retain(|p| !(p.group > 0 && p.path.eq_ignore_ascii_case(path)));
        } else {
            self.pins.retain(|p| !p.path.eq_ignore_ascii_case(path));
            self.pins.push(Pin { name: name.to_string(), path: path.to_string(), group: group.min(self.group_names.len().max(1) as u8 - 1) });
        }
        self.pins != before
    }
}

impl Settings {
    /// One-time changes to files saved by older versions.
    fn migrated(mut self) -> Settings {
        if self.version < 1 {
            // Hiding the Windows taskbar became the default. Older files carry the old
            // default (off) without the user ever having chosen it.
            self.hide_taskbar = true;
        }
        self.version = VERSION;
        self
    }
}

fn file(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

pub fn load(data_dir: &Path) -> Settings {
    fs::read_to_string(file(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .map(|s| s.migrated().sanitized())
        .unwrap_or_default()
}

pub fn save(data_dir: &Path, settings: &Settings) -> std::io::Result<()> {
    fs::create_dir_all(data_dir)?;
    let json = serde_json::to_string_pretty(settings).map_err(std::io::Error::other)?;
    fs::write(file(data_dir), json)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_numbers_and_rejects_unknown_words() {
        let s = Settings {
            position: "middle".into(),
            icon_size: 500,
            magnify: f32::NAN,
            hide_delay: 99_999,
            show_mode: "x".into(),
            click_action: "y".into(),
            accent: Some("javascript:alert(1)".into()),
            radius: Some(900),
            opacity: Some(1),
            language: "zz".into(),
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(s.position, "bottom");
        assert_eq!(s.icon_size, 72);
        assert_eq!(s.magnify, 1.3);
        assert_eq!(s.hide_delay, 2000);
        assert_eq!(s.show_mode, "both");
        assert_eq!(s.click_action, "focus-min");
        assert_eq!(s.accent, None);
        assert_eq!(s.radius, Some(32));
        assert_eq!(s.opacity, Some(30));
        assert_eq!(s.language, "pt");
    }

    #[test]
    fn groups_are_clamped_and_named() {
        let s = Settings {
            active_group: 9,
            group_names: vec!["  ".into(), "Música".into()],
            pins: vec![Pin { name: "A".into(), path: "a.exe".into(), group: 7 }, Pin { name: "A".into(), path: "a.exe".into(), group: 0 }],
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(s.active_group, 3);
        assert_eq!(s.group_names, ["Principal", "Música", "Jogos", "Criação"]);
        // The entry for a group that does not exist is dropped; the other one stays.
        assert_eq!(s.pins.len(), 1);
        assert_eq!(s.pins[0].group, 0);
    }

    #[test]
    fn moving_an_app_between_groups() {
        let mut s = Settings { pins: vec![Pin { name: "Fav".into(), path: "f.exe".into(), group: 0 }], ..Settings::default() };
        assert!(s.move_to_group("Code", "C:\\code.exe", 2));
        assert_eq!(s.pins[1].group, 2);
        // Moving again replaces the old place instead of duplicating.
        assert!(s.move_to_group("Code", "c:\\CODE.exe", 3));
        assert_eq!(s.pins.iter().filter(|p| p.group > 0).count(), 1);
        // Back to group 1: the app is simply no longer assigned.
        assert!(s.move_to_group("Code", "C:\\code.exe", 0));
        assert_eq!(s.pins.len(), 1);
        // A favorite sent to group 1 stays a favorite.
        assert!(!s.move_to_group("Fav", "f.exe", 0));
        assert_eq!(s.pins[0].group, 0);
    }

    #[test]
    fn keeps_valid_hex_and_drops_duplicate_or_empty_pins() {
        let pin = |n: &str, p: &str| Pin { name: n.into(), path: p.into(), group: 0 };
        let s = Settings {
            accent: Some("#5ee6e0".into()),
            pins: vec![pin("A", "C:\\a.exe"), pin("A again", "c:\\A.EXE"), pin("", "C:\\b.exe"), pin("C", "C:\\c.exe")],
            ..Settings::default()
        }
        .sanitized();
        assert_eq!(s.accent.as_deref(), Some("#5ee6e0"));
        assert_eq!(s.pins.len(), 2);
    }

    #[test]
    fn players_are_sorted_into_kinds() {
        assert_eq!(media_kind("Spotify"), "spotify");
        assert_eq!(media_kind("SpotifyAB.SpotifyMusic_zpdnekdrzrea0"), "spotify");
        assert_eq!(media_kind("Msedge"), "browsers");
        assert_eq!(media_kind("Chrome"), "browsers");
        assert_eq!(media_kind("Vlc"), "others");
        let s = Settings { media_blocked: vec!["others".into(), "radio".into(), "spotify".into(), "spotify".into()], ..Settings::default() }.sanitized();
        assert_eq!(s.media_blocked, ["spotify", "others"]);
    }

    #[test]
    fn groups_go_from_four_to_nine() {
        let names = |n: usize| (0..n).map(|i| format!("G{i}")).collect::<Vec<_>>();
        let pin = |group: u8| Pin { name: format!("p{group}"), path: format!("{group}.exe"), group };
        let s = Settings { group_names: names(12), pins: vec![pin(8), pin(9)], active_group: 20, ..Settings::default() }.sanitized();
        assert_eq!(s.group_names.len(), MAX_GROUPS);
        assert_eq!(s.active_group, 8);
        assert_eq!(s.pins.len(), 1);

        // Removing a group sends its apps back and never goes below four.
        let mut fewer = s.clone();
        fewer.group_names.truncate(2);
        fewer.group_names.push(String::new());
        let fewer = fewer.sanitized();
        assert_eq!(fewer.group_names, ["G0", "G1", "Jogos", "Criação"]);
        assert!(fewer.pins.is_empty());
        assert_eq!(fewer.active_group, 3);
        assert_eq!(default_group_name(6), "Grupo 7");
    }

    #[test]
    fn the_island_stays_on_a_horizontal_edge() {
        let s = Settings { layout: "island".into(), position: "left".into(), align: "diagonal".into(), ..Settings::default() }.sanitized();
        assert_eq!((s.layout.as_str(), s.position.as_str(), s.align.as_str()), ("island", "top", "center"));
    }

    #[test]
    fn old_settings_files_still_load() {
        let s: Settings = serde_json::from_str(r#"{"theme":"midnight-pill"}"#).unwrap();
        assert_eq!(s.theme, "midnight-pill");
        assert_eq!(s.icon_size, 48);
    }

    #[test]
    fn files_from_before_the_taskbar_default_get_it_once() {
        let old: Settings = serde_json::from_str(r#"{"theme":"papel","hideTaskbar":false}"#).unwrap();
        assert_eq!(old.version, 0);
        let old = old.migrated();
        assert!(old.hide_taskbar);
        assert_eq!(old.version, VERSION);

        // Once migrated, turning it off sticks.
        let chosen: Settings = serde_json::from_str(r#"{"version":1,"hideTaskbar":false}"#).unwrap();
        assert!(!chosen.migrated().hide_taskbar);
        assert!(Settings::default().hide_taskbar);
        assert_eq!(Settings::default().version, VERSION);
    }
}
