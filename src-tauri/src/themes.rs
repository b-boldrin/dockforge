//! Theme loading.
//!
//! A theme is a folder in %APPDATA%\DockForge\themes\<id>\ with theme.json and styles.css.
//! Before CSS reaches the web view, every url(...) is resolved here: a reference to an
//! image or font inside the theme's assets/ folder becomes a data: URL, and anything else
//! (internet addresses, paths that escape the folder) is replaced by an empty url().
//! The web view's Content-Security-Policy would block remote loads anyway; this is a
//! second layer and it makes local assets work without opening file access to the web side.

use base64::Engine;
use serde::Deserialize;
use std::fs;
use std::path::{Component, Path, PathBuf};

use crate::model::{ThemePayload, ThemeSummary};

pub const MAX_CSS_BYTES: u64 = 200 * 1024;
pub const MAX_ASSET_BYTES: u64 = 5 * 1024 * 1024;

/// Themes shipped inside the executable. Copied to the themes folder on first run so
/// people can open them, learn from them and duplicate them.
pub const BUILT_IN: &[(&str, &str, &str)] = &[
    (
        "forge-glass",
        include_str!("../themes/forge-glass/theme.json"),
        include_str!("../themes/forge-glass/styles.css"),
    ),
    (
        "midnight-pill",
        include_str!("../themes/midnight-pill/theme.json"),
        include_str!("../themes/midnight-pill/styles.css"),
    ),
    (
        "forja-samurai",
        include_str!("../themes/forja-samurai/theme.json"),
        include_str!("../themes/forja-samurai/styles.css"),
    ),
    (
        "natal-cristal",
        include_str!("../themes/natal-cristal/theme.json"),
        include_str!("../themes/natal-cristal/styles.css"),
    ),
    (
        "noite-de-natal",
        include_str!("../themes/noite-de-natal/theme.json"),
        include_str!("../themes/noite-de-natal/styles.css"),
    ),
    (
        "papel",
        include_str!("../themes/papel/theme.json"),
        include_str!("../themes/papel/styles.css"),
    ),
    (
        "ambar-crt",
        include_str!("../themes/ambar-crt/theme.json"),
        include_str!("../themes/ambar-crt/styles.css"),
    ),
    (
        "barra-classica",
        include_str!("../themes/barra-classica/theme.json"),
        include_str!("../themes/barra-classica/styles.css"),
    ),
    (
        "transparente",
        include_str!("../themes/transparente/theme.json"),
        include_str!("../themes/transparente/styles.css"),
    ),
    (
        "barra-flutuante",
        include_str!("../themes/barra-flutuante/theme.json"),
        include_str!("../themes/barra-flutuante/styles.css"),
    ),
    (
        "retro-cinza",
        include_str!("../themes/retro-cinza/theme.json"),
        include_str!("../themes/retro-cinza/styles.css"),
    ),
    (
        "prateleira",
        include_str!("../themes/prateleira/theme.json"),
        include_str!("../themes/prateleira/styles.css"),
    ),
];

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Author {
    #[serde(default)]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: Author,
    #[serde(default)]
    pub tags: Vec<String>,
}

pub fn themes_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("themes")
}

/// Writes the built-in themes that are not on disk yet. Never overwrites user edits.
pub fn install_built_in(data_dir: &Path) -> std::io::Result<()> {
    for (id, manifest, css) in BUILT_IN {
        let dir = themes_dir(data_dir).join(id);
        if dir.exists() {
            continue;
        }
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("theme.json"), manifest)?;
        fs::write(dir.join("styles.css"), css)?;
    }
    Ok(())
}

fn is_valid_id(id: &str) -> bool {
    (3..=40).contains(&id.len())
        && id.split('-').all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

fn read_manifest(dir: &Path) -> Option<Manifest> {
    let text = fs::read_to_string(dir.join("theme.json")).ok()?;
    serde_json::from_str(&text).ok()
}

/// Every installed theme with a readable manifest whose id matches its folder, sorted by name.
pub fn list(data_dir: &Path) -> Vec<ThemeSummary> {
    let Ok(entries) = fs::read_dir(themes_dir(data_dir)) else { return Vec::new() };
    let mut themes: Vec<ThemeSummary> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let folder = e.file_name().to_string_lossy().to_string();
            let manifest = read_manifest(&e.path())?;
            (manifest.id == folder && is_valid_id(&folder)).then(|| ThemeSummary {
                built_in: BUILT_IN.iter().any(|(id, _, _)| *id == manifest.id),
                author: if manifest.author.name.is_empty() { "?".into() } else { manifest.author.name },
                tags: manifest.tags,
                id: manifest.id,
                name: manifest.name,
            })
        })
        .collect();
    themes.sort_by_key(|t| t.name.to_lowercase());
    themes
}

pub fn styles_path(data_dir: &Path, id: &str) -> PathBuf {
    themes_dir(data_dir).join(id).join("styles.css")
}

/// Loads a theme and returns CSS that is safe to hand to the web view.
pub fn load(data_dir: &Path, id: &str) -> Result<ThemePayload, String> {
    if !is_valid_id(id) {
        return Err(format!("invalid theme id: {id}"));
    }
    let dir = themes_dir(data_dir).join(id);
    let manifest = read_manifest(&dir).ok_or_else(|| format!("theme {id}: theme.json missing or invalid"))?;
    if manifest.id != id {
        return Err(format!("theme {id}: id in theme.json does not match the folder"));
    }
    let css_path = dir.join("styles.css");
    let size = fs::metadata(&css_path).map_err(|e| format!("theme {id}: {e}"))?.len();
    if size > MAX_CSS_BYTES {
        return Err(format!("theme {id}: styles.css is larger than 200 KB"));
    }
    let css = fs::read_to_string(&css_path).map_err(|e| format!("theme {id}: {e}"))?;
    Ok(ThemePayload { id: manifest.id, name: manifest.name, css: sanitize_css(&css, &dir) })
}

/// The built-in default, used when nothing on disk can be loaded.
pub fn fallback() -> ThemePayload {
    let (id, _, css) = BUILT_IN[0];
    ThemePayload { id: id.to_string(), name: "Forge Glass".to_string(), css: css.to_string() }
}

fn mime_for(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "woff2" => Some("font/woff2"),
        _ => None,
    }
}

/// Resolves a url() reference to a data: URL if it names an allowed file inside `<theme>/assets/`.
fn resolve_asset(reference: &str, theme_dir: &Path) -> Option<String> {
    let reference = reference.trim().trim_start_matches("./");
    if reference.contains(':') || reference.starts_with('/') || reference.starts_with('\\') {
        return None;
    }
    let rel = PathBuf::from(reference.replace('\\', "/"));
    let mut parts = rel.components();
    if parts.next() != Some(Component::Normal("assets".as_ref())) {
        return None;
    }
    if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    let full = theme_dir.join(&rel);
    let mime = mime_for(&full)?;
    let meta = fs::metadata(&full).ok()?;
    if !meta.is_file() || meta.len() > MAX_ASSET_BYTES {
        return None;
    }
    let bytes = fs::read(&full).ok()?;
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// Rewrites every url(...) and drops @import rules. Everything else is passed through.
pub fn sanitize_css(css: &str, theme_dir: &Path) -> String {
    let mut out = String::with_capacity(css.len());
    let lower = css.to_ascii_lowercase();
    let mut i = 0;
    while i < css.len() {
        let rest = &lower[i..];
        if rest.starts_with("@import") {
            // Skip to the end of the rule.
            let end = rest.find(';').map(|p| i + p + 1).unwrap_or(css.len());
            out.push_str("/* @import removed */");
            i = end;
        } else if rest.starts_with("url(") {
            let close = rest.find(')').map(|p| i + p).unwrap_or(css.len());
            let inner = css[i + 4..close].trim().trim_matches(|c| c == '"' || c == '\'');
            match resolve_asset(inner, theme_dir) {
                Some(data) => out.push_str(&format!("url(\"{data}\")")),
                None => out.push_str("url(\"\")"),
            }
            i = (close + 1).min(css.len());
        } else {
            let ch = css[i..].chars().next().expect("index is on a char boundary");
            out.push(ch);
            i += ch.len_utf8();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_theme() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("df-test-{}", std::process::id()));
        fs::create_dir_all(dir.join("assets")).unwrap();
        fs::write(dir.join("assets/bg.png"), [137, 80, 78, 71]).unwrap();
        dir
    }

    #[test]
    fn keeps_plain_css() {
        let css = ".df-dock { --df-radius: 12px; } /* ção */";
        assert_eq!(sanitize_css(css, Path::new(".")), css);
    }

    #[test]
    fn inlines_local_assets() {
        let dir = temp_theme();
        let out = sanitize_css(".a { background: url('assets/bg.png') }", &dir);
        assert!(out.contains("url(\"data:image/png;base64,"), "{out}");
    }

    #[test]
    fn blocks_remote_and_escaping_urls() {
        let dir = temp_theme();
        for bad in [
            "url(https://evil.example/x.png)",
            "url(//evil.example/x.png)",
            "url(assets/../../secret.png)",
            "url(C:/Windows/win.ini)",
            "url(data:image/png;base64,AAAA)",
            "url(assets/missing.png)",
        ] {
            let out = sanitize_css(&format!(".a {{ background: {bad} }}"), &dir);
            assert!(out.contains("url(\"\")"), "{bad} -> {out}");
        }
    }

    #[test]
    fn drops_imports() {
        let out = sanitize_css("@import url(https://evil.example/x.css); .a{}", Path::new("."));
        assert!(!out.contains("evil"), "{out}");
        assert!(out.contains(".a{}"));
    }

    #[test]
    fn built_in_themes_parse() {
        for (id, manifest, _) in BUILT_IN {
            let m: Manifest = serde_json::from_str(manifest).unwrap();
            assert_eq!(&m.id, id);
            assert!(is_valid_id(id));
        }
    }

    #[test]
    fn rejects_bad_ids() {
        for id in ["..", "a", "Abc", "a--b", "-abc", "abc/def", "abc\\def"] {
            assert!(!is_valid_id(id), "{id}");
        }
    }
}
