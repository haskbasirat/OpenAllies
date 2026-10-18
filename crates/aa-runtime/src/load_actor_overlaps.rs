//! Actual4D67F0 over the current held Actor and the SAME load spatial index.
//!
//! Expiration mutates the one typed+110 list before the spatial query. Query
//! policy borrows that post-expiration owner immutably; no copied list masks
//! exclusions. Successful discovery references are prepended afterwards. Earlier
//! expiration and candidate flag changes remain committed on any later error.
use crate::{
    catalog::Catalog,
    live_object_init::Record,
    load_object_init_world::{self, Context},
    load_placement_children::Policy,
    load_spatial::{HeldShell, HeldView, QueryBindings},
};
use aa_actors::{
    actor_creation::{Error, Result},
    overlap_list,
};
use aa_numeric::Precision;

/// The same map/query owners, without borrowing unrelated object-list or
/// placement options already lent to Object Init's coordinator.
pub(crate) struct QueryContext<'a> {
    pub terrain:&'a crate::battle_terrain::BattleTerrain,
    pub header:&'a aa_mapworld::header::MapHeader,
    pub spatial:&'a mut crate::load_spatial::LoadSpatial,
    pub zones:&'a aa_zones::tile::ZoneMap,
    pub source_fields:&'a aa_rules::native_source_requirements::NativeSourceRequirements,
}
impl QueryContext<'_> {
    fn from_object<'a>(context:&'a mut Context<'_, '_>)->QueryContext<'a> {
        QueryContext {terrain:context.terrain,header:context.header,spatial:context.spatial,
            zones:context.zones,source_fields:context.source_fields}
    }
    pub(crate) fn geometry(&self,pc:Precision)->Result<aa_spatial::query::QueryGeometry> {
        load_object_init_world::map_geometry_parts(self.terrain,self.header,self.spatial,self.zones,pc)
            .map(|(_,geometry)|geometry)
    }
}

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}

/// `world_time_bits` must read the actual GWorld+E0 clock. It is lazy because
/// bit29, ineligible actor state and bit10-clear return before any clock read.
/// The actual current source/generation/layout is validated before use.
pub fn refresh(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    context: &mut Context<'_, '_>,
    pc: Precision,
    world_time_bits: impl FnOnce() -> Result<u32>,
) -> Result<bool> {
    refresh_inner(catalog, record, &mut QueryContext::from_object(context), pc, world_time_bits, None)
}

pub(crate) fn refresh_with_extra(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    context: &mut QueryContext<'_>,
    pc: Precision,
    world_time_bits: impl FnOnce() -> Result<u32>,
    extra:&mut HeldShell<'_>,
) -> Result<bool> {
    refresh_inner(catalog,record,context,pc,world_time_bits,Some(extra))
}
pub(crate) fn refresh_query(
    catalog:&Catalog<'_>,record:&mut Record<'_>,context:&mut QueryContext<'_>,pc:Precision,
    world_time_bits:impl FnOnce()->Result<u32>,
) -> Result<bool> {
    refresh_inner(catalog,record,context,pc,world_time_bits,None)
}

fn refresh_inner(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    context: &mut QueryContext<'_>,
    pc: Precision,
    world_time_bits: impl FnOnce() -> Result<u32>,
    extra:Option<&mut HeldShell<'_>>,
) -> Result<bool> {
    crate::load_actor_init_world::validate(catalog, record)?;
    let flags = record.shell.thing.flags;
    let actor = record
        .shell
        .actor
        .as_ref()
        .ok_or(Error::NullDereference("4D67F0 Actor"))?;
    if actor.words.contains_key(&0x110) || actor.words.contains_key(&0x114) {
        return Err(error(
            "4D67F0 obsolete raw overlap-list authority is present",
        ));
    }
    if flags & (1 << 29) != 0
        || !overlap_list::eligible(actor.components.state.low, actor.components.state.high)
        || flags & (1 << 10) == 0
    {
        return Ok(false);
    }
    let time = world_time_bits()?;
    record
        .shell
        .actor
        .as_mut()
        .unwrap()
        .overlap_list_110
        .expire_at(time)
        .map_err(error)?;
    let geometry = context.geometry(pc)?;
    let fields = context.source_fields;
    let found = if let Some(extra)=extra {
        context.spatial.with_readonly_query_extra(catalog,record,extra,|index,bindings,record,extra| {
            collect(catalog,record,fields,index,bindings,&geometry,Some(extra))
        })
    } else {
        context.spatial.with_readonly_query(catalog,record,|index,bindings,record| {
            collect(catalog,record,fields,index,bindings,&geometry,None)
        })
    }.map_err(error)?;
    let entries = &mut record.shell.actor.as_mut().unwrap().overlap_list_110;
    for reference in found {
        entries
            .prepend(overlap_list::Entry {
                reference,
                expiry_bits: 0,
            })
            .map_err(error)?;
    }
    Ok(true)
}

fn collect(
    catalog:&Catalog<'_>,record:&Record<'_>,
    fields:&aa_rules::native_source_requirements::NativeSourceRequirements,
    index:&mut aa_spatial::index::SpatialIndex,bindings:QueryBindings<'_>,
    geometry:&aa_spatial::query::QueryGeometry,extra:Option<HeldView<'_>>,
) -> aa_spatial::Result<Vec<u32>> {
    let key=bindings.key(record.owner).map_err(|e|aa_spatial::Error::World(e.to_string()))?;
    let mut policy=Policy::new(catalog,record,fields,bindings);
    if let Some(extra)=extra { policy=policy.with_extra(extra); }
    let mut query=aa_spatial::actor_context::prepare(index,key,false,&policy)?;
    aa_spatial::overlaps::all_overlaps(index,geometry,&mut query,true,&mut policy)
}

#[cfg(test)]
#[path = "load_actor_overlaps_tests.rs"]
mod tests;
