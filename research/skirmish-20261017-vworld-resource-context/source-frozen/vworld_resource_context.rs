//!43F0EE/5AE0A0: the current Biome texture subfolder in the SAME application's
//!ResourceManager+80. The closed Map/Film namespace is nonempty-capable; both
//!actual class chains exclude KKC_NiAVObjResource. A future GPU resource owner
//!must extend the namespace and provide its real5ABFF0 unload before admission.
use crate::{catalog::Catalog, definitions::Bank, resource_creator::Creator};
use aa_numeric::Precision;
use aa_rules::{
    actors::native_load::NativeDataEntry, native::TypedValue, resources::SourceIdentity,
};

pub(crate) type Result<T> = std::result::Result<T, String>;

///Diagnostic result of this one operation, not an admission/readiness token.
#[derive(Debug, PartialEq, Eq)]
pub struct Update {
    pub changed: bool,
    pub maps_examined: usize,
    pub films_examined: usize,
}

///The immutable Biome pointer retained by GMap+24 at World::Init. Capture
///BEFORE MapResource+88 selection can replace its Creator. This is source
///ownership, not a view/world readiness certificate; it stores no+98 value.
pub struct Biome {
    entry: NativeDataEntry,
    sources: Vec<SourceIdentity>,
    source_precision: Precision,
}
impl Biome {
    pub(crate) fn from_world_init_creator(
        catalog: &Catalog<'_>,
        bank: &Bank,
        creator: &Creator,
    ) -> Result<Self> {
        let entry = creator.retained_biome_registration(catalog, bank)?;
        let source = bank.combat_source()?;
        Ok(Self {
            entry: entry.clone(),
            sources: source.source_identities().to_vec(),
            source_precision: source.source_precision(),
        })
    }
    fn node<'a>(
        &self,
        catalog: &'a Catalog<'_>,
        bank: &Bank,
    ) -> Result<aa_rules::actors::ResolvedNode<'a>> {
        catalog.require_source_owner()?;
        let source = bank.combat_source()?;
        if source.source_identities() != self.sources.as_slice()
            || source.source_identities() != catalog.actors.sources.as_slice()
            || source.source_precision() != self.source_precision
        {
            return Err("GMap retained Biome source/Bank precision differs".into());
        }
        let registry = catalog
            .actors
            .native_data_registry("Biome")
            .ok_or("GMap retained Biome registry absent")?;
        let entry = registry
            .entries
            .iter()
            .find(|entry| {
                entry.ids == self.entry.ids
                    && entry.path == self.entry.path
                    && entry.source == self.entry.source
                    && entry.reloads.is_empty()
            })
            .ok_or("GMap retained canonical Biome differs")?;
        catalog
            .actors
            .native_data_node(entry)
            .map_err(|e| e.to_string())
    }
}

pub(crate) struct Input {
    pub(crate) units: Vec<u16>,
}
impl Input {
    pub(crate) fn bind(catalog: &Catalog<'_>, bank: &Bank, biome: &Biome) -> Result<Self> {
        let node = biome.node(catalog, bank)?;
        let precision = bank.combat_source()?.source_precision();
        let value = node
            .field("texture_subfolder_name", precision)
            .map_err(|e| e.to_string())?
            .ok_or("Biome+98 texture_subfolder_name source getter is absent")?;
        let TypedValue::String { value, source, .. } = value else {
            return Err("Biome+98 requires the actual string getter".into());
        };
        if !catalog
            .actors
            .sources
            .iter()
            .any(|entry| entry.sha256 == source.sha256)
        {
            return Err("Biome+98 getter source is outside the actual source owner".into());
        }
        Ok(Self {
            units: value.encode_utf16().collect(),
        })
    }
}

///Private constructor-only owner. Its empty CString is the actual5B04F0
///constructor value; it is never reconstructed from the current Biome.
pub(crate) struct State {
    current: Vec<u16>,
}
fn c_string(units: &[u16]) -> &[u16] {
    &units[..units.iter().position(|&v| v == 0).unwrap_or(units.len())]
}
impl State {
    pub(crate) fn constructed() -> Self {
        Self {
            current: Vec::new(),
        }
    }
    pub(crate) fn current(&self) -> &[u16] {
        &self.current
    }
    ///96CB83 compares UTF16 C-string prefixes case-sensitively.405B80 copies
    ///the complete counted string BEFORE5ACA50; a later service error retains
    ///that assignment. No callback is reached for equal prefixes.
    pub(crate) fn assign_and_invalidate(
        &mut self,
        input: &[u16],
        invalidate: impl FnOnce() -> Result<(usize, usize)>,
    ) -> Result<Update> {
        if c_string(&self.current) == c_string(input) {
            return Ok(Update {
                changed: false,
                maps_examined: 0,
                films_examined: 0,
            });
        }
        self.current = input.to_vec();
        let (maps_examined, films_examined) = invalidate()?;
        Ok(Update {
            changed: true,
            maps_examined,
            films_examined,
        })
    }
}

#[cfg(test)]
#[path = "vworld_resource_context_tests.rs"]
mod tests;
