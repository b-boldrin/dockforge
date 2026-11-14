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
    /// Which of the 4 groups this pin belongs to (0..=3). Only used when groups are on.
    pub group: u8,
}


pub const GROUPS: usize = 4;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    /// "bottom" | "top" | "left" | "right"
    pub position: String,
    pub icon_size: u32,
    pub magnify: f32,
    pub auto_hide: bool,
    /// Milliseconds to wait before an auto-hidden dock slides away.
    pub hide_delay: u32,
    /// "both" | "pinned" | "open"
    pub show_mode: String,
    /// "focus-min" | "cycle" | "focus"
    pub click_action: String,
    pub separator: bool,
    /// Optional tweaks on top of the theme. `None` means "whatever the theme says".
    pub accent: Option<String>,
    pub radius: Option<u32>,
    pub opacity: Option<u32>,
    pub pins: Vec<Pin>,
    pub start_with_windows: bool,
    /// "pt" | "en"
    pub language: String,
    /// "float": a dock sized to its icons. "bar": edge to edge, with the clock at the end.
    pub layout: String,
    /// Four sets of pinned apps, switched with Alt+1 … Alt+4.
    pub groups_enabled: bool,
    pub active_group: u8,
    pub group_names: Vec<String>,
    pub widget_media: bool,
    pub widget_clock: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: DEFAULT_THEME.to_string(),
            position: "bottom".into(),
            icon_size: 48,
            magnify: 1.3,
            auto_hide: false,
            hide_delay: 600,
            show_mode: "both".into(),
            click_action: "focus-min".into(),
            separator: true,
            accent: None,
            radius: None,
            opacity: None,
            pins: Vec::new(),
            start_with_windows: false,
            language: "pt".into(),
            layout: "float".into(),
            groups_enabled: false,
            active_group: 0,
            group_names: default_group_names(),
            widget_media: false,
            widget_clock: true,
        }
    }
}

fn default_group_names() -> Vec<String> {
    vec!["Trabalho".into(), "Estudo".into(), "Jogos".into(), "Criação".into()]
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
        let mut pins: Vec<Pin> = Vec::new();
        for pin in &self.pins {
            let name = pin.name.trim();
            let path = pin.path.trim();
            let duplicate = pins.iter().any(|p: &Pin| p.path.eq_ignore_ascii_case(path) && p.group == pin.group);
            if name.is_empty() || path.is_empty() || name.len() > 120 || path.len() > 520 || duplicate {
                continue;
            }
            pins.push(Pin { name: name.to_string(), path: path.to_string(), group: pin.group.min(GROUPS as u8 - 1) });
            if pins.len() >= MAX_PINS {
                break;
            }
        }
        Settings {
            theme: if self.theme.is_empty() || self.theme.len() > 40 { defaults.theme } else { self.theme.clone() },
            position: one_of(&self.position, &["bottom", "top", "left", "right"], "bottom"),
            icon_size: self.icon_size.clamp(32, 72),
            magnify: if self.magnify.is_finite() { self.magnify.clamp(1.0, 1.6) } else { 1.3 },
            auto_hide: self.auto_hide,
            hide_delay: self.hide_delay.min(2000),
            show_mode: one_of(&self.show_mode, &["both", "pinned", "open"], "both"),
            click_action: one_of(&self.click_action, &["focus-min", "cycle", "focus"], "focus-min"),
            separator: self.separator,
            accent: self.accent.clone().filter(|c| is_hex_color(c)),
            radius: self.radius.map(|r| r.min(32)),
            opacity: self.opacity.map(|o| o.clamp(30, 100)),
            pins,
            start_with_windows: self.start_with_windows,
            language: one_of(&self.language, &["pt", "en"], "pt"),
            layout: one_of(&self.layout, &["float", "bar"], "float"),
            groups_enabled: self.groups_enabled,
            active_group: self.active_group.min(GROUPS as u8 - 1),
            group_names: (0..GROUPS)
                .map(|i| {
                    let name: String = self.group_names.get(i).map(|n| n.trim().chars().take(24).collect()).unwrap_or_default();
                    if name.is_empty() { default_group_names()[i].clone() } else { name }
                })
                .collect(),
            widget_media: self.widget_media,
            widget_clock: self.widget_clock,
        }
    }
}

fn file(data_dir: &Path) -> PathBuf {
    data_dir.join("settings.json")
}

pub fn load(data_dir: &Path) -> Settings {
    fs::read_to_string(file(data_dir))
        .ok()
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .map(|s| s.sanitized())
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
        assert_eq!(s.group_names, ["Trabalho", "Música", "Jogos", "Criação"]);
        // The same app may be pinned in two different groups.
        assert_eq!(s.pins.len(), 2);
        assert_eq!(s.pins[0].group, 3);
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
    fn old_settings_files_still_load() {
        let s: Settings = serde_json::from_str(r#"{"theme":"midnight-pill"}"#).unwrap();
        assert_eq!(s.theme, "midnight-pill");
        assert_eq!(s.icon_size, 48);
    }
}
