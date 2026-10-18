//! Actual map TextData field-hash traversal and head-first entry order.
//!5B3995 starts5 buckets;496F60/491F80 insert/rehash;4BA710 walks the final
//!field map and4BA1E0 prepends each text entry. Values stay in actual live_text.
//!4738C0 formatting is representation-compatible, native-parser verified for
//!the admitted plain no-BOM C-locale DisplayString domain, not byte-identical.
use crate::localization::Localization;
use aa_mapworld::{Chunk, SourceIdentity};
use aa_rules::{
    resources::Provenance,
    tgi::{Document, Item, Limits},
};
use std::collections::BTreeSet;

type Result<T> = std::result::Result<T, String>;
struct Entry {
    key: String,
    source: Provenance,
}
/// Immutable order/provenance only; no cloned live text value is stored.
pub struct MapTextOrder {
    source: SourceIdentity,
    entries: Vec<Entry>,
}
fn hash(key: &str) -> u32 {
    key.bytes().fold(0u32, |h, c| {
        h.wrapping_mul(33)
            .wrapping_add(c.to_ascii_uppercase() as u32)
    })
}

fn native_order(names: &[String]) -> Result<Vec<usize>> {
    let mut buckets = vec![Vec::<usize>::new(); 5];
    for (ordinal, name) in names.iter().enumerate() {
        if ordinal
            >= buckets
                .len()
                .checked_mul(2)
                .ok_or("text field bucket count overflow")?
        {
            let twice = buckets
                .len()
                .checked_mul(2)
                .ok_or("text field bucket count overflow")?;
            let mut next = twice
                .checked_add(1)
                .ok_or("text field bucket count overflow")?;
            if next % 3 == 0 || next % 5 == 0 {
                next = twice
                    .checked_add(3)
                    .ok_or("text field bucket count overflow")?;
                if next % 3 == 0 || next % 5 == 0 {
                    next = twice
                        .checked_add(5)
                        .ok_or("text field bucket count overflow")?;
                }
            }
            let mut grown = Vec::new();
            grown.try_reserve(next).map_err(|e| e.to_string())?;
            grown.resize_with(next, Vec::new);
            for chain in &buckets {
                for &old in chain {
                    let slot = hash(&names[old]) as usize % next;
                    grown[slot].insert(0, old);
                }
            }
            buckets = grown;
        }
        let slot = hash(name) as usize % buckets.len();
        buckets[slot].insert(0, ordinal);
    }
    //Native4BA1E0 prepends each item produced by the ascending bucket walk.
    Ok(buckets.into_iter().flatten().rev().collect())
}
fn key_admitted(key: &str) -> bool {
    !key.is_empty() && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}
impl MapTextOrder {
    pub fn bind(chunk: &Chunk, live_text: &Localization) -> Result<Self> {
        chunk.verify().map_err(|e| e.to_string())?;
        if chunk.directory.name != "MAPSTRINGS" {
            return Err("film map text needs actualMAPSTRINGS chunk".into());
        }
        let doc = Document::parse_native(chunk.bytes.clone(), Limits::default())
            .map_err(|e| e.to_string())?;
        let nodes: Vec<_> = doc
            .items
            .iter()
            .filter_map(|item| match item {
                Item::Node(n) => Some(n),
                _ => None,
            })
            .collect();
        let [node] = nodes.as_slice() else {
            return Err("film map text requires one actual DefaultText node".into());
        };
        let h: Vec<_> = node
            .header
            .tokens
            .iter()
            .map(|t| doc.text(t.span))
            .collect();
        if h.len() != 4
            || !h[0].is_some_and(|v| v.eq_ignore_ascii_case("Text"))
            || !h[1].is_some_and(|v| v.eq_ignore_ascii_case("language"))
            || h[2] != Some("=")
            || !h[3].is_some_and(|v| v.eq_ignore_ascii_case("Default"))
        {
            return Err("film map text language selection requires its native producer".into());
        }
        let mut names = Vec::new();
        let mut sources = Vec::new();
        let mut unique = BTreeSet::new();
        for item in &node.items {
            let Item::Property(p) = item else {
                return Err("film map text nested fields are unsupported".into());
            };
            let key = doc
                .property_name(p)
                .ok_or("film map text field name is unbound")?
                .to_owned();
            if !key_admitted(&key) || !unique.insert(key.to_ascii_lowercase()) {
                return Err(
                    "film map text duplicate/non-ASCII field requires native error handling".into(),
                );
            }
            names.push(key);
            sources.push(Provenance {
                path: format!("MAPSTRINGS/{}", chunk.payload_source.sha256),
                sha256: chunk.payload_source.sha256.clone(),
                span: p.span,
            });
        }
        let entries = native_order(&names)?
            .into_iter()
            .filter(|&i| {
                !names[i].eq_ignore_ascii_case("ids") && !names[i].eq_ignore_ascii_case("name")
            })
            .map(|i| Entry {
                key: names[i].clone(),
                source: sources[i].clone(),
            })
            .collect();
        let value = Self {
            source: chunk.payload_source.clone(),
            entries,
        };
        value.validate(live_text)?;
        Ok(value)
    }
    pub fn source(&self) -> &SourceIdentity {
        &self.source
    }
    pub fn entry_names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.key.as_str())
    }
    fn validate(&self, current: &Localization) -> Result<()> {
        for entry in &self.entries {
            let rows = current
                .declarations
                .get(&entry.key.to_ascii_lowercase())
                .ok_or("current film map text entry disappeared")?;
            if rows.iter().filter(|row| row.source == entry.source).count() != 1 {
                return Err(
                    "current film map text entry/source allocation projection differs".into(),
                );
            }
        }
        let actual = current
            .declarations
            .iter()
            .flat_map(|(_, rows)| rows)
            .filter(|row| row.source.path == format!("MAPSTRINGS/{}", self.source.sha256))
            .count();
        if actual != self.entries.len() {
            return Err(
                "current map TextData entry set changed without native order publication".into(),
            );
        }
        Ok(())
    }
    /// Borrow the SAME actual Staged.live_text. Missing entry creation is not
    /// implemented by this immutable order owner; it must prepend a new entry
    /// through its proper native producer before this method can accept it.
    pub fn serialize_current(&self, current: &Localization) -> Result<Vec<u8>> {
        self.validate(current)?;
        let name = current
            .display(&"map_name".encode_utf16().collect::<Vec<_>>())
            .map_err(|e| e.to_string())?;
        let mut rows = Vec::new();
        for entry in &self.entries {
            let row = current.declarations[&entry.key.to_ascii_lowercase()]
                .iter()
                .find(|row| row.source == entry.source)
                .ok_or("current map text entry is absent")?;
            rows.push((
                entry.key.as_str(),
                row.plain_utf16.as_ref().map_err(Clone::clone)?.as_slice(),
            ));
        }
        encode(&name, &rows)
    }
    ///Logical formatting input for4738C0's actual TextStream856BF0. The
    ///stream itself expands LF to CRLF; this method does not pre-apply that
    ///encoding. Admitted text values exclude CR/LF, so removing the known
    ///formatter's CR bytes changes no current text value or field order.
    pub fn serialize_current_logical(&self, current: &Localization) -> Result<Vec<u8>> {
        Ok(self
            .serialize_current(current)?
            .into_iter()
            .filter(|b| *b != b'\r')
            .collect())
    }
}

fn literal(value: &[u16]) -> Result<Vec<u8>> {
    if value
        .iter()
        .any(|&u| u < 32 || u == 127 || u > 255 || matches!(u, 34 | 38 | 92))
    {
        return Err("film DisplayString requires unported native escaping/Unicode output".into());
    }
    let mut bytes = Vec::new();
    //Native494000/5BA3C0 removes one leading@. Preserve actual current text.
    if value.first() == Some(&64) {
        bytes.push(b'@')
    }
    bytes.extend(value.iter().map(|u| *u as u8));
    Ok(bytes)
}
///4738C0-compatible TGI representation in caller-supplied native head order.
/// Formatting bytes differ; native loader/49AD70 equivalence is the contract.
pub fn encode(name: &[u16], entries: &[(&str, &[u16])]) -> Result<Vec<u8>> {
    let mut output =
        b"[Text language=Default]\r\n{\r\n\tids = map_strings_table\r\n\tname = \"".to_vec();
    output.extend(literal(name)?);
    output.extend(b"\"\r\n\r\n");
    let mut names = BTreeSet::new();
    for (key, text) in entries {
        if !key_admitted(key)
            || key.eq_ignore_ascii_case("ids")
            || key.eq_ignore_ascii_case("name")
            || !names.insert(key.to_ascii_lowercase())
        {
            return Err("film map text key needs native identifier/duplicate handling".into());
        }
        output.push(b'\t');
        output.extend(key.as_bytes());
        output.extend(b" = \"");
        output.extend(literal(text)?);
        output.extend(b"\"\r\n");
    }
    output.extend(b"}\r\n");
    Ok(output)
}
