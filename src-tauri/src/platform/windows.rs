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

use crate::model::{AppEntry, MediaInfo, SystemStats, WindowInfo};

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

fn process_path(pid: u32) -> Option<String> {
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

fn display_name(exe: &str) -> String {
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

    for px in pixels.chunks_exact_mut(4) {
        px.swap(0, 2); // BGRA -> RGBA
    }
    // Very old icons have no alpha channel at all: treat them as opaque.
    if pixels.chunks_exact(4).all(|px| px[3] == 0) {
        for px in pixels.chunks_exact_mut(4) {
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
// Now playing (Windows media controls)
// ---------------------------------------------------------------------------

use std::cell::RefCell;
use windows::Media::Control::{
    GlobalSystemMediaTransportControlsSession as MediaSession,
    GlobalSystemMediaTransportControlsSessionManager as MediaManager,
    GlobalSystemMediaTransportControlsSessionPlaybackStatus as PlaybackStatus,
};
use windows::Storage::Streams::{DataReader, IRandomAccessStreamReference};
use windows::Win32::System::Com::COINIT_MULTITHREADED;

const MAX_COVER_BYTES: u64 = 2 * 1024 * 1024;

thread_local! {
    static MEDIA: RefCell<Option<MediaManager>> = const { RefCell::new(None) };
    /// (title + artist, cover) so the cover is read once per song, not every second.
    static COVER: RefCell<(String, Option<String>)> = const { RefCell::new((String::new(), None)) };
}

fn media_session() -> Option<MediaSession> {
    MEDIA.with(|cell| {
        let mut slot = cell.borrow_mut();
        if slot.is_none() {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            }
            *slot = MediaManager::RequestAsync().ok()?.join().ok();
        }
        slot.as_ref()?.GetCurrentSession().ok()
    })
}

fn read_cover(reference: &IRandomAccessStreamReference) -> Option<String> {
    use base64::Engine;
    let stream = reference.OpenReadAsync().ok()?.join().ok()?;
    let size = stream.Size().ok()?;
    if size == 0 || size > MAX_COVER_BYTES {
        return None;
    }
    let reader = DataReader::CreateDataReader(&stream).ok()?;
    reader.LoadAsync(size as u32).ok()?.join().ok()?;
    let mut bytes = vec![0u8; size as usize];
    reader.ReadBytes(&mut bytes).ok()?;
    let mime = if bytes.starts_with(&[0x89, b'P', b'N', b'G']) { "image/png" } else { "image/jpeg" };
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

fn friendly_source(id: &str) -> String {
    let last = id.rsplit(['\\', '!']).next().unwrap_or(id);
    let name = last.strip_suffix(".exe").or_else(|| last.strip_suffix(".EXE")).unwrap_or(last);
    let name = name.split('.').next_back().unwrap_or(name);
    let mut chars = name.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => String::new(),
    }
}

/// The current media session, or `None` if nothing is playing or paused. Call it from one
/// background thread: it keeps a per-thread connection to Windows.
pub fn media_now() -> Option<MediaInfo> {
    let session = media_session()?;
    let props = session.TryGetMediaPropertiesAsync().ok()?.join().ok()?;
    let title = props.Title().ok()?.to_string();
    if title.is_empty() {
        return None;
    }
    let artist = props.Artist().map(|a| a.to_string()).unwrap_or_default();
    let playing = session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|s| s == PlaybackStatus::Playing)
        .unwrap_or(false);
    let source = session.SourceAppUserModelId().map(|s| friendly_source(&s.to_string())).unwrap_or_default();

    let song = format!("{title}\u{1f}{artist}");
    let thumbnail = COVER.with(|cell| {
        let mut cache = cell.borrow_mut();
        if cache.0 != song {
            let cover = props.Thumbnail().ok().and_then(|r| read_cover(&r));
            *cache = (song, cover);
        }
        cache.1.clone()
    });

    Some(MediaInfo { title, artist, source, playing, thumbnail })
}

/// "toggle", "next" or "previous" on whatever is playing.
pub fn media_command(command: &str) -> Result<(), String> {
    let session = media_session().ok_or("nothing is playing")?;
    let op = match command {
        "toggle" => session.TryTogglePlayPauseAsync(),
        "next" => session.TrySkipNextAsync(),
        "previous" => session.TrySkipPreviousAsync(),
        _ => return Err("unknown media command".into()),
    };
    op.map_err(|e| e.to_string())?.join().map(|_| ()).map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// System widget: processor, memory, battery
// ---------------------------------------------------------------------------

use windows::Win32::Foundation::FILETIME;
use windows::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};
use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
use windows::Win32::System::Threading::GetSystemTimes;

/// Idle and total processor time at the previous call: usage is the change between two calls.
static CPU_LAST: Mutex<Option<(u64, u64)>> = Mutex::new(None);

fn ticks(t: FILETIME) -> u64 {
    ((t.dwHighDateTime as u64) << 32) | t.dwLowDateTime as u64
}

fn cpu_percent() -> u8 {
    let (mut idle, mut kernel, mut user) = (FILETIME::default(), FILETIME::default(), FILETIME::default());
    if unsafe { GetSystemTimes(Some(&mut idle), Some(&mut kernel), Some(&mut user)) }.is_err() {
        return 0;
    }
    // Kernel time already includes idle time.
    let (idle, total) = (ticks(idle), ticks(kernel) + ticks(user));
    let Ok(mut last) = CPU_LAST.lock() else { return 0 };
    let percent = match *last {
        Some((last_idle, last_total)) if total > last_total => {
            let span = total - last_total;
            let idle_span = idle.saturating_sub(last_idle).min(span);
            100 - idle_span * 100 / span
        }
        // First call: nothing to compare with yet.
        _ => 0,
    };
    *last = Some((idle, total));
    percent.min(100) as u8
}

pub fn system_stats() -> SystemStats {
    let mut memory = MEMORYSTATUSEX { dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32, ..Default::default() };
    let memory_load = if unsafe { GlobalMemoryStatusEx(&mut memory) }.is_ok() { memory.dwMemoryLoad.min(100) as u8 } else { 0 };

    let mut power = SYSTEM_POWER_STATUS::default();
    let has_power = unsafe { GetSystemPowerStatus(&mut power) }.is_ok();
    // BatteryFlag 128 = no battery; 255 = unknown. BatteryLifePercent 255 = unknown.
    let battery = (has_power && power.BatteryFlag & 128 == 0 && power.BatteryFlag != 255 && power.BatteryLifePercent <= 100)
        .then_some(power.BatteryLifePercent);

    SystemStats { cpu: cpu_percent(), memory: memory_load, battery, charging: has_power && power.ACLineStatus == 1 }
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
