//! Turns "what is running" plus "what the user pinned" into the list the dock shows.

use crate::model::AppEntry;
use crate::settings::Settings;

fn same_path(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// Order: pinned apps first (in the order the user chose), then the other running apps.
/// `show_mode` can hide either group.
pub fn compose(settings: &Settings, running: Vec<AppEntry>) -> Vec<AppEntry> {
    let mut rest = running;
    let mut out: Vec<AppEntry> = Vec::new();

    let group = settings.active_group;
    let pins = settings.pins.iter().filter(|p| !settings.groups_enabled || p.group == group);
    for pin in pins {
        let found = rest.iter().position(|a| a.exe.as_deref().is_some_and(|e| same_path(e, &pin.path)));
        let mut entry = match found {
            Some(i) => rest.remove(i),
            None => AppEntry {
                key: pin.path.to_lowercase(),
                name: pin.name.clone(),
                exe: Some(pin.path.clone()),
                windows: Vec::new(),
                active: false,
                pinned: true,
            },
        };
        entry.pinned = true;
        entry.name = pin.name.clone();
        out.push(entry);
    }
    out.extend(rest);

    match settings.show_mode.as_str() {
        "pinned" => out.retain(|a| a.pinned),
        "open" => out.retain(|a| !a.windows.is_empty()),
        _ => {}
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::WindowInfo;
    use crate::settings::Pin;

    fn running(path: &str, name: &str) -> AppEntry {
        AppEntry {
            key: path.to_lowercase(),
            name: name.into(),
            exe: Some(path.into()),
            windows: vec![WindowInfo { hwnd: 1, title: name.into() }],
            active: false,
            pinned: false,
        }
    }

    fn settings(mode: &str) -> Settings {
        Settings {
            show_mode: mode.into(),
            pins: vec![
                Pin { name: "Chrome".into(), path: "C:\\chrome.exe".into(), group: 0 },
                Pin { name: "Notes".into(), path: "C:\\notes.exe".into(), group: 0 },
            ],
            ..Settings::default()
        }
    }

    #[test]
    fn pinned_first_then_running_and_closed_pins_stay() {
        let apps = compose(&settings("both"), vec![running("C:\\code.exe", "Code"), running("C:\\CHROME.exe", "chrome")]);
        let names: Vec<_> = apps.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["Chrome", "Notes", "Code"]);
        assert!(apps[0].pinned && !apps[0].windows.is_empty());
        assert!(apps[1].pinned && apps[1].windows.is_empty());
        assert!(!apps[2].pinned);
    }

    #[test]
    fn groups_show_only_the_active_groups_pins() {
        let mut s = settings("both");
        s.groups_enabled = true;
        s.pins[1].group = 1;
        let names: Vec<_> = compose(&s, vec![]).into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["Chrome"]);
        s.active_group = 1;
        let names: Vec<_> = compose(&s, vec![]).into_iter().map(|a| a.name).collect();
        assert_eq!(names, ["Notes"]);
    }

    #[test]
    fn show_modes_filter() {
        let run = || vec![running("C:\\code.exe", "Code"), running("C:\\chrome.exe", "chrome")];
        assert_eq!(compose(&settings("pinned"), run()).len(), 2);
        let open: Vec<_> = compose(&settings("open"), run()).into_iter().map(|a| a.name).collect();
        assert_eq!(open, ["Chrome", "Code"]);
    }
}
