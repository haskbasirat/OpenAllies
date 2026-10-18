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
use aa_spatial::{gworld_placement::GMapView, query::QueryGeometry};
use aa_zones::tile::ZoneMap;

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
    pub zones: &'world mut ZoneMap,
    pub objects: &'world mut WorldObjectList<ActorId>,
    pub source_fields: &'world NativeSourceRequirements,
    /// The actual Catalog/InactiveLoad binding, retained by the map owner.
    pub map_mode: &'world mut Option<u32>,
    pub global_settings_mode: live_placement::SettingsMode,
    pub placement_clearance_bits: Option<u32>,
    pub zone_services: &'world mut load_zone_services::Context<'globals>,
}

struct Services<'source, 'world, 'globals, 'catalog, 'assets> {
    catalog: &'catalog Catalog<'assets>,
    source: &'source Source,
    map: GMapView,
    geometry: &'source QueryGeometry,
    spatial: &'world mut LoadSpatial,
    terrain: &'world BattleTerrain,
    header: &'world MapHeader,
    zones: &'world ZoneMap,
    source_fields: &'world NativeSourceRequirements,
    global_mode: live_placement::SettingsMode,
    clearance: Option<u32>,
    zone_services: &'world mut load_zone_services::Context<'globals>,
    precision: Precision,
}
impl live_object_init::Services for Services<'_, '_, '_, '_, '_> {
    fn thickness_warning(&mut self, _: &mut Record<'_>) -> Result<()> {
        Err(Error::Unsupported("Object thickness diagnostic owner"))
    }
    fn placement_null(
        &mut self,
        record: &mut Record<'_>,
        mode_owner: &Option<u32>,
        pc: Precision,
    ) -> Result<u32> {
        let mut facts = live_placement::MapFacts {
            map: self.map,
            geometry: self.geometry,
            local_settings_mode: None,
            settings_mode_owner: Some(mode_owner),
            global_settings_mode: self.global_mode,
            zones: Some(self.zones),
            source_fields: Some(self.source_fields),
            point_shape: Some(self.zone_services.point()),
            placement_clearance_bits: self.clearance,
        };
        let mut children = crate::load_placement_children::Children {
            catalog: self.catalog,
            map: self.map,
            geometry: self.geometry,
            terrain: self.terrain,
            spatial: self.spatial,
            zones: self.zones,
            fields: self.source_fields,
            phase: self.zone_services.phase(),
        };
        live_placement::check_restored(self.catalog, record, &mut facts, &mut children, pc)
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
        let catalog=self.catalog;
        let mut context=crate::load_actor_overlaps::QueryContext {
            terrain:self.terrain,header:self.header,spatial:self.spatial,
            zones:self.zones,source_fields:self.source_fields,
        };
        crate::load_object_blocking::notify(catalog,record,&mut context,self.precision,|| {
            catalog.load_clock.map(|clock|clock.begin.0).ok_or(Error::Unsupported(
                "4EB6E0 child4D67F0 requires actual shared GWorld+E0 clock"))
        })
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

pub(crate) fn map_geometry(
    context: &Context<'_, '_>,
    pc: Precision,
) -> Result<(GMapView, QueryGeometry)> {
    map_geometry_parts(context.terrain,context.header,context.spatial,&*context.zones,pc)
}
pub(crate) fn map_geometry_parts(
    terrain:&BattleTerrain,header:&MapHeader,spatial:&LoadSpatial,zones:&ZoneMap,pc:Precision,
) -> Result<(GMapView,QueryGeometry)> {
    map_geometry_index(terrain, header, spatial.index(), zones, pc)
}
/// The same geometry validation after the original index has moved into the
/// live SpatialWorld. This only borrows that index; it never rebuilds tiles.
pub(crate) fn map_geometry_index(
    terrain:&BattleTerrain,header:&MapHeader,index:&aa_spatial::index::SpatialIndex,zones:&ZoneMap,pc:Precision,
) -> Result<(GMapView,QueryGeometry)> {
    let layout = TerrainGridLayout::from_world_extents(
        header.creator.world_extents_bits,
        terrain.terrain.resolution_bits,
    )
    .map_err(error)?;
    let grid = index.grid();
    // Native556653..68 stores the ceiled integer extents as binary32 GMap
    // words. Raw WorldCreator input can be fractional and is not this field.
    let extents = layout.extent_i32.map(|n| (n as f32).to_bits());
    if layout != terrain.bound.layout
        || grid.dimensions != [layout.width, layout.height]
        || grid.coordinate_shift != layout.coordinate_shift
        || grid.cell_size_bits != layout.cell_size_bits
        || index.precision() != pc
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
        QueryGeometry::from_native_extents(index, map.extent_bits, pc).map_err(error)?;
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
        settings_mode_owner: Some(&*context.map_mode),
        global_settings_mode: context.global_settings_mode,
        zones: Some(&*context.zones),
        source_fields: Some(context.source_fields),
        point_shape: Some(context.zone_services.point()),
        placement_clearance_bits: context.placement_clearance_bits,
    };
    let mut children = crate::load_placement_children::Children {
        catalog,
        map,
        geometry: &geometry,
        terrain: context.terrain,
        spatial: context.spatial,
        zones: &*context.zones,
        fields: context.source_fields,
        phase: context.zone_services.phase(),
    };
    live_placement::check_restored(catalog, record, &mut facts, &mut children, pc).map_err(error)
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
/// every earlier mutation on an error. The reached placement callback borrows
/// the actual mode owner; local and proven-global reads never use a copied word.
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
        header,
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
    let mut services = Services {
        catalog,
        source: &source,
        map,
        geometry: &geometry,
        spatial,
        terrain,
        header,
        zones,
        source_fields,
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
