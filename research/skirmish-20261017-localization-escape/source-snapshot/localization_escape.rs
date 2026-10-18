//!494000 DisplayString escapes over retained fresh AVars and immutable Glyph
//!source projections. This is NOT a second native Glyph manager: every reached
//!lookup uses the actual ActorCatalog manager and verifies canonical provenance
//!and the actual typed index getter before a projected character can be used.
use crate::load_view_variables::NativeOverrides;
use aa_numeric::Precision;
use aa_rules::{
    actors::ActorCatalog,
    native::TypedValue,
    native::index::{native_expand_index, native_resource_matches},
    resources::Provenance,
    sources::V14Sources,
    tgi::{Document, Item, Limits},
};
use std::collections::BTreeMap;
type Result<T> = std::result::Result<T, String>;
#[derive(Debug)]
struct Glyph {
    source: Provenance,
    index: u16,
}
#[derive(Debug)]
pub(crate) struct Table {
    glyphs: BTreeMap<String, Glyph>,
    start: u16,
    end: u32,
    single: u16,
    variables: Vec<Provenance>,
    files: Vec<(String, String)>,
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn integer(raw: &str) -> Result<u32> {
    let raw = raw.trim();
    if raw.is_empty()
        || !raw
            .bytes()
            .enumerate()
            .all(|(i, c)| c.is_ascii_digit() || i == 0 && matches!(c, b'+' | b'-'))
    {
        return Err("escape source requires an explicit decimal integer getter".into());
    }
    raw.parse::<i32>().map(|v| v as u32).map_err(err)
}
impl Table {
    pub(crate) fn load(sources: &mut V14Sources, overrides: NativeOverrides) -> Result<Self> {
        if !matches!(overrides, NativeOverrides::ControlledNone) {
            return Err(
                "font escape AVars require the actual profile/console override owner".into(),
            );
        }
        let paths = sources
            .catalog()
            .into_iter()
            .map(|v| v.logical_path)
            .collect::<Vec<_>>();
        let chain = native_expand_index(&paths, &mut |path| {
            sources.read(path).map(|v| v.bytes).map_err(err)
        })?;
        //401BD0 registration defaults; source assignments below replace the same
        //retained current variables in native index order. In the shipped source,
        //TextOneCharEscapeCharacter has no assignment and retains compiled0x5C.
        let (mut start, mut end, mut single) = (38u32, 46u32, 92u32);
        let mut glyphs = BTreeMap::new();
        let mut variables = Vec::new();
        let mut files = Vec::new();
        let pattern = |p: &str, s: &str| {
            native_resource_matches(
                &p.encode_utf16().collect::<Vec<_>>(),
                &s.encode_utf16().collect::<Vec<_>>(),
            )
        };
        for file in chain.files {
            let vars = pattern("AVars*.tgi", &file.path);
            let font = pattern("Fonts/special_glyphs*.tgi", &file.path);
            if !vars && !font {
                continue;
            }
            let file = sources.read(&file.path).map_err(err)?;
            if aa_rules::sources::sha256(&file.bytes) != file.sha256 {
                return Err("font escape source bytes/hash differ".into());
            }
            files.push((file.logical_path.clone(), file.sha256.clone()));
            let doc = Document::parse_native(file.bytes, Limits::default()).map_err(err)?;
            for item in &doc.items {
                let Item::Node(node) = item else {
                    continue;
                };
                if vars
                    && doc
                        .header_name(&node.header)
                        .is_some_and(|v| v.eq_ignore_ascii_case("Vars"))
                {
                    for item in &node.items {
                        let Item::Property(p) = item else {
                            continue;
                        };
                        let Some(name) = doc.property_name(p) else {
                            continue;
                        };
                        let target = if name.eq_ignore_ascii_case("TextEscapeCharacterStart") {
                            &mut start
                        } else if name.eq_ignore_ascii_case("TextEscapeCharacterEnd") {
                            &mut end
                        } else if name.eq_ignore_ascii_case("TextOneCharEscapeCharacter") {
                            &mut single
                        } else {
                            continue;
                        };
                        if p.left.len() != 2
                            || !doc
                                .text(p.left[0].span)
                                .is_some_and(|v| v.eq_ignore_ascii_case("int"))
                        {
                            return Err(
                                "font escape AVar requires its actual integer declaration".into()
                            );
                        }
                        *target = integer(
                            doc.property_value(p)
                                .ok_or("font escape AVar assignment absent")?,
                        )?;
                        variables.push(Provenance {
                            path: file.logical_path.clone(),
                            sha256: file.sha256.clone(),
                            span: p.span,
                        });
                    }
                }
                if font
                    && doc
                        .header_name(&node.header)
                        .is_some_and(|v| v.eq_ignore_ascii_case("Glyph"))
                {
                    let header = node
                        .header
                        .tokens
                        .iter()
                        .map(|v| doc.text(v.span).unwrap_or("").to_ascii_lowercase())
                        .collect::<Vec<_>>();
                    if header != ["glyph", "template", "=", "glyph"] {
                        return Err("font escape projection requires the current explicit Glyph template root".into());
                    }
                    let (mut id, mut index) = (None, None);
                    for item in &node.items {
                        let Item::Property(p) = item else {
                            return Err(
                                "nested Glyph source is outside the current projection".into()
                            );
                        };
                        let Some(name) = doc.property_name(p) else {
                            continue;
                        };
                        if name.eq_ignore_ascii_case("ids") {
                            let raw = doc.property_value(p).ok_or("Glyph IDS absent")?.trim();
                            let raw = raw
                                .strip_prefix('"')
                                .and_then(|v| v.strip_suffix('"'))
                                .unwrap_or(raw);
                            if raw.is_empty()
                                || !raw.is_ascii()
                                || raw.contains(['\\', '&', '\0', '"'])
                                || id.is_some()
                            {
                                return Err("Glyph IDS needs one explicit ASCII assignment".into());
                            }
                            id = Some(raw.to_ascii_lowercase());
                        } else if name.eq_ignore_ascii_case("index") {
                            if index.is_some() {
                                return Err(
                                    "Glyph index reload is outside the current projection".into()
                                );
                            }
                            index =
                                Some(integer(doc.property_value(p).ok_or("Glyph index absent")?)?
                                    as u16);
                        }
                    }
                    let id = id.ok_or("Glyph explicit IDS missing")?;
                    let index = index.ok_or("Glyph explicit index missing")?;
                    if glyphs
                        .insert(
                            id,
                            Glyph {
                                source: Provenance {
                                    path: file.logical_path.clone(),
                                    sha256: file.sha256.clone(),
                                    span: node.span,
                                },
                                index,
                            },
                        )
                        .is_some()
                    {
                        return Err(
                            "duplicate Glyph source requires its actual manager reload owner"
                                .into(),
                        );
                    }
                }
            }
        }
        Ok(Self {
            glyphs,
            start: start as u16,
            end,
            single: single as u16,
            variables,
            files,
        })
    }
    pub(crate) fn requires_decode(&self, input: &[u16]) -> bool {
        input.iter().any(|&c| c == self.start || c == self.single)
    }
    pub(crate) fn decode_bound(
        &self,
        actors: &ActorCatalog,
        pc: Precision,
        input: &mut Vec<u16>,
    ) -> Result<()> {
        if self.files.iter().any(|(path, hash)| {
            !actors
                .sources
                .iter()
                .any(|v| v.path == *path && v.sha256 == *hash)
        }) {
            return Err(
                "font escape retained AVar/Glyph source universe differs from Catalog".into(),
            );
        }
        let registry = actors
            .native_data_registry("Glyph")
            .ok_or("actual Glyph manager is unavailable")?;
        let mut world = Bound {
            table: self,
            actors,
            pc,
        };
        decode(
            input,
            self.start,
            self.end,
            self.single,
            !registry.entries.is_empty(),
            &mut |name| world.find(name),
        )
    }
}
struct Bound<'a> {
    table: &'a Table,
    actors: &'a ActorCatalog,
    pc: Precision,
}
impl Bound<'_> {
    fn find(&mut self, name: &[u16]) -> Result<Option<u16>> {
        let end = name.iter().position(|&c| c == 0).unwrap_or(name.len());
        let name = String::from_utf16(&name[..end]).map_err(err)?;
        if !name.is_ascii() {
            return Err("Glyph lookup case folding outside native ASCII scope".into());
        }
        let registry = self
            .actors
            .native_data_registry("Glyph")
            .ok_or("actual Glyph manager absent")?;
        let Some(entry) = registry.find(&name) else {
            return Ok(None);
        };
        let raw = self
            .table
            .glyphs
            .get(&entry.ids.to_ascii_lowercase())
            .ok_or("reached Glyph has no source projection")?;
        if raw.source != entry.source || !entry.reloads.is_empty() {
            return Err("reached Glyph canonical registration/source differs".into());
        }
        let node = self.actors.native_data_node(entry).map_err(err)?;
        let Some(TypedValue::Integer { value, .. }) = node.field("index", self.pc).map_err(err)?
        else {
            return Err("Glyph493500 index requires its actual typed integer getter".into());
        };
        if value as u16 != raw.index {
            return Err("Glyph typed index differs from retained source projection".into());
        }
        Ok(Some(raw.index))
    }
}
///494000 copies only the replacement at the escape position, then erases its
///token. Search resumes AFTER that replacement, so an inserted '&' is not
///recursively interpreted. A required diagnostic preserves prior replacements.
fn decode(
    input: &mut Vec<u16>,
    start: u16,
    end: u32,
    single: u16,
    present: bool,
    find: &mut impl FnMut(&[u16]) -> Result<Option<u16>>,
) -> Result<()> {
    if !present {
        return Ok(());
    }
    let mut cursor = 0;
    loop {
        let Some(relative) = input[cursor..]
            .iter()
            .take_while(|&&c| c != 0)
            .position(|&c| c == start || c == single)
        else {
            return Ok(());
        };
        let at = cursor + relative;
        if at + 1 == input.len() {
            return Err("494000 required dangling-escape diagnostic410C40".into());
        }
        let short = input[at] == single || input[at + 1] == start;
        let begin = at + 1;
        let length = if short {
            1
        } else {
            let Some(n) = input[begin..].iter().position(|&c| u32::from(c) == end) else {
                return Err("494000 required unterminated-escape diagnostic410C40".into());
            };
            n
        };
        let key = &input[begin..begin + length];
        let value = match find(key)? {
            Some(value) => value,
            None => {
                if short && key.iter().any(|&c| c == start || c == single) {
                    key[0]
                } else {
                    return Err("494000 required unrecognized-escape diagnostic410C40".into());
                }
            }
        };
        input[at] = value;
        let remove = length + usize::from(!short);
        input.drain(begin..begin + remove);
        cursor = begin;
    }
}
#[cfg(test)]
#[path = "localization_escape_tests.rs"]
mod tests;
