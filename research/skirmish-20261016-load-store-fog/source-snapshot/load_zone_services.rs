//! Load-time Object zone services over the same owned map and held shell.
//!
//! Initial tile state follows5566AA..C6 and557E6E..91, with actual WorldRules
//! mode descriptors from4981A0/59E520. Reached unbound positive callbacks fail;
//! no fabricated empty query or copied phase supplies successful initialization.
use crate::{
    catalog::Catalog,
    live_object_init::{self, MemoryRecord, MemoryServices, Record, Source},
    load_spatial::LoadSpatial,
    load_world_globals::{PointShape, SessionPhase},
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
    loading::LoadKind,
};
use aa_mapworld::{header::MapHeader, restore::RestoredRecord, terrain::BoundTerrain};
use aa_numeric::Precision;
use aa_rules::native_source_requirements::NativeSourceRequirements;
use aa_spatial::index::SpatialIndex;
use aa_zones::tile::{MapGeometry, ZoneMap};

fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}
fn unsupported<T>(name: &'static str) -> Result<T> {
    Err(Error::Unsupported(name))
}

/// Called once after TERRAIN load and before the first world record. The caller
/// must retain this same map through later zone construction; this is not a reset
/// API. Only decoded original playable-map mode0 is admitted by MapHeader.
/// Constructor5566AA..C6 clears all48-byte tiles;4981A0 explicitly gives mode0 a
/// false explored-load gate. Thus retained saved masks are not applied here.
pub fn before_first_record(
    terrain: &BoundTerrain,
    index: &SpatialIndex,
    header: &MapHeader,
) -> Result<ZoneMap> {
    if header.rules.mode != 0 || header.creator.rules.mode != 0 {
        return unsupported("zone initial factory requires decoded playable WorldRules mode0");
    }
    let layout = terrain.layout;
    let expected = aa_mapworld::terrain::TerrainGridLayout::from_world_extents(
        header.creator.world_extents_bits,
        layout.coordinate_shift,
    )
    .map_err(error)?;
    let grid = index.grid();
    if expected != layout
        || terrain.cells.len() != layout.tile_count().map_err(error)?
        || grid.dimensions != [layout.width, layout.height]
        || grid.coordinate_shift != layout.coordinate_shift
        || grid.cell_size_bits != layout.cell_size_bits
    {
        return Err(error(
            "zone initial factory map/terrain/index layout differs",
        ));
    }
    for y in 0..layout.height {
        for x in 0..layout.width {
            let cell = index.tile([x, y]).map_err(error)?;
            if cell.first_node.is_some() || cell.last_node.is_some() || cell.primary_reference != 0
            {
                return Err(error("zone initial factory called after native membership"));
            }
            if cell.flags20
                != terrain.cells[y as usize * layout.width as usize + x as usize].flags20
            {
                return Err(error("zone initial factory terrain flags differ"));
            }
        }
    }
    ZoneMap::new(MapGeometry {
        shift: grid.coordinate_shift,
        tile_size_bits: grid.cell_size_bits,
        //556653..68 stores the ceiled integer dimensions back to binary32.
        extent_bits: layout.extent_i32.map(|n| (n as f32).to_bits()),
        width: grid.dimensions[0],
        height: grid.dimensions[1],
    })
    .map_err(error)
}

/// These are borrows of actual constructor/phase owners. In particular the
/// phase is the retained Session+F8 field, not MapResource+A8 or a cached scalar.
pub struct Context<'globals> {
    point: &'globals PointShape,
    phase: &'globals dyn SessionPhase,
    kingdoms: Option<&'globals crate::load_kingdom_init_world::KingdomRegistryState>,
    source_fields: Option<(
        &'globals dyn AnnounceSource,
        &'globals NativeSourceRequirements,
    )>,
}

// Erase only the Catalog's source lifetime, retaining its actual immutable
// owner by reference. No descriptor/source value is copied into the context.
trait AnnounceSource {
    fn same_catalog(&self, catalog: &Catalog<'_>) -> bool;
    fn announce_byte(
        &self,
        fields: &NativeSourceRequirements,
        record: &MemoryRecord<'_>,
    ) -> Result<u8>;
    fn announce_definition(
        &self,
        fields: &NativeSourceRequirements,
        record: &aa_mapworld::restore::RuntimeDefinition,
    ) -> Result<u8>;
}
impl AnnounceSource for Catalog<'_> {
    fn same_catalog(&self, catalog: &Catalog<'_>) -> bool {
        std::ptr::from_ref(self).cast::<()>() == std::ptr::from_ref(catalog).cast::<()>()
    }
    fn announce_byte(
        &self,
        fields: &NativeSourceRequirements,
        record: &MemoryRecord<'_>,
    ) -> Result<u8> {
        Source::bind(self, record.definition)?.validate_memory_record(record)?;
        self.announce_definition(fields, record.definition)
    }
    fn announce_definition(
        &self,
        fields: &NativeSourceRequirements,
        record: &aa_mapworld::restore::RuntimeDefinition,
    ) -> Result<u8> {
        let source = Source::bind(self, record)?;
        let definition = fields.definition(&record.proof.id).map_err(error)?;
        if fields.receipt().sources != self.actors.sources
            || fields.source_precision() != source.source_precision()
            || definition.token().source() != &record.proof.source
            || !definition
                .token()
                .canonical_id()
                .eq_ignore_ascii_case(&record.proof.id)
            || definition.native_kind() != record.proof.category.native_kind
        {
            return Err(error(
                "load ObjectData16D source receipt/Bank owner differs",
            ));
        }
        fields.byte(&record.proof.id, 0x16d).map_err(error)
    }
}
impl<'globals> Context<'globals> {
    pub fn new(point: &'globals PointShape, phase: &'globals dyn SessionPhase) -> Self {
        Self {
            point,
            phase,
            kingdoms: None,
            source_fields: None,
        }
    }
    pub fn with_kingdoms(
        point: &'globals PointShape,
        phase: &'globals dyn SessionPhase,
        kingdoms: &'globals crate::load_kingdom_init_world::KingdomRegistryState,
    ) -> Self {
        Self {
            point,
            phase,
            kingdoms: Some(kingdoms),
            source_fields: None,
        }
    }
    pub fn with_source_fields(
        mut self,
        catalog: &'globals Catalog<'_>,
        fields: &'globals NativeSourceRequirements,
    ) -> Self {
        self.source_fields = Some((catalog, fields));
        self
    }
    pub fn point(&self) -> &PointShape {
        self.point
    }
    pub fn phase(&self) -> &dyn SessionPhase {
        self.phase
    }
    pub(crate) fn native_memory_phase(&self) -> Result<u32> {
        self.phase.current_phase().map_err(error)
    }
    /// Caller validates its actual staged/held storage before this immutable
    /// canonical source getter. No Loading registry or staged flags are faked.
    pub(crate) fn native_announce_byte(
        &self,
        catalog: &Catalog<'_>,
        definition: &aa_mapworld::restore::RuntimeDefinition,
    ) -> Result<u8> {
        self.validate_catalog_borrow(catalog)?;
        let (source, fields) = self
            .source_fields
            .ok_or(Error::Unsupported("native memory source16D owner absent"))?;
        source.announce_definition(fields, definition)
    }
    fn validate_catalog_borrow(&self, catalog: &Catalog<'_>) -> Result<()> {
        if self
            .source_fields
            .is_some_and(|(source, _)| !source.same_catalog(catalog))
        {
            return Err(error(
                "load zone services Catalog borrow differs from its source owner",
            ));
        }
        Ok(())
    }

    ///4EA150: mode-one occupancy iterator then each ACTUAL current tile set in
    /// native order. Uses the same entry traversal as detection::refresh_from_tiles.
    /// Positive callback storage remains explicit until its actual owner binds.
    pub fn footprint(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        source: &Source,
        zones: &ZoneMap,
        p: Precision,
    ) -> Result<()> {
        self.validate_catalog_borrow(catalog)?;
        self.point.validate_catalog(catalog).map_err(error)?;
        for tile in live_object_init::occupancy_indices(
            record,
            source,
            &zones.geometry,
            self.point.shape(),
            p,
        )? {
            let Some(set) = zones.tile(tile).map_err(error)?.set else {
                continue;
            };
            for entry in zones.pool.entries(Some(set)).map_err(error)? {
                if entry.flags & 0x40 != 0 {
                    let owner = owner(record, ActorId::from_raw(entry.actor))?;
                    if owner != 0 && kingdom_word(record, ActorId::from_raw(owner), 0x190)? == 0 {
                        self.reveal(catalog, record, source, owner, true, true)?;
                    }
                }
                if entry.flags & 2 != 0 {
                    // A nonempty type1 entry reaches its actual Detection+10
                    // owner; no zero reveal mask is substituted.
                    return unsupported(
                        "load footprint requires actual Detection reveal-mask and world kingdom-vector owner",
                    );
                }
            }
        }
        Ok(())
    }

    fn reveal(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        source: &Source,
        kingdom: u32,
        tile_explored: bool,
        show: bool,
    ) -> Result<()> {
        let key = ActorId::from_raw(kingdom);
        let index = kingdom_word(record, key, 0x18c)?;
        let nonmajor = kingdom_word(record, key, 0x190)?;
        // These two fields are the only Kingdom facts read by native54EC70 /
        //54ECB0 /54EAE0 /54EA80 on this prefix. Shared masks are not invented.
        let major = nonmajor == 0;
        let mut word = record.word(0x54)?;
        let seen = !major || word & 1u32.wrapping_shl(index) != 0;
        let visible = !major || word & 1u32.wrapping_shl(index.wrapping_add(16)) != 0;
        if !(show && !visible || tile_explored && !seen) {
            return Ok(());
        }
        if show && major {
            word |= 1u32.wrapping_shl(index.wrapping_add(16));
            record.set_word(0x54, word)?;
        }
        if tile_explored {
            if major {
                word |= 1u32.wrapping_shl(index);
                record.set_word(0x54, word)?;
            }
            if matches!(record.definition.native_class, 3 | 4) {
                return unsupported("load reveal requires actual actor virtual100 container owner");
            }
        }
        self.visibility_inner(catalog, record, source)?;
        // Native reads +16D after memory refresh, even when the subsequent
        // tile-explored/was-seen conditions would skip the announcement.
        let announce = self.definition_announce_byte(&record.memory_record())?;
        if announce != 0 && tile_explored && !seen {
            return unsupported("load reveal requires actual first-sighting message/SAI owner");
        }
        Ok(())
    }

    pub fn visibility(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        source: &Source,
    ) -> Result<()> {
        self.visibility_inner(catalog, record, source)
    }
    fn visibility_inner(
        &mut self,
        catalog: &Catalog<'_>,
        record: &mut Record<'_>,
        source: &Source,
    ) -> Result<()> {
        self.validate_catalog_borrow(catalog)?;
        if let Some(kingdoms) = self.kingdoms {
            return crate::load_detection_memory::refresh(catalog, record, source, self, kingdoms);
        }
        source.validate_record(record)?;
        require_layout(
            record.definition.native_class,
            record.shell.kind,
            record.shell.object.is_some(),
            record.shell.actor.is_some(),
        )?;
        // Actual virtual8C4EA3D0 tests SIGNED Object+58 >0.
        if record.word(0x58)? as i32 > 0 {
            return unsupported("load visibility requires actual detection-memory slot owner");
        }
        if !live_object_init::memory_eligible(record, source, self)? {
            return Ok(());
        }
        unsupported(
            "load visibility requires actual world kingdom-vector and memory create/delete owner",
        )
    }

    ///4EB6E0's current bit10 gate. The reached positive branch requires the
    /// actual overlap query and4D67F0 child, not an empty candidate collection.
    pub fn blocking(
        &mut self,
        _: &Catalog<'_>,
        record: &mut Record<'_>,
        _: &Source,
        _: &mut LoadSpatial,
    ) -> Result<()> {
        if record.word(8)? & (1 << 10) == 0 {
            return Ok(());
        }
        unsupported("load blocking notification requires actual554350/555620 query and4D67F0 owner")
    }
}

impl MemoryServices for Context<'_> {
    fn simulation_phase(&mut self, _: &MemoryRecord<'_>) -> Result<u32> {
        self.phase.current_phase().map_err(error)
    }
    fn definition_announce_byte(&mut self, record: &MemoryRecord<'_>) -> Result<u8> {
        let Some((catalog, fields)) = self.source_fields else {
            return unsupported("load memory requires source-owned ObjectData/ActorData16D byte");
        };
        catalog.announce_byte(fields, record)
    }
}
impl crate::load_actor_reveal::Services for Context<'_> {
    fn reveal(
        &mut self,
        catalog: &Catalog<'_>,
        fields: &mut crate::load_detection_memory::Fields<'_>,
        kingdom: u32,
        explored: bool,
        visible: bool,
    ) -> Result<()> {
        self.validate_catalog_borrow(catalog)?;
        let kingdoms = self.kingdoms.ok_or(Error::Unsupported(
            "Actor64 actual World+154 Kingdom registry owner",
        ))?;
        crate::load_actor_reveal::reveal(
            catalog,
            fields,
            self,
            kingdoms,
            ActorId::from_raw(kingdom),
            explored,
            visible,
        )
    }
}

fn owner(record: &Record<'_>, id: ActorId) -> Result<u32> {
    let (shell, class) = if id == record.owner {
        (&*record.shell, record.definition.native_class)
    } else {
        let entry = record
            .registry
            .get(id)
            .ok_or_else(|| error("zone actor generation is not current"))?;
        match &entry.value {
            RestoredRecord::Object(v) | RestoredRecord::Actor(v) => {
                (&v.shell, v.definition.native_class)
            }
            RestoredRecord::Kingdom(v) => {
                let v = v
                    .runtime
                    .as_ref()
                    .ok_or_else(|| error("zone Kingdom runtime is absent"))?;
                (&v.base, v.definition.native_class)
            }
            _ => return unsupported("zone actor has no supported current runtime shell"),
        }
    };
    require_layout(
        class,
        shell.kind,
        shell.object.is_some(),
        shell.actor.is_some(),
    )?;
    match class {
        // Constructor-owned Object subclasses inherit the same actual
        //+1084C9FA0 full-EAX-zero getter (separate original/v14 slot proof).
        2 | 6 | 7 | 9 => Ok(0),
        3 | 4 => shell
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&0xe8))
            .copied()
            .ok_or_else(|| error("zone actor actualE8 owner field is absent")),
        _ => unsupported("zone actor virtual108 class is not bound"),
    }
}

fn require_layout(class: u32, kind: LoadKind, object: bool, actor: bool) -> Result<()> {
    let valid = match class {
        2 | 6 | 7 | 9 => kind == LoadKind::Object && object && !actor,
        3 | 4 => kind == LoadKind::Actor && object && actor,
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(error(
            "load zone runtime constructor storage differs from class",
        ))
    }
}

fn kingdom_word(record: &Record<'_>, id: ActorId, offset: u32) -> Result<u32> {
    if id == record.owner {
        return unsupported(
            "held current Kingdom derived fields require its actual derived-shell borrow",
        );
    }
    let entry = record
        .registry
        .get(id)
        .ok_or_else(|| error("zone Kingdom generation is not current"))?;
    let RestoredRecord::Kingdom(value) = &entry.value else {
        return Err(error("zone owner is not an actual Kingdom"));
    };
    let runtime = value
        .runtime
        .as_ref()
        .ok_or_else(|| error("actual Kingdom runtime is absent"))?;
    if runtime.definition.native_class != 4 {
        return Err(error("actual Kingdom runtime class differs"));
    }
    require_layout(
        4,
        runtime.base.kind,
        runtime.base.object.is_some(),
        runtime.base.actor.is_some(),
    )?;
    value
        .runtime
        .as_ref()
        .and_then(|r| r.derived.as_ref())
        .and_then(|s| s.words.get(&offset))
        .copied()
        .ok_or_else(|| error(format!("actual Kingdom+{offset:x} field is absent")))
}

#[cfg(test)]
#[path = "load_zone_services_tests.rs"]
mod tests;
