//! Win32 implementation: which apps have windows open, focusing/minimizing them,
//! app icons and names, and a watcher that reports window changes.
//!
//! Nothing here injects code into other processes or touches Explorer: the dock is an
//! ordinary always-on-top window that reads public window information.

use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::atomic::{AtomicIsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use windows::core::{w, BOOL, PCWSTR, PWSTR};
use windows::Win32::Foundation::{CloseHandle, HWND, LPARAM, WPARAM};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_CLOAKED};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, GetDC, GetDIBits, GetObjectW, ReleaseDC, BITMAP, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, HGDIOBJ,
};
use windows::Win32::Storage::FileSystem::{
    GetFileVersionInfoSizeW, GetFileVersionInfoW, VerQueryValueW, FILE_FLAGS_AND_ATTRIBUTES,
};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::Accessibility::{SetWinEventHook, HWINEVENTHOOK};
use windows::Win32::UI::Controls::{IImageList, ILD_TRANSPARENT};
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHGetImageList, SHFILEINFOW, SHGFI_SYSICONINDEX, SHIL_EXTRALARGE, SHIL_JUMBO};
use windows::Win32::UI::WindowsAndMessaging::{
    DestroyIcon, DispatchMessageW, EnumWindows, GetForegroundWindow, GetIconInfo, GetMessageW, GetWindow,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, GetWindowThreadProcessId, IsIconic, IsWindow,
    IsWindowVisible, PostMessageW, SetForegroundWindow, ShowWindow, TranslateMessage, CHILDID_SELF, EVENT_OBJECT_CLOAKED,
    EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY, EVENT_OBJECT_HIDE, EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_SHOW,
    EVENT_OBJECT_UNCLOAKED, EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_MINIMIZEEND, EVENT_SYSTEM_MINIMIZESTART,
    GWL_EXSTYLE, GW_OWNER, HICON, ICONINFO, MSG, OBJID_WINDOW, SW_MINIMIZE, SW_RESTORE, WINEVENT_OUTOFCONTEXT,
    WINEVENT_SKIPOWNPROCESS, WM_CLOSE, WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
};

use crate::model::{AppEntry, WindowInfo};

/// The last window of another app that was in front. Clicking the dock brings the dock
/// itself to the front, so this is how we know what the user was looking at before.
static LAST_FOREGROUND: AtomicIsize = AtomicIsize::new(0);
static NOTIFY: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

/// First-seen order of apps, so icons do not jump around when focus changes.
static ORDER: OnceLock<Mutex<(u64, HashMap<String, u64>)>> = OnceLock::new();
static NAMES: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn hwnd(value: i64) -> HWND {
    HWND(value as isize as *mut c_void)
}

// ---------------------------------------------------------------------------
// Window list
// ---------------------------------------------------------------------------

unsafe extern "system" fn collect(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let list = &mut *(lparam.0 as *mut Vec<HWND>);
    list.push(hwnd);
    BOOL(1)
}

/// Same rules the Windows taskbar uses, roughly: visible, top-level, not a tool
/// window, not cloaked (hidden UWP frames, other virtual desktops) and with a title.
fn is_taskbar_window(h: HWND) -> bool {
    unsafe {
        if !IsWindowVisible(h).as_bool() {
            return false;
        }
        if let Ok(owner) = GetWindow(h, GW_OWNER) {
            if !owner.is_invalid() {
                return false;
            }
        }
        let ex = GetWindowLongPtrW(h, GWL_EXSTYLE) as u32;
        let app_window = ex & WS_EX_APPWINDOW.0 != 0;
        if !app_window && (ex & WS_EX_TOOLWINDOW.0 != 0 || ex & WS_EX_NOACTIVATE.0 != 0) {
            return false;
        }
        let mut cloaked: u32 = 0;
        let res = DwmGetWindowAttribute(
            h,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut c_void,
            std::mem::size_of::<u32>() as u32,
        );
        if res.is_ok() && cloaked != 0 {
            return false;
        }
        GetWindowTextLengthW(h) > 0
    }
}

fn window_title(h: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(h);
        if len <= 0 {
            return String::new();
        }
        let mut buf = vec![0u16; len as usize + 1];
        let copied = GetWindowTextW(h, &mut buf);
        String::from_utf16_lossy(&buf[..copied.max(0) as usize])
    }
}

fn window_pid(h: HWND) -> u32 {
    let mut pid = 0u32;
    unsafe { GetWindowThreadProcessId(h, Some(&mut pid)) };
    pid
}

pub fn process_path(pid: u32) -> Option<String> {
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
        let mut buf = vec![0u16; 1024];
        let mut len = buf.len() as u32;
        let res = QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, PWSTR(buf.as_mut_ptr()), &mut len);
        let _ = CloseHandle(handle);
        res.ok()?;
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    }
}

/// "FileDescription" from the executable's version info, e.g. "Google Chrome".
fn file_description(exe: &str) -> Option<String> {
    unsafe {
        let wide = to_wide(exe);
        let size = GetFileVersionInfoSizeW(PCWSTR(wide.as_ptr()), None);
        if size == 0 {
            return None;
        }
        let mut data = vec![0u8; size as usize];
        GetFileVersionInfoW(PCWSTR(wide.as_ptr()), None, size, data.as_mut_ptr() as *mut c_void).ok()?;

        let mut ptr: *mut c_void = std::ptr::null_mut();
        let mut len: u32 = 0;
        let mut langs: Vec<(u16, u16)> = Vec::new();
        if VerQueryValueW(data.as_ptr() as *const c_void, w!("\\VarFileInfo\\Translation"), &mut ptr, &mut len)
            .as_bool()
            && len >= 4
        {
            let pairs = ptr as *const u16;
            for i in 0..(len / 4) as usize {
                langs.push((*pairs.add(i * 2), *pairs.add(i * 2 + 1)));
            }
        }
        langs.extend([(0x0409, 0x04B0), (0x0409, 0x04E4)]);

        for (lang, cp) in langs {
            let key = to_wide(&format!("\\StringFileInfo\\{lang:04x}{cp:04x}\\FileDescription"));
            if VerQueryValueW(data.as_ptr() as *const c_void, PCWSTR(key.as_ptr()), &mut ptr, &mut len).as_bool()
                && len > 1
            {
                let text = std::slice::from_raw_parts(ptr as *const u16, (len - 1) as usize);
                let text = String::from_utf16_lossy(text).trim().to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
        None
    }
}

pub fn display_name(exe: &str) -> String {
    let names = NAMES.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(name) = names.lock().ok().and_then(|n| n.get(exe).cloned()) {
        return name;
    }
    let name = file_description(exe).unwrap_or_else(|| {
        std::path::Path::new(exe)
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| exe.to_string())
    });
    if let Ok(mut n) = names.lock() {
        n.insert(exe.to_string(), name.clone());
    }
    name
}

fn first_seen(key: &str) -> u64 {
    let order = ORDER.get_or_init(|| Mutex::new((0, HashMap::new())));
    let mut guard = order.lock().expect("order lock");
    let (next, map) = &mut *guard;
    *map.entry(key.to_string()).or_insert_with(|| {
        *next += 1;
        *next
    })
}

/// Apps with at least one taskbar-style window, grouped by executable, in first-seen order.
pub fn snapshot() -> Vec<AppEntry> {
    let own_pid = std::process::id();
    let last = LAST_FOREGROUND.load(Ordering::Relaxed);

    let mut handles: Vec<HWND> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(collect), LPARAM(&mut handles as *mut Vec<HWND> as isize));
    }

    let mut apps: Vec<(u64, AppEntry)> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();

    // EnumWindows walks the z-order from the top, so each app's first window is its most recent.
    for h in handles {
        if !is_taskbar_window(h) {
            continue;
        }
        let pid = window_pid(h);
        if pid == own_pid {
            continue;
        }
        let exe = process_path(pid);
        // UWP/Store apps all live inside ApplicationFrameHost.exe: keep them as separate entries.
        let hosted = exe.as_deref().is_some_and(|p| p.to_ascii_lowercase().ends_with("\\applicationframehost.exe"));
        let title = window_title(h);
        let raw = h.0 as isize;

        let (key, name, exe) = match exe {
            Some(path) if !hosted => (path.to_lowercase(), display_name(&path), Some(path)),
            _ => (format!("window:{raw}"), title.clone(), None),
        };

        let window = WindowInfo { hwnd: raw as i64, title };
        match index.get(&key) {
            Some(&i) => {
                let entry = &mut apps[i].1;
                entry.active |= raw == last;
                entry.windows.push(window);
            }
            None => {
                index.insert(key.clone(), apps.len());
                apps.push((
                    first_seen(&key),
                    AppEntry { key, name, exe, windows: vec![window], active: raw == last, pinned: false },
                ));
            }
        }
    }

    apps.sort_by_key(|(order, _)| *order);
    apps.into_iter().map(|(_, app)| app).collect()
}

/// Click on a dock icon. `action` is the user's choice:
/// - "focus-min": minimize the app if it was in front, otherwise bring it to the front;
/// - "cycle": if it was in front, go to its next window, otherwise bring it to the front;
/// - "focus": always just bring it to the front.
pub fn activate(hwnds: &[i64], action: &str) -> Result<(), String> {
    let Some(&first) = hwnds.first() else { return Ok(()) };
    let last = LAST_FOREGROUND.load(Ordering::Relaxed) as i64;
    let in_front = last != 0 && hwnds.contains(&last);

    let target_value = match (action, in_front) {
        ("focus-min", true) => {
            unsafe {
                let _ = ShowWindow(hwnd(last), SW_MINIMIZE);
            }
            LAST_FOREGROUND.store(0, Ordering::Relaxed);
            return Ok(());
        }
        ("cycle", true) if hwnds.len() > 1 => {
            let at = hwnds.iter().position(|&h| h == last).unwrap_or(0);
            hwnds[(at + 1) % hwnds.len()]
        }
        ("focus", true) => last,
        _ => first,
    };

    unsafe {
        let target = hwnd(target_value);
        if !IsWindow(Some(target)).as_bool() {
            return Err("window no longer exists".into());
        }
        if IsIconic(target).as_bool() {
            let _ = ShowWindow(target, SW_RESTORE);
        }
        // Allowed because the dock is the foreground window right after being clicked.
        if !SetForegroundWindow(target).as_bool() {
            return Err("Windows refused to bring the window to the front".into());
        }
        LAST_FOREGROUND.store(target_value as isize, Ordering::Relaxed);
    }
    Ok(())
}

/// Politely asks windows to close (like clicking their X), so apps can ask to save.
pub fn close_windows(hwnds: &[i64]) {
    for &h in hwnds {
        unsafe {
            let _ = PostMessageW(Some(hwnd(h)), WM_CLOSE, WPARAM(0), LPARAM(0));
        }
    }
}

// ---------------------------------------------------------------------------
// Watcher
// ---------------------------------------------------------------------------

unsafe extern "system" fn on_win_event(
    _hook: HWINEVENTHOOK,
    event: u32,
    h: HWND,
    id_object: i32,
    id_child: i32,
    _thread: u32,
    _time: u32,
) {
    if id_object != OBJID_WINDOW.0 || id_child != CHILDID_SELF as i32 || h.is_invalid() {
        return;
    }
    if event == EVENT_SYSTEM_FOREGROUND {
        // WINEVENT_SKIPOWNPROCESS already filters the dock itself out.
        LAST_FOREGROUND.store(h.0 as isize, Ordering::Relaxed);
    }
    if let Some(notify) = NOTIFY.get() {
        notify();
    }
}

/// Starts a background thread that calls `notify` whenever windows are created,
/// destroyed, shown, hidden, renamed, minimized or focused. Calls can be frequent:
/// the caller is expected to debounce.
pub fn start_watcher(notify: impl Fn() + Send + Sync + 'static) {
    if NOTIFY.set(Box::new(notify)).is_err() {
        return;
    }
    unsafe {
        let fg = GetForegroundWindow();
        if !fg.is_invalid() && window_pid(fg) != std::process::id() {
            LAST_FOREGROUND.store(fg.0 as isize, Ordering::Relaxed);
        }
    }

    std::thread::Builder::new()
        .name("dockforge-window-events".into())
        .spawn(|| unsafe {
            let flags = WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS;
            let ranges = [
                (EVENT_SYSTEM_FOREGROUND, EVENT_SYSTEM_FOREGROUND),
                (EVENT_SYSTEM_MINIMIZESTART, EVENT_SYSTEM_MINIMIZEEND),
                (EVENT_OBJECT_CREATE, EVENT_OBJECT_DESTROY),
                (EVENT_OBJECT_SHOW, EVENT_OBJECT_HIDE),
                (EVENT_OBJECT_NAMECHANGE, EVENT_OBJECT_NAMECHANGE),
                (EVENT_OBJECT_CLOAKED, EVENT_OBJECT_UNCLOAKED),
            ];
            let _hooks: Vec<HWINEVENTHOOK> = ranges
                .iter()
                .map(|&(min, max)| SetWinEventHook(min, max, None, Some(on_win_event), 0, 0, flags))
                .collect();

            // Out-of-context hooks are delivered through this thread's message queue.
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).0 > 0 {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        })
        .expect("failed to start the window event thread");
}

// ---------------------------------------------------------------------------
// Icons
// ---------------------------------------------------------------------------

struct Rgba {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

struct IconRequest {
    exe: String,
    reply: Sender<Option<String>>,
}

/// Shell icon APIs want a COM apartment, so icons are extracted on one dedicated thread.
fn icon_worker() -> &'static Mutex<Sender<IconRequest>> {
    static WORKER: OnceLock<Mutex<Sender<IconRequest>>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<IconRequest>();
        std::thread::Builder::new()
            .name("dockforge-icons".into())
            .spawn(move || {
                unsafe {
                    let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
                }
                let mut cache: HashMap<String, Option<String>> = HashMap::new();
                for req in rx {
                    let icon = cache.entry(req.exe.to_lowercase()).or_insert_with(|| unsafe { extract_icon(&req.exe) });
                    let _ = req.reply.send(icon.clone());
                }
            })
            .expect("failed to start the icon thread");
        Mutex::new(tx)
    })
}

/// The app's icon as a PNG data URL (up to 128×128), or None.
pub fn icon_data_url(exe: &str) -> Option<String> {
    let path = std::path::Path::new(exe);
    let is_exe = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    if !path.is_absolute() || !is_exe || !path.is_file() {
        return None;
    }
    let (tx, rx) = mpsc::channel();
    icon_worker().lock().ok()?.send(IconRequest { exe: exe.to_string(), reply: tx }).ok()?;
    rx.recv_timeout(Duration::from_secs(5)).ok().flatten()
}

unsafe fn extract_icon(exe: &str) -> Option<String> {
    let wide = to_wide(exe);
    let mut info = SHFILEINFOW::default();
    let found = SHGetFileInfoW(
        PCWSTR(wide.as_ptr()),
        FILE_FLAGS_AND_ATTRIBUTES(0),
        Some(&mut info),
        std::mem::size_of::<SHFILEINFOW>() as u32,
        SHGFI_SYSICONINDEX,
    );
    if found == 0 {
        return None;
    }
    // The 256px "jumbo" list looks best, but apps without a big icon get a tiny one in
    // its top-left corner; fall back to the 48px list for those.
    let image = match icon_from_list(SHIL_JUMBO as i32, info.iIcon) {
        Some(img) if !is_small_in_corner(&img) => img,
        _ => icon_from_list(SHIL_EXTRALARGE as i32, info.iIcon)?,
    };
    let image = if image.width >= 256 { half_size(&image) } else { image };
    encode_png(&image).map(|bytes| {
        use base64::Engine;
        format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes))
    })
}

unsafe fn icon_from_list(list_kind: i32, index: i32) -> Option<Rgba> {
    let list: IImageList = SHGetImageList(list_kind).ok()?;
    let icon = list.GetIcon(index, ILD_TRANSPARENT.0).ok()?;
    let image = hicon_to_rgba(icon);
    let _ = DestroyIcon(icon);
    image
}

unsafe fn hicon_to_rgba(icon: HICON) -> Option<Rgba> {
    let mut info = ICONINFO::default();
    GetIconInfo(icon, &mut info).ok()?;
    let color = info.hbmColor;
    let mask = info.hbmMask;
    let free = || {
        if !color.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(color.0));
        }
        if !mask.is_invalid() {
            let _ = DeleteObject(HGDIOBJ(mask.0));
        }
    };
    if color.is_invalid() {
        free();
        return None;
    }

    let mut bitmap = BITMAP::default();
    let got = GetObjectW(
        HGDIOBJ(color.0),
        std::mem::size_of::<BITMAP>() as i32,
        Some(&mut bitmap as *mut BITMAP as *mut c_void),
    );
    let (width, height) = (bitmap.bmWidth, bitmap.bmHeight);
    if got == 0 || width <= 0 || height <= 0 || width > 512 || height > 512 {
        free();
        return None;
    }

    let mut bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height, // top-down rows
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let dc = GetDC(None);
    let lines = GetDIBits(
        dc,
        color,
        0,
        height as u32,
        Some(pixels.as_mut_ptr() as *mut c_void),
        &mut bmi,
        DIB_RGB_COLORS,
    );
    ReleaseDC(None, dc);
    free();
    if lines == 0 {
        return None;
    }

    for px in pixels.as_chunks_mut::<4>().0 {
        px.swap(0, 2); // BGRA -> RGBA
    }
    // Very old icons have no alpha channel at all: treat them as opaque.
    if pixels.as_chunks::<4>().0.iter().all(|px| px[3] == 0) {
        for px in pixels.as_chunks_mut::<4>().0 {
            px[3] = 255;
        }
    }
    Some(Rgba { width: width as u32, height: height as u32, pixels })
}

fn is_small_in_corner(img: &Rgba) -> bool {
    if img.width < 256 {
        return false;
    }
    let (w, h) = (img.width as usize, img.height as usize);
    (0..h).all(|y| (0..w).all(|x| (x < 48 && y < 48) || img.pixels[(y * w + x) * 4 + 3] == 0))
}

/// 2×2 box filter, weighted by alpha so edges do not get dark fringes.
fn half_size(img: &Rgba) -> Rgba {
    let (w, h) = (img.width as usize / 2, img.height as usize / 2);
    let src_w = img.width as usize;
    let mut out = vec![0u8; w * h * 4];
    for y in 0..h {
        for x in 0..w {
            let (mut r, mut g, mut b, mut a) = (0u32, 0u32, 0u32, 0u32);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                let i = ((y * 2 + dy) * src_w + (x * 2 + dx)) * 4;
                let pa = img.pixels[i + 3] as u32;
                r += img.pixels[i] as u32 * pa;
                g += img.pixels[i + 1] as u32 * pa;
                b += img.pixels[i + 2] as u32 * pa;
                a += pa;
            }
            let o = (y * w + x) * 4;
            out[o] = r.checked_div(a).unwrap_or(0) as u8;
            out[o + 1] = g.checked_div(a).unwrap_or(0) as u8;
            out[o + 2] = b.checked_div(a).unwrap_or(0) as u8;
            out[o + 3] = (a / 4) as u8;
        }
    }
    Rgba { width: w as u32, height: h as u32, pixels: out }
}

fn encode_png(img: &Rgba) -> Option<Vec<u8>> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, img.width, img.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(&img.pixels).ok()?;
    }
    Some(bytes)
}

// ---------------------------------------------------------------------------
// Hiding the Windows taskbar
// ---------------------------------------------------------------------------
//
// Two steps, both undone by `show_taskbar`:
// 1. switch the taskbar to auto-hide, so Windows gives its strip of screen back to the apps
//    (otherwise maximized windows would leave an empty band where the taskbar was);
// 2. hide its window, so it does not slide back in when the mouse touches the edge.
// Nothing is injected into Explorer and no system file or policy is changed.

use windows::Win32::UI::Shell::{SHAppBarMessage, ABM_GETSTATE, ABM_SETSTATE, ABS_AUTOHIDE, APPBARDATA};
use windows::Win32::UI::WindowsAndMessaging::{FindWindowExW, FindWindowW, SW_HIDE, SW_SHOWNOACTIVATE};

/// The main taskbar and the ones on the other monitors.
fn taskbars() -> Vec<HWND> {
    let mut found = Vec::new();
    if let Ok(main) = unsafe { FindWindowW(w!("Shell_TrayWnd"), PCWSTR::null()) } {
        if !main.is_invalid() {
            found.push(main);
        }
    }
    let mut after: Option<HWND> = None;
    // Bounded: a computer does not have 16 monitors' worth of taskbars.
    for _ in 0..16 {
        match unsafe { FindWindowExW(None, after, w!("Shell_SecondaryTrayWnd"), PCWSTR::null()) } {
            Ok(next) if !next.is_invalid() => {
                found.push(next);
                after = Some(next);
            }
            _ => break,
        }
    }
    found
}

fn appbar_data() -> APPBARDATA {
    APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: taskbars().first().copied().unwrap_or_default(),
        ..Default::default()
    }
}

/// The taskbar's current mode (auto-hide or not), to be given back to `show_taskbar` later.
pub fn taskbar_state() -> u32 {
    let mut data = appbar_data();
    (unsafe { SHAppBarMessage(ABM_GETSTATE, &mut data) }) as u32
}

fn set_taskbar_state(state: u32) {
    let mut data = appbar_data();
    data.lParam = LPARAM(state as isize);
    unsafe { SHAppBarMessage(ABM_SETSTATE, &mut data) };
}

/// Safe to call again and again: Explorer shows the taskbar again by itself now and then
/// (when it restarts, for example), so the caller repeats this while the option is on.
pub fn hide_taskbar() {
    if taskbar_state() & ABS_AUTOHIDE == 0 {
        set_taskbar_state(ABS_AUTOHIDE);
    }
    for bar in taskbars() {
        if unsafe { IsWindowVisible(bar) }.as_bool() {
            let _ = unsafe { ShowWindow(bar, SW_HIDE) };
        }
    }
}

/// Brings the taskbar back, in the mode it had before (`state` from `taskbar_state`).
pub fn show_taskbar(state: u32) {
    set_taskbar_state(state);
    for bar in taskbars() {
        let _ = unsafe { ShowWindow(bar, SW_SHOWNOACTIVATE) };
    }
}

// ---------------------------------------------------------------------------
// Keeping a strip of the screen for the dock
// ---------------------------------------------------------------------------
//
// Windows calls a window that owns a strip along a screen edge an "application desktop
// toolbar" (appbar). Maximized windows stop where the strip begins. This is the documented
// way to do it, the same one the Windows taskbar uses.

use windows::Win32::Foundation::RECT;
use windows::Win32::UI::Shell::{ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_QUERYPOS, ABM_REMOVE, ABM_SETPOS};
use windows::Win32::UI::WindowsAndMessaging::WM_APP;

/// The window registered as an appbar, if any.
static APPBAR: Mutex<Option<isize>> = Mutex::new(None);
/// Windows wants a message number to notify the appbar with. The notifications are not
/// needed, and a number in the application's private range is ignored by the window.
const APPBAR_MESSAGE: u32 = WM_APP + 0x0DF;

fn appbar(window: isize) -> APPBARDATA {
    APPBARDATA {
        cbSize: std::mem::size_of::<APPBARDATA>() as u32,
        hWnd: HWND(window as *mut core::ffi::c_void),
        uCallbackMessage: APPBAR_MESSAGE,
        ..Default::default()
    }
}

/// Reserves `thickness` pixels along `edge` ("bottom" | "top" | "left" | "right") of the
/// monitor `(left, top, right, bottom)` for `window` and returns the strip.
///
/// With `yield_to_others`, Windows may move the strip inwards to make room for another
/// toolbar on that edge (the Windows taskbar, when it is visible). Without it the strip is
/// exactly at the screen edge: that is what a dock that replaces the taskbar needs, since a
/// hidden taskbar can still be holding on to its old place.
/// Calling it again just updates the strip.
pub fn reserve_edge(
    window: isize,
    edge: &str,
    thickness: i32,
    monitor: (i32, i32, i32, i32),
    yield_to_others: bool,
) -> Option<(i32, i32, i32, i32)> {
    let mut registered = APPBAR.lock().ok()?;
    if *registered != Some(window) {
        if let Some(old) = registered.take() {
            unsafe { SHAppBarMessage(ABM_REMOVE, &mut appbar(old)) };
        }
        if unsafe { SHAppBarMessage(ABM_NEW, &mut appbar(window)) } == 0 {
            return None;
        }
        *registered = Some(window);
    }

    let (left, top, right, bottom) = monitor;
    let thickness = thickness.max(1);
    let mut data = appbar(window);
    data.uEdge = match edge {
        "top" => ABE_TOP,
        "left" => ABE_LEFT,
        "right" => ABE_RIGHT,
        _ => ABE_BOTTOM,
    };
    data.rc = RECT { left, top, right, bottom };
    let narrow = |rc: &mut RECT| match edge {
        "top" => rc.bottom = rc.top + thickness,
        "left" => rc.right = rc.left + thickness,
        "right" => rc.left = rc.right - thickness,
        _ => rc.top = rc.bottom - thickness,
    };
    narrow(&mut data.rc);
    if yield_to_others {
        // Windows pushes the strip away from the edge if another toolbar is there; the
        // thickness is then applied again from wherever the strip ended up.
        unsafe { SHAppBarMessage(ABM_QUERYPOS, &mut data) };
        narrow(&mut data.rc);
    }
    unsafe { SHAppBarMessage(ABM_SETPOS, &mut data) };
    Some((data.rc.left, data.rc.top, data.rc.right, data.rc.bottom))
}

/// Gives the strip back. Safe to call when nothing is reserved.
pub fn release_edge() {
    let Ok(mut registered) = APPBAR.lock() else { return };
    if let Some(window) = registered.take() {
        unsafe { SHAppBarMessage(ABM_REMOVE, &mut appbar(window)) };
    }
}

// ---------------------------------------------------------------------------
// Is something in full screen?
// ---------------------------------------------------------------------------

use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetWindowRect, IsZoomed};

/// The window in front, unless it is the desktop or the taskbar (which are as big as the
/// screen but are not apps).
fn front_app_window() -> Option<HWND> {
    unsafe {
        let front = GetForegroundWindow();
        if front.is_invalid() {
            return None;
        }
        let mut class = [0u16; 64];
        let len = GetClassNameW(front, &mut class).max(0) as usize;
        let class = String::from_utf16_lossy(&class[..len.min(class.len())]);
        let shell = matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "XamlExplorerHostIslandWindow");
        (!shell).then_some(front)
    }
}

/// True when something is in full screen: a game, a video, a browser after F11, a slideshow.
///
/// A maximized window is not full screen, even though it can cover the whole monitor once
/// the taskbar is hidden. What tells them apart:
/// - Windows itself knows about games, Direct3D full screen and presentations;
/// - a maximized window keeps its resizable frame, a full screen one drops it.
pub fn fullscreen_active() -> bool {
    use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
    use windows::Win32::UI::WindowsAndMessaging::{GWL_STYLE, WS_CAPTION, WS_THICKFRAME};

    if let Ok(state) = unsafe { SHQueryUserNotificationState() } {
        if state == QUNS_BUSY || state == QUNS_RUNNING_D3D_FULL_SCREEN || state == QUNS_PRESENTATION_MODE {
            return true;
        }
    }
    let Some(front) = front_app_window() else { return false };
    unsafe {
        let mut rect = RECT::default();
        if GetWindowRect(front, &mut rect).is_err() {
            return false;
        }
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        if !GetMonitorInfoW(MonitorFromWindow(front, MONITOR_DEFAULTTONEAREST), &mut info).as_bool() {
            return false;
        }
        let screen = info.rcMonitor;
        let covers = rect.left <= screen.left && rect.top <= screen.top && rect.right >= screen.right && rect.bottom >= screen.bottom;
        if !covers {
            return false;
        }
        let style = GetWindowLongPtrW(front, GWL_STYLE) as u32;
        let framed = style & WS_CAPTION.0 == WS_CAPTION.0 || style & WS_THICKFRAME.0 != 0;
        !IsZoomed(front).as_bool() || !framed
    }
}

/// True when the window in front is maximized.
/// The window in front, as a plain number.
pub fn foreground_window() -> isize {
    unsafe { GetForegroundWindow() }.0 as isize
}

pub fn maximized_in_front() -> bool {
    front_app_window().is_some_and(|w| unsafe { IsZoomed(w) }.as_bool())
}
