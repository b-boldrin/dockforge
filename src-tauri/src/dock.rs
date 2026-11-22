//! Turns "what is running" plus "what the user pinned" into the list the dock shows.

use crate::model::AppEntry;
use crate::settings::Settings;

fn same_path(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}

/// What the dock shows:
/// - favorites first (if they are on), then the other running apps;
/// - with groups on, group 1 is exactly that, minus the apps the user moved to another
///   group. Groups 2 to 4 show only the apps moved there, open or not.
///
/// `show_mode` can then hide the pinned or the open ones.
pub fn compose(settings: &Settings, running: Vec<AppEntry>) -> Vec<AppEntry> {
    let groups = settings.groups_enabled;
    let active = if groups { settings.active_group } else { 0 };
    let mut rest = running;
    let mut out: Vec<AppEntry> = Vec::new();

    if groups && active == 0 {
        // Apps that live in another group do not show up here.
        let elsewhere = |a: &AppEntry| {
            a.exe.as_deref().is_some_and(|e| settings.pins.iter().any(|p| p.group > 0 && same_path(e, &p.path)))
        };
        rest.retain(|a| !elsewhere(a));
    }

    let show_pins = active > 0 || settings.favorites_enabled;
    for pin in settings.pins.iter().filter(|p| show_pins && p.group == active) {
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
    if active == 0 {
        // The open apps in the order the user dragged them into; the rest after, as they opened.
        let place = |a: &AppEntry| settings.app_order.iter().position(|k| k == &a.key).unwrap_or(usize::MAX);
        rest.sort_by_key(place);
        out.extend(rest);
    }

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
    fn group_one_has_favorites_and_open_apps_and_the_others_start_empty() {
        let mut s = settings("both");
        s.groups_enabled = true;
        let run = || vec![running("C:\\code.exe", "Code"), running("C:\\game.exe", "Game")];
        let names = |s: &Settings| compose(s, run()).into_iter().map(|a| a.name).collect::<Vec<_>>();
        assert_eq!(names(&s), ["Chrome", "Notes", "Code", "Game"]);
        s.active_group = 2;
        assert!(names(&s).is_empty());

        // Moving an open app takes it out of group 1 and into its new group.
        s.move_to_group("Game", "C:\\game.exe", 2);
        assert_eq!(names(&s), ["Game"]);
        s.active_group = 0;
        assert_eq!(names(&s), ["Chrome", "Notes", "Code"]);

        // With groups off everything open shows again.
        s.groups_enabled = false;
        assert_eq!(names(&s), ["Chrome", "Notes", "Code", "Game"]);
    }

    #[test]
    fn dragged_order_is_kept_for_open_apps_and_favorites() {
        let run = || vec![running("C:\\code.exe", "Code"), running("C:\\game.exe", "Game"), running("C:\\mail.exe", "Mail")];
        let mut s = settings("both");
        let names = |s: &Settings| compose(s, run()).into_iter().map(|a| a.name).collect::<Vec<_>>();
        assert_eq!(names(&s), ["Chrome", "Notes", "Code", "Game", "Mail"]);

        // Mail dragged in front of Code; an app not in the list stays after the dragged ones.
        let shown: Vec<String> = ["c:\\code.exe", "c:\\game.exe", "c:\\mail.exe"].map(String::from).to_vec();
        assert!(s.reorder_open("c:\\mail.exe", Some("c:\\code.exe"), &shown));
        assert_eq!(names(&s), ["Chrome", "Notes", "Mail", "Code", "Game"]);
        // The same move again changes nothing.
        let shown: Vec<String> = ["c:\\mail.exe", "c:\\code.exe", "c:\\game.exe"].map(String::from).to_vec();
        assert!(!s.reorder_open("c:\\mail.exe", Some("c:\\code.exe"), &shown));

        // Notes dragged in front of Chrome, then Chrome to the end of the favorites.
        assert!(s.reorder_pin("C:\\notes.exe", Some("C:\\chrome.exe")));
        assert_eq!(names(&s)[..2], ["Notes", "Chrome"]);
        assert!(s.reorder_pin("C:\\notes.exe", None));
        assert_eq!(names(&s)[..2], ["Chrome", "Notes"]);
    }

    #[test]
    fn favorites_can_be_turned_off() {
        let mut s = settings("both");
        s.favorites_enabled = false;
        let apps = compose(&s, vec![running("C:\\chrome.exe", "chrome")]);
        assert_eq!(apps.len(), 1);
        assert!(!apps[0].pinned);
    }

    #[test]
    fn show_modes_filter() {
        let run = || vec![running("C:\\code.exe", "Code"), running("C:\\chrome.exe", "chrome")];
        assert_eq!(compose(&settings("pinned"), run()).len(), 2);
        let open: Vec<_> = compose(&settings("open"), run()).into_iter().map(|a| a.name).collect();
        assert_eq!(open, ["Chrome", "Code"]);
    }
}
