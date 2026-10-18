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
pub(crate) use crate::actor_terrain_fields::gates;

pub(crate) fn terrain_id_at(
    catalog: &Catalog<'_>,
    terrain: &BattleTerrain,
    tile: [i32;2],
    source_pc: Precision,
) -> Result<String> {
    let definition = terrain
        .terrain_definition_at(tile[0],tile[1])
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
    Ok(entry.ids.clone())
}
fn terrain_token(catalog:&Catalog<'_>,terrain:&BattleTerrain,record:&Record<'_>,source_pc:Precision)->Result<u32>{
    let id=terrain_id_at(catalog,terrain,[record.word(0x44)? as i32,record.word(0x48)? as i32],source_pc)?;
    catalog
        .definition_bank
        .try_borrow_mut()
        .map_err(error)?
        .intern("TerrainData", &id)
        .map_err(error)
}
fn checked_terrain_id(catalog: &Catalog<'_>, token: u32) -> Result<String> {
    let bank = catalog.definition_bank.try_borrow().map_err(error)?;
    checked_terrain_id_in_bank(&bank,token)
}
fn checked_terrain_id_in_bank(bank:&crate::definitions::Bank,token:u32)->Result<String>{
    let entry = bank.require_usable(token).map_err(error)?;
    if entry.key.domain != "terraindata" {
        return Err(required("4D4130 F8 actual TerrainData identity domain"));
    }
    Ok(entry.key.name.clone())
}
fn parent_terrain(
    catalog:&Catalog<'_>,record: &Record<'_>, parent: ActorId,
    held:&[crate::load_spatial::HeldView<'_>],
) -> Result<Option<u32>> {
    let shell = if parent == record.owner {
        &*record.shell
    } else if let Some(extra)=held.iter().find(|extra|extra.owner==parent) {
        // A real ancestor's F8 is visible during579EB0 recursion even while
        // its matching registry entry remains Loading. This is a borrow of
        // that allocation, not a copied parent word or detached shell.
        crate::live_object_init::Source::bind(catalog,extra.definition)?;
        if !record.registry.get(parent).is_some_and(|entry|matches!(entry.value,RestoredRecord::Loading))
            || !matches!(extra.definition.native_class,3|4)
            || !crate::load_spatial::ordinary_layout(extra.definition.native_class,
                extra.shell.kind,extra.shell.object.is_some(),extra.shell.actor.is_some())
            || !crate::load_spatial::same_proof(&extra.shell.thing.definition,&extra.definition.proof)
        {
            return Err(required("570560 actual held parent source/generation/layout differs"));
        }
        extra.shell
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
    held:&[crate::load_spatial::HeldView<'_>],
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
                && let Some(value) = parent_terrain(catalog,record, parent,held)?
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
    properties_for_id(catalog,&id,source_pc)
}
pub(crate) fn properties_in_bank(catalog:&Catalog<'_>,bank:&crate::definitions::Bank,token:u32,source_pc:Precision)->Result<Vec<Property>>{
    let id=checked_terrain_id_in_bank(bank,token)?;
    properties_for_id(catalog,&id,source_pc)
}
fn properties_for_id(catalog:&Catalog<'_>,id:&str,source_pc:Precision)->Result<Vec<Property>>{
    let registry = catalog
        .actors
        .native_data_registry("Terrain")
        .ok_or(required("Terrain manager"))?;
    let mut entries = registry
        .entries
        .iter()
        .filter(|e| e.ids.eq_ignore_ascii_case(id));
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
    refresh_with_held(catalog,record,terrain,pc,&[],effects)
}

/// Opt-in ancestor service used by actual Organization member recursion.
/// The original public entry deliberately supplies no held parent and retains
/// its precise required error when that ancestor is unavailable.
pub(crate) fn refresh_with_held(
    catalog:&Catalog<'_>,record:&mut Record<'_>,terrain:&BattleTerrain,pc:Precision,
    held:&[crate::load_spatial::HeldView<'_>],effects:&mut impl Effects,
) -> Result<()> {
    let source_pc = crate::load_actor_init_world::validate(catalog, record)?;
    crate::actor_terrain_fields::transition(&mut HeldFields{catalog,record,terrain,pc,source_pc,held,effects})
}
struct HeldFields<'a,'r,'source,'parents,E>{
    catalog:&'a Catalog<'source>,record:&'a mut Record<'r>,terrain:&'a BattleTerrain,
    pc:Precision,source_pc:Precision,held:&'a [crate::load_spatial::HeldView<'parents>],effects:&'a mut E,
}
impl<E:Effects> crate::actor_terrain_fields::Fields for HeldFields<'_,'_,'_,'_,E>{
    type Property=Property;
    fn flags_state(&self)->Result<(u32,u32,u32)>{
        let state=self.record.shell.actor.as_ref().ok_or(Error::NullDereference("4D4130 Actor"))?.components.state;
        Ok((self.record.shell.thing.flags,state.low,state.high))
    }
    fn current(&self)->Result<u32>{word(self.record,0xF8)}
    fn select(&mut self)->Result<u32>{select(self.catalog,self.record,self.terrain,self.source_pc,self.held)}
    fn store(&mut self,token:u32)->Result<()>{self.record.shell.actor.as_mut().unwrap().words.insert(0xF8,token);Ok(())}
    fn organization(&self)->Result<bool>{organization_component(self.record)}
    fn refresh_organization(&mut self)->Result<()>{self.effects.organization_refresh(self.record,self.pc)}
    fn properties(&self)->Result<bool>{property_component(self.record)}
    fn property_list(&self,token:u32)->Result<Vec<Property>>{properties(self.catalog,token,self.source_pc)}
    fn remove(&mut self,p:&Property)->Result<()>{self.effects.remove_property(self.record,p,self.pc)}
    fn applies(&mut self,p:&Property)->Result<bool>{self.effects.property_applies(self.record,p,self.pc)}
    fn add(&mut self,p:&Property,kind:u32)->Result<()>{self.effects.add_property(self.record,p,kind,self.pc)}
    fn null_diagnostic(&mut self)->Result<()>{self.effects.null_terrain_diagnostic(self.record)}
}
