//!4F1660 over the borrowed load shell, current registry, native tile lists,
//! constructed ZoneMap and source-owned TerrainData. No viewer geometry,
//! synthesized empty world or diagnostic manager radius authorizes placement.
use crate::{battle_terrain::BattleTerrain, catalog::Catalog, live_object_init::Record};
use aa_actors::{ActorId, loading::LoadShell};
use aa_mapworld::{
    bootstrap::DefinitionProof,
    restore::{RestoredComponent, RestoredRecord},
};
use aa_numeric::Precision;
use aa_spatial::{
    Error, Result,
    collision::Shape,
    creation_placement::candidate_tiles,
    geometry::Placement,
    gworld_placement::GMapView,
    index::{NodeKey, SpatialIndex},
    query::QueryGeometry,
    required_placement::{self as native, Arguments, World},
};
use aa_zones::{query::ZoneActors, tile::ZoneMap};

fn fail(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}

#[cfg(test)]
#[path = "live_required_placement_tests.rs"]
mod tests;

/// Immutable map owners are borrowed together for one operation. The index
/// supplies *current native-order list links*; shell fields remain in Record.
pub struct MapFacts<'a> {
    pub map: GMapView,
    pub geometry: &'a QueryGeometry,
    pub index: &'a SpatialIndex,
    pub zones: &'a ZoneMap,
    pub terrain: &'a BattleTerrain,
    pub source_fields: Option<&'a aa_rules::native_source_requirements::NativeSourceRequirements>,
}

/// Remaining exact owners. Parent/zone/hostility/component services must act
/// on this Record and its current registry; none has an implicit false/null.
/// ZoneActors keys must match the supplied ZoneMap's actor-key domain; word4
/// must return the same Catalog Bank's ActorDefinition token, not an ordinal,
/// GTID or a handle allocated by another definition arena.
pub trait Services: ZoneActors {
    fn actor_link(
        &mut self,
        record: &mut Record<'_>,
        actor: ActorId,
        slot: u32,
    ) -> Result<Option<ActorId>>;
    fn mobile_4d1120(&mut self, record: &mut Record<'_>, actor: ActorId) -> Result<bool>;
    fn enemy_140(&mut self, record: &mut Record<'_>, actor: ActorId, owner: u32) -> Result<bool>;
    fn zone_actor(&mut self, zone: u32) -> Result<ActorId>;
    fn game_mode_e4(&mut self) -> Result<u32>;
    fn current_thread_is_simulation(&mut self) -> Result<bool>;
    fn global_mode_b28158(&mut self) -> Result<u32>;
}

struct Live<'a, 'c, 'r, 's, 'm, S> {
    catalog: &'c Catalog<'a>,
    record: &'r mut Record<'s>,
    map: &'m MapFacts<'m>,
    services: &'r mut S,
}
impl<S: Services> Live<'_, '_, '_, '_, '_, S> {
    fn id(&self, token: u32) -> Result<String> {
        self.catalog.require_source_owner().map_err(fail)?;
        let bank = self.catalog.definition_bank.try_borrow().map_err(fail)?;
        let entry = bank.entry(token).map_err(fail)?;
        if entry.key.domain != "actordefinition" {
            return Err(fail("4F1660 definition domain differs"));
        }
        let id = entry.key.name.clone();
        self.catalog
            .require_untranslated_actor_identity(&id)
            .map_err(fail)?;
        Ok(id)
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
    fn source_fields(
        &self,
        id: &str,
    ) -> Result<&aa_rules::native_source_requirements::NativeSourceRequirements> {
        let fields = self
            .map
            .source_fields
            .ok_or_else(|| fail("4F1660 completed source-field owner missing"))?;
        let bound = fields.definition(id).map_err(fail)?;
        let registered = self
            .catalog
            .actors
            .find(id)
            .ok_or_else(|| fail("4F1660 canonical source missing"))?;
        if fields.source_precision() != self.source_precision()? {
            return Err(fail(
                "4F1660 source-field precision differs from the actual Bank source owner",
            ));
        }
        if fields.receipt().sources != self.catalog.actors.sources
            || !bound.token().canonical_id().eq_ignore_ascii_case(id)
            || bound.token().source() != &registered.source
        {
            return Err(fail("4F1660 completed source-field registry differs"));
        }
        Ok(fields)
    }
    fn source_precision(&self) -> Result<Precision> {
        self.catalog.require_source_owner().map_err(fail)?;
        let bank = self.catalog.definition_bank.try_borrow().map_err(fail)?;
        let source = bank.combat_source().map_err(fail)?;
        if source.source_identities() != self.catalog.actors.sources.as_slice() {
            return Err(fail("4F1660 source precision registry differs"));
        }
        Ok(source.source_precision())
    }
    fn shell(&self, actor: ActorId) -> Result<&LoadShell<DefinitionProof, RestoredComponent>> {
        if actor == self.record.owner {
            return Ok(self.record.shell);
        }
        let record = &self
            .record
            .registry
            .get(actor)
            .ok_or_else(|| fail("4F1660 object GTID is stale"))?
            .value;
        match record {
            RestoredRecord::Actor(v) | RestoredRecord::Object(v) | RestoredRecord::Thing(v) => {
                Ok(&v.shell)
            }
            RestoredRecord::Team(v) => Ok(&v
                .runtime
                .as_ref()
                .ok_or_else(|| fail("4F1660 Team runtime missing"))?
                .base),
            RestoredRecord::Kingdom(v) => Ok(&v
                .runtime
                .as_ref()
                .ok_or_else(|| fail("4F1660 Kingdom runtime missing"))?
                .base),
            RestoredRecord::Loading => {
                Err(fail("4F1660 object is borrowed by another active Load"))
            }
        }
    }
    fn category(&self, actor: ActorId) -> Result<aa_rules::native::CategoryDescriptor> {
        let source = &self.shell(actor)?.thing.definition;
        self.catalog
            .require_untranslated_actor_identity(&source.id)
            .map_err(fail)?;
        let registered = self
            .catalog
            .actors
            .find(&source.id)
            .ok_or_else(|| fail("4F1660 source owner missing"))?;
        let category = self
            .catalog
            .actors
            .native_category(&source.id)
            .map_err(fail)?;
        if registered.source != source.source
            || registered.template_chain != source.template_chain
            || category.registry_index != source.category.registry_index
            || category.runtime_class != source.category.runtime_class
        {
            return Err(fail("4F1660 actual shell definition provenance differs"));
        }
        Ok(category)
    }
    fn ordinary(&self, actor: ActorId) -> Result<aa_rules::native::CategoryDescriptor> {
        let category = self.category(actor)?;
        if !matches!(category.native_kind, 1 | 2 | 20 | 21)
            || !matches!(category.runtime_class, 2 | 3 | 4 | 6 | 7 | 9)
        {
            return Err(fail("4F1660 derived object virtual dispatch is not bound"));
        }
        let shell = self.shell(actor)?;
        let actor_class = matches!(category.runtime_class, 3 | 4);
        let expected_kind = if actor_class {
            aa_actors::loading::LoadKind::Actor
        } else {
            aa_actors::loading::LoadKind::Object
        };
        if shell.kind != expected_kind
            || shell.object.is_none()
            || shell.actor.is_some() != actor_class
        {
            return Err(fail(
                "4F1660 runtime shell/storage class differs from virtual dispatch",
            ));
        }
        let definition = self.runtime_definition(actor)?;
        let proof = &shell.thing.definition;
        if definition.native_class != category.runtime_class
            || definition.proof.category != category
            || !definition.proof.id.eq_ignore_ascii_case(&proof.id)
            || definition.proof.source != proof.source
            || definition.proof.template_chain != proof.template_chain
        {
            return Err(fail(
                "4F1660 runtime definition differs from its actual shell/source",
            ));
        }
        Ok(category)
    }
    fn runtime_definition(
        &self,
        actor: ActorId,
    ) -> Result<&aa_mapworld::restore::RuntimeDefinition> {
        if actor == self.record.owner {
            return Ok(self.record.definition);
        }
        let record = &self
            .record
            .registry
            .get(actor)
            .ok_or_else(|| fail("4F1660 runtime GTID stale"))?
            .value;
        match record {
            RestoredRecord::Actor(v) | RestoredRecord::Object(v) | RestoredRecord::Thing(v) => {
                Ok(&v.definition)
            }
            RestoredRecord::Team(v) => Ok(&v
                .runtime
                .as_ref()
                .ok_or_else(|| fail("4F1660 Team runtime missing"))?
                .definition),
            RestoredRecord::Kingdom(v) => Ok(&v
                .runtime
                .as_ref()
                .ok_or_else(|| fail("4F1660 Kingdom runtime missing"))?
                .definition),
            RestoredRecord::Loading => Err(fail("4F1660 runtime is held by another Load")),
        }
    }
    fn alive(&self, actor: ActorId) -> bool {
        self.record.registry.get(actor).is_some()
    }
    fn cell(tiles: &(Vec<[i32; 2]>, usize)) -> Result<[i32; 2]> {
        tiles
            .0
            .get(tiles.1)
            .copied()
            .ok_or_else(|| fail("4F1660 exhausted tile cursor"))
    }
    fn zone_overlap(
        &mut self,
        actor: ActorId,
        placement: &Placement,
        p: Precision,
    ) -> Result<bool> {
        self.ordinary(actor)?;
        let actor_data = self
            .shell(actor)?
            .actor
            .as_ref()
            .ok_or_else(|| fail("4F0AE0 zone actor lacks Actor storage"))?;
        let count = u32::try_from(actor_data.zones_148.len()).map_err(fail)? as i32;
        if count <= 2 {
            return Ok(false);
        }
        if self.services.zone_slot(actor.raw(), 2).map_err(fail)? == 0 {
            return Ok(false);
        }
        let radius = self.manager_radius_74()?;
        let s = self.shell(actor)?;
        aa_spatial::shape_overlap::overlaps(
            &Placement {
                shape: Shape {
                    kind: 1,
                    radius_bits: radius,
                    bounds: [0; 4],
                },
                facing: [0, 0xbf800000],
                position: [s.thing.position_bits[0], s.thing.position_bits[1]],
            },
            placement,
            p,
        )
    }
}
impl<S: Services> World for Live<'_, '_, '_, '_, '_, S> {
    type Definition = u32;
    type Actor = ActorId;
    type Node = NodeKey;
    type Zone = u32;
    type Tiles = (Vec<[i32; 2]>, usize);
    fn manager_radius_74(&mut self) -> Result<u32> {
        self.catalog.require_source_owner().map_err(fail)?;
        let bank = self.catalog.definition_bank.try_borrow().map_err(fail)?;
        let source = bank.combat_source().map_err(fail)?;
        if source.source_identities() != self.catalog.actors.sources.as_slice() {
            return Err(fail("4F1660 manager source registry differs"));
        }
        source
            .completed_maximum_74()
            .map(|value| value.0)
            .map_err(fail)
    }
    fn outside_553370(&mut self, placement: &Placement, p: Precision) -> Result<bool> {
        aa_spatial::query::outside(placement, self.map.map.extent_bits, p)
    }
    fn definition_is_actor(&mut self, d: u32) -> Result<bool> {
        let id = self.id(d)?;
        let c = self.catalog.actors.native_category(&id).map_err(fail)?;
        if !matches!(c.native_kind, 1 | 2 | 20 | 21) {
            return Err(fail("4F1660 definition virtual38 class unavailable"));
        }
        Ok(c.native_kind == 2)
    }
    fn definition_word(&mut self, d: u32, offset: u32) -> Result<u32> {
        let id = self.id(d)?;
        if offset == 0x174 {
            let source_precision = self.source_precision()?;
            if id.eq_ignore_ascii_case(&self.record.definition.proof.id) {
                let supplied = self
                    .record
                    .definition
                    .component_mask
                    .ok_or_else(|| fail("4F1660 current ActorData+174 incomplete"));
                let supplied = supplied?;
                let definition = self.record.definition;
                let registered = self
                    .catalog
                    .actors
                    .find(&id)
                    .ok_or_else(|| fail("4F1660 current source missing"))?;
                let category = self.catalog.actors.native_category(&id).map_err(fail)?;
                if definition.proof.source != registered.source
                    || definition.proof.template_chain != registered.template_chain
                    || definition.proof.category != category
                    || definition.native_class != category.runtime_class
                    || !self
                        .record
                        .shell
                        .thing
                        .definition
                        .id
                        .eq_ignore_ascii_case(&definition.proof.id)
                    || self.record.shell.thing.definition.source != definition.proof.source
                    || self.record.shell.thing.definition.template_chain
                        != definition.proof.template_chain
                    || self.record.shell.thing.definition.category != definition.proof.category
                {
                    return Err(fail(
                        "4F1660 current component-mask source provenance differs",
                    ));
                }
                let expected = self
                    .catalog
                    .canonical_component_mask(&id, source_precision)
                    .map_err(fail)?;
                if supplied != expected {
                    return Err(fail(
                        "4F1660 current component mask differs from its actual source-PC projection",
                    ));
                }
                return Ok(supplied);
            }
            return self
                .catalog
                .canonical_component_mask(&id, source_precision)
                .map_err(fail);
        }
        self.source_fields(&id)?.word(&id, offset).map_err(fail)
    }
    fn definition_byte(&mut self, d: u32, offset: u32) -> Result<u8> {
        let id = self.id(d)?;
        self.source_fields(&id)?.byte(&id, offset).map_err(fail)
    }
    fn definition_reference(&mut self, d: u32, offset: u32) -> Result<Option<u32>> {
        let id = self.id(d)?;
        self.source_fields(&id)?
            .reference(&id, offset)
            .map_err(fail)?
            .map(|id| self.token(id))
            .transpose()
    }
    fn actor_virtual(&mut self, actor: ActorId, slot: u32) -> Result<bool> {
        let category = self.ordinary(actor)?;
        match slot {
            0xb0 => Ok(category.runtime_class >= 2),
            0xb4 => Ok(matches!(category.runtime_class, 3 | 4)),
            _ => Err(fail("4F1660 actor virtual slot unavailable")),
        }
    }
    fn actor_link(&mut self, actor: ActorId, slot: u32) -> Result<Option<ActorId>> {
        self.ordinary(actor)?;
        let value = self.services.actor_link(self.record, actor, slot)?;
        if value.is_some_and(|id| !self.alive(id)) {
            return Err(fail("4F1660 virtual parent identity is stale"));
        }
        Ok(value)
    }
    fn actor_reference_50(&mut self, actor: ActorId) -> Result<u32> {
        Ok(self
            .shell(actor)?
            .object
            .as_ref()
            .ok_or_else(|| fail("4F1660 Object+50 unavailable"))?
            .words_50_to_54[0])
    }
    fn resolve_reference(&mut self, reference: u32) -> Result<Option<ActorId>> {
        let id = ActorId::from_raw(reference);
        Ok(self.record.registry.get(id).map(|e| e.id))
    }
    fn actor_definition(&mut self, actor: ActorId) -> Result<u32> {
        self.category(actor)?;
        let id = self.shell(actor)?.thing.definition.id.clone();
        self.token(&id)
    }
    fn actor_flags(&mut self, actor: ActorId) -> Result<u32> {
        Ok(self.shell(actor)?.thing.flags)
    }
    fn begin_tiles(&mut self, placement: &Placement, p: Precision) -> Result<Self::Tiles> {
        Ok((
            candidate_tiles(
                self.map.index.grid(),
                self.map.map.extent_bits,
                &self.map.geometry.cell_shape,
                placement,
                p,
            )?,
            0,
        ))
    }
    fn tiles_valid(&mut self, t: &Self::Tiles) -> Result<bool> {
        Ok(t.1 < t.0.len())
    }
    fn advance_tile(&mut self, t: &mut Self::Tiles) -> Result<()> {
        t.1 += 1;
        Ok(())
    }
    fn game_mode_e4(&mut self) -> Result<u32> {
        self.services.game_mode_e4()
    }
    fn current_thread_is_simulation(&mut self) -> Result<bool> {
        self.services.current_thread_is_simulation()
    }
    fn tile_zone_54ef80(
        &mut self,
        t: &Self::Tiles,
        a: Option<ActorId>,
        d: Option<u32>,
    ) -> Result<Option<u32>> {
        let c = Self::cell(t)?;
        let i = self
            .map
            .zones
            .index(c[0], c[1])
            .ok_or_else(|| fail("4F1660 zone tile outside"))?;
        let result = aa_zones::query::tile_first_zone(
            &self.map.zones.pool,
            self.map.zones.tile(i).map_err(fail)?.set,
            self.services,
            2,
            a.map_or(0, ActorId::raw),
            d.unwrap_or(0),
        )
        .map_err(fail)?;
        Ok((result != 0).then_some(result))
    }
    fn zone_actor(&mut self, z: u32) -> Result<ActorId> {
        let actor = self.services.zone_actor(z)?;
        if !self.alive(actor) {
            return Err(fail("4F1660 zone actor identity is stale"));
        }
        Ok(actor)
    }
    fn zone_overlap_4f0ae0(&mut self, a: ActorId, p: &Placement, pc: Precision) -> Result<bool> {
        self.zone_overlap(a, p, pc)
    }
    fn tile_terrain_flags_4c(&mut self, t: &Self::Tiles) -> Result<u8> {
        let c = Self::cell(t)?;
        self.map
            .terrain
            .passability_bits_at(c[0], c[1])
            .map_err(fail)
    }
    fn first_node(&mut self, t: &Self::Tiles) -> Result<Option<NodeKey>> {
        Ok(self.map.index.tile(Self::cell(t)?)?.first_node)
    }
    fn node_actor(&mut self, node: NodeKey) -> Result<ActorId> {
        let key = self.map.index.node(node)?.actor;
        let id = ActorId::from_raw(self.map.index.reference_for_key(key)?);
        if !self.alive(id) {
            return Err(fail("4F1660 tile list object GTID is stale"));
        }
        Ok(id)
    }
    fn next_node(&mut self, node: NodeKey) -> Result<Option<NodeKey>> {
        Ok(self.map.index.node(node)?.next)
    }
    fn actor_overlap(
        &mut self,
        actor: ActorId,
        placement: &Placement,
        p: Precision,
    ) -> Result<bool> {
        self.ordinary(actor)?;
        let source_precision = self.source_precision()?;
        let shell = self.shell(actor)?;
        let id = &shell.thing.definition.id;
        let source = self
            .catalog
            .actors
            .native_object_definition(id, source_precision)
            .map_err(fail)?;
        let w = source.blocking.words;
        aa_spatial::shape_overlap::overlaps(
            placement,
            &Placement {
                shape: Shape {
                    kind: w[0],
                    radius_bits: w[1],
                    bounds: [w[2], w[3], w[4], w[5]],
                },
                facing: [shell.thing.facing_bits[0], shell.thing.facing_bits[1]],
                position: [shell.thing.position_bits[0], shell.thing.position_bits[1]],
            },
            p,
        )
    }
    fn actor_mobile_4d1120(&mut self, a: ActorId) -> Result<bool> {
        self.services.mobile_4d1120(self.record, a)
    }
    fn actor_enemy_140(&mut self, a: ActorId, o: u32) -> Result<bool> {
        self.services.enemy_140(self.record, a, o)
    }
    fn global_mode_b28158(&mut self) -> Result<u32> {
        self.services.global_mode_b28158()
    }
}

/// Borrow current ownership for the entire child. The GMap, index, zone map
/// and terrain must describe the same constructed allocation and precision.
/// The caller owns that association; shape/dimension checks reject conflicting
/// views but do not authenticate an unrelated map with identical dimensions.
pub fn check<S: Services>(
    catalog: &Catalog<'_>,
    record: &mut Record<'_>,
    map: &MapFacts<'_>,
    services: &mut S,
    arguments: Arguments<u32, ActorId>,
    p: Precision,
) -> Result<u32> {
    let grid = map.index.grid();
    let terrain = &map.terrain.grid;
    let zone = map.zones.geometry;
    if grid.dimensions != [map.map.width, map.map.height]
        || grid.coordinate_shift != map.map.coordinate_shift
        || grid.cell_size_bits != map.map.cell_size_bits
        || map.index.precision() != p
        || map.geometry.extent_bits != map.map.extent_bits
        || zone.extent_bits != map.map.extent_bits
        || [zone.width, zone.height] != grid.dimensions
        || zone.shift != grid.coordinate_shift
        || zone.tile_size_bits != grid.cell_size_bits
        || [terrain.width, terrain.height] != grid.dimensions
        || terrain.coordinate_shift != grid.coordinate_shift
        || terrain.cell_size != grid.cell_size_bits
    {
        return Err(fail("4F1660 map/index/zone/terrain owners disagree"));
    }
    native::check(
        arguments,
        p,
        &mut Live {
            catalog,
            record,
            map,
            services,
        },
    )
}
