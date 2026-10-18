//! Actual Actor registration grid half4CFEA0/554CC0/554330/54EF10.
//! The same persistent zones, held shell and phase/world owners remain borrowed.
use crate::{
    catalog::Catalog,
    live_object_init::{Record, Source},
    load_spatial::LoadSpatial,
    load_world_globals::{PointShape, SessionPhase},
};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot,
    loading::LoadKind,
};
use aa_mapworld::world::WorldState;
use aa_numeric::{Binary, Precision};
use aa_spatial::{
    collision::{Grid, Shape},
    geometry::{Placement, TileRect},
};
use aa_zones::{set::Entry, tile::ZoneMap};

///Same World zone allocations and shared RNG, borrowed only during actual
///registration. The older diagnostic entry point has no such capability.
pub struct CityContext<'a> {
    pub identities: &'a crate::load_detection_activation::Identities,
    pub rng: &'a mut aa_simulation::NativeRng,
}

fn error(value: impl std::fmt::Display) -> Error {
    Error::World(value.to_string())
}

///554CC0 differs from554DD0: non-multi-tile actors use their cached44/48
/// rectangle, while multi-tile actors use5534F0 shape bounds. Both use mode1.
pub(crate) fn visit_indices(
    grid: &Grid,
    extents: [u32; 2],
    point: &Shape,
    placement: &Placement,
    flags: u32,
    cached: [i32; 2],
    p: Precision,
    mut visit: impl FnMut(usize) -> Result<()>,
) -> Result<()> {
    if point.kind != 0 || grid.dimensions.iter().any(|&n| n <= 0) {
        return Err(error(
            "registration requires proved point and positive map dimensions",
        ));
    }
    let multi = aa_spatial::geometry::marks_blocker(
        flags,
        placement.shape.radius_bits,
        grid.cell_size_bits,
        p,
    )
    .map_err(error)?;
    let rect = if multi {
        aa_spatial::geometry::tile_rectangle(grid, placement, p).map_err(error)?
    } else {
        let cell = [
            cached[0].clamp(0, grid.dimensions[0] - 1),
            cached[1].clamp(0, grid.dimensions[1] - 1),
        ];
        TileRect {
            min: cell,
            max: cell,
        }
    };
    let mut center = rect.min;
    let inside = placement
        .position
        .into_iter()
        .zip(extents)
        .all(|(xy, max)| f32::from_bits(xy) >= 0.0 && f32::from_bits(xy) < f32::from_bits(max));
    if inside {
        for (axis, c) in center.iter_mut().enumerate() {
            *c = Binary::from_f32_bits(placement.position[axis])?.truncate_i32()?
                >> (grid.coordinate_shift & 31);
        }
    }
    let half = Binary::from_f32_bits(grid.cell_size_bits)?
        .mul(&Binary::from_f32_bits(0x3f000000)?, p)?
        .store_f32()?;
    for y in rect.min[1]..=rect.max[1] {
        for x in rect.min[0]..=rect.max[0] {
            let cell = [x, y];
            let include = if cell == center {
                true
            } else {
                let mut position = [0; 2];
                for (axis, bits) in position.iter_mut().enumerate() {
                    *bits = Binary::from_i32(cell[axis].wrapping_shl(grid.coordinate_shift & 31))
                        .add(&Binary::from_f32_bits(half)?, p)?
                        .store_f32()?;
                }
                aa_spatial::shape_overlap::overlaps(
                    placement,
                    &Placement {
                        shape: point.clone(),
                        facing: [0, 0xbf800000],
                        position,
                    },
                    p,
                )
                .map_err(error)?
            };
            if include {
                visit(y as usize * grid.dimensions[0] as usize + x as usize)?;
            }
        }
    }
    Ok(())
}

///4CFEA0 over the current actual Actor after Object Init membership. Native
///54EF10 rereads SessionE4 at every tile; World48 is read only when E4==3.
/// Entry.actor remains ActorId.raw(), never an internal spatial key.
#[allow(clippy::too_many_arguments)]
pub fn grid_register(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    source: &Source,
    zones: &mut ZoneMap,
    spatial: &LoadSpatial,
    point: &PointShape,
    session: &dyn SessionPhase,
    world: &WorldState,
    p: Precision,
) -> Result<()> {
    grid_register_bound(
        catalog, record, source, zones, spatial, point, session, world, None, None, p,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn grid_register_bound(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    source: &Source,
    zones: &mut ZoneMap,
    spatial: &LoadSpatial,
    point: &PointShape,
    session: &dyn SessionPhase,
    world: &WorldState,
    mut city: Option<&mut CityContext<'_>>,
    live_world48: Option<&crate::mission_fields::State>,
    p: Precision,
) -> Result<()> {
    source.validate_record(record)?;
    let canonical = Source::bind(catalog, record.definition)?;
    let a = canonical.blocking_shape();
    let b = source.blocking_shape();
    if canonical.source_precision() != source.source_precision()
        || a.kind != b.kind
        || a.radius_bits != b.radius_bits
        || a.bounds != b.bounds
    {
        return Err(error(
            "registration source owner/geometry differs from actual Catalog",
        ));
    }
    if !matches!(record.definition.native_class, 3 | 4)
        || record.shell.kind != LoadKind::Actor
        || record.shell.actor.is_none()
        || record.shell.object.is_none()
    {
        return Err(error(
            "registration requires actual Actor/Kingdom constructor storage",
        ));
    }
    let index = spatial.index();
    let grid = index.grid();
    let z = zones.geometry;
    if grid.dimensions != [z.width, z.height]
        || grid.coordinate_shift != z.shift
        || grid.cell_size_bits != z.tile_size_bits
        || index.precision() != p
    {
        return Err(error("registration map/index owner geometry differs"));
    }
    let key = spatial.key(record.owner).map_err(error)?;
    if index.resolve_reference(record.owner.raw()) != Some(key) {
        return Err(error(
            "registration generation differs from persistent membership",
        ));
    }
    point.validate_catalog(catalog).map_err(error)?;
    let placement = Placement {
        shape: source.blocking_shape().clone(),
        facing: [record.word(0x2c)?, record.word(0x30)?],
        position: [record.word(0x20)?, record.word(0x24)?],
    };
    visit_indices(
        grid,
        z.extent_bits,
        point.shape(),
        &placement,
        record.word(8)?,
        [record.word(0x44)? as i32, record.word(0x48)? as i32],
        p,
        |tile| {
            let app = session.application_state().map_err(error)?;
            if app == 3 {
                //The real World route loans its current+48 owner. The saved
                //DTO is retained only for the explicitly older diagnostic API.
                let word = if let Some(live) = live_world48 {
                    live.started48()
                } else {
                    world.word_48.ok_or(Error::Unsupported(
                        "registration requires actual saved World48 owner",
                    ))?
                };
                if word == 0 {
                    return Ok(());
                }
            }
            let Some(set) = zones.tile(tile).map_err(error)?.set else {
                return Ok(());
            };
            for entry in zones.pool.entries(Some(set)).map_err(error)? {
                entered_prefix(catalog, record, entry, city.as_deref_mut(), p)?;
            }
            Ok(())
        },
    )
}

fn entered_prefix(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    entry: Entry,
    city: Option<&mut CityContext<'_>>,
    pc: Precision,
) -> Result<()> {
    entered_fields(entry,&mut SavedEntered{catalog,record,city,pc})
}

///Storage-independent515620 prefix. Reached positive component effects are
///mandatory; the saved and published adapters borrow their actual owners.
pub(crate) trait EnteredFields{
    fn control_entered(&mut self, _owner:aa_actors::ActorId)->Result<()>{
        Err(Error::Unsupported("registration zone slot4/control owner"))
    }
    fn city_entered(&mut self,owner:aa_actors::ActorId)->Result<()>;
    fn detection_present(&mut self)->Result<bool>;
}
pub(crate) fn entered_fields(entry:Entry,fields:&mut impl EnteredFields)->Result<()>{
    //515620 reaches these zone-slot getters in precisely this order. Their
    // owning zone/component effects must bind before a positive call succeeds.
    if entry.flags & 0x20 != 0 {
        return Err(Error::Unsupported("registration zone slot5/guard owner"));
    }
    if entry.flags & 0x10 != 0 {
        fields.control_entered(aa_actors::ActorId::from_raw(entry.actor))?;
    }
    if entry.flags & 8 != 0 {
        return Err(Error::Unsupported("registration zone slot3 component owner"));
    }
    if entry.flags & 0x100 != 0 {
        fields.city_entered(aa_actors::ActorId::from_raw(entry.actor))?;
    }
    if entry.flags & 4 != 0 && fields.detection_present()? {
        return Err(Error::Unsupported(
            "registration actual marker/discovery owner",
        ));
    }
    Ok(())
}
struct SavedEntered<'a,'r,'s,'city>{catalog:&'a Catalog<'s>,record:&'a mut Record<'r>,city:Option<&'a mut CityContext<'city>>,pc:Precision}
impl EnteredFields for SavedEntered<'_,'_,'_,'_>{
    fn city_entered(&mut self,owner:aa_actors::ActorId)->Result<()>{
        let city = self.city.as_mut().ok_or(Error::Unsupported(
            "registration zone slot8 component owner",
        ))?;
        crate::load_detection_activation::city_entered(
            self.record,
            owner,
            city.identities,
            self.pc,
            &mut CityEffects {
                catalog:self.catalog,
                rng: &mut *city.rng,
            },
        )
        .map_err(error)
    }
    fn detection_present(&mut self)->Result<bool>{
        Ok(self.record
            .shell
            .actor
            .as_ref()
            .is_some_and(|a| a.components.present(ComponentSlot::Detection)))
    }
}

struct CityEffects<'a, 's> {
    catalog: &'a Catalog<'s>,
    rng: &'a mut aa_simulation::NativeRng,
}
impl crate::load_detection_activation::Effects for CityEffects<'_, '_> {
    fn city_definition(
        &mut self,
        definition: &aa_mapworld::restore::RuntimeDefinition,
    ) -> aa_zones::Result<()> {
        Source::bind(self.catalog, definition)
            .map(|_| ())
            .map_err(|e| aa_zones::Error::World(e.to_string()))
    }
    fn city_source_byte(
        &mut self,
        definition: &aa_mapworld::restore::RuntimeDefinition,
        offset: u32,
    ) -> aa_zones::Result<u8> {
        crate::load_city_candidate::source_byte(self.catalog, definition, offset)
            .map_err(aa_zones::Error::World)
    }
    fn city_pending(&mut self, owner: aa_actors::ActorId, pc: Precision) -> aa_zones::Result<bool> {
        crate::load_detection_init_world::pending_component(
            self.catalog,
            owner,
            ComponentSlot::City,
            pc,
        )
        .map_err(|e| aa_zones::Error::World(e.to_string()))
    }
    fn city_schedule(&mut self, owner: aa_actors::ActorId, pc: Precision) -> aa_zones::Result<()> {
        crate::load_detection_init_world::schedule_component(
            self.catalog,
            owner,
            ComponentSlot::City,
            self.rng,
            pc,
        )
        .map_err(|e| aa_zones::Error::World(e.to_string()))
    }
}

#[cfg(test)]
#[path = "load_zone_entry_tests.rs"]
mod tests;
