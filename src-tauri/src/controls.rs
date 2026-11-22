//! What the modules of a MyStyle bar do on the system: the volume, the power menu and the
//! Windows settings pages. Windows only; other systems get stand-ins so the editor still works.

/// The master volume of the default speakers.
#[derive(Debug, Clone, Copy, serde::Serialize)]
pub struct Volume {
    /// 0..=100.
    pub level: u8,
    pub muted: bool,
}

/// One app that is playing (or recently played) sound, with its own volume.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppVolume {
    /// 0 = the Windows system sounds.
    pub pid: u32,
    pub name: String,
    pub exe: Option<String>,
    pub level: u8,
    pub muted: bool,
}

/// What the power menu can do. The names are the ones a power element keeps in `Settings`.
pub const POWER_ACTIONS: [&str; 5] = ["shutdown", "restart", "sleep", "lock", "logout"];

#[cfg(windows)]
mod win {
    use super::Volume;
    use std::os::windows::process::CommandExt;
    use windows::core::{w, PCWSTR};
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::core::Interface;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IAudioSessionControl2, IAudioSessionManager2, IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_ALL, COINIT_MULTITHREADED};
    use windows::Win32::System::Power::SetSuspendState;
    use windows::Win32::System::Shutdown::LockWorkStation;
    use windows::Win32::UI::Input::KeyboardAndMouse::{keybd_event, KEYEVENTF_KEYUP, VK_LWIN};
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

    const CREATE_NO_WINDOW: u32 = 0x0800_0000;

    fn endpoint() -> Result<IAudioEndpointVolume, String> {
        // SAFETY: plain COM calls on interfaces this function creates and owns. A thread that
        // already has COM in another mode answers with an error that can be ignored.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let speakers = devices.GetDefaultAudioEndpoint(eRender, eConsole).map_err(|e| e.to_string())?;
            speakers.Activate::<IAudioEndpointVolume>(CLSCTX_ALL, None).map_err(|e| e.to_string())
        }
    }

    pub fn volume() -> Result<Volume, String> {
        let ep = endpoint()?;
        // SAFETY: reads from a live interface.
        unsafe {
            let level = ep.GetMasterVolumeLevelScalar().map_err(|e| e.to_string())?;
            let muted = ep.GetMute().map_err(|e| e.to_string())?.as_bool();
            Ok(Volume { level: (level * 100.0).round().clamp(0.0, 100.0) as u8, muted })
        }
    }

    pub fn set_volume(level: u8) -> Result<(), String> {
        let ep = endpoint()?;
        // SAFETY: writes to a live interface; a null context means "no one in particular".
        unsafe {
            ep.SetMasterVolumeLevelScalar(f32::from(level.min(100)) / 100.0, std::ptr::null()).map_err(|e| e.to_string())?;
            // Raising the volume of a muted device is how people take it off mute.
            if level > 0 && ep.GetMute().map_err(|e| e.to_string())?.as_bool() {
                ep.SetMute(false, std::ptr::null()).map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub fn set_mute(muted: bool) -> Result<(), String> {
        let ep = endpoint()?;
        // SAFETY: as above.
        unsafe { ep.SetMute(muted, std::ptr::null()).map_err(|e| e.to_string()) }
    }

    /// The apps with an audio session on the default speakers, one entry per process.
    pub fn app_volumes() -> Result<Vec<super::AppVolume>, String> {
        let mut out: Vec<super::AppVolume> = Vec::new();
        for_each_session(|pid, system, volume| {
            if out.iter().any(|a| a.pid == pid) {
                return;
            }
            // SAFETY: reads from a live interface.
            let (level, muted) = unsafe {
                let level = volume.GetMasterVolume().unwrap_or(1.0);
                (level, volume.GetMute().map(|m| m.as_bool()).unwrap_or(false))
            };
            let exe = if system { None } else { crate::platform::process_path(pid) };
            // A process whose executable cannot be read is a protected system service (an
            // svchost): the Windows mixer does not list those either.
            if !system && exe.is_none() {
                return;
            }
            let name = if system {
                "Sons do sistema".to_string()
            } else {
                exe.as_deref().map(crate::platform::display_name).unwrap_or_default()
            };
            out.push(super::AppVolume { pid, name, exe, level: (level * 100.0).round().clamp(0.0, 100.0) as u8, muted });
        })?;
        Ok(out)
    }

    /// Sets the volume and/or the mute of every session of one process.
    pub fn set_app_volume(pid: u32, level: Option<u8>, muted: Option<bool>) -> Result<(), String> {
        for_each_session(|p, _, volume| {
            if p != pid {
                return;
            }
            // SAFETY: writes to a live interface.
            unsafe {
                if let Some(level) = level {
                    let _ = volume.SetMasterVolume(f32::from(level.min(100)) / 100.0, std::ptr::null());
                }
                if let Some(muted) = muted {
                    let _ = volume.SetMute(muted, std::ptr::null());
                }
            }
        })
    }

    /// Calls `each(pid, is_system_sounds, volume)` for every live audio session.
    fn for_each_session(mut each: impl FnMut(u32, bool, &ISimpleAudioVolume)) -> Result<(), String> {
        // SAFETY: COM calls on interfaces this function creates and owns.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
            let devices: IMMDeviceEnumerator = CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(|e| e.to_string())?;
            let speakers = devices.GetDefaultAudioEndpoint(eRender, eConsole).map_err(|e| e.to_string())?;
            let manager = speakers.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None).map_err(|e| e.to_string())?;
            let sessions = manager.GetSessionEnumerator().map_err(|e| e.to_string())?;
            for i in 0..sessions.GetCount().map_err(|e| e.to_string())? {
                let Ok(control) = sessions.GetSession(i) else { continue };
                let Ok(control2) = control.cast::<IAudioSessionControl2>() else { continue };
                let Ok(volume) = control.cast::<ISimpleAudioVolume>() else { continue };
                let system = control2.IsSystemSoundsSession().0 == 0;
                let pid = control2.GetProcessId().unwrap_or(0);
                if system || pid != 0 {
                    each(if system { 0 } else { pid }, system, &volume);
                }
            }
        }
        Ok(())
    }

    fn run(program: &str, args: &[&str]) -> Result<(), String> {
        std::process::Command::new(program).args(args).creation_flags(CREATE_NO_WINDOW).spawn().map(|_| ()).map_err(|e| e.to_string())
    }

    pub fn power(action: &str) -> Result<(), String> {
        match action {
            // Not forced: an app with unsaved work still gets to say so.
            "shutdown" => run("shutdown", &["/s", "/t", "0"]),
            "restart" => run("shutdown", &["/r", "/t", "0"]),
            "logout" => run("shutdown", &["/l"]),
            // SAFETY: no arguments; Windows does the work.
            "lock" => unsafe { LockWorkStation().map_err(|e| e.to_string()) },
            "sleep" => {
                // Sleep, not hibernate, and not forced.
                // SAFETY: no pointers involved.
                unsafe { SetSuspendState(false, false, false) };
                Ok(())
            }
            _ => Err("unknown power action".into()),
        }
    }

    fn open_uri(uri: PCWSTR) {
        // SAFETY: `uri` outlives the call.
        unsafe {
            ShellExecuteW(None, w!("open"), uri, PCWSTR::null(), PCWSTR::null(), SW_SHOWNORMAL);
        }
    }

    pub fn open(target: &str) -> Result<(), String> {
        match target {
            // The quick settings panel (Wi-Fi, Bluetooth, sound...) is the Win+A shortcut.
            "quick" => {
                // SAFETY: synthesizes the key presses of a shortcut.
                unsafe {
                    keybd_event(VK_LWIN.0 as u8, 0, Default::default(), 0);
                    keybd_event(b'A', 0, Default::default(), 0);
                    keybd_event(b'A', 0, KEYEVENTF_KEYUP, 0);
                    keybd_event(VK_LWIN.0 as u8, 0, KEYEVENTF_KEYUP, 0);
                }
            }
            // A tap on the Windows key opens the Start menu.
            "start" => {
                // SAFETY: synthesizes one key press.
                unsafe {
                    keybd_event(VK_LWIN.0 as u8, 0, Default::default(), 0);
                    keybd_event(VK_LWIN.0 as u8, 0, KEYEVENTF_KEYUP, 0);
                }
            }
            "settings" => open_uri(w!("ms-settings:")),
            // The classic volume mixer, with a volume per app.
            "mixer" => open_uri(w!("sndvol.exe")),
            "wifi" => open_uri(w!("ms-settings:network-wifi")),
            "sound" => open_uri(w!("ms-settings:sound")),
            "bluetooth" => open_uri(w!("ms-settings:bluetooth")),
            _ => return Err("unknown target".into()),
        }
        Ok(())
    }
}

#[cfg(windows)]
pub use win::{app_volumes, open as open_target, power as power_action, set_app_volume, set_mute, set_volume, volume};

#[cfg(not(windows))]
mod other {
    use super::Volume;
    pub fn volume() -> Result<Volume, String> {
        Ok(Volume { level: 60, muted: false })
    }
    pub fn set_volume(_level: u8) -> Result<(), String> {
        Ok(())
    }
    pub fn set_mute(_muted: bool) -> Result<(), String> {
        Ok(())
    }
    pub fn app_volumes() -> Result<Vec<super::AppVolume>, String> {
        Ok(Vec::new())
    }
    pub fn set_app_volume(_pid: u32, _level: Option<u8>, _muted: Option<bool>) -> Result<(), String> {
        Ok(())
    }
    pub fn power(_action: &str) -> Result<(), String> {
        Ok(())
    }
    pub fn open(_target: &str) -> Result<(), String> {
        Ok(())
    }
}
#[cfg(not(windows))]
pub use other::{app_volumes, open as open_target, power as power_action, set_app_volume, set_mute, set_volume, volume};
