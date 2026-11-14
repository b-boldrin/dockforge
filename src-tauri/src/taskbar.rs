//! Reads the apps pinned to the Windows taskbar, so they can become DockForge favorites.
//!
//! Windows keeps one shortcut (.lnk) per pinned app in
//! `%APPDATA%\Microsoft\Internet Explorer\Quick Launch\User Pinned\TaskBar`.
//! The shortcut format is documented by Microsoft ([MS-SHLLINK]); only the part that holds
//! the target path is read here. Store apps and File Explorer have no path in their
//! shortcut and are skipped.

use crate::settings::Pin;
use std::path::{Path, PathBuf};

const HEADER_SIZE: usize = 0x4C;
const HAS_ID_LIST: u32 = 1;
const HAS_LINK_INFO: u32 = 1 << 1;

fn u16_at(b: &[u8], at: usize) -> Option<usize> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?) as usize)
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// A NUL-terminated string in the system's 8-bit code page. Only ASCII is certain to be
/// right, so anything else is read as Latin-1 (the Unicode copy is preferred when present).
fn ansi_at(b: &[u8], at: usize) -> Option<String> {
    let rest = b.get(at..)?;
    let end = rest.iter().position(|&c| c == 0)?;
    Some(rest[..end].iter().map(|&c| c as char).collect())
}

fn utf16_at(b: &[u8], at: usize) -> Option<String> {
    let rest = b.get(at..)?;
    let units: Vec<u16> = rest.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
    String::from_utf16(&units).ok()
}

/// The path a shortcut points to, if it stores one.
pub fn target_of(lnk: &[u8]) -> Option<String> {
    if u32_at(lnk, 0)? as usize != HEADER_SIZE {
        return None;
    }
    let flags = u32_at(lnk, 0x14)?;
    if flags & HAS_LINK_INFO == 0 {
        return None;
    }
    let mut at = HEADER_SIZE;
    if flags & HAS_ID_LIST != 0 {
        at += 2 + u16_at(lnk, at)?;
    }

    let info = lnk.get(at..)?;
    let size = u32_at(info, 0)? as usize;
    let info = info.get(..size)?;
    let header = u32_at(info, 4)? as usize;
    let has_local_path = u32_at(info, 8)? & 1 != 0;
    if !has_local_path {
        return None;
    }
    let (base, suffix) = if header >= 0x24 {
        (utf16_at(info, u32_at(info, 0x1C)? as usize)?, utf16_at(info, u32_at(info, 0x20)? as usize)?)
    } else {
        (ansi_at(info, u32_at(info, 0x10)? as usize)?, ansi_at(info, u32_at(info, 0x18)? as usize)?)
    };
    let path = format!("{base}{suffix}");
    (!path.is_empty()).then_some(path)
}

fn taskbar_dir() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(Path::new(&appdata).join("Microsoft\\Internet Explorer\\Quick Launch\\User Pinned\\TaskBar"))
}

/// The programs pinned to the Windows taskbar, in alphabetical order (Windows keeps the
/// real order in a private format). Only programs that still exist are returned.
pub fn pinned() -> Vec<Pin> {
    let Some(dir) = taskbar_dir() else { return Vec::new() };
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")))
        .collect();
    files.sort();

    let mut pins = Vec::new();
    for file in files {
        // A shortcut is a few kB; anything huge is not one.
        if std::fs::metadata(&file).map(|m| m.len() > 1_000_000).unwrap_or(true) {
            continue;
        }
        let Some(target) = std::fs::read(&file).ok().and_then(|bytes| target_of(&bytes)) else { continue };
        let is_program = Path::new(&target).extension().is_some_and(|x| x.eq_ignore_ascii_case("exe"));
        if !is_program || !Path::new(&target).exists() {
            continue;
        }
        let name = file.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
        pins.push(Pin { name, path: target, group: 0 });
    }
    pins
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a minimal shortcut: header, optional ID list, and a LinkInfo block.
    fn shortcut(path: &str, unicode: bool, id_list: bool) -> Vec<u8> {
        let mut out = vec![0u8; HEADER_SIZE];
        out[0..4].copy_from_slice(&(HEADER_SIZE as u32).to_le_bytes());
        let flags = HAS_LINK_INFO | if id_list { HAS_ID_LIST } else { 0 };
        out[0x14..0x18].copy_from_slice(&flags.to_le_bytes());
        if id_list {
            out.extend_from_slice(&6u16.to_le_bytes());
            out.extend_from_slice(&[9, 9, 9, 9, 0, 0]);
        }

        let header: u32 = if unicode { 0x24 } else { 0x1C };
        let mut info = vec![0u8; header as usize];
        let ansi_base = info.len() as u32;
        info.extend_from_slice(if unicode { b"C:\\short.exe\0" } else { path.as_bytes() });
        if !unicode {
            info.push(0);
        }
        let ansi_suffix = info.len() as u32;
        info.push(0);
        info[4..8].copy_from_slice(&header.to_le_bytes());
        info[8..12].copy_from_slice(&1u32.to_le_bytes());
        info[0x10..0x14].copy_from_slice(&ansi_base.to_le_bytes());
        info[0x18..0x1C].copy_from_slice(&ansi_suffix.to_le_bytes());
        if unicode {
            let base = info.len() as u32;
            for unit in path.encode_utf16().chain([0]) {
                info.extend_from_slice(&unit.to_le_bytes());
            }
            let suffix = info.len() as u32;
            info.extend_from_slice(&[0, 0]);
            info[0x1C..0x20].copy_from_slice(&base.to_le_bytes());
            info[0x20..0x24].copy_from_slice(&suffix.to_le_bytes());
        }
        let size = info.len() as u32;
        info[0..4].copy_from_slice(&size.to_le_bytes());
        out.extend_from_slice(&info);
        out
    }

    #[test]
    fn reads_the_target_path() {
        let path = "C:\\Program Files\\App\\app.exe";
        assert_eq!(target_of(&shortcut(path, false, false)).as_deref(), Some(path));
        assert_eq!(target_of(&shortcut(path, false, true)).as_deref(), Some(path));
    }

    #[test]
    fn prefers_the_unicode_path() {
        let path = "C:\\Usuários\\João\\música.exe";
        assert_eq!(target_of(&shortcut(path, true, true)).as_deref(), Some(path));
    }

    #[test]
    fn rejects_files_that_are_not_shortcuts_or_are_cut_short() {
        assert_eq!(target_of(b"not a shortcut"), None);
        let full = shortcut("C:\\a.exe", true, true);
        for len in 0..full.len() - 1 {
            // Must never panic, whatever the length.
            let _ = target_of(&full[..len]);
        }
        let mut no_info = shortcut("C:\\a.exe", false, false);
        no_info[0x14] = 0;
        assert_eq!(target_of(&no_info), None);
    }
}
