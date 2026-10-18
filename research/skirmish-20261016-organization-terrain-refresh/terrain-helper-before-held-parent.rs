//! Concrete4D4130 terrain transition on the actual held Actor.
//! Terrain identity comes from the loaded Variation/TerrainData owner, not zone
//! emptiness. The actual F8 store precedes Organization/Properties effects.
use crate::{battle_terrain::BattleTerrain, catalog::Catalog, live_object_init::Record};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    components::ComponentSlot,
    overlap_list,
};
use aa_mapworld::restore::{RestoredComponent, RestoredRecord};
use aa_numeric::Precision;
use aa_rules::native::{
    TypedValue,
    layout_data::{NativeLayoutData, PropertyRef, RootPosition},
};
use std::sync::Arc;

#[cfg(test)]
#[path = "load_actor_terrain_refresh_tests.rs"]
mod tests;

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn required(name: &'static str) -> Error {
    Error::Unsupported(name)
}

/// Linked registered Property identity in the SAME source manager Arc. This
/// does not claim that criteria or live instance/effect storage is implemented.
#[derive(Clone)]
pub struct Property {
    manager: Arc<NativeLayoutData>,
    reference: PropertyRef,
    source_precision: Precision,
}
impl Property {
    pub fn manager(&self) -> &Arc<NativeLayoutData> {
        &self.manager
    }
    pub fn reference(&self) -> PropertyRef {
        self.reference
    }
    pub fn source_precision(&self) -> Precision {
        self.source_precision
    }
}
/// Positive native effects remain explicit until their actual owners bind.
pub trait Effects {
    fn organization_refresh(&mut self, _: &mut Record<'_>, _: Precision) -> Result<()> {
        Err(required("579EB0 actual Organization terrain refresh"))
    }
    fn remove_property(&mut self, _: &mut Record<'_>, _: &Property, _: Precision) -> Result<()> {
        Err(required("581810 actual Properties instance owner/removal"))
    }
    fn property_applies(&mut self, _: &mut Record<'_>, _: &Property, _: Precision) -> Result<bool> {
        Err(required("5035E0 actual compiled Property criteria owner"))
    }
    fn add_property(
        &mut self,
        _: &mut Record<'_>,
        _: &Property,
        _: u32,
        _: Precision,
    ) -> Result<()> {
        Err(required(
            "5817A0 actual Properties instance admission/effects",
        ))
    }
    fn null_terrain_diagnostic(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(required(
            "4D4211 native null TerrainData diagnostic and required dereference",
        ))
    }
}
pub struct RequiredEffects;
impl Effects for RequiredEffects {}

fn word(record: &Record<'_>, offset: u32) -> Result<u32> {
    record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("4D4130 Actor"))?
        .words
        .get(&offset)
        .copied()
        .ok_or_else(|| error(format!("4D4130 initialized Actor+{offset:X} absent")))
}
fn property_component(record: &Record<'_>) -> Result<bool> {
    match record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("4D4130 Actor"))?
        .components
        .get(ComponentSlot::Properties)
    {
        None => Ok(false),
        Some(RestoredComponent::Properties(p)) if p.owner.0 == record.owner.raw() => Ok(true),
        _ => Err(required("4D4130 actual Properties payload/owner")),
    }
}
fn organization_component(record: &Record<'_>) -> Result<bool> {
    match record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("4D4130 Actor"))?
        .components
        .get(ComponentSlot::Organization)
    {
        None => Ok(false),
        Some(RestoredComponent::Organization(p)) if p.header.owner == record.owner => Ok(true),
        _ => Err(required("4D4130 actual Organization payload/owner")),
    }
}
fn gates(flags: u32, low: u32, high: u32) -> bool {
    flags & (1 << 29) == 0
        && overlap_list::eligible(low, high)
        && low & (1 << 24) == 0
        && flags & (1 << 12) == 0
}

fn terrain_token(
    catalog: &Catalog<'_>,
    terrain: &BattleTerrain,
    record: &Record<'_>,
    source_pc: Precision,
) -> Result<u32> {
    let definition = terrain
        .terrain_definition_at(record.word(0x44)? as i32, record.word(0x48)? as i32)
        .map_err(error)?;
    let registry = catalog
        .actors
        .native_data_registry("Terrain")
        .ok_or(required("native Terrain manager"))?;
    let mut matches = registry
        .entries
        .iter()
        .filter(|e| e.ids.eq_ignore_ascii_case(&definition.id));
    let entry = matches
        .next()
        .ok_or(required("actual retained TerrainData registration"))?;
    if matches.next().is_some() || !entry.reloads.is_empty() {
        return Err(required(
            "actual unique retained TerrainData without unported reload",
        ));
    }
    let node = catalog.actors.native_data_node(entry).map_err(error)?;
    if node.sources() != definition.sources || terrain.source_precision() != source_pc {
        return Err(required(
            "4D4130 retained TerrainData/source precision differs",
        ));
    }
    catalog
        .definition_bank
        .try_borrow_mut()
        .map_err(error)?
        .intern("TerrainData", &entry.ids)
        .map_err(error)
}
fn checked_terrain_id(catalog: &Catalog<'_>, token: u32) -> Result<String> {
    let bank = catalog.definition_bank.try_borrow().map_err(error)?;
    let entry = bank.require_usable(token).map_err(error)?;
    if entry.key.domain != "terraindata" {
        return Err(required("4D4130 F8 actual TerrainData identity domain"));
    }
    Ok(entry.key.name.clone())
}
fn parent_terrain(record: &Record<'_>, parent: ActorId) -> Result<Option<u32>> {
    let shell = if parent == record.owner {
        &*record.shell
    } else {
        match record.registry.get(parent).map(|e| &e.value) {
            Some(RestoredRecord::Actor(v)) => &v.shell,
            Some(RestoredRecord::Kingdom(v)) => {
                &v.runtime
                    .as_ref()
                    .ok_or(required("570560 parent Kingdom runtime"))?
                    .base
            }
            Some(RestoredRecord::Loading) => {
                return Err(required("570560 actual parent held outside registry"));
            }
            _ => return Err(required("570560 actual parent generation/Actor allocation")),
        }
    };
    let actor = shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("570560 parent Actor"))?;
    if shell.thing.flags & (1 << 29) != 0
        || !overlap_list::eligible(actor.components.state.low, actor.components.state.high)
    {
        return Ok(None);
    }
    Ok(Some(*actor.words.get(&0xF8).ok_or(required(
        "570560 initialized parent TerrainData F8",
    ))?))
}
fn select(
    catalog: &Catalog<'_>,
    record: &Record<'_>,
    terrain: &BattleTerrain,
    source_pc: Precision,
) -> Result<u32> {
    let actor = record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("570560 Actor"))?;
    match actor.components.get(ComponentSlot::Element) {
        None => (),
        Some(RestoredComponent::Element(e)) if e.header.owner == record.owner => {
            if e.flag_1c
                && let Some(parent) = e.organization
                && let Some(value) = parent_terrain(record, parent)?
            {
                return Ok(value);
            }
        }
        _ => return Err(required("570560 actual Element payload/owner")),
    }
    terrain_token(catalog, terrain, record, source_pc)
}

/// Native TerrainData+B0 is a HEAD-prepended list, hence reverse authored
/// child order. Only reached lists are resolved. Property lookup uses its real
/// source-manager load position, not a final translated name re-lookup.
fn properties(catalog: &Catalog<'_>, token: u32, source_pc: Precision) -> Result<Vec<Property>> {
    let id = checked_terrain_id(catalog, token)?;
    let registry = catalog
        .actors
        .native_data_registry("Terrain")
        .ok_or(required("Terrain manager"))?;
    let mut entries = registry
        .entries
        .iter()
        .filter(|e| e.ids.eq_ignore_ascii_case(&id));
    let entry = entries
        .next()
        .ok_or(required("4D4130 TerrainData source registration"))?;
    if entries.next().is_some() || !entry.reloads.is_empty() {
        return Err(required("TerrainData Property append/reload source owner"));
    }
    let node = catalog.actors.native_data_node(entry).map_err(error)?;
    let children = node.children("Property").map_err(error)?;
    if children.is_empty() {
        return Ok(vec![]);
    }
    let file = catalog
        .actors
        .sources
        .iter()
        .position(|s| s.path == entry.source.path && s.sha256 == entry.source.sha256)
        .ok_or(required("TerrainData Property source file position"))?;
    let at = RootPosition {
        file,
        offset: entry.source.span.start,
    };
    let manager = catalog.layout_data(source_pc).map_err(error)?;
    let mut values = Vec::with_capacity(children.len());
    for child in children {
        let value = child
            .field("property_ids", source_pc)
            .map_err(error)?
            .ok_or(required("TerrainData authored Property IDS"))?;
        let TypedValue::String { value, .. } = value else {
            return Err(required("TerrainData Property IDS typed string"));
        };
        if value.contains('\0') {
            return Err(required(
                "TerrainData Property embedded-null lookup semantics",
            ));
        }
        let reference = manager.find_property(&value, Some(at)).ok_or(required(
            "TerrainData linked Property missing at native source load position",
        ))?;
        values.push(Property {
            manager: Arc::clone(&manager),
            reference,
            source_precision: source_pc,
        });
    }
    values.reverse();
    Ok(values)
}

/// Exact current-record terrain refresh. Earlier F8/callback effects survive
/// later errors. Live arithmetic PC is passed to effects; source reads retain
/// the independently verified definition-time PC.
pub fn refresh(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    terrain: &BattleTerrain,
    pc: Precision,
    effects: &mut impl Effects,
) -> Result<()> {
    let source_pc = crate::load_actor_init_world::validate(catalog, record)?;
    let state = record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("4D4130 Actor"))?
        .components
        .state;
    if !gates(record.shell.thing.flags, state.low, state.high) {
        return Ok(());
    }
    let old = word(record, 0xF8)?;
    let next = select(catalog, record, terrain, source_pc)?;
    record
        .shell
        .actor
        .as_mut()
        .unwrap()
        .words
        .insert(0xF8, next);
    if next == old {
        return Ok(());
    }
    if organization_component(record)? {
        effects.organization_refresh(record, pc)?;
    }
    if !property_component(record)? {
        return Ok(());
    }
    if old != 0 {
        for property in properties(catalog, old, source_pc)? {
            effects.remove_property(record, &property, pc)?;
        }
    }
    let next = word(record, 0xF8)?;
    if next == 0 {
        effects.null_terrain_diagnostic(record)?;
        // Native would subsequently dereference null+B0; never turn this into
        // an empty source list even if a diagnostic service returns.
        return Err(Error::NullDereference("4D438D TerrainData+B0"));
    }
    for property in properties(catalog, next, source_pc)? {
        if property_component(record)? && effects.property_applies(record, &property, pc)? {
            effects.add_property(record, &property, 5, pc)?;
        }
    }
    Ok(())
}
