//! Concrete saved Object Init wiring over the current load world.
//!
//! This composes previously proved native ports; it does not reconstruct
//! mutable object state from the renderer or create a second pose owner.
use crate::{
    battle_terrain::BattleTerrain,
    catalog::Catalog,
    live_object_init::{self, Record, Source},
    live_placement,
    load_spatial::LoadSpatial,
    load_zone_services,
    world_object_list::WorldObjectList,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
};
use aa_mapworld::{header::MapHeader, terrain::TerrainGridLayout};
use aa_numeric::Precision;
use aa_rules::native_source_requirements::NativeSourceRequirements;
use aa_spatial::{
    creation_placement::Context as CollisionContext, gworld_placement::GMapView,
    loaded_object_placement::Descriptor, query::QueryGeometry,
};
use aa_zones::{query::ZoneActors, tile::ZoneMap};

fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}

fn require_terrain_source(catalog: &Catalog<'_>, terrain: &BattleTerrain) -> Result<()> {
    catalog.require_source_owner().map_err(error)?;
    let bank = catalog.definition_bank.try_borrow().map_err(error)?;
    let owner = bank.combat_source().map_err(error)?;
    if owner.source_identities() != catalog.actors.sources.as_slice()
        || terrain.source_precision() != owner.source_precision()
    {
        return Err(error(
            "Object Init terrain differs from its immutable source FP owner",
        ));
    }
    Ok(())
}

/// These are borrows of the one live load owner. Constructor/global/source
/// receipts belong to their concrete owners, not to a boolean in this adapter.
pub struct Context<'world, 'globals> {
    pub terrain: &'world BattleTerrain,
    pub header: &'world MapHeader,
    pub spatial: &'world mut LoadSpatial,
    pub zones: &'world ZoneMap,
    pub objects: &'world mut WorldObjectList<ActorId>,
    pub source_fields: &'world NativeSourceRequirements,
    /// The actual Catalog/InactiveLoad binding, retained by the map owner.
    pub map_mode: &'world mut Option<u32>,
    pub global_settings_mode: live_placement::SettingsMode,
    pub placement_clearance_bits: Option<u32>,
    pub zone_services: &'world mut load_zone_services::Context<'globals>,
}

/// Reached services not bound by this concrete dependency slice fail at their
/// exact call boundary. They never return a clear tile or an empty actor list.
struct PlacementChildren;
fn placement_child<T>(name: &str) -> aa_spatial::Result<T> {
    Err(aa_spatial::Error::World(format!(
        "saved Object Init requires concrete {name} owner"
    )))
}
impl ZoneActors for PlacementChildren {
    fn owner(&mut self, _: u32) -> aa_zones::Result<u32> {
        Err(aa_zones::Error::World(
            "placement zone owner not bound".into(),
        ))
    }
    fn allied(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
        Err(aa_zones::Error::World(
            "placement alliance owner not bound".into(),
        ))
    }
    fn enemy(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
        Err(aa_zones::Error::World(
            "placement hostility owner not bound".into(),
        ))
    }
    fn zone_slot(&mut self, _: u32, _: u32) -> aa_zones::Result<u32> {
        Err(aa_zones::Error::World(
            "placement zone storage not bound".into(),
        ))
    }
    fn word4(&mut self, _: u32) -> aa_zones::Result<u32> {
        Err(aa_zones::Error::World(
            "placement zone definition not bound".into(),
        ))
    }
    fn component_b0(&mut self, _: u32) -> aa_zones::Result<u32> {
        Err(aa_zones::Error::World(
            "placement zone component not bound".into(),
        ))
    }
    fn reveal_mask(&mut self, _: u32) -> aa_zones::Result<Option<u32>> {
        Err(aa_zones::Error::World(
            "placement reveal storage not bound".into(),
        ))
    }
}
impl live_placement::Services for PlacementChildren {
    fn required_zone(
        &mut self,
        _: &mut Record<'_>,
        _: [u32; 2],
        _: u32,
        _: u32,
        _: Option<ActorId>,
        _: u32,
        _: u32,
        _: Precision,
    ) -> aa_spatial::Result<u32> {
        placement_child("4F1660 placement child")
    }
    fn zone_actor(&mut self, _: u32) -> aa_spatial::Result<ActorId> {
        placement_child("zone actor storage")
    }
    fn structure_at(
        &mut self,
        _: &mut Record<'_>,
        _: [u32; 2],
    ) -> aa_spatial::Result<Option<ActorId>> {
        placement_child("4F2220 structure child")
    }
    fn collision(
        &mut self,
        _: &mut Record<'_>,
        _: &mut CollisionContext<u32, ActorId>,
        _: u32,
        _: Precision,
    ) -> aa_spatial::Result<u32> {
        placement_child("4F0A70 current-shell query policy")
    }
    fn footprint(
        &mut self,
        _: &mut Record<'_>,
        _: &Descriptor<u32, ()>,
        _: Precision,
    ) -> aa_spatial::Result<u32> {
        placement_child("4F0B70 placement footprint child")
    }
}

struct Services<'source, 'world, 'globals, 'catalog, 'assets> {
    catalog: &'catalog Catalog<'assets>,
    source: &'source Source,
    map: GMapView,
    geometry: &'source QueryGeometry,
    spatial: &'world mut LoadSpatial,
    zones: &'world ZoneMap,
    source_fields: &'world NativeSourceRequirements,
    local_mode: Option<u32>,
    global_mode: live_placement::SettingsMode,
    clearance: Option<u32>,
    zone_services: &'world mut load_zone_services::Context<'globals>,
    precision: Precision,
}
impl live_object_init::Services for Services<'_, '_, '_, '_, '_> {
    fn thickness_warning(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(Error::Unsupported("Object thickness diagnostic owner"))
    }
    fn placement_null(&mut self, record: &mut Record<'_>, pc: Precision) -> Result<u32> {
        let mut facts = live_placement::MapFacts {
            map: self.map,
            geometry: self.geometry,
            local_settings_mode: self.local_mode,
            global_settings_mode: self.global_mode,
            zones: Some(self.zones),
            source_fields: Some(self.source_fields),
            point_shape: Some(self.zone_services.point()),
            placement_clearance_bits: self.clearance,
        };
        live_placement::check_restored(self.catalog, record, &mut facts, &mut PlacementChildren, pc)
            .map_err(error)
    }
    fn placement_failure(&mut self, _: &mut Record<'_>, _: u32) -> Result<()> {
        Err(Error::Unsupported(
            "Object placement-failure diagnostic owner",
        ))
    }
    fn placement_map_warning(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(Error::Unsupported("Object map-placement diagnostic owner"))
    }
    fn spatial_membership(&mut self, record: &mut Record<'_>) -> Result<()> {
        self.spatial
            .refresh_membership(self.catalog, record)
            .map_err(error)
    }
    fn blocking_notify(&mut self, record: &mut Record<'_>) -> Result<()> {
        self.zone_services
            .blocking(self.catalog, record, self.source, self.spatial)
    }
    fn tile_footprint(&mut self, record: &mut Record<'_>) -> Result<()> {
        self.zone_services.footprint(
            self.catalog,
            record,
            self.source,
            self.zones,
            self.precision,
        )
    }
    fn retain_instances(&mut self, record: &mut Record<'_>) -> Result<bool> {
        live_object_init::memory_eligible(record, self.source, self.zone_services)
    }
    fn clear_instances(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(Error::Unsupported("Object memory destruction/SAI owner"))
    }
    fn visibility_refresh(&mut self, record: &mut Record<'_>) -> Result<()> {
        self.zone_services
            .visibility(self.catalog, record, self.source)
    }
}

fn map_geometry(context: &Context<'_, '_>, pc: Precision) -> Result<(GMapView, QueryGeometry)> {
    let terrain = context.terrain;
    let header = context.header;
    let spatial = &*context.spatial;
    let zones = context.zones;
    let layout = TerrainGridLayout::from_world_extents(
        header.creator.world_extents_bits,
        terrain.terrain.resolution_bits,
    )
    .map_err(error)?;
    let grid = spatial.index().grid();
    // Native556653..68 stores the ceiled integer extents as binary32 GMap
    // words. Raw WorldCreator input can be fractional and is not this field.
    let extents = layout.extent_i32.map(|n| (n as f32).to_bits());
    if layout != terrain.bound.layout
        || grid.dimensions != [layout.width, layout.height]
        || grid.coordinate_shift != layout.coordinate_shift
        || grid.cell_size_bits != layout.cell_size_bits
        || spatial.index().precision() != pc
        || zones.geometry.width != layout.width
        || zones.geometry.height != layout.height
        || zones.geometry.shift != layout.coordinate_shift
        || zones.geometry.tile_size_bits != layout.cell_size_bits
        || zones.geometry.extent_bits != extents
    {
        return Err(Error::World("Object Init live map owners disagree".into()));
    }
    let map = GMapView {
        width: layout.width,
        height: layout.height,
        coordinate_shift: layout.coordinate_shift,
        cell_size_bits: layout.cell_size_bits,
        extent_bits: extents,
    };
    let geometry =
        QueryGeometry::from_native_extents(spatial.index(), map.extent_bits, pc).map_err(error)?;
    Ok((map, geometry))
}

/// The same exact saved 4F4590 binding is shared by the Team and Object
/// coordinators. Unsupported definition/runtime classes still fail in that
/// binding; calling this helper does not admit an unproved Team virtual.
pub fn placement_null(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    context: &mut Context<'_, '_>,
    pc: Precision,
) -> Result<u32> {
    require_terrain_source(catalog, context.terrain)?;
    if *context.map_mode != catalog.native_load_mode {
        return Err(error(
            "Object placement mode differs from its Catalog load binding",
        ));
    }
    let (map, geometry) = map_geometry(context, pc)?;
    let mut facts = live_placement::MapFacts {
        map,
        geometry: &geometry,
        local_settings_mode: *context.map_mode,
        global_settings_mode: context.global_settings_mode,
        zones: Some(context.zones),
        source_fields: Some(context.source_fields),
        point_shape: Some(context.zone_services.point()),
        placement_clearance_bits: context.placement_clearance_bits,
    };
    live_placement::check_restored(catalog, record, &mut facts, &mut PlacementChildren, pc)
        .map_err(error)
}

/// Whole 4EE4C0's class/flag entry gate must be applied by the native caller.
/// The concrete member/index binder itself requires actual Object storage.
pub fn spatial_membership(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    context: &mut Context<'_, '_>,
) -> Result<()> {
    context
        .spatial
        .refresh_membership(catalog, record)
        .map_err(error)
}

/// Full already-ported Object Init over the actual current Record, preserving
/// every earlier mutation on an error. No external host callback is admitted by
/// this concrete composition, and none of its leaf services writes map settings;
/// the local mode capture therefore remains current for this synchronous call.
pub fn initialize(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    context: &mut Context<'_, '_>,
    pc: Precision,
) -> Result<bool> {
    require_terrain_source(catalog, context.terrain)?;
    if *context.map_mode != catalog.native_load_mode {
        return Err(error(
            "Object Init mode differs from its Catalog load binding",
        ));
    }
    let (map, geometry) = map_geometry(context, pc)?;
    let Context {
        terrain,
        header: _,
        spatial,
        zones,
        objects,
        source_fields,
        map_mode,
        global_settings_mode,
        placement_clearance_bits,
        zone_services,
    } = context;
    let source = Source::bind(catalog, record.definition)?;
    let surface = terrain.surface().map_err(error)?;
    let local_mode = **map_mode;
    let mut services = Services {
        catalog,
        source: &source,
        map,
        geometry: &geometry,
        spatial,
        zones,
        source_fields,
        local_mode,
        global_mode: *global_settings_mode,
        clearance: *placement_clearance_bits,
        zone_services,
        precision: pc,
    };
    live_object_init::initialize(
        live_object_init::Context {
            record: Record {
                owner: record.owner,
                shell: &mut *record.shell,
                definition: record.definition,
                registry: &mut *record.registry,
            },
            source: &source,
            surface: &surface,
            map_mode,
            objects,
        },
        pc,
        &mut services,
    )
}
