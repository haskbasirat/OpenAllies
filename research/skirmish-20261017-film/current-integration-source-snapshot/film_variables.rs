//! Current application/user variables reached by76FE60/7757F0 film recording.
//! The fresh Rust path explicitly excludes native profile/console overrides.
//! Values are constructed once from actual wildcard source files and retained;
//! later calls read/mutate this same owner, never reconstruct source defaults.
use crate::{catalog::Catalog, load_view_variables::NativeOverrides, localization::Localization};
use aa_rules::{
    native::index::native_enumerate,
    resources::Provenance,
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};
use std::collections::BTreeMap;

#[cfg(test)]
#[path = "film_variables_tests.rs"]
mod tests;

type Result<T> = std::result::Result<T, String>;
struct Declaration {
    kind: String,
    raw: String,
    source: Provenance,
}
pub struct State {
    path_films: Vec<u16>,
    last_film_name: Vec<u16>,
    rotation: i32,
    last_index: i32,
    map_save_compressed: u8,
    editor_map_strings_separate: u8,
    source_files: Vec<(String, String)>,
}
fn string(raw: &str) -> Result<Vec<u16>> {
    if !raw.is_ascii() {
        return Err("film AVar source string needs native encoding producer".into());
    }
    let value = if let Some(tail) = raw.strip_prefix('"') {
        let Some(end) = tail.find('"') else {
            return Err("film AVar source string quote is unclosed".into());
        };
        if !tail[end + 1..].trim().is_empty() {
            return Err("film AVar string expression is unsupported".into());
        }
        &tail[..end]
    } else {
        raw
    };
    if value.contains(['\0', '\\', '&']) {
        return Err("film AVar string escaping is unbound".into());
    }
    Ok(value.encode_utf16().collect())
}
impl State {
    pub fn from_fresh_sources(
        catalog: &Catalog<'_>,
        sources: &mut V14Sources,
        overrides: NativeOverrides,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        if !matches!(overrides, NativeOverrides::ControlledNone) {
            return Err("film current profile/console override owner is unbound".into());
        }
        let paths: Vec<_> = sources
            .catalog()
            .into_iter()
            .map(|e| e.logical_path)
            .collect();
        let mut rows = BTreeMap::<String, Vec<Declaration>>::new();
        let mut source_files = Vec::new();
        for pattern in ["AVars*.tgi", "UVars*.tgi"] {
            for path in native_enumerate(pattern, &paths) {
                let file = sources.read(&path).map_err(|e| e.to_string())?;
                if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                    return Err("film variable source bytes/hash differ".into());
                }
                let doc = Document::parse_native(file.bytes, Limits::default())
                    .map_err(|e| e.to_string())?;
                source_files.push((file.logical_path.clone(), file.sha256.clone()));
                for item in &doc.items {
                    let Item::Node(node) = item else { continue };
                    if !doc
                        .header_name(&node.header)
                        .is_some_and(|n| n.eq_ignore_ascii_case("Vars"))
                    {
                        continue;
                    }
                    for item in &node.items {
                        let Item::Property(p) = item else { continue };
                        let Some(name) = doc.property_name(p) else {
                            continue;
                        };
                        if ![
                            "PathFilms",
                            "LastFilmName",
                            "FilmRotationQueueSize",
                            "LastFilmIndex",
                            "MapSaveCompressed",
                            "EditorMapStringsSeparate",
                        ]
                        .iter()
                        .any(|n| name.eq_ignore_ascii_case(n))
                        {
                            continue;
                        }
                        if p.left.len() != 2 {
                            return Err(
                                "film variable source assignment override order/type is unbound"
                                    .into(),
                            );
                        }
                        rows.entry(name.to_ascii_lowercase())
                            .or_default()
                            .push(Declaration {
                                kind: doc
                                    .text(p.left[0].span)
                                    .ok_or("film variable source type bytes invalid")?
                                    .into(),
                                raw: doc
                                    .property_value(p)
                                    .ok_or("film variable source value absent")?
                                    .into(),
                                source: Provenance {
                                    path: file.logical_path.clone(),
                                    sha256: file.sha256.clone(),
                                    span: p.span,
                                },
                            });
                    }
                }
            }
        }
        let one = |name: &str, kind: &str| -> Result<&Declaration> {
            let [row] = rows
                .get(&name.to_ascii_lowercase())
                .ok_or("film variable source declaration missing")?
                .as_slice()
            else {
                return Err("film variable duplicate source producer is unbound".into());
            };
            if !row.kind.eq_ignore_ascii_case(kind) {
                return Err("film variable native source type differs".into());
            }
            Ok(row)
        };
        //AVars projection must still match the actual private file parse. The
        //source declarations are not themselves the mutable current owner.
        let settings = catalog
            .settings
            .as_ref()
            .ok_or("film Settings owner missing")?;
        for name in ["PathFilms", "LastFilmName", "FilmRotationQueueSize"] {
            let row = rows
                .get(&name.to_ascii_lowercase())
                .and_then(|r| if r.len() == 1 { Some(&r[0]) } else { None })
                .ok_or("film AVar source producer differs")?;
            let [projection] = settings
                .app_variables
                .get(&name.to_ascii_lowercase())
                .ok_or("film AVar projection missing")?
                .as_slice()
            else {
                return Err("film AVar projection duplicate order is unbound".into());
            };
            if projection.source != row.source
                || projection.raw != row.raw
                || !projection
                    .field_type
                    .as_deref()
                    .is_some_and(|k| k.eq_ignore_ascii_case(&row.kind))
            {
                return Err("film AVar projection value/span differs from actual source".into());
            }
        }
        if rows.contains_key("lastfilmindex") {
            return Err(
                "film LastFilmIndex positive source replacement needs its native producer".into(),
            );
        }
        let path_films = string(&one("PathFilms", "string")?.raw)?;
        let last_film_name = string(&one("LastFilmName", "string")?.raw)?;
        let rotation = one("FilmRotationQueueSize", "int")?
            .raw
            .parse::<i32>()
            .map_err(|e| e.to_string())?;
        let map_save_compressed = match one("MapSaveCompressed", "flag")?
            .raw
            .to_ascii_lowercase()
            .as_str()
        {
            "true" => 1,
            "false" => 0,
            _ => return Err("film MapSaveCompressed source Boolean expression is unbound".into()),
        };
        let editor_map_strings_separate = match one("EditorMapStringsSeparate", "flag")?
            .raw
            .to_ascii_lowercase()
            .as_str()
        {
            "true" => 1,
            "false" => 0,
            _ => {
                return Err(
                    "film EditorMapStringsSeparate source Boolean expression is unbound".into(),
                );
            }
        };
        //41FA13 EBX=0;422E62..92 registers LastFilmIndex Int0/flags10.
        Ok(Self {
            path_films,
            last_film_name,
            rotation,
            last_index: 0,
            map_save_compressed,
            editor_map_strings_separate,
            source_files,
        })
    }
    pub fn path_films(&self) -> &[u16] {
        &self.path_films
    }
    pub fn rotation_queue_size(&self) -> i32 {
        self.rotation
    }
    pub fn last_index(&self) -> i32 {
        self.last_index
    }
    pub fn compressed_byte(&self) -> u8 {
        self.map_save_compressed
    }
    ///Current UVar+278, read only after MAPSTRINGS serialization and the
    ///unconditional separate-path removal in59FDA0. The source default is
    ///retained until a real application configuration writes this same cell.
    pub fn separate_map_strings_byte(&self) -> u8 {
        self.editor_map_strings_separate
    }
    pub fn source_files(&self) -> &[(String, String)] {
        &self.source_files
    }
    pub fn assign_rotation_queue_size(&mut self, value: i32) {
        self.rotation = value
    }
    pub fn assign_last_index(&mut self, value: i32) {
        self.last_index = value
    }
    pub fn assign_compressed_byte(&mut self, value: u8) {
        self.map_save_compressed = value
    }
    ///Actual4B3190/4B29B0 type1/flags10 current Bool byte store. A portable backend may explicitly
    ///choose0; source declarations/defaults remain unchanged and recording
    ///still writes the full embedded MAPSTRINGS through its real TextStream.
    pub fn assign_separate_map_strings_byte(&mut self, value: u8) {
        self.editor_map_strings_separate = value;
    }
    pub fn assign_last_film_name(&mut self, value: &[u16]) {
        self.last_film_name = value.to_vec()
    }
    /// Actual76FE60. The current LastFilmIndex is advanced BEFORE formatting
    /// and Session+150 assignment. The filename uses the clamped OLD index.
    pub fn filename<'a>(
        &mut self,
        current_session_name150: &'a mut Vec<u16>,
        rotate: u8,
        current_text: &Localization,
    ) -> Result<&'a [u16]> {
        if rotate == 0 {
            return Ok(current_session_name150);
        }
        let localized = current_text
            .display(&"last_film_filename".encode_utf16().collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        let base = if localized.is_empty() {
            &self.last_film_name
        } else {
            &localized
        };
        if self.rotation <= 0 {
            return Err("reached native film rotation IDIV has a nonpositive queue size".into());
        }
        let index = self.last_index.max(0).min(self.rotation - 1);
        self.last_index = (index + 1) % self.rotation;
        let suffix = format!("_{index}");
        let mut next = base.clone();
        next.extend(suffix.encode_utf16());
        *current_session_name150 = next;
        Ok(current_session_name150)
    }
}
