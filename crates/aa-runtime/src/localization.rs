//! Source-bound text for original saved localization keys. This covers unique
//! C-locale byte DisplayString literals; unsupported font escapes and duplicate
//! registration order remain explicit instead of becoming untranslated keys.
use aa_rules::{
    resources::Provenance,
    sources::{ResolvedFile, V14Sources},
    tgi::{Document, Item, Limits, TokenKind},
};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    sync::{Arc, Weak},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Source(#[from] aa_rules::sources::SourceError),
    #[error(transparent)]
    Syntax(#[from] aa_rules::tgi::ParseError),
    #[error("unsupported localization {key}: {reason}")]
    Field { key: String, reason: String },
}
pub type Result<T> = std::result::Result<T, Error>;
fn invalid(key: impl Into<String>, reason: impl Into<String>) -> Error {
    Error::Field {
        key: key.into(),
        reason: reason.into(),
    }
}
#[derive(Debug, Serialize)]
pub struct Text {
    pub source: Provenance,
    pub plain_utf16: std::result::Result<Vec<u16>, String>,
    #[serde(skip)]
    allocation: Arc<()>,
    ///Membership in the actual separately replaceable map_strings object.
    ///Only add_map_strings supplies this metadata; a path spelling is not
    ///authority to remove a global Text's registered backing allocation.
    #[serde(skip)]
    map_origin: bool,
}
impl Text {
    pub(crate) fn new(
        source: Provenance,
        plain_utf16: std::result::Result<Vec<u16>, String>,
    ) -> Self {
        Self {
            source,
            plain_utf16,
            allocation: Arc::new(()),
            map_origin: false,
        }
    }
}
impl Clone for Text {
    fn clone(&self) -> Self {
        let mut result=Self::new(self.source.clone(), self.plain_utf16.clone());
        result.map_origin=self.map_origin;
        result
    }
}
#[derive(Debug, Default, Serialize)]
pub struct Localization {
    pub declarations: BTreeMap<String, Vec<Text>>,
    #[serde(skip)]
    escapes: Option<Arc<crate::localization_escape::Table>>,
    #[serde(skip)]
    escaped_inputs: BTreeMap<(String, usize), EscapedInput>,
    #[serde(skip)]
    identity: Arc<()>,
    #[serde(skip)]
    nil_entry: Arc<()>,
}
impl Clone for Localization {
    fn clone(&self) -> Self {
        Self {
            declarations: self.declarations.clone(),
            escapes: self.escapes.clone(),
            escaped_inputs: self.escaped_inputs.clone(),
            identity: Arc::new(()),
            nil_entry: Arc::new(()),
        }
    }
}
/// Logical current-entry reference retained by MapResource+7C/+80. No text
/// payload is copied into this handle. A moved table keeps its identity; a
/// cloned table, replaced row, changed source or destroyed owner is rejected.
#[derive(Clone, Debug)]
pub struct EntryReference {
    owner: Weak<()>,
    allocation: Arc<()>,
    key: Option<String>,
    source: Option<Provenance>,
}
impl EntryReference {
    pub fn same_entry(&self, other: &Self) -> bool {
        Weak::ptr_eq(&self.owner, &other.owner) && Arc::ptr_eq(&self.allocation, &other.allocation)
    }
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
    pub fn source(&self) -> Option<&Provenance> {
        self.source.as_ref()
    }
}
#[derive(Clone, Debug)]
struct EscapedInput {
    source: Provenance,
    units: Vec<u16>,
}
struct TextSource {
    logical_path: String,
    sha256: String,
    bytes: Vec<u8>,
}

/// Original 856F90 -> 96CBD7/9710C6 reads a no-BOM file in the unchanged C
/// locale: each byte becomes one UTF-16 unit, including 80..FF. This is not
/// UTF-8 or Windows-1252 decoding. After 5BEE60 removes quotes, 5BA3C0 calls
/// 494000 and removes one leading '@'. Only its no-escape path is bound here.
/// See research/phase8-localization/native-display-bytes.json for original
/// instruction runs, including the two GER_Art_Map names previously rejected.
fn literal_units(raw: &[u8], utf8_bom: bool) -> std::result::Result<Vec<u16>, String> {
    if raw.len() < 2 || raw.first() != Some(&b'"') || raw.last() != Some(&b'"') {
        return Err("expected one DisplayString literal".into());
    }
    let text = &raw[1..raw.len() - 1];
    if text.iter().any(|b| *b < 32 || *b == 127 || *b == b'"') {
        return Err(
            "DisplayString control characters or inner quotes are outside the bound subset".into(),
        );
    }
    if utf8_bom && !text.is_ascii() {
        return Err(
            "non-ASCII UTF-8 BOM text requires the distinct native character-reader path".into(),
        );
    }
    Ok(text.iter().map(|&b| u16::from(b)).collect())
}
fn display_literal(raw: &[u8], utf8_bom: bool) -> std::result::Result<Vec<u16>, String> {
    let mut units = literal_units(raw, utf8_bom)?;
    if units
        .iter()
        .any(|&c| c == u16::from(b'&') || c == u16::from(b'\\'))
    {
        return Err("font escapes require native DisplayString escape-table decoding".into());
    }
    if units.first() == Some(&u16::from(b'@')) {
        units.remove(0);
    }
    Ok(units)
}

impl Localization {
    ///Existing-entry branch of4BAA20. Empty IDS selects the real4BAD50
    ///constructor's+60 nil entry (49AB70 with two empty CString words).
    ///Positive creation, its warning and+5C fallback are separate required
    ///services, so an absent nonempty key returns None without mutation.
    pub fn reference_existing(&self, key: &[u16]) -> Result<Option<EntryReference>> {
        let make = |allocation: Arc<()>, key, source| EntryReference {
            owner: Arc::downgrade(&self.identity),
            allocation,
            key,
            source,
        };
        if key.is_empty() {
            return Ok(Some(make(self.nil_entry.clone(), None, None)));
        }
        if key.iter().any(|&c| c > 127 || c == 0) {
            return Err(invalid(
                String::from_utf16_lossy(key),
                "entry reference needs native non-ASCII/embedded-NUL key lookup",
            ));
        }
        let key = String::from_utf16(key).expect("ASCII").to_ascii_lowercase();
        let Some(rows) = self.declarations.get(&key) else {
            return Ok(None);
        };
        let [row] = rows.as_slice() else {
            return Err(invalid(
                &key,
                "duplicate entries require native manager registration order",
            ));
        };
        Ok(Some(make(
            row.allocation.clone(),
            Some(key),
            Some(row.source.clone()),
        )))
    }
    ///Validate against the SAME current table on every use. Values may change
    ///through49AE00; replacing/reloading a row must not reuse an old handle.
    pub fn validate_reference(&self, reference: &EntryReference) -> Result<()> {
        if reference.owner.strong_count() == 0
            || !Weak::ptr_eq(&reference.owner, &Arc::downgrade(&self.identity))
        {
            return Err(invalid(
                "entry reference",
                "foreign, cloned or destroyed localization owner",
            ));
        }
        let Some(key) = &reference.key else {
            return if Arc::ptr_eq(&reference.allocation, &self.nil_entry)
                && reference.source.is_none()
            {
                Ok(())
            } else {
                Err(invalid(
                    "nil entry reference",
                    "constructor allocation differs",
                ))
            };
        };
        let rows = self
            .declarations
            .get(key)
            .ok_or_else(|| invalid(key, "retained entry was removed"))?;
        let [row] = rows.as_slice() else {
            return Err(invalid(key, "retained entry manager order changed"));
        };
        if !Arc::ptr_eq(&row.allocation, &reference.allocation)
            || Some(&row.source) != reference.source.as_ref()
        {
            return Err(invalid(
                key,
                "retained entry was replaced/reloaded or source changed",
            ));
        }
        Ok(())
    }
    /// These three nonrecursive groups are explicit in original index_aa.lst.
    /// Order cannot affect unique keys. Duplicates require the actual native
    /// registration order and DataCheckingStrict behavior before lookup.
    pub fn load_v14(sources: &mut V14Sources, limits: Limits) -> Result<Self> {
        //Explicit fresh application scope. This retained projection is not a
        //Glyph registry: lookup_bound later validates the real manager entry.
        let escapes = Arc::new(
            crate::localization_escape::Table::load(
                sources,
                crate::load_view_variables::NativeOverrides::ControlledNone,
            )
            .map_err(|e| invalid("font escapes", e))?,
        );
        let mut files = Vec::new();
        for entry in sources.catalog() {
            let lower = entry.logical_path.to_ascii_lowercase();
            let Some((directory, _)) = lower.rsplit_once('/') else {
                continue;
            };
            if lower.ends_with(".tgi")
                && matches!(
                    directory,
                    "localization" | "localization/sai" | "localization/randommap"
                )
            {
                files.push(sources.read(&entry.logical_path)?);
            }
        }
        Self::from_sources(
            files.into_iter().map(|file| TextSource {
                logical_path: file.logical_path,
                sha256: file.sha256,
                bytes: file.bytes,
            }),
            limits,
            Some(escapes),
        )
    }
    pub fn from_files(
        files: impl IntoIterator<Item = ResolvedFile>,
        limits: Limits,
    ) -> Result<Self> {
        Self::from_sources(
            files.into_iter().map(|file| TextSource {
                logical_path: file.logical_path,
                sha256: file.sha256,
                bytes: file.bytes,
            }),
            limits,
            None,
        )
    }
    fn from_sources(
        files: impl IntoIterator<Item = TextSource>,
        limits: Limits,
        escapes: Option<Arc<crate::localization_escape::Table>>,
    ) -> Result<Self> {
        let mut out = Self {
            escapes,
            ..Self::default()
        };
        for file in files {
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err(invalid(
                    &file.logical_path,
                    "source hash differs from bytes",
                ));
            }
            let utf8_bom = file.bytes.starts_with(&[0xef, 0xbb, 0xbf]);
            let doc = Document::parse_native(file.bytes, limits)?;
            for item in &doc.items {
                let Item::Node(node) = item else { continue };
                if !doc
                    .header_name(&node.header)
                    .is_some_and(|s| s.eq_ignore_ascii_case("Text"))
                {
                    continue;
                }
                let h: Vec<_> = node
                    .header
                    .tokens
                    .iter()
                    .map(|t| doc.text(t.span))
                    .collect();
                if h.len() != 4
                    || !h[1].is_some_and(|s| s.eq_ignore_ascii_case("language"))
                    || h[2] != Some("=")
                    || !h[3].is_some_and(|s| s.eq_ignore_ascii_case("default"))
                {
                    return Err(invalid(
                        &file.logical_path,
                        "unresolved Text language selection",
                    ));
                }
                for item in &node.items {
                    let Item::Property(p) = item else { continue };
                    let name = doc
                        .property_name(p)
                        .ok_or_else(|| invalid(&file.logical_path, "non-ASCII key"))?;
                    if name.eq_ignore_ascii_case("ids") || name.eq_ignore_ascii_case("name") {
                        continue;
                    }
                    if !name.is_ascii() {
                        return Err(invalid(name, "non-ASCII case-folding is not bound"));
                    }
                    let raw = if p.value_tokens.len() == 1
                        && p.value_tokens[0].kind == TokenKind::Quoted
                    {
                        doc.bytes(p.value_tokens[0].span).unwrap()
                    } else {
                        &[]
                    };
                    let mut plain_utf16 = display_literal(raw, utf8_bom);
                    if let Ok(units) = literal_units(raw, utf8_bom) {
                        let legacy = units
                            .iter()
                            .any(|&c| c == u16::from(b'&') || c == u16::from(b'\\'));
                        let configured = out
                            .escapes
                            .as_ref()
                            .is_some_and(|v| v.requires_decode(&units));
                        if legacy || configured {
                            plain_utf16 = Err(
                                "font escapes require native DisplayString escape-table decoding"
                                    .into(),
                            );
                            let key = name.to_ascii_lowercase();
                            let index = out.declarations.get(&key).map_or(0, Vec::len);
                            out.escaped_inputs.insert(
                                (key, index),
                                EscapedInput {
                                    source: Provenance {
                                        path: file.logical_path.clone(),
                                        sha256: file.sha256.clone(),
                                        span: p.span,
                                    },
                                    units,
                                },
                            );
                        }
                    }
                    out.declarations
                        .entry(name.to_ascii_lowercase())
                        .or_default()
                        .push(Text::new(
                            Provenance {
                                path: file.logical_path.clone(),
                                sha256: file.sha256.clone(),
                                span: p.span,
                            },
                            plain_utf16,
                        ));
                }
            }
        }
        Ok(out)
    }
    /// Original map-specific text is the MAPSTRINGS TGI chunk. Retain its
    /// independent hash and decode it before any saved Team/Kingdom name lookup.
    pub fn add_map_strings(&mut self, chunk: &aa_mapworld::Chunk, limits: Limits) -> Result<()> {
        chunk
            .verify()
            .map_err(|e| invalid("MAPSTRINGS", e.to_string()))?;
        if chunk.directory.name != "MAPSTRINGS" {
            return Err(invalid("MAPSTRINGS", "wrong chunk"));
        }
        let map = Self::from_sources(
            [TextSource {
                logical_path: format!("MAPSTRINGS/{}", chunk.payload_source.sha256),
                sha256: aa_rules::sources::sha256(&chunk.bytes),
                bytes: chunk.bytes.clone(),
            }],
            limits,
            self.escapes.clone(),
        )?;
        let offsets = map
            .declarations
            .keys()
            .map(|key| (key.clone(), self.declarations.get(key).map_or(0, Vec::len)))
            .collect::<BTreeMap<_, _>>();
        for ((key, index), input) in map.escaped_inputs {
            self.escaped_inputs
                .insert((key.clone(), offsets[&key] + index), input);
        }
        for (key, mut rows) in map.declarations {
            for row in &mut rows {row.map_origin=true;}
            self.declarations.entry(key).or_default().append(&mut rows);
        }
        Ok(())
    }
    ///478D20 destroys/unregisters the prior map_strings before47EA40 reads
    ///its replacement. Keep this table's identity and every surviving global
    ///entry allocation. Source-read/parse failure retains the removal prefix.
    ///The new source parser remains the existing successful Text language
    ///Default domain; this is not a generic partial TextData LoadData port.
    pub(crate) fn replace_map_strings(
        &mut self,
        read:impl FnOnce()->std::result::Result<aa_mapworld::Chunk,String>,
        limits:Limits,
    )->Result<aa_mapworld::Chunk> {
        let mut old_escapes=std::mem::take(&mut self.escaped_inputs);
        for (key,rows) in &mut self.declarations {
            let old=std::mem::take(rows);
            for (index,row) in old.into_iter().enumerate() {
                if row.map_origin {continue}
                let new_index=rows.len();
                if let Some(input)=old_escapes.remove(&(key.clone(),index)) {
                    self.escaped_inputs.insert((key.clone(),new_index),input);
                }
                rows.push(row);
            }
        }
        self.declarations.retain(|_,rows|!rows.is_empty());
        let chunk=read().map_err(|e|invalid("MAPSTRINGS",e))?;
        self.add_map_strings(&chunk,limits)?;
        Ok(chunk)
    }
    /// 4B9F40 with an explicit not-found result: `Ok(None)` where the
    /// original returns its default. Other refusals match [`Self::display`].
    pub fn lookup(&self, key: &[u16]) -> Result<Option<Vec<u16>>> {
        if key.iter().any(|&c| c > 127 || c == 0) {
            return Err(invalid(
                String::from_utf16_lossy(key),
                "non-ASCII or embedded-NUL lookup key",
            ));
        }
        let key = String::from_utf16(key).expect("ASCII UTF16");
        let Some(rows) = self.declarations.get(&key.to_ascii_lowercase()) else {
            return Ok(None);
        };
        let [row] = rows.as_slice() else {
            return Err(invalid(
                key,
                "duplicate entries require native registration order",
            ));
        };
        row.plain_utf16
            .clone()
            .map(Some)
            .map_err(|reason| invalid(key, reason))
    }
    ///Definition-time4B9F40 with494000 bound to the actual native Glyph
    ///manager. Plain/missing/duplicate lookups retain their existing results.
    ///A standalone from_files table without the source owner still refuses
    ///escapes; no index is inferred from a localized token or viewer metadata.
    pub fn lookup_bound(
        &self,
        key: &[u16],
        actors: &aa_rules::actors::ActorCatalog,
        pc: aa_numeric::Precision,
    ) -> Result<Option<Vec<u16>>> {
        let original = match self.lookup(key) {
            Ok(value) => return Ok(value),
            Err(error) => error,
        };
        if key.iter().any(|&c| c > 127 || c == 0) {
            return Err(original);
        }
        let key = String::from_utf16(key)
            .expect("ASCII UTF16")
            .to_ascii_lowercase();
        let Some(rows) = self.declarations.get(&key) else {
            return Err(original);
        };
        let [row] = rows.as_slice() else {
            return Err(original);
        };
        let Some(input) = self.escaped_inputs.get(&(key.clone(), 0)) else {
            return Err(original);
        };
        if input.source != row.source {
            return Err(invalid(
                key,
                "escaped literal source differs from current text entry",
            ));
        }
        let Some(table) = &self.escapes else {
            return Err(original);
        };
        let mut units = input.units.clone();
        table
            .decode_bound(actors, pc, &mut units)
            .map_err(|e| invalid(&key, e))?;
        //5BA3C0 removes the leading '@' AFTER494000, including an inserted @.
        if units.first() == Some(&u16::from(b'@')) {
            units.remove(0);
        }
        Ok(Some(units))
    }
    /// Original49AD70 key-only branch uses empty output for an empty key and
    /// original4B9F40/4B9D40 for other keys. Missing-entry diagnostic/fallback
    /// configuration is not guessed; successful lookups return owned UTF16.
    pub fn display(&self, key: &[u16]) -> Result<Vec<u16>> {
        if key.is_empty() {
            return Ok(Vec::new());
        }
        if key.iter().any(|&c| c > 127 || c == 0) {
            return Err(invalid(
                String::from_utf16_lossy(key),
                "non-ASCII or embedded-NUL lookup key",
            ));
        }
        let key = String::from_utf16(key).expect("ASCII UTF16");
        let rows = self
            .declarations
            .get(&key.to_ascii_lowercase())
            .ok_or_else(|| {
                invalid(
                    &key,
                    "missing source entry; native diagnostic/fallback unbound",
                )
            })?;
        let [row] = rows.as_slice() else {
            return Err(invalid(
                key,
                "duplicate entries require native registration order",
            ));
        };
        row.plain_utf16
            .clone()
            .map_err(|reason| invalid(key, reason))
    }
}
