//! Original data access for the UI: case-insensitive lookup inside the
//! extracted `Data` directory (the original runs on Windows, where every
//! lookup is case-insensitive), the `%s` resolution-folder rule for UI art,
//! and image header sizes.
//!
//! `%s` rule, `0x490ED0` (`FUN_00490ed0`): the property text is formatted with
//! the screen width (`swprintf(L"%d", *(int*)0xB396D0)`), normalised and
//! looked up; if that file does not exist the original text is used with the
//! `%s` and everything up to and including the next `/` (or `\`) removed. So
//! `/UI/%s/Default/x.tga` is `/UI/1280/Default/x.tga` at 1280 pixels wide and
//! `/UI/Default/x.tga` otherwise (there is no `UI/2560` folder).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;

/// One searchable directory.
#[derive(Debug)]
struct Layer {
    /// Name reported to callers (`skin:USA`, `v14`, `data`).
    name: String,
    root: PathBuf,
    /// directory path -> (lower-case entry name -> real name)
    dirs: RwLock<HashMap<PathBuf, Option<HashMap<String, String>>>>,
}

impl Clone for Layer {
    fn clone(&self) -> Self {
        Layer { name: self.name.clone(), root: self.root.clone(), dirs: RwLock::new(HashMap::new()) }
    }
}

impl Layer {
    fn new(name: &str, root: PathBuf) -> Self {
        Layer { name: name.to_string(), root, dirs: RwLock::new(HashMap::new()) }
    }

    fn listing(&self, dir: &Path) -> Option<HashMap<String, String>> {
        if let Some(found) = self.dirs.read().ok()?.get(dir) {
            return found.clone();
        }
        let listing = std::fs::read_dir(dir).ok().map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter_map(|e| e.file_name().into_string().ok())
                .map(|n| (n.to_ascii_lowercase(), n))
                .collect::<HashMap<_, _>>()
        });
        if let Ok(mut dirs) = self.dirs.write() {
            dirs.insert(dir.to_path_buf(), listing.clone());
        }
        listing
    }

    fn find(&self, data_path: &str) -> Option<PathBuf> {
        let mut at = self.root.clone();
        for part in data_path.split(['/', '\\']).filter(|p| !p.is_empty() && *p != ".") {
            let listing = self.listing(&at)?;
            let real = listing.get(&part.to_ascii_lowercase())?;
            at.push(real);
        }
        Some(at)
    }

    fn list_dir(&self, data_dir: &str) -> Vec<String> {
        let Some(dir) = self.find(data_dir) else { return Vec::new() };
        std::fs::read_dir(dir)
            .map(|rd| {
                rd.flatten()
                    .filter(|e| e.path().is_file())
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Case-insensitive view of the original data: ordered layers searched
/// first to last - an optional nation skin (`Skins/<NATION>`, the in-game
/// art swapped by `KKC_UI::Reskin` 0x5C3DC0; precedence inferred from the
/// in-game screenshots, which show the USA skin's control panel), the
/// accepted v14 loose overrides, then the extracted `Data.rwd`.
#[derive(Debug, Clone)]
pub struct DataRoot {
    layers: Vec<Layer>,
}

impl DataRoot {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { layers: vec![Layer::new("data", root.into())] }
    }

    /// `base` (extracted archive) with a loose-file `overlay` searched first.
    pub fn with_overlay(base: impl Into<PathBuf>, overlay: impl Into<PathBuf>) -> Self {
        Self { layers: vec![Layer::new("v14", overlay.into()), Layer::new("data", base.into())] }
    }

    /// Add (or replace) the skin layer searched before everything else.
    pub fn set_skin(&mut self, nation: Option<&str>, skins_root: impl Into<PathBuf>) {
        self.layers.retain(|l| !l.name.starts_with("skin:"));
        if let Some(n) = nation {
            self.layers.insert(0, Layer::new(&format!("skin:{n}"), skins_root.into().join(n)));
        }
    }

    /// The active skin nation, if any.
    pub fn skin(&self) -> Option<&str> {
        self.layers.first().and_then(|l| l.name.strip_prefix("skin:"))
    }

    /// Root of the base (archive) layer.
    pub fn root(&self) -> &Path {
        &self.layers.last().expect("base layer").root
    }

    /// Workspace default: `reconstruction/assets/original/Data` with the
    /// accepted v14 loose overrides from
    /// `reconstruction/native-game/runtime-expanded-v14/Data`, located from
    /// this crate's manifest directory.
    pub fn workspace_default() -> Self {
        let recon = Self::reconstruction_dir();
        Self::with_overlay(recon.join("assets/original/Data"), recon.join("native-game/runtime-expanded-v14/Data"))
    }

    /// `reconstruction/` beside this workspace.
    pub fn reconstruction_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../reconstruction")
    }

    /// Workspace skins folder (`reconstruction/assets/original/Skins`).
    pub fn workspace_skins() -> PathBuf {
        Self::reconstruction_dir().join("assets/original/Skins")
    }

    /// Resolve a data-relative path (`/UI/Default/x.tga`, `ui\menus.lst`)
    /// to the real on-disk path, ignoring case. `None` when absent.
    pub fn find(&self, data_path: &str) -> Option<PathBuf> {
        self.locate(data_path).map(|(_, p)| p)
    }

    /// Layer name and on-disk path of a data path.
    pub fn locate(&self, data_path: &str) -> Option<(&str, PathBuf)> {
        self.layers.iter().find_map(|l| l.find(data_path).map(|p| (l.name.as_str(), p)))
    }

    /// The data-relative path with the on-disk spelling of every component
    /// (`/UI/Default/Menus/MainMenu/MainMenuTop.TGA`).
    pub fn canonical(&self, data_path: &str) -> Option<String> {
        let (layer, real) = self.locate(data_path)?;
        let root = &self.layers.iter().find(|l| l.name == layer)?.root;
        let rel = real.strip_prefix(root).ok()?.to_path_buf();
        let mut out = String::new();
        for c in rel.components() {
            out.push('/');
            out.push_str(&c.as_os_str().to_string_lossy());
        }
        Some(out)
    }

    /// File names directly inside a data directory, first layer first,
    /// unique case-insensitively.
    pub fn list_dir(&self, data_dir: &str) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for l in &self.layers {
            for n in l.list_dir(data_dir) {
                if !out.iter().any(|o| o.eq_ignore_ascii_case(&n)) {
                    out.push(n);
                }
            }
        }
        out
    }

    pub fn exists(&self, data_path: &str) -> bool {
        self.find(data_path).is_some_and(|p| p.is_file())
    }

    pub fn read(&self, data_path: &str) -> std::io::Result<Vec<u8>> {
        match self.find(data_path) {
            Some(p) => std::fs::read(p),
            None => Err(std::io::Error::new(std::io::ErrorKind::NotFound, data_path.to_string())),
        }
    }
}

/// `0x490ED0` path rule. `exists` reports whether a data path is present.
/// Returns the path to load (it may still not exist; the caller reports the
/// missing texture as the original's loader does).
pub fn resolve_resolution_path(raw: &str, screen_width: i32, exists: impl Fn(&str) -> bool) -> String {
    let formatted = raw.replacen("%s", &screen_width.to_string(), 1);
    if exists(&formatted) {
        return formatted;
    }
    match raw.find('%') {
        Some(at) => {
            let rest = &raw[at..];
            // first '/' after '%', otherwise the first '\' after it (0x40DE80
            // searches 0x5C from the same index); otherwise to the end.
            let end = rest
                .find('/')
                .or_else(|| rest.find('\\'))
                .map(|i| at + i)
                .unwrap_or(raw.len() - 1);
            let mut out = String::with_capacity(raw.len());
            out.push_str(&raw[..at]);
            out.push_str(&raw[(end + 1).min(raw.len())..]);
            out
        }
        None => raw.to_string(),
    }
}

/// Pixel size of a TGA, PNG, DDS or BMP image from its header.
pub fn image_size(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() >= 24 && bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        let w = u32::from_be_bytes(bytes[16..20].try_into().ok()?);
        let h = u32::from_be_bytes(bytes[20..24].try_into().ok()?);
        return Some((w, h));
    }
    if bytes.len() >= 20 && bytes.starts_with(b"DDS ") {
        let h = u32::from_le_bytes(bytes[12..16].try_into().ok()?);
        let w = u32::from_le_bytes(bytes[16..20].try_into().ok()?);
        return Some((w, h));
    }
    if bytes.len() >= 26 && bytes.starts_with(b"BM") {
        let w = i32::from_le_bytes(bytes[18..22].try_into().ok()?);
        let h = i32::from_le_bytes(bytes[22..26].try_into().ok()?);
        return Some((w.unsigned_abs(), h.unsigned_abs()));
    }
    if bytes.len() >= 18 {
        // TGA: image type 1,2,3,9,10,11 with width/height at 12/14
        let kind = bytes[2];
        if matches!(kind, 1 | 2 | 3 | 9 | 10 | 11) {
            let w = u16::from_le_bytes([bytes[12], bytes[13]]) as u32;
            let h = u16::from_le_bytes([bytes[14], bytes[15]]) as u32;
            if w > 0 && h > 0 {
                return Some((w, h));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_s_rule() {
        let have = |p: &str| p == "/UI/1280/Default/x.tga";
        assert_eq!(resolve_resolution_path("/UI/%s/Default/x.tga", 1280, have), "/UI/1280/Default/x.tga");
        assert_eq!(resolve_resolution_path("/UI/%s/Default/x.tga", 2560, have), "/UI/Default/x.tga");
        assert_eq!(resolve_resolution_path("/UI/%s\\Default\\x.tga", 2560, |_| false), "/UI/Default\\x.tga");
        assert_eq!(resolve_resolution_path("/UI/x.tga", 2560, |_| false), "/UI/x.tga");
    }
}
