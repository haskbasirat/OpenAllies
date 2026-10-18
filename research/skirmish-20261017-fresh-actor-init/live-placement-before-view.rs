//! Borrowed saved-shell binding of original 4F4590 -> 4F3640.
//!
//! Source shape/category/flags, shell pose and GMap geometry are real owners.
//! Completed ActorData link fields, zone actors and collision/footprint children
//! must be supplied by their actual owners. No viewer placement or clear-world
//! answer is substituted for an absent producer.
use crate::{catalog::Catalog, live_object_init::Record};
use aa_actors::ActorId;
use aa_numeric::{Binary, Precision};
use aa_spatial::{
    Error, Result,
    collision::{Grid, Shape},
    creation_placement::{self as cp, Arguments, Context, World},
    geometry::Placement,
    gworld_placement::GMapView,
    index::{ActorKey, SpatialIndex},
    loaded_object_placement::{self as loaded, Descriptor},
    query::{PreparedQuery, QueryGeometry, QueryPolicy},
};
use aa_zones::{query::ZoneActors, tile::ZoneMap};

fn fail(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}

/// The arithmetic branch49BD59..49BE07 after the real virtual+38/source flags
/// choose expansion. Inputs are current source shape and GameData+44 words.
/// Source classification and lazy field reads belong to the caller below.
pub fn expanded_blocking_shape(
    original: &Shape,
    clearance_bits: u32,
    p: Precision,
) -> Result<Shape> {
    if original.kind > 3 {
        return Ok(original.clone());
    }
    let clearance = Binary::from_f32_bits(clearance_bits)?;
    if original.kind <= 1 {
        let radius = clearance
            .add(&Binary::from_f32_bits(original.radius_bits)?, p)?
            .store_f32()?;
        return Ok(Shape {
            kind: 1,
            radius_bits: radius,
            bounds: [0; 4],
        });
    }
    let mut dimensions = [0; 2];
    for a in [1, 0] {
        let v = Binary::from_f32_bits(original.bounds[a + 2])?.add(&clearance, p)?;
        dimensions[a] = v.add(&v, p)?.store_f32()?;
    }
    let words = aa_rules::objects::shape(3, &dimensions, p)
        .map_err(fail)?
        .words;
    Ok(Shape {
        kind: words[0],
        radius_bits: words[1],
        bounds: [words[2], words[3], words[4], words[5]],
    })
}

/// An unbound producer is distinct from an actual null settings pointer.
#[derive(Clone, Copy, Debug)]
pub enum SettingsMode {
    Unbound,
    Null,
    Present(u32),
    SameWorld(crate::load_world_settings::SameWorldSettings),
}

pub(crate) fn read_settings_mode(
    global: bool,
    local_probe: Option<u32>,
    binding: SettingsMode,
    owner: Option<&Option<u32>>,
) -> Result<Option<u32>> {
    if !global {
        return Ok(owner.copied().unwrap_or(local_probe));
    }
    match binding {
        SettingsMode::Unbound => Err(fail(
            "placement requires the actual global GWorld settings owner",
        )),
        SettingsMode::Null => Ok(None),
        SettingsMode::Present(mode) => Ok(Some(mode)),
        SettingsMode::SameWorld(alias) => alias
            .read(owner.ok_or_else(|| {
                fail("placement global alias requires its actual mode owner borrow")
            })?)
            .map_err(fail),
    }
}

/// Actual GMap/world owners, supplied from native map construction/load. A
/// missing zone map differs from a constructed map whose current sets are null.
pub struct MapFacts<'a> {
    pub map: GMapView,
    pub geometry: &'a QueryGeometry,
    pub local_settings_mode: Option<u32>,
    /// Actual stable current MapResource mode owner. Concrete managed loads
    /// supply it; standalone historical probes retain their explicit scalar.
    pub settings_mode_owner: Option<&'a Option<u32>>,
    pub global_settings_mode: SettingsMode,
    pub zones: Option<&'a ZoneMap>,
    /// Canonical completed dependency slice, owned alongside Catalog's source
    /// graph. No missing slice is replaced with constructor-zero guesses.
    pub source_fields: Option<&'a aa_rules::native_source_requirements::NativeSourceRequirements>,
    /// Actual global B3A8C4 owner, required only by TeamData4942F0's shape copy.
    pub point_shape: Option<&'a crate::load_world_globals::PointShape>,
    /// Current [GameData+44] scalar used only by virtual+28 expansion.
    pub placement_clearance_bits: Option<u32>,
}

/// Remaining services belong to the canonical source, zone and live world
/// owners. Methods intentionally have no permissive/default implementation.
pub trait Services: ZoneActors {
    #[allow(clippy::too_many_arguments)] // Explicit original call arguments.
    fn required_zone(
        &mut self,
        record: &mut Record<'_>,
        position: [u32; 2],
        definition: u32,
        owner: u32,
        other: Option<ActorId>,
        argument_7: u32,
        argument_8: u32,
        p: Precision,
    ) -> Result<u32>;
    fn zone_actor(&mut self, zone: u32) -> Result<ActorId>;
    fn structure_at(
        &mut self,
        record: &mut Record<'_>,
        position: [u32; 2],
    ) -> Result<Option<ActorId>>;
    fn collision(
        &mut self,
        record: &mut Record<'_>,
        context: &mut Context<u32, ActorId>,
        original_definition: u32,
        p: Precision,
    ) -> Result<u32>;
    fn footprint(
        &mut self,
        record: &mut Record<'_>,
        descriptor: &Descriptor<u32, ()>,
        p: Precision,
    ) -> Result<u32>;
}

/// Actual 4F0A70 child using one authoritative SpatialIndex. The caller supplies
/// the prepared typed-to-index mapping and policy. Native +38 is called after
/// the query even when its hit is nonzero; it is not an optional cleanup.
pub fn collision<P: QueryPolicy>(
    index: &mut SpatialIndex,
    geometry: &QueryGeometry,
    query: &PreparedQuery,
    policy: &mut P,
    mut definition_virtual_38: impl FnMut() -> Result<bool>,
) -> Result<u32> {
    let hit = aa_spatial::query::first_hit(index, geometry, query, policy)?;
    definition_virtual_38()?;
    Ok(match hit.kind {
        0 => 0,
        1 => 1,
        3 => 3,
        _ => 2,
    })
}

/// Convert only already bound logical identities. An unpublished actor does not
/// silently become a null source or an unchecked GTID cast.
pub fn prepare_collision(
    context: &Context<u32, ActorId>,
    mut actor: impl FnMut(ActorId) -> Result<ActorKey>,
) -> Result<PreparedQuery> {
    Ok(PreparedQuery {
        placement: context.placement.clone(),
        required_actor_flags: context.required_actor_flags,
        descriptor: aa_spatial::collision::Descriptor {
            source_definition: Some(context.definition),
            source_actor: context.source_actor.map(&mut actor).transpose()?,
            ignore_actor: context.ignore_actor.map(&mut actor).transpose()?,
            ignore_definition: context.ignore_definition,
            collision_owner: (context.collision_owner != 0).then_some(context.collision_owner),
            hostility_context: (context.hostility_context != 0)
                .then_some(context.hostility_context),
            ignore_all: context.ignore_all,
            ignore_moving: context.ignore_moving,
            custom_filter: context.custom_filter != 0,
            mask: context.mask,
        },
    })
}

struct Live<'a, 'r, 'c, 's, 'm, S> {
    catalog: &'c Catalog<'a>,
    record: &'r mut Record<'s>,
    map: &'r mut MapFacts<'m>,
    services: &'r mut S,
    descriptor: Option<Descriptor<u32, ()>>,
    precision: Precision,
    empty_name: Option<String>,
}
impl<S: Services> Live<'_, '_, '_, '_, '_, S> {
    fn source_precision(&self) -> Result<Precision> {
        self.catalog.require_source_owner().map_err(fail)?;
        let bank = self.catalog.definition_bank.try_borrow().map_err(fail)?;
        let source = bank.combat_source().map_err(fail)?;
        if source.source_identities() != self.catalog.actors.sources.as_slice() {
            return Err(fail("placement source FP owner differs from Catalog"));
        }
        Ok(source.source_precision())
    }
    fn id(&self, token: u32) -> Result<String> {
        let bank = self.catalog.definition_bank.try_borrow().map_err(fail)?;
        let entry = bank.entry(token).map_err(fail)?;
        if entry.key.domain != "actordefinition" {
            return Err(fail("placement definition token domain differs"));
        }
        Ok(entry.key.name.clone())
    }
    fn token(&self, id: &str) -> Result<u32> {
        self.catalog
            .require_untranslated_actor_identity(id)
            .map_err(fail)?;
        self.catalog
            .definition_bank
            .try_borrow_mut()
            .map_err(fail)?
            .intern("ActorDefinition", id)
            .map_err(fail)
    }
    fn flags(&self, id: &str) -> Result<u32> {
        let expected = self
            .catalog
            .actors
            .native_final_flags(id, self.source_precision()?)
            .map_err(fail)?;
        let category = self.catalog.actors.native_category(id).map_err(fail)?;
        if category.registry_index == 31 {
            let fields=self.map.source_fields.ok_or_else(||fail("Placeholder flags require actual source-field writer owner"))?;
            let verified=self.catalog.verified_placeholder_flags(id,fields).map_err(fail)?;
            if verified!=expected{return Err(fail("Placeholder verified flags differ from actual held source"))}
        }
        if matches!(category.native_kind, 1 | 20 | 21) && expected & (1 << 23) != 0 {
            return Err(fail(
                "placement non-Actor ownable flags require completed49C460 verification storage",
            ));
        }
        if id.eq_ignore_ascii_case(&self.record.definition.proof.id)
            && self.record.definition.flags != expected
        {
            return Err(fail(
                "placement held definition flags differ from canonical source PC",
            ));
        }
        Ok(expected)
    }
    fn source_fields(
        &self,
        id: &str,
    ) -> Result<&aa_rules::native_source_requirements::NativeSourceRequirements> {
        let fields = self
            .map
            .source_fields
            .ok_or_else(|| fail("placement requires canonical completed source fields"))?;
        if fields.receipt().sources != self.catalog.actors.sources {
            return Err(fail(
                "placement source-field writer universe differs from Catalog",
            ));
        }
        if fields.source_precision() != self.source_precision()? {
            return Err(fail(
                "placement source-field arena PC differs from immutable startup source PC",
            ));
        }
        let bound = fields.definition(id).map_err(fail)?;
        let canonical = self
            .catalog
            .actors
            .find(id)
            .ok_or_else(|| fail("placement canonical source disappeared"))?;
        if !bound.token().canonical_id().eq_ignore_ascii_case(id)
            || bound.token().source() != &canonical.source
        {
            return Err(fail(
                "placement source-field arena differs from canonical source owner",
            ));
        }
        Ok(fields)
    }
    fn mask(&self, id: &str) -> Result<u32> {
        let expected = self
            .catalog
            .canonical_component_mask(id, self.source_precision()?)
            .map_err(fail)?;
        if id.eq_ignore_ascii_case(&self.record.definition.proof.id) {
            let held = self
                .record
                .definition
                .component_mask
                .ok_or_else(|| fail("placement requires completed current ActorData+174"))?;
            if held != expected {
                return Err(fail(
                    "placement held component mask differs from canonical source PC",
                ));
            }
        }
        Ok(expected)
    }
    fn shell(
        &self,
        id: ActorId,
    ) -> Result<
        &aa_actors::loading::LoadShell<
            aa_mapworld::bootstrap::DefinitionProof,
            aa_mapworld::restore::RestoredComponent,
        >,
    > {
        if id == self.record.owner {
            return Ok(self.record.shell);
        }
        let record = &self
            .record
            .registry
            .get(id)
            .ok_or_else(|| fail("placement actor GTID is stale"))?
            .value;
        use aa_mapworld::restore::RestoredRecord;
        match record {
            RestoredRecord::Actor(v) | RestoredRecord::Object(v) | RestoredRecord::Thing(v) => {
                Ok(&v.shell)
            }
            RestoredRecord::Team(v) => Ok(&v
                .runtime
                .as_ref()
                .ok_or_else(|| fail("placement Team has no constructed runtime"))?
                .base),
            RestoredRecord::Kingdom(v) => Ok(&v
                .runtime
                .as_ref()
                .ok_or_else(|| fail("placement Kingdom has no constructed runtime"))?
                .base),
            RestoredRecord::Loading => Err(fail("placement actor is held by another active Load")),
        }
    }
    fn desc(&self) -> Result<&Descriptor<u32, ()>> {
        self.descriptor
            .as_ref()
            .ok_or_else(|| fail("placement descriptor has not been constructed"))
    }
    fn source_shape(&mut self, d: u32, blocking: bool, p: Precision) -> Result<Shape> {
        let id = self.id(d)?;
        let category = self.catalog.actors.native_category(&id).map_err(fail)?;
        if category.native_kind == 0 && category.registry_index == 25 {
            if blocking {
                return Err(fail("TeamData has no ObjectData blocking view"));
            }
            let point = self
                .map
                .point_shape
                .ok_or_else(|| fail("TeamData4942F0 requires the actual global point owner"))?;
            point.validate_catalog(self.catalog).map_err(fail)?;
            return Ok(point.shape().clone());
        }
        let source = self
            .catalog
            .actors
            .native_object_definition(&id, self.source_precision()?)
            .map_err(fail)?;
        let words = source.blocking.words;
        let original = Shape {
            kind: words[0],
            radius_bits: words[1],
            bounds: [words[2], words[3], words[4], words[5]],
        };
        if blocking {
            return Ok(original);
        }
        //49BD10 performs its own +38 virtual and reads +174/+1A8 before flags.
        if self.definition_virtual(d, 0x38)?
            && (self.mask(&id)? & (1 << 12) != 0
                || self.source_fields(&id)?.byte(&id, 0x1a8).map_err(fail)? != 0)
        {
            return Ok(original);
        }
        let flags = self.flags(&id)?;
        if flags & 0x20000400 != 0x20000400 {
            return Ok(original);
        }
        let clearance = self
            .map
            .placement_clearance_bits
            .ok_or_else(|| fail("virtual28 requires current GameData+44 placement clearance"))?;
        expanded_blocking_shape(&original, clearance, p)
    }
}
impl<S: Services> World for Live<'_, '_, '_, '_, '_, S> {
    type Actor = ActorId;
    type Definition = u32;
    type Zone = u32;
    type Tiles = (Vec<[i32; 2]>, usize);
    fn descriptor_definition(&mut self) -> Result<u32> {
        Ok(self.desc()?.definition)
    }
    fn descriptor_word(&mut self, o: u32) -> Result<u32> {
        let d = self.desc()?;
        Ok(match o {
            8 | 0xc | 0x10 => d.position_bits[((o - 8) / 4) as usize],
            0x14 | 0x18 => d.facing_bits[((o - 0x14) / 4) as usize],
            0x20..=0x4c if o.is_multiple_of(4) => d.words_20_to_4c[((o - 0x20) / 4) as usize],
            _ => return Err(fail("placement descriptor word absent")),
        })
    }
    fn descriptor_byte(&mut self, o: u32) -> Result<u8> {
        self.desc()?
            .bytes_50_to_52
            .get(o.wrapping_sub(0x50) as usize)
            .copied()
            .ok_or_else(|| fail("placement descriptor byte absent"))
    }
    fn definition_virtual(&mut self, d: u32, slot: u32) -> Result<bool> {
        let id = self.id(d)?;
        let category = self.catalog.actors.native_category(&id).map_err(fail)?;
        if category.native_kind == 0 && category.registry_index == 25 {
            // Actual A20770 TeamData slots: +2C6A71F0, +30/+38/+3C4C9FA0.
            // The former returns AL0, the latter returns the full EAX0.
            return match slot {
                0x2c | 0x30 | 0x38 | 0x3c => Ok(false),
                _ => Err(fail("TeamData virtual slot is not bound")),
            };
        }
        if !matches!(category.native_kind, 1 | 2 | 20 | 21) {
            return Err(fail("placement definition virtual class is not bound"));
        }
        match slot {
            0x30 => Ok(true),
            0x38 => Ok(category.native_kind == 2),
            0x3c => Ok(false),
            0x2c => {
                let source_pc = self.source_precision()?;
                let space = |id: &str| -> Result<bool> {
                    self.catalog
                        .actors
                        .native_field(id, "needs_space_for_settlement_zone", source_pc)
                        .map_err(fail)?
                        .ok_or_else(|| {
                            fail("placement requires typed needs_space_for_settlement_zone")
                        })?
                        .boolean()
                        .map_err(fail)
                };
                if space(&id)? {
                    return Ok(true);
                }
                if category.native_kind != 2 {
                    return Ok(false);
                }
                if self.mask(&id)? & (1 << 14) != 0
                    || self.source_fields(&id)?.word(&id, 0x430).map_err(fail)? != 0
                {
                    return Ok(true);
                }
                let target = self
                    .source_fields(&id)?
                    .reference(&id, 0x4e0)
                    .map_err(fail)?;
                match target {
                    None => Ok(false),
                    Some(t) => space(t),
                }
            }
            _ => Err(fail("placement definition virtual slot is not bound")),
        }
    }
    fn definition_word(&mut self, d: u32, o: u32) -> Result<u32> {
        let id = self.id(d)?;
        match o {
            0x1c => self.flags(&id),
            0x174 => self.mask(&id),
            _ => self.source_fields(&id)?.word(&id, o).map_err(fail),
        }
    }
    fn definition_byte(&mut self, d: u32, o: u32) -> Result<u8> {
        let id = self.id(d)?;
        self.source_fields(&id)?.byte(&id, o).map_err(fail)
    }
    fn definition_shape(&mut self, d: u32, b: bool, p: Precision) -> Result<Shape> {
        self.source_shape(d, b, p)
    }
    fn definition_reference(&mut self, d: u32, o: u32) -> Result<Option<u32>> {
        let id = self.id(d)?;
        self.source_fields(&id)?
            .reference(&id, o)
            .map_err(fail)?
            .map(|id| self.token(id))
            .transpose()
    }
    fn actor_definition(&mut self, a: ActorId) -> Result<u32> {
        let id = self.shell(a)?.thing.definition.id.clone();
        self.token(&id)
    }
    fn actor_word(&mut self, a: ActorId, o: u32) -> Result<u32> {
        let s = self.shell(a)?;
        match o {
            8 => Ok(s.thing.flags),
            0x20 | 0x24 | 0x28 => Ok(s.thing.position_bits[((o - 0x20) / 4) as usize]),
            _ => s
                .actor
                .as_ref()
                .and_then(|a| a.words.get(&o))
                .copied()
                .ok_or_else(|| fail(format!("placement requires current Actor+{o:x}"))),
        }
    }
    fn map_settings_mode(&mut self, global: bool) -> Result<Option<u32>> {
        read_settings_mode(
            global,
            self.map.local_settings_mode,
            self.map.global_settings_mode,
            self.map.settings_mode_owner,
        )
    }
    fn map_cell_size(&mut self) -> Result<u32> {
        Ok(self.map.map.cell_size_bits)
    }
    fn outside_553370(&mut self, p: &Placement, pc: Precision) -> Result<bool> {
        aa_spatial::query::outside(p, self.map.map.extent_bits, pc)
    }
    fn required_zone_4f1660(
        &mut self,
        pos: [u32; 2],
        d: u32,
        o: u32,
        a: Option<ActorId>,
        a7: u32,
        a8: u32,
        p: Precision,
    ) -> Result<u32> {
        self.services
            .required_zone(self.record, pos, d, o, a, a7, a8, p)
    }
    fn begin_tiles(&mut self, p: &Placement, pc: Precision) -> Result<Self::Tiles> {
        let m = self.map.map;
        let cells = cp::candidate_tiles(
            &Grid {
                dimensions: [m.width, m.height],
                coordinate_shift: m.coordinate_shift,
                cell_size_bits: m.cell_size_bits,
            },
            m.extent_bits,
            &self.map.geometry.cell_shape,
            p,
            pc,
        )?;
        Ok((cells, 0))
    }
    fn tile_zone_54ef80(&mut self, t: &Self::Tiles) -> Result<Option<u32>> {
        let cell =
            t.0.get(t.1)
                .ok_or_else(|| fail("placement tile cursor exhausted"))?;
        let map = self
            .map
            .zones
            .ok_or_else(|| fail("placement requires actual constructed ZoneMap"))?;
        let i = map
            .index(cell[0], cell[1])
            .ok_or_else(|| fail("placement zone tile outside"))?;
        let result = aa_zones::query::tile_first_zone(
            &map.pool,
            map.tile(i).map_err(fail)?.set,
            self.services,
            2,
            0,
            0,
        )
        .map_err(fail)?;
        Ok((result != 0).then_some(result))
    }
    fn advance_tile(&mut self, t: &mut Self::Tiles) -> Result<bool> {
        t.1 += 1;
        Ok(t.1 < t.0.len())
    }
    fn tiles_valid(&mut self, t: &Self::Tiles) -> Result<bool> {
        Ok(t.1 < t.0.len())
    }
    fn zone_actor(&mut self, z: u32) -> Result<ActorId> {
        self.services.zone_actor(z)
    }
    fn zone_overlap_4f0ae0(&mut self, a: ActorId, p: &Placement, pc: Precision) -> Result<bool> {
        if a.is_null() {
            return Ok(false);
        }
        let s = self.shell(a)?;
        let count = s
            .actor
            .as_ref()
            .map(|a| a.zones_148.len())
            .ok_or_else(|| fail("4F0AE0 requires current Actor zone vector"))?;
        let count = u32::try_from(count).map_err(fail)?;
        if count as i32 <= 2 {
            return Ok(false);
        }
        if self.services.zone_slot(a.raw(), 2).map_err(fail)? == 0 {
            return Ok(false);
        }
        self.catalog.require_source_owner().map_err(fail)?;
        let radius = {
            let bank = self.catalog.definition_bank.try_borrow().map_err(fail)?;
            let source = bank.combat_source().map_err(fail)?;
            if source.source_identities() != self.catalog.actors.sources.as_slice() {
                return Err(fail("4F0AE0 manager source registry differs"));
            }
            source.completed_maximum_74().map_err(fail)?.0
        };
        let x = self.actor_word(a, 0x20)?;
        let y = self.actor_word(a, 0x24)?;
        aa_spatial::shape_overlap::overlaps(
            &Placement {
                shape: Shape {
                    kind: 1,
                    radius_bits: radius,
                    bounds: [0; 4],
                },
                facing: [0, 0xbf800000],
                position: [x, y],
            },
            p,
            pc,
        )
    }
    fn actor_category_id(&mut self, a: ActorId) -> Result<u32> {
        let d = self.actor_definition(a)?;
        let id = self.id(d)?;
        Ok(self
            .catalog
            .actors
            .native_category(&id)
            .map_err(fail)?
            .registry_index)
    }
    fn structure_at_4f2220(&mut self, p: [u32; 2]) -> Result<Option<ActorId>> {
        self.services.structure_at(self.record, p)
    }
    fn collision_4f0a70(
        &mut self,
        c: &mut Context<u32, ActorId>,
        d: u32,
        p: Precision,
    ) -> Result<u32> {
        self.services.collision(self.record, c, d, p)
    }
    fn footprint_4f0b70(&mut self) -> Result<u32> {
        let descriptor = self.desc()?.clone();
        self.services
            .footprint(self.record, &descriptor, self.precision)
    }
}
impl<S: Services> loaded::World for Live<'_, '_, '_, '_, '_, S> {
    type Object = ActorId;
    type Vector = ();
    type Definition = u32;
    type Name = ();
    fn empty_temporary_name(&mut self) -> Result<()> {
        if self.empty_name.is_some() {
            return Err(fail("saved placement temporary name already owned"));
        }
        self.empty_name = Some(String::new());
        Ok(())
    }
    fn definition(&mut self, a: ActorId) -> Result<u32> {
        self.actor_definition(a)
    }
    fn object_word(&mut self, a: ActorId, o: u32) -> Result<u32> {
        if o == 0x2c || o == 0x30 {
            return Ok(self.shell(a)?.thing.facing_bits[((o - 0x2c) / 4) as usize]);
        }
        self.actor_word(a, o)
    }
    fn vector_word(&mut self, _: (), _: u32) -> Result<u32> {
        Err(fail("saved placement never supplies a vector override"))
    }
    fn virtual_b4(&mut self, a: ActorId) -> Result<bool> {
        let shell = self.shell(a)?;
        let proof = &shell.thing.definition;
        self.catalog
            .require_untranslated_actor_identity(&proof.id)
            .map_err(fail)?;
        let canonical = self
            .catalog
            .actors
            .find(&proof.id)
            .ok_or_else(|| fail("placement B4 canonical source absent"))?;
        let category = self
            .catalog
            .actors
            .native_category(&proof.id)
            .map_err(fail)?;
        let class = category.runtime_class;
        let layout = match class {
            3 | 4 => {
                shell.kind == aa_actors::loading::LoadKind::Actor
                    && shell.object.is_some()
                    && shell.actor.is_some()
            }
            2 | 6 | 7 | 9 => {
                shell.kind == aa_actors::loading::LoadKind::Object
                    && shell.object.is_some()
                    && shell.actor.is_none()
            }
            5 => {
                shell.kind == aa_actors::loading::LoadKind::Thing
                    && shell.object.is_none()
                    && shell.actor.is_none()
            }
            _ => false,
        };
        if !layout
            || canonical.source != proof.source
            || canonical.template_chain != proof.template_chain
            || category != proof.category
            || (a == self.record.owner
                && (proof.id != self.record.definition.proof.id
                    || proof.source != self.record.definition.proof.source
                    || proof.template_chain != self.record.definition.proof.template_chain
                    || proof.category != self.record.definition.proof.category
                    || class != self.record.definition.native_class))
        {
            return Err(fail("placement B4 current shell/source/storage differs"));
        }
        match class {
            3 | 4 => Ok(true),
            // Actual Team constructor installs A29780; +B4 is6A71F0(AL0).
            2 | 6 | 7 | 9 | 5 => Ok(false),
            _ => Err(fail("saved placement object virtual+B4 class is not bound")),
        }
    }
    fn virtual_108(&mut self, a: ActorId) -> Result<u32> {
        if a != self.record.owner {
            return Err(fail("saved placement owner receiver differs"));
        }
        // Native Actor/Kingdom shared leaf4D1E80 is exactly [this+E8]. The
        // restore owner stores a canonical GTID here rather than a pointer.
        self.record
            .shell
            .actor
            .as_ref()
            .and_then(|a| a.words.get(&0xe8))
            .copied()
            .ok_or_else(|| fail("saved placement requires current Actor+E8 owner"))
    }
    fn coordinator_4f3640(
        &mut self,
        d: &mut Descriptor<u32, ()>,
        a: loaded::CoordinatorArguments<ActorId>,
        p: Precision,
    ) -> Result<u32> {
        self.descriptor = Some(d.clone());
        cp::check(
            Arguments {
                source: Some(a.source),
                other: Some(a.other),
                argument_5: a.zero_4,
                argument_6: a.zero_5,
                argument_7: a.argument_6,
                argument_8: if a.argument_7 == 0 {
                    None
                } else {
                    return Err(fail("saved placement final pointer argument is not null"));
                },
            },
            p,
            self,
        )
    }
    fn release_temporary_name(&mut self, _: ()) -> Result<()> {
        self.empty_name
            .take()
            .ok_or_else(|| fail("saved placement temporary name owner missing"))?;
        self.descriptor = None;
        Ok(())
    }
}

/// Saved NULL path, with one Rust-owned empty name token whose scope is this
/// local descriptor. No persistent string allocator or native pointer is faked.
pub fn check_restored<'a, 'r, 'c, 's, 'm, S: Services>(
    catalog: &'c Catalog<'a>,
    record: &'r mut Record<'s>,
    map: &'r mut MapFacts<'m>,
    services: &'r mut S,
    p: Precision,
) -> Result<u32> {
    let canonical = catalog
        .actors
        .find(&record.definition.proof.id)
        .ok_or_else(|| fail("placement source identity absent"))?;
    if !canonical
        .id
        .eq_ignore_ascii_case(&record.shell.thing.definition.id)
        || canonical.source != record.definition.proof.source
        || canonical.template_chain != record.definition.proof.template_chain
        || record.definition.proof.category.runtime_class != record.definition.native_class
    {
        return Err(fail(
            "placement canonical source/class differs from current shell",
        ));
    }
    if map.geometry.extent_bits != map.map.extent_bits {
        return Err(fail(
            "placement query geometry differs from current GMap extents",
        ));
    }
    let owner = record.owner;
    let mut world = Live {
        catalog,
        record,
        map,
        services,
        descriptor: None,
        precision: p,
        empty_name: None,
    };
    loaded::check_saved(owner, p, &mut world)
}
