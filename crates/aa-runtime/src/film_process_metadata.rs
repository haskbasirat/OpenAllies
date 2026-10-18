//! Application-owned metadata read by fresh Header reset5A14C0.
//!419DFE/419E11 copy current AVars;419E62 folds Version;419E9A combines
//!SKU with the compiled suffix initialized by9C7AC0. Construct once at the
//!explicit fresh application boundary, then lend the retained values.
use crate::{catalog::Catalog, load_view_variables::NativeOverrides};
use aa_rules::{
    native::index::native_enumerate,
    resources::Provenance,
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};
type Result<T> = std::result::Result<T, String>;

/// No raw/default/Clone constructor: these are the same copied application
/// globals, separate from both source declarations and per-header fields.
pub struct State {
    version: Vec<u16>,
    application: Vec<u16>,
    release: Vec<u16>,
    numeric_version: u32,
    source_files: Vec<(String, String)>,
    header_count: HeaderConstructorCounter,
}
///Only the fresh application metadata constructor mints this counter. A
///worker may retain another capability to the SAME global word; no rawcount
///constructor or reset is available. It does not certify any map/activation.
#[derive(Clone)]
pub struct HeaderConstructorCounter {
    word: Arc<AtomicU32>,
}
impl HeaderConstructorCounter {
    fn fresh_application() -> Self {
        Self {
            word: Arc::new(AtomicU32::new(0)),
        }
    }
    pub fn value(&self) -> u32 {
        self.word.load(Ordering::SeqCst)
    }
    pub fn same_owner(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.word, &other.word)
    }
    pub(crate) fn header_reset_5a14c0(&self) {
        self.word.fetch_add(1, Ordering::SeqCst);
    }
    #[cfg(test)]
    fn laboratory_value(value: u32) -> Self {
        Self {
            word: Arc::new(AtomicU32::new(value)),
        }
    }
}
struct Declaration {
    kind: String,
    raw: String,
    source: Provenance,
}
fn literal(raw: &str) -> Result<String> {
    if !raw.is_ascii() || raw.contains(['\0', '\\']) {
        return Err("application metadata requires proved plain ASCII source text".into());
    }
    if let Some(tail) = raw.strip_prefix('"') {
        let end = tail
            .find('"')
            .ok_or("application metadata has an unclosed quote")?;
        if !tail[end + 1..].trim().is_empty() {
            return Err("application metadata source expression is unbound".into());
        }
        Ok(tail[..end].into())
    } else if raw.contains('"') {
        Err("application metadata source quoting is unbound".into())
    } else {
        Ok(raw.into())
    }
}
/// Closed four-component decimal source domain. Native410D80 uses swscanf;
/// partial conversions, signs and out-of-range components remain unported.
fn version(raw: &str) -> Result<u32> {
    let parts = raw.split('.').collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err("Version needs the proven four-component source domain".into());
    }
    let mut result = 0u32;
    for part in parts {
        if part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()) {
            return Err("Version needs the proven decimal source domain".into());
        }
        let value = part.parse::<u32>().map_err(|e| e.to_string())?;
        if value > 99 {
            return Err("Version outside the proven component range".into());
        }
        result = result.wrapping_mul(100).wrapping_add(value);
    }
    Ok(result)
}
impl State {
    pub fn from_fresh_sources(
        catalog: &Catalog<'_>,
        sources: &mut V14Sources,
        overrides: NativeOverrides,
    ) -> Result<Self> {
        catalog.require_source_owner()?;
        if !matches!(overrides, NativeOverrides::ControlledNone) {
            return Err(
                "application metadata requires actual profile/console override ownership".into(),
            );
        }
        let paths = sources
            .catalog()
            .into_iter()
            .map(|r| r.logical_path)
            .collect::<Vec<_>>();
        let mut rows = BTreeMap::<String, Vec<Declaration>>::new();
        let mut source_files = Vec::new();
        for path in native_enumerate("AVars*.tgi", &paths) {
            let file = sources.read(&path).map_err(|e| e.to_string())?;
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err("metadata source hash differs".into());
            }
            let document =
                Document::parse_native(file.bytes, Limits::default()).map_err(|e| e.to_string())?;
            source_files.push((file.logical_path.clone(), file.sha256.clone()));
            for item in &document.items {
                let Item::Node(node) = item else { continue };
                if !document
                    .header_name(&node.header)
                    .is_some_and(|s| s.eq_ignore_ascii_case("Vars"))
                {
                    continue;
                }
                for item in &node.items {
                    let Item::Property(p) = item else { continue };
                    let Some(name) = document.property_name(p) else {
                        continue;
                    };
                    if !["Version", "ApplicationIDS", "SKU"]
                        .iter()
                        .any(|s| s.eq_ignore_ascii_case(name))
                    {
                        continue;
                    }
                    if p.left.len() != 2 {
                        return Err("metadata assignment override ordering is unbound".into());
                    }
                    rows.entry(name.to_ascii_lowercase())
                        .or_default()
                        .push(Declaration {
                            kind: document
                                .text(p.left[0].span)
                                .ok_or("metadata type text missing")?
                                .into(),
                            raw: document
                                .property_value(p)
                                .ok_or("metadata value text missing")?
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
        let get = |key: &str| -> Result<String> {
            let declarations = rows
                .get(&key.to_ascii_lowercase())
                .ok_or_else(|| format!("metadata {key} absent"))?;
            let [row] = declarations.as_slice() else {
                return Err(format!("metadata {key} reload/override is unbound"));
            };
            if !row.kind.eq_ignore_ascii_case("string") {
                return Err(format!("metadata {key} has wrong source type"));
            }
            if !source_files
                .iter()
                .any(|(p, h)| p == &row.source.path && h == &row.source.sha256)
            {
                return Err("metadata declaration belongs to another source".into());
            }
            literal(&row.raw)
        };
        let version_text = get("Version")?;
        let numeric_version = version(&version_text)?;
        let application = get("ApplicationIDS")?;
        let release = format!("{} AA", get("SKU")?);
        Ok(Self {
            version: version_text.encode_utf16().collect(),
            application: application.encode_utf16().collect(),
            release: release.encode_utf16().collect(),
            numeric_version,
            source_files,
            header_count: HeaderConstructorCounter::fresh_application(),
        })
    }
    ///B27A80 is the source executable's initialized binary32 build number,
    ///not a wall-clock timestamp. Both original/v14 image bytes are pinned.
    pub fn build_bits(&self) -> u32 {
        0x4581_0D9A
    }
    pub fn numeric_version(&self) -> u32 {
        self.numeric_version
    }
    pub fn version_text(&self) -> &[u16] {
        &self.version
    }
    pub fn application(&self) -> &[u16] {
        &self.application
    }
    pub fn release(&self) -> &[u16] {
        &self.release
    }
    pub fn source_files(&self) -> &[(String, String)] {
        &self.source_files
    }
    ///Only the actual Header constructor/reset may perform this process
    ///B3D8CC increment. Merely decoding a saved header never calls it.
    pub(crate) fn header_reset_5a14c0(&mut self) {
        self.header_count.header_reset_5a14c0()
    }
    pub fn constructed_headers(&self) -> u32 {
        self.header_count.value()
    }
    pub fn constructor_counter(&self) -> HeaderConstructorCounter {
        self.header_count.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn header_constructor_capability_keeps_one_wrapping_application_word() {
        let actual = HeaderConstructorCounter::laboratory_value(u32::MAX);
        let source = actual.clone();
        let target = actual.clone();
        assert!(source.same_owner(&target));
        std::thread::spawn(move || source.header_reset_5a14c0())
            .join()
            .unwrap();
        assert_eq!(target.value(), 0);
        target.header_reset_5a14c0();
        assert_eq!(actual.value(), 1);
        let other = HeaderConstructorCounter::fresh_application();
        assert!(!other.same_owner(&actual));
        assert_eq!(other.value(), 0);
    }
    #[test]
    fn actual_native_metadata_prefix_and_version_fold() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/development10/native-film-metadata.json"
        ))
        .unwrap();
        assert_eq!(fixture["executions"], 60);
        let zero = &fixture["header_count_zero_tail"];
        assert!(zero["raw_size"].as_u64().unwrap() <= zero["word_offset"].as_u64().unwrap());
        for row in fixture["rows"].as_array().unwrap() {
            let input = &row["input"];
            let output = &row["output"];
            match input["kind"].as_str().unwrap() {
                "version" => {
                    let text = input["components"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap().to_string())
                        .collect::<Vec<_>>()
                        .join(".");
                    assert_eq!(u64::from(version(&text).unwrap()), output["numeric"]);
                }
                "static" => {
                    assert_eq!(output["suffix"], " AA");
                    assert_eq!(output["build_bits"], 0x4581_0d9au32);
                }
                "header_prefix" => {
                    let mut state = State {
                        version: input["version"].as_str().unwrap().encode_utf16().collect(),
                        application: input["application"]
                            .as_str()
                            .unwrap()
                            .encode_utf16()
                            .collect(),
                        release: input["release"].as_str().unwrap().encode_utf16().collect(),
                        numeric_version: input["numeric"].as_u64().unwrap() as u32,
                        source_files: Vec::new(),
                        header_count: HeaderConstructorCounter::laboratory_value(
                            input["count"].as_u64().unwrap() as u32,
                        ),
                    };
                    state.header_reset_5a14c0();
                    assert_eq!(u64::from(state.constructed_headers()), output["count"]);
                    assert_eq!(u64::from(state.build_bits()), output["words"]["80"]);
                    assert_eq!(u64::from(state.numeric_version()), output["words"]["84"]);
                    assert_eq!(
                        String::from_utf16(state.version_text()).unwrap(),
                        output["version"]
                    );
                    assert_eq!(
                        String::from_utf16(state.application()).unwrap(),
                        output["application"]
                    );
                    assert_eq!(
                        String::from_utf16(state.release()).unwrap(),
                        output["release"]
                    );
                }
                _ => unreachable!(),
            }
        }
    }
}
