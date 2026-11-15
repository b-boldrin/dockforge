//! DockForge: an open-source, themeable dock for Windows.
//!
//! Layout:
//! - `platform`: everything that talks to Windows (window list, focus, icons, events)
//! - `dock`:     merges running apps with the user's pinned apps
//! - `themes`:   finding, loading and sanitizing themes
//! - `settings`: the user's choices, saved as JSON
//! - `system`:   start with Windows
//! - `taskbar`:  reads the apps pinned to the Windows taskbar
//! - this file:  Tauri commands, the windows, the tray menu and the background watchers

mod dock;
mod model;
mod platform;
mod settings;
mod system;
mod taskbar;
mod themes;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, ContextMenu, IconMenuItem, IsMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, State, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder, Wry,
};

use model::{AppEntry, MediaInfo, SystemStats, ThemePayload, ThemeSummary};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use settings::{Pin, Settings};

const TRAY_ID: &str = "main";

struct AppState {
    data_dir: PathBuf,
    settings: Mutex<Settings>,
    theme_items: Mutex<Vec<(String, CheckMenuItem<Wry>)>>,
    /// True while a right-click menu is open: the dock must not auto-hide under it.
    menu_open: AtomicBool,
    /// The dock entry the right-click menu was opened for.
    menu_target: Mutex<Option<AppEntry>>,
    /// True while an app is in full screen: the dock window is hidden and must stay hidden.
    fullscreen: AtomicBool,
    /// True while the dock is slid away because of auto-hide.
    hidden: AtomicBool,
    /// The dock's rectangle inside its window (x, y, width, height in CSS pixels).
    hit: Mutex<Option<(f64, f64, f64, f64)>>,
}

impl AppState {
    fn settings(&self) -> Settings {
        self.settings.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn current_theme(&self) -> ThemePayload {
        let id = self.settings().theme;
        themes::load(&self.data_dir, &id).unwrap_or_else(|err| {
            eprintln!("[dockforge] {err}; using the built-in theme");
            themes::fallback()
        })
    }

    fn apps(&self) -> Vec<AppEntry> {
        dock::compose(&self.settings(), platform::snapshot())
    }
}

/// Picks the Portuguese or English text for the interface the Rust side draws itself.
fn tr<'a>(lang: &str, pt: &'a str, en: &'a str) -> &'a str {
    if lang == "en" { en } else { pt }
}

// ---------------------------------------------------------------------------
// Commands called from the web side
// ---------------------------------------------------------------------------

#[tauri::command]
fn list_apps(state: State<'_, AppState>) -> Vec<AppEntry> {
    state.apps()
}

#[tauri::command]
fn activate_app(state: State<'_, AppState>, hwnds: Vec<i64>) -> Result<(), String> {
    platform::activate(&hwnds, &state.settings().click_action)
}

/// Starts a program, but only one that the user pinned or that is already running:
/// the web side cannot ask for an arbitrary path.
#[tauri::command]
fn launch_app(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let allowed = state.settings().pins.iter().any(|p| p.path.eq_ignore_ascii_case(&path))
        || platform::snapshot().iter().any(|a| a.exe.as_deref().is_some_and(|e| e.eq_ignore_ascii_case(&path)));
    if !allowed {
        return Err("this program is not pinned or running".into());
    }
    std::process::Command::new(&path).spawn().map(|_| ()).map_err(|e| format!("could not start {path}: {e}"))
}

#[tauri::command]
async fn get_icon(exe: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || platform::icon_data_url(&exe)).await.ok().flatten()
}

#[tauri::command]
fn get_theme(state: State<'_, AppState>) -> ThemePayload {
    state.current_theme()
}

#[tauri::command]
fn list_themes(state: State<'_, AppState>) -> Vec<ThemeSummary> {
    themes::list(&state.data_dir)
}

/// Any installed theme, for the previews in the store.
#[tauri::command]
fn load_theme(state: State<'_, AppState>, id: String) -> Result<ThemePayload, String> {
    themes::load(&state.data_dir, &id)
}

#[tauri::command]
fn open_themes_folder(state: State<'_, AppState>) {
    open_folder(&themes::themes_dir(&state.data_dir));
}

/// Opens one of the project's pages in the default browser. Only DockForge's own GitHub
/// pages are accepted, so the web side cannot open arbitrary addresses or programs.
#[tauri::command]
fn open_link(url: String) -> Result<(), String> {
    let allowed = url.starts_with("https://github.com/b-boldrin/dockforge")
        && url.chars().all(|c| c.is_ascii_alphanumeric() || "/:.-_".contains(c));
    if !allowed {
        return Err("link not allowed".into());
    }
    open_folder_or_url(&url);
    Ok(())
}

#[tauri::command]
fn reset_settings(app: AppHandle, state: State<'_, AppState>) -> Result<Settings, String> {
    // Keep the pins: losing them by accident would be annoying.
    let fresh = Settings { pins: state.settings().pins, ..Settings::default() };
    update_settings(app, state, fresh)
}

#[tauri::command]
fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings()
}

/// Saves new settings (after clamping them), applies what changed and tells every window.
#[tauri::command]
fn update_settings(app: AppHandle, state: State<'_, AppState>, settings: Settings) -> Result<Settings, String> {
    let next = settings.sanitized();
    let previous = state.settings();
    if next == previous {
        return Ok(next);
    }

    if next.start_with_windows != previous.start_with_windows {
        system::set_autostart(next.start_with_windows)?;
    }
    if next.hide_taskbar != previous.hide_taskbar {
        apply_taskbar(&state.data_dir, next.hide_taskbar);
    }
    if let Ok(mut current) = state.settings.lock() {
        *current = next.clone();
    }
    settings::save(&state.data_dir, &next).map_err(|e| format!("could not save settings: {e}"))?;

    if next.theme != previous.theme {
        emit_theme(&app);
    }
    if next.groups_enabled != previous.groups_enabled || next.group_names.len() != previous.group_names.len() {
        sync_shortcuts(&app, &next);
    }
    if next.theme != previous.theme || next.language != previous.language {
        let _ = build_tray(&app);
    }
    let _ = app.emit("settings-changed", &next);
    let _ = app.emit("apps-changed", state.apps());
    Ok(next)
}

/// The dock window's native handle, as a plain number (Windows only).
fn native_handle(window: &WebviewWindow) -> Option<isize> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    match window.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(handle) => Some(handle.hwnd.get()),
        _ => None,
    }
}

/// Resizes the dock window to `width`×`height` (CSS pixels) and puts it against the screen
/// edge chosen in the settings.
///
/// `thickness` is how far the dock itself reaches from that edge. A dock that is always
/// visible reserves that strip, so maximized windows end where the dock begins. An
/// auto-hiding dock and the island float over the windows instead.
#[tauri::command]
fn fit_dock(window: WebviewWindow, state: State<'_, AppState>, width: f64, height: f64, thickness: Option<f64>) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|e| e.to_string())?
        .or(window.primary_monitor().map_err(|e| e.to_string())?)
        .ok_or("no monitor found")?;

    let scale = monitor.scale_factor();
    let settings = state.settings();
    let edge = settings.position.as_str();
    let vertical = matches!(edge, "left" | "right");
    let bar = settings.layout == "bar";

    // The area the dock is placed in: (left, top, right, bottom). It starts as the work
    // area (the screen minus the Windows taskbar and other toolbars).
    let work = monitor.work_area();
    let mut area = (
        work.position.x,
        work.position.y,
        work.position.x + work.size.width as i32,
        work.position.y + work.size.height as i32,
    );

    let reserve = settings.reserve_space && !settings.auto_hide && settings.layout != "island";
    let strip = match (reserve, thickness, native_handle(&window)) {
        (true, Some(thickness), Some(handle)) if thickness > 0.0 => {
            let (pos, size) = (monitor.position(), monitor.size());
            let screen = (pos.x, pos.y, pos.x + size.width as i32, pos.y + size.height as i32);
            // A dock that replaces the Windows taskbar takes the screen edge itself.
            platform::reserve_edge(handle, edge, (thickness * scale).round() as i32, screen, !settings.hide_taskbar)
        }
        _ => {
            platform::release_edge();
            None
        }
    };
    if let Some((left, top, right, bottom)) = strip {
        // The work area no longer includes the dock's own strip, so the dock's edge is the
        // outer edge of the strip.
        match edge {
            "top" => area.1 = top,
            "left" => area.0 = left,
            "right" => area.2 = right,
            _ => area.3 = bottom,
        }
    }

    let (area_w, area_h) = ((area.2 - area.0).max(1), (area.3 - area.1).max(1));
    let mut w = ((width.max(1.0) * scale).round() as i32).min(area_w);
    let mut h = ((height.max(1.0) * scale).round() as i32).min(area_h);
    // "Barra inteira" goes from one end of the screen to the other.
    if bar && !vertical {
        w = area_w;
    }
    if bar && vertical {
        h = area_h;
    }

    // Along its edge the dock sits at the start, the middle or the end.
    let along = |room: i32| match settings.align.as_str() {
        "start" => 0,
        "end" => room,
        _ => room / 2,
    };
    let (x, y) = match edge {
        "top" => (area.0 + along(area_w - w), area.1),
        "left" => (area.0, area.1 + along(area_h - h)),
        "right" => (area.2 - w, area.1 + along(area_h - h)),
        _ => (area.0 + along(area_w - w), area.3 - h),
    };

    window.set_size(PhysicalSize::new(w as u32, h as u32)).map_err(|e| e.to_string())?;
    window.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    if !state.fullscreen.load(Ordering::Relaxed) && !window.is_visible().unwrap_or(true) {
        window.show().map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// What is playing, unless the widget is off or that kind of player was turned off.
fn media_for(settings: &Settings) -> Option<MediaInfo> {
    if !settings.widget_media {
        return None;
    }
    platform::media_now().filter(|m| !settings.media_blocked.iter().any(|b| b == settings::media_kind(&m.source)))
}

#[tauri::command]
async fn get_media(state: State<'_, AppState>) -> Result<Option<MediaInfo>, String> {
    let settings = state.settings();
    tauri::async_runtime::spawn_blocking(move || media_for(&settings)).await.map_err(|e| e.to_string())
}

#[tauri::command]
fn get_system(state: State<'_, AppState>) -> Option<SystemStats> {
    state.settings().widget_system.then(platform::system_stats)
}

/// Play/pause, next, previous. Runs off the main thread: Windows answers asynchronously.
#[tauri::command]
async fn media_control(command: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || platform::media_command(&command))
        .await
        .map_err(|e| e.to_string())?
}

/// The web side tells where the dock is drawn, so the rest of the window can let clicks through.
#[tauri::command]
fn set_hit_rect(state: State<'_, AppState>, x: f64, y: f64, width: f64, height: f64) {
    let valid = [x, y, width, height].iter().all(|v| v.is_finite()) && width > 0.0 && height > 0.0;
    if let Ok(mut hit) = state.hit.lock() {
        *hit = valid.then_some((x, y, width, height));
    }
}

/// Adds the apps pinned to the Windows taskbar to the favorites and turns the favorites on.
/// Returns how many were added (apps that already have a place are left alone).
#[tauri::command]
fn import_taskbar_pins(app: AppHandle, state: State<'_, AppState>) -> Result<usize, String> {
    let mut next = state.settings();
    let before = next.pins.len();
    for pin in taskbar::pinned() {
        if !next.pins.iter().any(|p| p.path.eq_ignore_ascii_case(&pin.path)) {
            next.pins.push(pin);
        }
    }
    next.favorites_enabled = true;
    let saved = update_settings(app, state, next)?;
    Ok(saved.pins.len().saturating_sub(before))
}

/// Moves one of the apps on the dock to a group (0 = back to group 1).
#[tauri::command]
fn move_app_to_group(app: AppHandle, state: State<'_, AppState>, key: String, group: u8) -> Result<(), String> {
    let Some(entry) = state.apps().into_iter().find(|a| a.key == key) else { return Ok(()) };
    move_entry(&app, &entry, group);
    Ok(())
}

fn move_entry(app: &AppHandle, entry: &AppEntry, group: u8) {
    let Some(exe) = &entry.exe else { return };
    let mut next = app.state::<AppState>().settings();
    if !next.groups_enabled || !next.move_to_group(&entry.name, exe, group) {
        return;
    }
    match update_settings(app.clone(), app.state::<AppState>(), next) {
        // Lets the dock blink the dot of the group that received the app.
        Ok(_) => {
            let _ = app.emit("group-received", group);
        }
        Err(err) => eprintln!("[dockforge] could not move the app: {err}"),
    }
}

#[tauri::command]
fn open_settings(app: AppHandle) -> Result<(), String> {
    open_hub(&app).map_err(|e| e.to_string())
}

/// Right-click menu for a dock icon: list of windows, pin/unpin, new window, close.
#[tauri::command]
fn show_app_menu(app: AppHandle, window: tauri::Window, state: State<'_, AppState>, key: String) -> Result<(), String> {
    let Some(entry) = state.apps().into_iter().find(|a| a.key == key) else { return Ok(()) };
    let settings = state.settings();
    let lang = settings.language.clone();
    // Where the app lives now: None = not pinned, Some(0) = favorite, Some(n) = group n+1.
    let place = entry.exe.as_deref().and_then(|e| settings.pins.iter().find(|p| p.path.eq_ignore_ascii_case(e))).map(|p| p.group);

    let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
    let mk = |id: String, label: &str| {
        let icon = menu_icon(&id);
        IconMenuItem::with_id(&app, id, label, true, icon, None::<&str>)
    };

    if entry.windows.len() > 1 {
        items.push(Box::new(MenuItem::new(&app, tr(&lang, "Janelas abertas", "Open windows"), false, None::<&str>).map_err(|e| e.to_string())?));
        for w in &entry.windows {
            let title: String = w.title.chars().take(48).collect();
            items.push(Box::new(mk(format!("ctx-win:{}", w.hwnd), &title).map_err(|e| e.to_string())?));
        }
        items.push(Box::new(PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?));
    }
    if entry.exe.is_some() {
        let label = if entry.windows.is_empty() { tr(&lang, "Abrir", "Open") } else { tr(&lang, "Nova janela", "New window") };
        items.push(Box::new(mk("ctx-launch".into(), label).map_err(|e| e.to_string())?));
        match place {
            Some(0) => items.push(Box::new(mk("ctx-unpin".into(), tr(&lang, "Desafixar da dock", "Unpin from dock")).map_err(|e| e.to_string())?)),
            None => items.push(Box::new(mk("ctx-pin".into(), tr(&lang, "Fixar na dock", "Pin to dock")).map_err(|e| e.to_string())?)),
            // An app that lives in a group stays there, open or closed, until it is taken out.
            Some(_) => items.push(Box::new(mk("ctx-group:0".into(), tr(&lang, "Tirar do grupo", "Remove from group")).map_err(|e| e.to_string())?)),
        }
        if settings.groups_enabled {
            let mut targets: Vec<MenuItem<Wry>> = Vec::new();
            for (i, name) in settings.group_names.iter().enumerate() {
                let here = if i == 0 { !matches!(place, Some(g) if g > 0) } else { place == Some(i as u8) };
                let label = format!("{}  ·  Alt+Shift+{}", name, i + 1);
                targets.push(MenuItem::with_id(&app, format!("ctx-group:{i}"), label, !here, None::<&str>).map_err(|e| e.to_string())?);
            }
            let refs: Vec<&dyn IsMenuItem<Wry>> = targets.iter().map(|t| t as &dyn IsMenuItem<Wry>).collect();
            let submenu = Submenu::with_items(&app, tr(&lang, "Mover para o grupo", "Move to group"), true, &refs).map_err(|e| e.to_string())?;
            let _ = submenu.set_icon(menu_icon("ctx-group"));
            items.push(Box::new(submenu));
        }
    }
    if !entry.windows.is_empty() {
        items.push(Box::new(PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?));
        items.push(Box::new(mk("ctx-close".into(), tr(&lang, "Fechar janela", "Close window")).map_err(|e| e.to_string())?));
        if entry.windows.len() > 1 {
            let label = format!("{} ({})", tr(&lang, "Fechar todas", "Close all"), entry.windows.len());
            items.push(Box::new(mk("ctx-close-all".into(), &label).map_err(|e| e.to_string())?));
        }
    }
    items.push(Box::new(PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?));
    items.push(Box::new(mk("ctx-settings".into(), tr(&lang, "Configurações…", "Settings…")).map_err(|e| e.to_string())?));

    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|i| i.as_ref()).collect();
    let menu = Menu::with_items(&app, &refs).map_err(|e| e.to_string())?;

    if let Ok(mut target) = state.menu_target.lock() {
        *target = Some(entry);
    }
    state.menu_open.store(true, Ordering::Relaxed);
    let result = menu.popup(window).map_err(|e| e.to_string());
    state.menu_open.store(false, Ordering::Relaxed);
    result
}

/// The small picture shown next to a menu entry, picked from the entry's id.
fn menu_icon(id: &str) -> Option<Image<'static>> {
    let png: &[u8] = match id.split(':').next().unwrap_or(id) {
        "ctx-win" => include_bytes!("../icons/menu/window.png"),
        "ctx-launch" => include_bytes!("../icons/menu/new.png"),
        "ctx-pin" => include_bytes!("../icons/menu/pin.png"),
        "ctx-unpin" => include_bytes!("../icons/menu/unpin.png"),
        "ctx-group" if id == "ctx-group:0" => include_bytes!("../icons/menu/out.png"),
        "ctx-group" if id == "ctx-group" => include_bytes!("../icons/menu/group.png"),
        "ctx-close" => include_bytes!("../icons/menu/close.png"),
        "ctx-close-all" => include_bytes!("../icons/menu/close-all.png"),
        "ctx-settings" => include_bytes!("../icons/menu/settings.png"),
        "ctx-quit" => include_bytes!("../icons/menu/quit.png"),
        _ => return None,
    };
    Image::from_bytes(png).ok()
}

/// The Windows options that a hidden Windows taskbar takes away. Each entry opens Windows' own
/// settings page; the last two open the taskbar settings and DockForge's own panel.
fn popup_tray(app: &AppHandle, window: tauri::Window, state: &AppState) -> Result<(), String> {
    let lang = state.settings().language;
    let pages: [(&str, &str, &str); 6] = [
        ("tray-sound", "Som", "Sound"),
        ("tray-network", "Rede e internet", "Network and internet"),
        ("tray-battery", "Bateria", "Battery"),
        ("tray-datetime", "Data e hora", "Date and time"),
        ("tray-notifications", "Notificações", "Notifications"),
        ("tray-settings", "Configurações do Windows", "Windows settings"),
    ];
    let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
    for (id, pt, en) in pages {
        items.push(Box::new(MenuItem::with_id(app, id, tr(&lang, pt, en), true, None::<&str>).map_err(|e| e.to_string())?));
    }
    items.push(Box::new(PredefinedMenuItem::separator(app).map_err(|e| e.to_string())?));
    items.push(Box::new(MenuItem::with_id(app, "tray-taskbar", tr(&lang, "Configurações da barra de tarefas", "Taskbar settings"), true, None::<&str>).map_err(|e| e.to_string())?));
    items.push(Box::new(MenuItem::with_id(app, "tray-personalize", tr(&lang, "Personalizar o DockForge…", "Customize DockForge…"), true, None::<&str>).map_err(|e| e.to_string())?));
    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|i| i.as_ref()).collect();
    let menu = Menu::with_items(app, &refs).map_err(|e| e.to_string())?;

    state.menu_open.store(true, Ordering::Relaxed);
    let result = menu.popup(window).map_err(|e| e.to_string());
    state.menu_open.store(false, Ordering::Relaxed);
    result
}

/// Opened from the arrow at the end of a bar.
#[tauri::command]
fn show_tray_menu(app: AppHandle, window: tauri::Window, state: State<'_, AppState>) -> Result<(), String> {
    popup_tray(&app, window, &state)
}

/// Opens one of Windows' settings pages. Only the pages named here can be opened.
fn open_windows_page(id: &str) {
    let uri = match id {
        "sound" => "ms-settings:sound",
        "network" => "ms-settings:network",
        "battery" => "ms-settings:batterysaver",
        "datetime" => "ms-settings:dateandtime",
        "notifications" => "ms-settings:notifications",
        "settings" => "ms-settings:",
        "taskbar" => "ms-settings:taskbar",
        _ => return,
    };
    #[cfg(windows)]
    {
        let _ = std::process::Command::new("explorer").arg(uri).spawn();
    }
    #[cfg(not(windows))]
    let _ = uri;
}

/// The menu for a right click on the dock itself, away from any app.
#[tauri::command]
fn show_dock_menu(app: AppHandle, window: tauri::Window, state: State<'_, AppState>) -> Result<(), String> {
    let lang = state.settings().language;
    let settings = IconMenuItem::with_id(&app, "ctx-settings", tr(&lang, "Configurações…", "Settings…"), true, menu_icon("ctx-settings"), None::<&str>).map_err(|e| e.to_string())?;
    let quit = IconMenuItem::with_id(&app, "ctx-quit", tr(&lang, "Sair do DockForge", "Quit DockForge"), true, menu_icon("ctx-quit"), None::<&str>).map_err(|e| e.to_string())?;
    let separator = PredefinedMenuItem::separator(&app).map_err(|e| e.to_string())?;
    let menu = Menu::with_items(&app, &[&settings, &separator, &quit]).map_err(|e| e.to_string())?;

    state.menu_open.store(true, Ordering::Relaxed);
    let result = menu.popup(window).map_err(|e| e.to_string());
    state.menu_open.store(false, Ordering::Relaxed);
    result
}

fn on_context_menu(app: &AppHandle, event: MenuEvent) {
    let id = event.id().as_ref().to_string();
    if let Some(item) = id.strip_prefix("tray-") {
        match item {
            "personalize" => {
                let _ = open_hub(app);
            }
            other => open_windows_page(other),
        }
        return;
    }
    if !id.starts_with("ctx-") {
        return;
    }
    let state = app.state::<AppState>();
    let target = state.menu_target.lock().ok().and_then(|t| t.clone());

    if id == "ctx-settings" {
        let _ = open_hub(app);
        return;
    }
    if id == "ctx-quit" {
        app.exit(0);
        return;
    }
    let Some(entry) = target else { return };
    let hwnds: Vec<i64> = entry.windows.iter().map(|w| w.hwnd).collect();

    match id.as_str() {
        "ctx-launch" => {
            if let Some(exe) = &entry.exe {
                let _ = std::process::Command::new(exe).spawn();
            }
        }
        "ctx-close" => platform::close_windows(&hwnds[..hwnds.len().min(1)]),
        "ctx-close-all" => platform::close_windows(&hwnds),
        "ctx-pin" | "ctx-unpin" => {
            let mut next = state.settings();
            if let Some(exe) = &entry.exe {
                next.pins.retain(|p| !p.path.eq_ignore_ascii_case(exe));
                if id == "ctx-pin" {
                    next.pins.push(Pin { name: entry.name.clone(), path: exe.clone(), group: 0 });
                    next.favorites_enabled = true;
                }
                let _ = update_settings(app.clone(), app.state::<AppState>(), next);
            }
        }
        other => {
            if let Some(group) = other.strip_prefix("ctx-group:").and_then(|g| g.parse::<u8>().ok()) {
                move_entry(app, &entry, group);
                return;
            }
            if let Some(hwnd) = other.strip_prefix("ctx-win:").and_then(|h| h.parse::<i64>().ok()) {
                let _ = platform::activate(&[hwnd], "focus");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Groups: Alt+1 … Alt+4 switch, Alt+Shift+1 … Alt+Shift+4 move the app in front
// ---------------------------------------------------------------------------

const GROUP_KEYS: [Code; settings::MAX_GROUPS] = [
    Code::Digit1,
    Code::Digit2,
    Code::Digit3,
    Code::Digit4,
    Code::Digit5,
    Code::Digit6,
    Code::Digit7,
    Code::Digit8,
    Code::Digit9,
];
const SWITCH: Modifiers = Modifiers::ALT;
const MOVE: Modifiers = Modifiers::ALT.union(Modifiers::SHIFT);

/// Registers one pair of keys per existing group while groups are on, and releases every
/// other one, so other programs get those keys back.
fn sync_shortcuts(app: &AppHandle, settings: &Settings) {
    let wanted = if settings.groups_enabled { settings.group_names.len() } else { 0 };
    let shortcuts = app.global_shortcut();
    for (index, code) in GROUP_KEYS.into_iter().enumerate() {
        for mods in [SWITCH, MOVE] {
            let shortcut = Shortcut::new(Some(mods), code);
            let registered = shortcuts.is_registered(shortcut);
            let result = match (index < wanted, registered) {
                (true, false) => shortcuts.register(shortcut),
                (false, true) => shortcuts.unregister(shortcut),
                _ => Ok(()),
            };
            if let Err(err) = result {
                // Usually another program already owns the key. The rest keep working.
                eprintln!("[dockforge] {mods:?}+{code:?}: {err}");
            }
        }
    }
}

fn on_shortcut(app: &AppHandle, shortcut: &Shortcut, pressed: ShortcutState) {
    if pressed != ShortcutState::Pressed {
        return;
    }
    let state = app.state::<AppState>();
    let mut next = state.settings();
    if !next.groups_enabled {
        return;
    }

    if let Some(index) = GROUP_KEYS.iter().position(|code| shortcut.matches(MOVE, *code)) {
        // The app whose window is in front goes to that group.
        let front = platform::snapshot().into_iter().find(|a| a.active && a.exe.is_some());
        if let Some(entry) = front {
            move_entry(app, &entry, index as u8);
        }
        return;
    }

    let Some(index) = GROUP_KEYS.iter().position(|code| shortcut.matches(SWITCH, *code)) else { return };
    if next.active_group == index as u8 {
        return;
    }
    next.active_group = index as u8;
    if let Err(err) = update_settings(app.clone(), app.state::<AppState>(), next) {
        eprintln!("[dockforge] could not switch group: {err}");
    }
}

// ---------------------------------------------------------------------------
// Settings window (the hub)
// ---------------------------------------------------------------------------

fn open_hub(app: &AppHandle) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window("hub") {
        window.show()?;
        window.unminimize()?;
        window.set_focus()?;
        return Ok(());
    }
    WebviewWindowBuilder::new(app, "hub", WebviewUrl::App("index.html".into()))
        .title("DockForge")
        .inner_size(1200.0, 800.0)
        .min_inner_size(980.0, 640.0)
        .center()
        .build()?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Themes and tray menu
// ---------------------------------------------------------------------------

fn emit_theme(app: &AppHandle) {
    let theme = app.state::<AppState>().current_theme();
    let _ = app.emit("theme-changed", theme);
}

fn select_theme(app: &AppHandle, id: &str) {
    let state = app.state::<AppState>();
    let mut next = state.settings();
    next.theme = id.to_string();
    // A new theme starts clean: tweaks made on top of the old one do not carry over.
    next.accent = None;
    next.radius = None;
    next.opacity = None;
    if let Err(err) = update_settings(app.clone(), app.state::<AppState>(), next) {
        eprintln!("[dockforge] could not change theme: {err}");
    }
}

fn open_folder(path: &Path) {
    let _ = std::fs::create_dir_all(path);
    open_folder_or_url(&path.to_string_lossy());
}

fn open_folder_or_url(target: &str) {
    #[cfg(windows)]
    let opener = "explorer";
    #[cfg(target_os = "macos")]
    let opener = "open";
    #[cfg(all(unix, not(target_os = "macos")))]
    let opener = "xdg-open";
    if let Err(err) = std::process::Command::new(opener).arg(target).spawn() {
        eprintln!("[dockforge] could not open {target}: {err}");
    }
}

fn on_tray_menu(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "quit" => app.exit(0),
        "open-hub" => {
            let _ = open_hub(app);
        }
        "open-themes" => open_folder(&themes::themes_dir(&app.state::<AppState>().data_dir)),
        "reload" => {
            if let Err(err) = build_tray(app) {
                eprintln!("[dockforge] could not rebuild the tray menu: {err}");
            }
            emit_theme(app);
        }
        id => {
            if let Some(theme) = id.strip_prefix("theme:") {
                select_theme(app, theme);
            }
        }
    }
}

/// Creates the tray icon, or refreshes its menu (after new themes or a language change).
fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let lang = settings.language.as_str();

    let mut items = Vec::new();
    for theme in themes::list(&state.data_dir) {
        let item = CheckMenuItem::with_id(
            app,
            format!("theme:{}", theme.id),
            &theme.name,
            true,
            theme.id == settings.theme,
            None::<&str>,
        )?;
        items.push((theme.id, item));
    }
    let theme_refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|(_, item)| item as &dyn IsMenuItem<Wry>).collect();
    let theme_menu = Submenu::with_items(app, tr(lang, "Temas", "Themes"), true, &theme_refs)?;
    let hub = MenuItem::with_id(app, "open-hub", tr(lang, "Abrir o DockForge…", "Open DockForge…"), true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open-themes", tr(lang, "Abrir pasta de temas", "Open themes folder"), true, None::<&str>)?;
    let reload = MenuItem::with_id(app, "reload", tr(lang, "Recarregar temas", "Reload themes"), true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", tr(lang, "Sair do DockForge", "Quit DockForge"), true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&hub, &theme_menu, &open, &reload, &separator, &quit])?;

    if let Ok(mut stored) = state.theme_items.lock() {
        *stored = items;
    }

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        tray.set_menu(Some(menu))?;
    } else {
        let mut builder = TrayIconBuilder::with_id(TRAY_ID)
            .tooltip("DockForge")
            .menu(&menu)
            .show_menu_on_left_click(true)
            .on_menu_event(on_tray_menu);
        if let Some(icon) = app.default_window_icon() {
            builder = builder.icon(icon.clone());
        }
        builder.build(app)?;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Background watchers
// ---------------------------------------------------------------------------

/// Pushes the app list to the dock whenever windows change. Events are debounced
/// (a burst of changes becomes one update), and a refresh also runs every 3 s in case
/// an event was missed.
fn start_window_events(app: AppHandle) {
    let (tx, rx) = mpsc::channel::<()>();
    platform::start_watcher(move || {
        let _ = tx.send(());
    });

    std::thread::Builder::new()
        .name("dockforge-app-list".into())
        .spawn(move || {
            let mut last: Vec<AppEntry> = Vec::new();
            loop {
                match rx.recv_timeout(Duration::from_secs(3)) {
                    Ok(()) => {
                        std::thread::sleep(Duration::from_millis(120));
                        while rx.try_recv().is_ok() {}
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    // No event source on this platform: just poll.
                    Err(RecvTimeoutError::Disconnected) => std::thread::sleep(Duration::from_secs(3)),
                }
                let apps = app.state::<AppState>().apps();
                if apps != last {
                    let _ = app.emit("apps-changed", &apps);
                    last = apps;
                }
            }
        })
        .expect("failed to start the app list thread");
}

/// Reloads the theme as soon as its styles.css is saved, so theme authors see changes live.
fn start_theme_watcher(app: AppHandle) {
    std::thread::Builder::new()
        .name("dockforge-theme-watch".into())
        .spawn(move || {
            let mut seen: Option<(String, SystemTime)> = None;
            loop {
                std::thread::sleep(Duration::from_millis(700));
                let state = app.state::<AppState>();
                let id = state.settings().theme;
                let modified = std::fs::metadata(themes::styles_path(&state.data_dir, &id))
                    .and_then(|m| m.modified())
                    .ok();
                let Some(modified) = modified else { continue };
                let changed = matches!(&seen, Some((seen_id, seen_time)) if *seen_id == id && *seen_time != modified);
                if changed {
                    emit_theme(&app);
                }
                seen = Some((id, modified));
            }
        })
        .expect("failed to start the theme watcher");
}

/// Mouse handling for the dock window, checked every 80 ms:
/// - click-through: the window is bigger than the dock (room for the hover zoom and the
///   tooltips). Outside the dock itself the window ignores the mouse, so the empty space
///   never blocks clicks on what is behind it;
/// - auto-hide: the dock slides away when the mouse has been elsewhere for a while, and comes
///   back when the mouse touches the screen edge it lives on.
fn start_mouse_watcher(app: AppHandle) {
    std::thread::Builder::new()
        .name("dockforge-mouse".into())
        .spawn(move || {
            let mut last_inside = Instant::now();
            let mut ignoring = false;
            // The screen corner that stands in for the Windows options when there is no bar.
            let mut corner_since: Option<Instant> = None;
            let mut corner_shown = false;
            loop {
                std::thread::sleep(Duration::from_millis(80));
                let state = app.state::<AppState>();
                let Some(window) = app.get_webview_window("dock") else { continue };
                let settings = state.settings();
                let menu_open = state.menu_open.load(Ordering::Relaxed);

                // An app in full screen owns the whole screen: the dock is taken off it
                // entirely, whatever its settings, and comes back when full screen ends.
                let fullscreen = !menu_open && platform::fullscreen_active();
                if fullscreen != state.fullscreen.swap(fullscreen, Ordering::Relaxed) {
                    let _ = if fullscreen { window.hide() } else { window.show() };
                }
                if fullscreen {
                    continue;
                }

                let (Ok(cursor), Ok(pos), Ok(size), Ok(scale)) =
                    (window.cursor_position(), window.outer_position(), window.outer_size(), window.scale_factor())
                else {
                    continue;
                };
                let (x0, y0) = (pos.x as f64, pos.y as f64);
                let (x1, y1) = (x0 + size.width as f64, y0 + size.height as f64);
                let in_window = cursor.x >= x0 && cursor.x <= x1 && cursor.y >= y0 && cursor.y <= y1;

                // Taskbar hidden and no bar along an edge: resting the pointer in the bottom-right
                // corner for half a second opens the Windows options. Leaving the corner re-arms it.
                let in_corner = settings.hide_taskbar
                    && settings.layout != "bar"
                    && matches!(window.current_monitor(), Ok(Some(ref mon)) if {
                        let reach = 6.0 * scale;
                        let right = mon.position().x as f64 + mon.size().width as f64;
                        let bottom = mon.position().y as f64 + mon.size().height as f64;
                        cursor.x >= right - reach && cursor.y >= bottom - reach
                    });
                if !in_corner {
                    corner_since = None;
                    corner_shown = false;
                } else if !menu_open && !corner_shown {
                    let since = *corner_since.get_or_insert_with(Instant::now);
                    if since.elapsed() >= Duration::from_millis(500) {
                        corner_shown = true;
                        let handle = app.clone();
                        let _ = app.run_on_main_thread(move || {
                            if let Some(w) = handle.get_webview_window("dock") {
                                let state = handle.state::<AppState>();
                                let _ = popup_tray(&handle, w.as_ref().window(), &state);
                            }
                        });
                    }
                }

                // The dock's own rectangle, reported by the web side in CSS pixels.
                let over_dock = match state.hit.lock().ok().and_then(|h| *h) {
                    Some((hx, hy, hw, hh)) => {
                        let pad = 4.0 * scale;
                        let (ax, ay) = (x0 + hx * scale - pad, y0 + hy * scale - pad);
                        cursor.x >= ax && cursor.x <= ax + hw * scale + pad * 2.0 && cursor.y >= ay && cursor.y <= ay + hh * scale + pad * 2.0
                    }
                    None => in_window,
                };

                let mut hidden = state.hidden.load(Ordering::Relaxed);
                if settings.auto_hide {
                    let edge = 3.0;
                    let touching_edge = in_window
                        && match settings.position.as_str() {
                            "top" => cursor.y <= y0 + edge,
                            "left" => cursor.x <= x0 + edge,
                            "right" => cursor.x >= x1 - edge,
                            _ => cursor.y >= y1 - edge,
                        };
                    // Nothing maximized in front: the dock can stay out, if the user asked for that.
                    let room = settings.hide_only_maximized && !platform::maximized_in_front();
                    let wants_visible = menu_open || room || if hidden { touching_edge } else { over_dock };
                    if wants_visible {
                        last_inside = Instant::now();
                    }
                    let should_hide =
                        !wants_visible && last_inside.elapsed() >= Duration::from_millis(settings.hide_delay as u64);
                    if hidden && wants_visible || !hidden && should_hide {
                        hidden = !hidden;
                        state.hidden.store(hidden, Ordering::Relaxed);
                        let _ = app.emit("dock-hidden", hidden);
                    }
                } else {
                    last_inside = Instant::now();
                    if hidden {
                        hidden = false;
                        state.hidden.store(false, Ordering::Relaxed);
                        let _ = app.emit("dock-hidden", false);
                    }
                }

                let ignore = !menu_open && (hidden || !over_dock);
                if ignore != ignoring && window.set_ignore_cursor_events(ignore).is_ok() {
                    ignoring = ignore;
                }
            }
        })
        .expect("failed to start the mouse thread");
}

/// Asks Windows what is playing once a second, while the widget is on, and tells the dock
/// when it changes.
fn start_media_watcher(app: AppHandle) {
    std::thread::Builder::new()
        .name("dockforge-media".into())
        .spawn(move || {
            let mut last: Option<MediaInfo> = None;
            loop {
                std::thread::sleep(Duration::from_millis(1000));
                let now = media_for(&app.state::<AppState>().settings());
                if now != last {
                    let _ = app.emit("media-changed", &now);
                    last = now;
                }
            }
        })
        .expect("failed to start the media thread");
}

// ---------------------------------------------------------------------------
// Hiding the Windows taskbar while DockForge runs
// ---------------------------------------------------------------------------

/// While the taskbar is hidden, this file holds the mode it was in before. It is what lets
/// the next start put the taskbar back if DockForge was killed without the chance to do it.
fn taskbar_note(data_dir: &std::path::Path) -> PathBuf {
    data_dir.join("taskbar-hidden")
}

/// Hides the Windows taskbar, or puts it back exactly as it was. Safe to repeat.
fn apply_taskbar(data_dir: &std::path::Path, hide: bool) {
    let note = taskbar_note(data_dir);
    if hide {
        if !note.exists() {
            // Remember the original mode before touching anything. Without the note the
            // taskbar could not be restored, so do not hide it if the note cannot be written.
            if let Err(err) = std::fs::write(&note, platform::taskbar_state().to_string()) {
                eprintln!("[dockforge] taskbar left alone, could not write {}: {err}", note.display());
                return;
            }
        }
        platform::hide_taskbar();
    } else if note.exists() {
        let before = std::fs::read_to_string(&note).ok().and_then(|s| s.trim().parse::<u32>().ok()).unwrap_or(0);
        platform::show_taskbar(before);
        let _ = std::fs::remove_file(&note);
    }
}

/// Explorer shows the taskbar again on its own sometimes (after it restarts, for example),
/// so while the option is on it is hidden again every 1.5 s.
fn start_taskbar_keeper(app: AppHandle) {
    std::thread::Builder::new()
        .name("dockforge-taskbar".into())
        .spawn(move || loop {
            std::thread::sleep(Duration::from_millis(1500));
            let state = app.state::<AppState>();
            if state.settings().hide_taskbar && taskbar_note(&state.data_dir).exists() {
                platform::hide_taskbar();
            }
        })
        .expect("failed to start the taskbar thread");
}

/// Reads processor, memory and battery every 2 s while the system widget is on.
fn start_system_watcher(app: AppHandle) {
    std::thread::Builder::new()
        .name("dockforge-system".into())
        .spawn(move || {
            let mut last: Option<SystemStats> = None;
            loop {
                std::thread::sleep(Duration::from_millis(2000));
                let enabled = app.state::<AppState>().settings().widget_system;
                let now = enabled.then(platform::system_stats);
                if now != last {
                    let _ = app.emit("system-changed", now);
                    last = now;
                }
            }
        })
        .expect("failed to start the system thread");
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run() {
    tauri::Builder::default()
        // A second launch just exits: one dock is enough.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Launching DockForge again is how people reopen the settings window.
            let _ = open_hub(app);
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().with_handler(|app, shortcut, event| on_shortcut(app, shortcut, event.state())).build())
        .on_menu_event(on_context_menu)
        .setup(|app| {
            let data_dir = app.path().data_dir()?.join("DockForge");
            if let Err(err) = themes::install_built_in(&data_dir) {
                eprintln!("[dockforge] could not install built-in themes: {err}");
            }
            let first_run = !data_dir.join("settings.json").exists();
            let settings = settings::load(&data_dir);
            // Hides the taskbar if the option is on. If it is off but the last run was killed
            // with the taskbar hidden, this is what brings it back.
            apply_taskbar(&data_dir, settings.hide_taskbar);
            app.manage(AppState {
                data_dir,
                settings: Mutex::new(settings),
                theme_items: Mutex::new(Vec::new()),
                menu_open: AtomicBool::new(false),
                menu_target: Mutex::new(None),
                fullscreen: AtomicBool::new(false),
                hidden: AtomicBool::new(false),
                hit: Mutex::new(None),
            });
            build_tray(app.handle())?;
            start_window_events(app.handle().clone());
            start_theme_watcher(app.handle().clone());
            start_mouse_watcher(app.handle().clone());
            start_media_watcher(app.handle().clone());
            start_system_watcher(app.handle().clone());
            start_taskbar_keeper(app.handle().clone());
            sync_shortcuts(app.handle(), &app.state::<AppState>().settings());
            if first_run {
                open_hub(app.handle())?;
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_apps,
            activate_app,
            launch_app,
            get_icon,
            get_theme,
            list_themes,
            get_settings,
            update_settings,
            fit_dock,
            open_settings,
            show_app_menu,
            show_dock_menu,
            show_tray_menu,
            get_media,
            get_system,
            media_control,
            load_theme,
            open_themes_folder,
            open_link,
            reset_settings,
            set_hit_rect,
            import_taskbar_pins,
            move_app_to_group
        ])
        .build(tauri::generate_context!())
        .expect("error while running DockForge")
        .run(|app, event| {
            // Whatever way DockForge closes, the Windows taskbar comes back first.
            if let tauri::RunEvent::Exit = event {
                platform::release_edge();
                apply_taskbar(&app.state::<AppState>().data_dir, false);
            }
        });
}
