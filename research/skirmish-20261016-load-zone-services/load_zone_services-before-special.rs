//! Load-time Object zone services over the same owned map and held shell.
//!
//! Initial tile state follows5566AA..C6 and557E6E..91, with actual WorldRules
//! mode descriptors from4981A0/59E520. Reached unbound positive callbacks fail;
//! no fabricated empty query or copied phase supplies successful initialization.
use crate::{
    catalog::Catalog,
    live_object_init::{self, MemoryServices, Record, Source},
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
}
impl<'globals> Context<'globals> {
    pub fn new(point: &'globals PointShape, phase: &'globals dyn SessionPhase) -> Self {
        Self { point, phase }
    }
    pub fn point(&self) -> &PointShape {
        self.point
    }
    pub fn phase(&self) -> &dyn SessionPhase {
        self.phase
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
                        self.reveal(record, source, owner, true, true)?;
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
        self.visibility_inner(record, source)?;
        // Native reads +16D after memory refresh, even when the subsequent
        // tile-explored/was-seen conditions would skip the announcement.
        let announce = self.definition_announce_byte(record)?;
        if announce != 0 && tile_explored && !seen {
            return unsupported("load reveal requires actual first-sighting message/SAI owner");
        }
        Ok(())
    }

    pub fn visibility(
        &mut self,
        _: &Catalog<'_>,
        record: &mut Record<'_>,
        source: &Source,
    ) -> Result<()> {
        self.visibility_inner(record, source)
    }
    fn visibility_inner(&mut self, record: &mut Record<'_>, source: &Source) -> Result<()> {
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
    fn simulation_phase(&mut self, _: &Record<'_>) -> Result<u32> {
        self.phase.current_phase().map_err(error)
    }
    fn definition_announce_byte(&mut self, _: &Record<'_>) -> Result<u8> {
        unsupported("load memory requires source-owned ObjectData/ActorData16D byte")
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
        2 => Ok(0), // actual Object+1084C9FA0
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
        2 => kind == LoadKind::Object && object && !actor,
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
