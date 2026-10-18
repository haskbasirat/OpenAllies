//! ObjectData49C760/49DC40/49D750 BaseTerrain fields used by4EA260.
//! Values resolve against the same moved Bank and canonical native registries.
//! A missing BaseTerrain keeps constructor count0; a present all-null table is
//! distinct and still requires the actual retained Biome index at lookup.
use crate::{catalog::Catalog, definitions::Bank};
use aa_mapworld::restore::RuntimeDefinition;
use aa_rules::{actors::ResolvedNode, native::TypedValue, resources::Provenance};
type Result<T> = std::result::Result<T, String>;
fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn string(node: &ResolvedNode<'_>, name: &str, pc: aa_numeric::Precision) -> Result<String> {
    match node
        .field(name, pc)
        .map_err(text)?
        .ok_or_else(|| format!("49DC40 missing typed {name}"))?
    {
        TypedValue::String { value, .. } => Ok(value),
        _ => Err(format!("49DC40 {name} requires CString source")),
    }
}
#[derive(Debug)]
pub(crate) struct Source {
    padding: u32,
    variations: Vec<Option<usize>>,
    connecting: Vec<Option<usize>>,
    provenance: Vec<Provenance>,
}
impl Source {
    pub(crate) fn bind(
        catalog: &Catalog<'_>,
        bank: &Bank,
        definition: &RuntimeDefinition,
    ) -> Result<Self> {
        crate::live_object_init::Source::bind_in_bank(catalog, bank, definition).map_err(text)?;
        let pc = bank.combat_source()?.source_precision();
        // Reusing an existing ObjectData is not equivalent to a fresh zero
        // count. Refuse a source reload whose old table is not owned here.
        let mut declarations = 0;
        for tag in aa_rules::actors::native_load::THING_TAGS {
            for node in catalog.actors.resolved_definitions(tag).map_err(text)? {
                if let Some(TypedValue::String { value, .. }) =
                    node.field("IDS", pc).map_err(text)?
                    && value.eq_ignore_ascii_case(&definition.proof.id)
                {
                    declarations += 1;
                }
            }
        }
        if declarations != 1 {
            return Err("49DC40 BaseTerrain requires one native ObjectData declaration".into());
        }
        let node = catalog
            .actors
            .resolved_actor(&definition.proof.id)
            .map_err(text)?;
        let mut result = Self {
            padding: 0,
            variations: vec![],
            connecting: vec![],
            provenance: node.sources(),
        };
        let Some(base) = node.child("BaseTerrain").map_err(text)? else {
            return Ok(result);
        };
        let biomes = catalog
            .actors
            .native_data_registry("Biome")
            .ok_or("49DC40 native Biome registry absent")?;
        if biomes.entries.is_empty() {
            return Err("49DC40 BaseTerrain no biomes loaded diagnostic required".into());
        }
        result.padding = base
            .field("padding", pc)
            .map_err(text)?
            .ok_or("49DC40 padding source absent")?
            .fixed_bits()
            .map_err(text)?;
        let resolve = |id: &str| -> Result<Option<usize>> {
            if id.is_empty() {
                return Ok(None);
            }
            let entry = catalog
                .actors
                .native_data_find("Variation", id)
                .ok_or_else(|| format!("49DC40 unknown Variation {id} diagnostic required"))?;
            let registry = catalog
                .actors
                .native_data_registry("Variation")
                .ok_or("49DC40 Variation registry absent")?;
            if !entry.reloads.is_empty() {
                return Err("49DC40 Variation reload ownership required".into());
            }
            registry
                .entries
                .iter()
                .position(|e| e.path == entry.path && e.source == entry.source)
                .map(Some)
                .ok_or_else(|| "49DC40 canonical Variation ordinal absent".into())
        };
        let default = resolve(&string(&base, "default_variation_IDS", pc)?)?;
        let connecting = resolve(&string(&base, "default_connecting_variation_IDS", pc)?)?;
        result.variations = vec![default; biomes.entries.len()];
        result.connecting = vec![connecting; biomes.entries.len()];
        // Native49D750 processes inherited list items in their source order;
        // repeated Biome entries overwrite the same retained table cell.
        for item in base.children("BiomeBaseTerrain").map_err(text)? {
            let id = string(&item, "biome_IDS", pc)?;
            let biome = catalog
                .actors
                .native_data_find("Biome", &id)
                .ok_or_else(|| format!("49D750 unknown Biome {id} diagnostic required"))?;
            let ordinal = biomes
                .entries
                .iter()
                .position(|e| e.path == biome.path && e.source == biome.source)
                .ok_or("49D750 Biome ordinal absent")?;
            let variation = resolve(&string(&item, "variation_IDS", pc)?)?
                .ok_or("49D750 required Variation resolved null")?;
            // The native child validates that the Variation's Terrain is one
            // of this Biome's actual Terrain pointers before storing the cell.
            Self::belongs(catalog, biome, variation, pc)?;
            result.variations[ordinal] = Some(variation);
            let connecting = string(&item, "connecting_variation_IDS", pc)?;
            if !connecting.is_empty() {
                let variation =
                    resolve(&connecting)?.ok_or("49D750 connecting Variation resolved null")?;
                Self::belongs(catalog, biome, variation, pc)?;
                result.connecting[ordinal] = Some(variation);
            }
            result.provenance.extend(item.sources());
        }
        //49EFBC validates defaults and overrides only for cells whose normal
        //Variation is nonnull; a connecting-only null-normal cell is skipped.
        for (ordinal, variation) in result.variations.iter().enumerate() {
            if let Some(index) = variation {
                Self::belongs(catalog, &biomes.entries[ordinal], *index, pc)?;
                if let Some(index) = result.connecting[ordinal] {
                    Self::belongs(catalog, &biomes.entries[ordinal], index, pc)?;
                }
            }
        }
        result.provenance.extend(base.sources());
        Ok(result)
    }
    fn belongs(
        catalog: &Catalog<'_>,
        biome: &aa_rules::actors::native_load::NativeDataEntry,
        index: usize,
        pc: aa_numeric::Precision,
    ) -> Result<()> {
        let variations = catalog
            .actors
            .native_data_registry("Variation")
            .ok_or("49D750 Variation registry absent")?;
        let variation = variations
            .entries
            .get(index)
            .ok_or("49D750 Variation ordinal outside native registry")?;
        let [root, _] = variation.path.as_slice() else {
            return Err("49D750 Variation has no actual Terrain parent".into());
        };
        let terrains = catalog
            .actors
            .native_data_registry("Terrain")
            .ok_or("49D750 Terrain registry absent")?;
        let node = catalog.actors.native_data_node(biome).map_err(text)?;
        for item in node.children("Terrain").map_err(text)? {
            let id = string(&item, "terrain_IDS", pc)?;
            let terrain = catalog
                .actors
                .native_data_find("Terrain", &id)
                .ok_or("49D750 Biome Terrain reference absent")?;
            if terrain.path.as_slice() == [*root]
                && terrains
                    .entries
                    .iter()
                    .any(|e| e.path == terrain.path && e.source == terrain.source)
            {
                return Ok(());
            }
        }
        Err("49D750 Variation does not belong to actual Biome diagnostic required".into())
    }
    pub(crate) fn variation(&self, biome_ordinal: usize) -> Result<Option<usize>> {
        if self.variations.is_empty() {
            return Ok(None);
        }
        self.variations
            .get(biome_ordinal)
            .copied()
            .ok_or_else(|| "4EA260 current Biome outside ObjectData table".into())
    }
    pub(crate) fn padding(&self) -> u32 {
        self.padding
    }
}
