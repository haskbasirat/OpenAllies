//!49C760 +118 constructor and49DC40 normal-aspect append/default-index stores.
//!This is a bounded immutable scalar projection of fresh registered source. It
//!does not certify model/resource loading or a complete ObjectData LoadData.
use crate::{catalog::Catalog, definitions::Bank, live_object_init::Source};
use aa_actors::actor_creation::{Error, Result};
use aa_mapworld::restore::RuntimeDefinition;
use aa_numeric::{Binary, Precision};
use aa_rules::native::TypedValue;

///Private construction retains the exact canonical source owner and its source
///precision. No caller-supplied index can become a definition field.
#[derive(Debug)]
pub struct DefaultIndex {
    index: u32,
    source: aa_rules::resources::Provenance,
    source_precision: Precision,
}
impl DefaultIndex {
    pub fn bind(
        catalog: &Catalog<'_>,
        bank: &Bank,
        definition: &RuntimeDefinition,
    ) -> Result<Self> {
        let source = Source::bind_in_bank(catalog, bank, definition)?;
        let pc = source.source_precision();
        let creation = catalog
            .actors
            .native_creation_definition(&definition.proof.id, pc)
            .map_err(|e| Error::World(e.to_string()))?;
        if creation.id != definition.proof.id || creation.source != definition.proof.source {
            return Err(Error::World(
                "aspect scalar canonical source differs".into(),
            ));
        }
        //Thing definitions are fresh native registry entries (duplicate Thing
        //IDS is an error), not generic manager reload overlays. The existing
        //source API preserves AspectNormal first when variations are disabled.
        let mut state = Scalar::new();
        state.variations = creation.variations_enabled;
        for (position, aspect) in creation.aspects.iter().enumerate() {
            let TypedValue::String { value, .. } = &aspect.id else {
                return Err(Error::World(
                    "aspect scalar requires actual typed IDS".into(),
                ));
            };
            state
                .append(value, aspect.weight.bits, pc)
                .map_err(Error::World)?;
            if !creation.variations_enabled && position == 0 {
                state.normal_finished().map_err(Error::World)?;
            }
        }
        if !creation.variations_enabled && creation.aspects.is_empty() {
            return Err(Error::World(
                "aspect scalar missing actual AspectNormal append".into(),
            ));
        }
        Ok(Self {
            index: state.default,
            source: creation.source,
            source_precision: pc,
        })
    }
    pub fn index(&self) -> u32 {
        self.index
    }
    pub fn source(&self) -> &aa_rules::resources::Provenance {
        &self.source
    }
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }
}

///Only index/count/IDS/weight scalar effects are projected; no model, native
///pointer, construction-list or loader readiness state is manufactured here.
#[derive(Debug)]
struct Scalar {
    variations: bool,
    default: u32,
    ids: Vec<String>,
    total: u32,
}
impl Scalar {
    fn new() -> Self {
        Self {
            variations: false,
            default: u32::MAX,
            ids: Vec::new(),
            total: 0,
        }
    }
    fn append(&mut self, id: &str, weight: u32, pc: Precision) -> std::result::Result<(), String> {
        if self.ids.iter().any(|v| v == id) {
            return Err("native duplicate aspect IDS".into());
        }
        self.ids.push(id.into());
        self.total = Binary::from_f32_bits(weight)
            .and_then(|a| a.add(&Binary::from_f32_bits(self.total)?, pc))
            .and_then(|v| v.store_f32())
            .map_err(|e| e.to_string())?;
        Ok(())
    }
    fn normal_finished(&mut self) -> std::result::Result<(), String> {
        self.default = u32::try_from(self.ids.len())
            .map_err(|_| "aspect index wire domain exhausted")?
            .wrapping_sub(1);
        Ok(())
    }
}
#[cfg(test)]
#[path = "object_aspect_source_tests.rs"]
mod tests;
