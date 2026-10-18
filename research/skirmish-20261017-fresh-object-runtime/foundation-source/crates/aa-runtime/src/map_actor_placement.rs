//! Actual nonNULL4EF6A0/4F3640 over one ActorStore, source Bank and adopted index.
//! This view reads the caller's original Params. Publishing the placement loan
//! registers an identity/shape only; native tile membership remains4EE4C0 later.
use crate::{
    actor_world::{ActorStore, ObjectKey, RuntimeObject},
    catalog::Catalog,
    definitions::Bank,
    live_placement::{self, CreationServices, MapFacts, PlacementRecord},
    map_actor_init_existing::Params,
    restoration::Staged,
    spatial_world::SpatialBindings,
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_numeric::Precision;
use aa_spatial::{
    Error, Result,
    collision::{self, Descriptor},
    creation_placement::Context,
    index::{ActorKey, SpatialIndex},
    index_world::CollisionPolicy,
    loaded_object_placement::Descriptor as SavedDescriptor,
    query::{QueryGeometry, QueryPolicy},
    source_exclusion,
};
use aa_zones::query::ZoneActors;
fn fail(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn required<T>(e: &str) -> Result<T> {
    Err(fail(e))
}
fn collision_fail(e: impl std::fmt::Display) -> collision::Error {
    collision::Error::World(e.to_string())
}

struct Actors<'a, 'source> {
    catalog: &'a Catalog<'source>,
    store: &'a ActorStore,
    bank: &'a Bank,
    fields: &'a aa_rules::native_source_requirements::NativeSourceRequirements,
    bindings: SpatialBindings<'a>,
}
impl Actors<'_, '_> {
    fn pc(&self) -> Result<Precision> {
        let source = self.bank.combat_source().map_err(fail)?;
        source.completed_means().map_err(fail)?;
        if source.source_identities() != self.catalog.actors.sources.as_slice()
            || self.fields.receipt().sources != self.catalog.actors.sources
            || self.fields.source_precision() != source.source_precision()
        {
            return required("fresh placement actual Bank/source fields differ");
        }
        Ok(source.source_precision())
    }
    fn object(&self, id: ActorId) -> Result<&RuntimeObject> {
        let key = self
            .store
            .key(id)
            .ok_or_else(|| fail("fresh placement GTID generation is stale"))?;
        let object = self.store.object(key).map_err(fail)?;
        let d = &object.definition;
        if !crate::load_spatial::same_proof(&object.thing.definition, &d.proof)
            || !crate::load_spatial::ordinary_layout(
                d.native_class,
                object.kind,
                object.object.is_some(),
                object.actor.is_some(),
            )
            || d.native_class != d.proof.category.runtime_class
        {
            return required("fresh placement actual allocation/source/layout differs");
        }
        let canonical = self
            .catalog
            .actors
            .find(&d.proof.id)
            .ok_or_else(|| fail("fresh placement canonical source absent"))?;
        if canonical.source != d.proof.source
            || canonical.template_chain != d.proof.template_chain
            || self
                .catalog
                .actors
                .native_category(&d.proof.id)
                .map_err(fail)?
                != d.proof.category
        {
            return required("fresh placement canonical proof differs");
        }
        let flags = self.flags_id(&d.proof.id)?;
        if d.flags != flags {
            return required("fresh placement allocation definition flags differ");
        }
        Ok(object)
    }
    fn id(&self, token: u32) -> Result<&str> {
        let entry = self.bank.require_usable(token).map_err(fail)?;
        if entry.key.domain != "actordefinition" {
            return required("fresh placement definition token domain differs");
        }
        Ok(&entry.key.name)
    }
    fn token(&self, id: &str) -> Result<u32> {
        self.catalog
            .require_untranslated_actor_identity(id)
            .map_err(fail)?;
        let token = self.bank.token("ActorDefinition", id).ok_or_else(|| {
            fail("fresh placement canonical definition token absent in same Bank")
        })?;
        self.id(token)?;
        Ok(token)
    }
    fn flags_id(&self, id: &str) -> Result<u32> {
        let expected = self
            .catalog
            .actors
            .native_final_flags(id, self.pc()?)
            .map_err(fail)?;
        if self
            .catalog
            .actors
            .native_category(id)
            .map_err(fail)?
            .registry_index
            == 31
            && self
                .catalog
                .verified_placeholder_flags_in_bank(self.bank, id, self.fields)
                .map_err(fail)?
                != expected
        {
            return required("fresh placement verified Placeholder flags differ");
        }
        Ok(expected)
    }
    fn key(&self, id: ActorId) -> Result<ActorKey> {
        self.object(id)?;
        self.bindings
            .key(
                self.store
                    .key(id)
                    .ok_or_else(|| fail("fresh placement GTID retired"))?,
            )
            .map_err(fail)
    }
    fn actor(&self, key: ActorKey) -> Result<ActorId> {
        let object = self.bindings.object(key).map_err(fail)?;
        let id = self
            .store
            .id(object)
            .ok_or_else(|| fail("fresh placement spatial generation retired"))?;
        self.object(id)?;
        Ok(id)
    }
    fn member<'a>(
        &self,
        index: &'a SpatialIndex,
        id: ActorId,
    ) -> Result<&'a aa_spatial::index::Member> {
        let key = self.key(id)?;
        let member = index.member(key)?;
        if member.reference != id.raw()
            || index.resolve_reference(id.raw()) != Some(key)
            || !self
                .id(member.actor.definition)?
                .eq_ignore_ascii_case(&self.object(id)?.definition.proof.id)
        {
            return required("fresh placement spatial/source identity differs");
        }
        Ok(member)
    }
    fn actor_data(
        &self,
        id: ActorId,
    ) -> Result<&aa_actors::loading::ActorShell<crate::actor_world::RuntimeComponent>> {
        self.object(id)?
            .actor
            .as_ref()
            .ok_or_else(|| fail("fresh placement required Actor storage absent"))
    }
    fn owner(&self, id: ActorId) -> Result<Option<u32>> {
        let object = self.object(id)?;
        match object.definition.native_class {
            2 | 6 | 7 | 9 => Ok(None),
            3 | 4 => {
                let raw = *self
                    .actor_data(id)?
                    .words
                    .get(&0xe8)
                    .ok_or_else(|| fail("fresh placement Actor+E8 absent"))?;
                if raw != 0 && self.object(ActorId::from_raw(raw))?.definition.native_class != 4 {
                    return required("fresh placement owner is not actual Kingdom");
                }
                Ok((raw != 0).then_some(raw))
            }
            _ => required("fresh placement virtual108 dispatch"),
        }
    }
    fn parent(&self, id: ActorId) -> Result<Option<ActorId>> {
        let object = self.object(id)?;
        if matches!(object.definition.native_class, 2 | 6 | 7 | 9) {
            return Ok(None);
        }
        let actor = self.actor_data(id)?;
        if let Some(crate::actor_world::RuntimeComponent::Structure(s)) =
            actor.components.get(ComponentSlot::Structure)
        {
            if s.header.owner != id {
                return required("fresh placement Structure owner differs");
            }
            if s.object_handle_10 != 0 {
                return required("fresh placement positive Structure+10 parent");
            }
        }
        if actor.components.present(ComponentSlot::Wall) {
            return required("fresh placement positive58CB30 Wall parent");
        }
        Ok(None)
    }
}
struct View<'a, 'source> {
    actors: Actors<'a, 'source>,
    index: &'a mut SpatialIndex,
    current: ActorId,
    params: &'a Params,
}
impl PlacementRecord for View<'_, '_> {
    fn current_owner(&self) -> ActorId {
        self.current
    }
    fn current_definition(&self) -> &aa_mapworld::restore::RuntimeDefinition {
        &self
            .actors
            .store
            .object(
                self.actors
                    .store
                    .key(self.current)
                    .expect("validated current identity"),
            )
            .expect("validated current allocation")
            .definition
    }
    fn proof_of(&self, id: ActorId) -> Result<&aa_mapworld::bootstrap::DefinitionProof> {
        Ok(&self.actors.object(id)?.thing.definition)
    }
    fn word_of(&self, id: ActorId, o: u32) -> Result<u32> {
        let object = self.actors.object(id)?;
        match o {
            8 => Ok(self.actors.member(self.index, id)?.actor.flags),
            0x20 | 0x24 => {
                Ok(self.actors.member(self.index, id)?.actor.position[((o - 0x20) / 4) as usize])
            }
            0x28 => Ok(self.actors.member(self.index, id)?.height_bits),
            0x2c | 0x30 => {
                Ok(self.actors.member(self.index, id)?.actor.facing[((o - 0x2c) / 4) as usize])
            }
            0x44 | 0x48 => Ok(
                self.actors.member(self.index, id)?.cached_tile[((o - 0x44) / 4) as usize] as u32
            ),
            0x50 | 0x54 => Ok(object
                .object
                .as_ref()
                .ok_or_else(|| fail("fresh Object storage absent"))?
                .words_50_to_54[((o - 0x50) / 4) as usize]),
            _ => object
                .actor
                .as_ref()
                .and_then(|a| a.words.get(&o))
                .copied()
                .ok_or_else(|| fail(format!("fresh placement Actor+{o:x} absent"))),
        }
    }
    fn zone_count_of(&self, id: ActorId) -> Result<u32> {
        u32::try_from(self.actors.actor_data(id)?.zones_148.len()).map_err(fail)
    }
    fn with_bank<T>(&self, _: &Catalog<'_>, read: impl FnOnce(&Bank) -> Result<T>) -> Result<T> {
        read(self.actors.bank)
    }
    fn token(&self, _: &Catalog<'_>, id: &str) -> Result<u32> {
        self.actors.token(id)
    }
    fn descriptor_definition(&self, saved: Option<&SavedDescriptor<u32, ()>>) -> Result<u32> {
        if saved.is_some() {
            return required("fresh descriptor cannot be replaced by saved wrapper");
        }
        Ok(self.params.definition.token)
    }
    fn descriptor_word(&self, saved: Option<&SavedDescriptor<u32, ()>>, o: u32) -> Result<u32> {
        if saved.is_some() {
            return required("fresh descriptor cannot be replaced by saved wrapper");
        }
        Ok(match o {
            8 | 0xc | 0x10 => self.params.position_bits[((o - 8) / 4) as usize],
            0x14 | 0x18 => self.params.direction_bits[((o - 0x14) / 4) as usize],
            0x20 => self.params.variant,
            0x24 => self.params.requested_id,
            0x28 => self.params.owner,
            0x2c => match &self.params.organization_layout {
                None => 0,
                Some(_) => return required("fresh positive Layout pointer token required"),
            },
            0x30 => self.params.word_30,
            0x34 => self.params.word_34,
            0x38 => self.params.mode,
            0x3c => self.params.word_3c,
            0x40 => self.params.word_40,
            0x44 => self.params.word_44,
            0x48 => self.params.word_48,
            0x4c => self.params.word_4c,
            _ => return required("fresh descriptor unsupported native word"),
        })
    }
    fn descriptor_byte(&self, saved: Option<&SavedDescriptor<u32, ()>>, o: u32) -> Result<u8> {
        if saved.is_some() {
            return required("fresh descriptor cannot be replaced by saved wrapper");
        }
        match o {
            0x50 => Ok(self.params.byte_50),
            0x51 => Ok(self.params.byte_51),
            0x52 => Ok(self.params.byte_52),
            _ => required("fresh descriptor unsupported native byte"),
        }
    }
    fn validate_current(&self, catalog: &Catalog<'_>) -> Result<()> {
        catalog.require_source_owner().map_err(fail)?;
        let actual = self.actors.object(self.current)?;
        crate::live_object_init::Source::bind_in_bank(
            catalog,
            self.actors.bank,
            &actual.definition,
        )
        .map_err(fail)?;
        if self.actors.token(&actual.definition.proof.id)? != self.params.definition.token {
            return required("fresh Params/current source definition differs");
        }
        self.actors.member(self.index, self.current)?;
        Ok(())
    }
}
struct Children<'a, 'source> {
    geometry: &'a QueryGeometry,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    identities: &'a crate::load_detection_activation::Identities,
    store: &'a ActorStore,
}
impl ZoneActors for Children<'_, '_> {
    fn owner(&mut self, _: u32) -> aa_zones::Result<u32> {
        Err(aa_zones::Error::World(
            "fresh placement zone owner service required".into(),
        ))
    }
    fn allied(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
        Err(aa_zones::Error::World(
            "fresh placement zone alliance required".into(),
        ))
    }
    fn enemy(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
        Err(aa_zones::Error::World(
            "fresh placement zone hostility required".into(),
        ))
    }
    fn zone_slot(&mut self, raw: u32, slot: u32) -> aa_zones::Result<u32> {
        let id = ActorId::from_raw(raw);
        let key = self
            .store
            .key(id)
            .ok_or_else(|| aa_zones::Error::World("fresh zone owner generation stale".into()))?;
        let object = self
            .store
            .object(key)
            .map_err(|e| aa_zones::Error::World(e.to_string()))?;
        crate::live_object_init::Source::bind_in_bank(self.catalog, self.bank, &object.definition)
            .map_err(|e| aa_zones::Error::World(e.to_string()))?;
        if !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
            || !matches!(object.definition.native_class, 3 | 4)
            || !crate::load_spatial::ordinary_layout(
                object.definition.native_class,
                object.kind,
                object.object.is_some(),
                object.actor.is_some(),
            )
        {
            return Err(aa_zones::Error::World(
                "fresh zone allocation/source/layout differs".into(),
            ));
        }
        let actor = object
            .actor
            .as_ref()
            .ok_or_else(|| aa_zones::Error::World("fresh zone Actor storage absent".into()))?;
        let Some(Some(zone)) = actor.zones_148.zones().slots.get(slot as usize) else {
            return Ok(0);
        };
        if self.identities.allocation(zone.key) != Some((id, slot)) {
            return Err(aa_zones::Error::World(
                "fresh zone allocation/owner identity differs".into(),
            ));
        }
        Ok(zone.key)
    }
    fn word4(&mut self, _: u32) -> aa_zones::Result<u32> {
        Err(aa_zones::Error::World(
            "fresh zone source definition pointer domain required".into(),
        ))
    }
    fn component_b0(&mut self, _: u32) -> aa_zones::Result<u32> {
        Err(aa_zones::Error::World(
            "fresh zone componentB0 required".into(),
        ))
    }
    fn reveal_mask(&mut self, _: u32) -> aa_zones::Result<Option<u32>> {
        Err(aa_zones::Error::World(
            "fresh zone reveal mask required".into(),
        ))
    }
}
impl CreationServices<View<'_, '_>> for Children<'_, '_> {
    fn required_zone(
        &mut self,
        _: &mut View<'_, '_>,
        _: [u32; 2],
        _: u32,
        _: u32,
        _: Option<ActorId>,
        _: u32,
        _: u32,
        _: Precision,
    ) -> Result<u32> {
        required("fresh4F1660 required-zone owner")
    }
    fn zone_actor(&mut self, key: u32) -> Result<ActorId> {
        let (actor, slot) = self
            .identities
            .allocation(key)
            .ok_or_else(|| fail("fresh zone allocation identity absent"))?;
        if self.zone_slot(actor.raw(), slot).map_err(fail)? != key {
            return required("fresh zone actual allocation differs");
        }
        Ok(actor)
    }
    fn structure_at(&mut self, _: &mut View<'_, '_>, _: [u32; 2]) -> Result<Option<ActorId>> {
        required("fresh4F2220 Structure query owner")
    }
    fn collision(
        &mut self,
        record: &mut View<'_, '_>,
        context: &mut Context<u32, ActorId>,
        d: u32,
        _: Precision,
    ) -> Result<u32> {
        let prepared = live_placement::prepare_collision(context, |id| record.actors.key(id))?;
        let class = record
            .actors
            .catalog
            .actors
            .native_category(record.actors.id(d)?)
            .map_err(fail)?;
        let mut policy = Policy {
            actors: &record.actors,
        };
        live_placement::collision(record.index, self.geometry, &prepared, &mut policy, || {
            match class.native_kind {
                1 | 20 | 21 => Ok(false),
                2 => Ok(true),
                _ => required("fresh definition virtual38 class"),
            }
        })
    }
    fn footprint(
        &mut self,
        _: &mut View<'_, '_>,
        saved: Option<&SavedDescriptor<u32, ()>>,
        _: Precision,
    ) -> Result<u32> {
        if saved.is_some() {
            return required("fresh footprint got saved descriptor");
        }
        required("fresh4F0B70 footprint validation owner")
    }
}
struct Policy<'a, 'b, 'source> {
    actors: &'a Actors<'b, 'source>,
}
impl QueryPolicy for Policy<'_, '_, '_> {
    fn definition_has_object_data(&self, d: u32) -> collision::Result<bool> {
        (|| -> Result<_> {
            match self
                .actors
                .catalog
                .actors
                .native_category(self.actors.id(d)?)
                .map_err(fail)?
                .native_kind
            {
                1 | 2 | 20 | 21 => Ok(true),
                0 => Ok(false),
                _ => required("fresh definition virtual30 class"),
            }
        })()
        .map_err(collision_fail)
    }
}
impl CollisionPolicy for Policy<'_, '_, '_> {
    fn definition_flags(&self, d: u32) -> collision::Result<u32> {
        (|| -> Result<_> { self.actors.flags_id(self.actors.id(d)?) })().map_err(collision_fail)
    }
    fn definition_flag_1a8(&self, d: u32) -> collision::Result<bool> {
        (|| -> Result<_> {
            self.actors.pc()?;
            Ok(self
                .actors
                .fields
                .byte(self.actors.id(d)?, 0x1a8)
                .map_err(fail)?
                != 0)
        })()
        .map_err(collision_fail)
    }
    fn custom_filter(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<bool> {
        (|| -> Result<_> { required("fresh custom collision function") })().map_err(collision_fail)
    }
    fn source_excludes(
        &mut self,
        index: &mut SpatialIndex,
        source: ActorKey,
        candidate: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<bool> {
        (|| -> Result<_> {
            let source = self.actors.actor(source)?;
            let candidate = self.actors.actor(candidate)?;
            if source == candidate {
                return Ok(true);
            }
            let object = self.actors.object(source)?;
            let link = object
                .object
                .as_ref()
                .ok_or_else(|| fail("fresh source Object absent"))?
                .words_50_to_54[0];
            if link != 0
                && self.actors.store.key(ActorId::from_raw(link))
                    == self.actors.store.key(candidate)
            {
                return Ok(true);
            }
            if matches!(object.definition.native_class, 2 | 6 | 7 | 9) {
                return Ok(false);
            }
            if matches!(
                self.actors.object(candidate)?.definition.native_class,
                2 | 6 | 7 | 9
            ) {
                let actor = self.actors.actor_data(source)?;
                let mut node = actor.overlap_list_110.head();
                let mut seen = std::collections::BTreeSet::new();
                while let Some(id) = node {
                    if !seen.insert(id.raw()) {
                        return required("cyclic fresh overlap list");
                    }
                    let n = actor.overlap_list_110.node_by_raw(id.raw()).map_err(fail)?;
                    if n.entry.reference == candidate.raw() {
                        return Ok(true);
                    }
                    node = n.next;
                }
                let sm = self.actors.member(index, source)?;
                let cm = self.actors.member(index, candidate)?;
                let required = self
                    .actors
                    .fields
                    .reference(self.actors.id(sm.actor.definition)?, 0x4e0)
                    .map_err(fail)?
                    .map(|id| self.actors.token(id))
                    .transpose()?;
                return Ok(required == Some(cm.actor.definition)
                    && sm
                        .actor
                        .position
                        .into_iter()
                        .zip(cm.actor.position)
                        .all(|(a, b)| f32::from_bits(a) == f32::from_bits(b)));
            }
            source_exclusion::excludes(
                source,
                candidate,
                &mut Exclusion {
                    actors: self.actors,
                    index,
                    source,
                },
            )
            .map_err(fail)
        })()
        .map_err(collision_fail)
    }
    fn allegiance(
        &mut self,
        _: &mut SpatialIndex,
        a: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<Option<u32>> {
        (|| -> Result<_> { self.actors.owner(self.actors.actor(a)?) })().map_err(collision_fail)
    }
    fn hostility_dispatch(&self, _: u32) -> collision::Result<u32> {
        (|| -> Result<_> { required("fresh actual Kingdom hostility dispatch") })()
            .map_err(collision_fail)
    }
    fn hostile(
        &mut self,
        _: &mut SpatialIndex,
        _: u32,
        _: Option<u32>,
        _: Option<u32>,
        _: &mut Descriptor,
    ) -> collision::Result<bool> {
        (|| -> Result<_> { required("fresh actual Kingdom relation owner") })()
            .map_err(collision_fail)
    }
    fn moving_exclusion(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<bool> {
        (|| -> Result<_> { required("fresh4CEDE0 actual moving dispatch") })()
            .map_err(collision_fail)
    }
    fn outer_actor(
        &mut self,
        _: &mut SpatialIndex,
        a: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<Option<u32>> {
        (|| -> Result<_> {
            self.actors
                .parent(self.actors.actor(a)?)?
                .map(|id| self.actors.key(id))
                .transpose()
        })()
        .map_err(collision_fail)
    }
}
struct Exclusion<'a, 'b, 'source> {
    actors: &'a Actors<'b, 'source>,
    index: &'a SpatialIndex,
    source: ActorId,
}
impl source_exclusion::World for Exclusion<'_, '_, '_> {
    type Actor = ActorId;
    type Mind = ActorId;
    type AcceptanceDispatch = u32;
    type Owner = Option<u32>;
    fn actor(&self, id: ActorId) -> collision::Result<source_exclusion::Actor> {
        (|| -> Result<_> {
            let actor = self.actors.actor_data(id)?;
            let m = self.actors.member(self.index, id)?;
            Ok(source_exclusion::Actor {
                definition: m.actor.definition,
                reference: id.raw(),
                flags: m.actor.flags,
                position: m.actor.position,
                linked_50: self
                    .actors
                    .object(id)?
                    .object
                    .as_ref()
                    .ok_or_else(|| fail("fresh exclusion Object absent"))?
                    .words_50_to_54[0],
                linked_168: *actor
                    .words
                    .get(&0x168)
                    .ok_or_else(|| fail("fresh Actor168 absent"))?,
                has_element: actor.components.present(ComponentSlot::Element),
                has_wall: actor.components.present(ComponentSlot::Wall),
            })
        })()
        .map_err(collision_fail)
    }
    fn definition(&self, d: u32) -> collision::Result<source_exclusion::Definition> {
        (|| -> Result<_> {
            let id = self.actors.id(d)?;
            self.actors.pc()?;
            let required_object = self
                .actors
                .fields
                .reference(id, 0x4e0)
                .map_err(fail)?
                .map(|id| self.actors.token(id))
                .transpose()?;
            let mask = self
                .actors
                .catalog
                .canonical_component_mask(id, self.actors.pc()?)
                .map_err(fail)?;
            let gate = if mask & (1 << 12) != 0 {
                self.actors
                    .catalog
                    .actors
                    .native_field(id, "gate", self.actors.pc()?)
                    .map_err(fail)?
                    .ok_or_else(|| fail("fresh Wall gate typed source absent"))?
                    .boolean()
                    .map_err(fail)?
            } else {
                false
            };
            Ok(source_exclusion::Definition {
                required_object,
                settlement_wall: self.actors.fields.byte(id, 0x1a8).map_err(fail)? != 0,
                gate,
            })
        })()
        .map_err(collision_fail)
    }
    fn resolve(&self, raw: u32) -> collision::Result<Option<ActorId>> {
        (|| -> Result<_> {
            let id = ActorId::from_raw(raw);
            Ok(self.actors.store.key(id).map(|_| id))
        })()
        .map_err(collision_fail)
    }
    fn exclusion_head(&self, id: ActorId) -> collision::Result<Option<u32>> {
        (|| -> Result<_> {
            if id != self.source {
                return required("fresh overlap owner differs");
            }
            let a = self.actors.actor_data(id)?;
            if a.words.contains_key(&0x110) || a.words.contains_key(&0x114) {
                return required("obsolete raw overlap authority");
            }
            a.overlap_list_110
                .head()
                .map(|n| u32::try_from(n.raw()).map_err(fail))
                .transpose()
        })()
        .map_err(collision_fail)
    }
    fn exclusion_node(&self, key: u32) -> collision::Result<source_exclusion::ExclusionNode> {
        (|| -> Result<_> {
            let n = self
                .actors
                .actor_data(self.source)?
                .overlap_list_110
                .node_by_raw(key as usize)
                .map_err(fail)?;
            Ok(source_exclusion::ExclusionNode {
                reference: n.entry.reference,
                next: n
                    .next
                    .map(|n| u32::try_from(n.raw()).map_err(fail))
                    .transpose()?,
            })
        })()
        .map_err(collision_fail)
    }
    fn actor_category(&mut self, id: ActorId) -> collision::Result<bool> {
        (|| -> Result<_> {
            match self.actors.object(id)?.definition.native_class {
                3 | 4 => Ok(true),
                2 | 6 | 7 | 9 => Ok(false),
                _ => required("fresh candidate B4 dispatch"),
            }
        })()
        .map_err(collision_fail)
    }
    fn acceptance_dispatch(&self, id: ActorId) -> collision::Result<u32> {
        (|| -> Result<_> { Ok(self.actors.object(id)?.definition.native_class) })()
            .map_err(collision_fail)
    }
    fn owner(&mut self, id: ActorId) -> collision::Result<Option<u32>> {
        (|| -> Result<_> { self.actors.owner(id) })().map_err(collision_fail)
    }
    fn accepts_owner(
        &mut self,
        class: u32,
        id: ActorId,
        owner: Option<u32>,
    ) -> collision::Result<bool> {
        (|| -> Result<_> {
            if class != 3 {
                return required("fresh derived owner acceptance dispatch");
            }
            source_exclusion::ordinary_accepts_owner(self.actors.owner(id)?, owner, || {
                Err(collision_fail("fresh owner relation144"))
            })
            .map_err(fail)
        })()
        .map_err(collision_fail)
    }
    fn organization(&self, id: ActorId) -> collision::Result<Option<ActorId>> {
        (|| -> Result<_> {
            match self
                .actors
                .store
                .component(id, ComponentSlot::Element)
                .map_err(fail)?
            {
                Some(crate::actor_world::RuntimeComponent::Element(e)) if e.header.owner == id => {
                    if let Some(org) = e.organization {
                        self.actors.object(org)?;
                    }
                    Ok(e.organization)
                }
                _ => required("fresh actual Element payload/owner"),
            }
        })()
        .map_err(collision_fail)
    }
    fn mind(&self, id: ActorId) -> collision::Result<Option<ActorId>> {
        (|| -> Result<_> {
            match self
                .actors
                .store
                .component(id, ComponentSlot::Mind)
                .map_err(fail)?
            {
                Some(crate::actor_world::RuntimeComponent::Mind(_)) => Ok(Some(id)),
                None => Ok(None),
                _ => required("fresh Mind payload differs"),
            }
        })()
        .map_err(collision_fail)
    }
    fn mind_ready_64(&mut self, _: ActorId) -> collision::Result<bool> {
        (|| -> Result<_> { required("fresh actual Mind64 moving state") })().map_err(collision_fail)
    }
    fn wall_allows_source(&mut self, _: ActorId, _: ActorId) -> collision::Result<bool> {
        (|| -> Result<_> { required("fresh58CBC0 Wall state") })().map_err(collision_fail)
    }
    fn parent(&mut self, id: ActorId) -> collision::Result<Option<ActorId>> {
        (|| -> Result<_> { self.actors.parent(id) })().map_err(collision_fail)
    }
    fn parent_collision_required(
        &mut self,
        _: ActorId,
        _: Option<ActorId>,
    ) -> collision::Result<bool> {
        (|| -> Result<_> { required("fresh4D36F0 parent collision") })().map_err(collision_fail)
    }
}

pub(crate) fn check(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    params: &Params,
    pc: Precision,
) -> Result<u32> {
    catalog.require_source_owner().map_err(fail)?;
    let source = staged.definitions.combat_source().map_err(fail)?;
    source.completed_means().map_err(fail)?;
    let source_pc = source.source_precision();
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return required("fresh placement moved Bank universe differs");
    }
    let world = staged
        .load_world
        .as_mut()
        .ok_or_else(|| fail("fresh placement actual World absent"))?;
    if world.terrain.source_precision() != source_pc
        || world.source_fields.source_precision() != source_pc
        || world.source_fields.receipt().sources != catalog.actors.sources
    {
        return required("fresh placement terrain/source fields differ from same Bank");
    }
    let crate::load_world::SpatialOwner::Adopted(spatial) = &mut world.spatial else {
        return required("fresh placement actual adopted index absent");
    };
    if spatial.error.is_some() || !spatial.pending.is_empty() {
        return required("fresh placement actual index adoption is incomplete");
    }
    if spatial.world.index().precision() != pc {
        return required("fresh placement current arithmetic/index PC differs");
    }
    if staged.actors.staged_placement(key).is_ok() {
        match staged.actors.object(key).map_err(fail)?.definition.native_class {
            2|9 => spatial.world.prepare_object_bound(&staged.actors,catalog,&staged.definitions,key).map_err(fail)?,
            3 => spatial.world.prepare_actor_bound(&staged.actors,catalog,&staged.definitions,key).map_err(fail)?,
            _ => return required("fresh placement source class has no constructor binding"),
        }
        staged
            .actors
            .publish_placement(key, &mut spatial.world)
            .map_err(fail)?;
    }
    let current = staged
        .actors
        .id(key)
        .ok_or_else(|| fail("fresh placement current GTID absent"))?;
    let (map, geometry) = crate::load_object_init_world::map_geometry_index(
        &world.terrain,
        &world.header,
        spatial.world.index(),
        &world.zones,
        pc,
    )
    .map_err(fail)?;
    let (index, bindings) = spatial.world.split_index();
    let actors = Actors {
        catalog,
        store: &staged.actors,
        bank: &staged.definitions,
        fields: &world.source_fields,
        bindings,
    };
    let mut record = View {
        actors,
        index,
        current,
        params,
    };
    let mut facts = MapFacts {
        map,
        geometry: &geometry,
        local_settings_mode: None,
        settings_mode_owner: Some(world.map_mode.as_ref()),
        global_settings_mode: world.global_settings_mode,
        zones: Some(&world.zones),
        source_fields: Some(&world.source_fields),
        point_shape: Some(world.point.as_ref()),
        placement_clearance_bits: world.placement_clearance_bits,
    };
    let mut children = Children {
        geometry: &geometry,
        catalog,
        bank: &staged.definitions,
        identities: &world.zone_identities,
        store: &staged.actors,
    };
    live_placement::check_created(catalog, &mut record, &mut facts, &mut children, pc)
}

///Actual4EB6E0 or4D67F0 query over the already published SAME member. Policy
///borrows actor/component/list owners immutably, while native query marks stay
///in this one index on both success and failure. No callback mutates actors.
pub(crate) fn overlaps(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    object: ObjectKey,
    pc: Precision,
    blocking_notification: bool,
) -> Result<Vec<u32>> {
    catalog.require_source_owner().map_err(fail)?;
    let source = staged.definitions.combat_source().map_err(fail)?;
    source.completed_means().map_err(fail)?;
    let source_pc = source.source_precision();
    if source.source_identities() != catalog.actors.sources.as_slice() {
        return required("stored overlap source Bank universe differs");
    }
    let world = staged
        .load_world
        .as_mut()
        .ok_or_else(|| fail("stored overlap actual World absent"))?;
    if world.terrain.source_precision() != source_pc
        || world.source_fields.source_precision() != source_pc
        || world.source_fields.receipt().sources != catalog.actors.sources
    {
        return required("stored overlap actual World/source PC differs");
    }
    let crate::load_world::SpatialOwner::Adopted(adopted) = &mut world.spatial else {
        return required("stored overlap same adopted index absent");
    };
    if adopted.error.is_some() || !adopted.pending.is_empty() {
        return required("stored overlap actual index adoption is incomplete");
    }
    let (_, geometry) = crate::load_object_init_world::map_geometry_index(
        &world.terrain,
        &world.header,
        adopted.world.index(),
        &world.zones,
        pc,
    )
    .map_err(fail)?;
    let (index, bindings) = adopted.world.split_index();
    let actors = Actors {
        catalog,
        store: &staged.actors,
        bank: &staged.definitions,
        fields: &world.source_fields,
        bindings,
    };
    let id = staged
        .actors
        .id(object)
        .ok_or_else(|| fail("stored overlap actual current generation absent"))?;
    let key = actors.key(id)?;
    actors.member(index, id)?;
    let mut policy = Policy { actors: &actors };
    let mut query = aa_spatial::actor_context::prepare(index, key, false, &policy)?;
    if blocking_notification {
        query.descriptor.source_actor = None;
        query.descriptor.ignore_actor = Some(key);
    }
    aa_spatial::overlaps::all_overlaps(index, &geometry, &mut query, true, &mut policy)
}

#[cfg(test)]
#[path = "map_actor_placement_tests.rs"]
mod tests;

///5530F0's empty descriptor plus an actual placed shape, used by4F5560.
///This is the same555620 query/policy/index, including non-Actor Things.
///Collection finishes and intrusive marks clear before the caller destroys.
pub(crate) fn collect_placed_overlaps(
    catalog:&Catalog<'_>,store:&ActorStore,bank:&Bank,
    fields:&aa_rules::native_source_requirements::NativeSourceRequirements,
    index:&mut SpatialIndex,bindings:SpatialBindings<'_>,
    geometry:&QueryGeometry,placement:aa_spatial::geometry::Placement,
)->Result<Vec<u32>> {
    let actors=Actors{catalog,store,bank,fields,bindings};
    actors.pc()?;
    let mut policy=Policy{actors:&actors};
    let mut query=aa_spatial::query::PreparedQuery{
        placement,descriptor:Descriptor::default(),required_actor_flags:0,
    };
    aa_spatial::overlaps::all_overlaps(index,geometry,&mut query,false,&mut policy)
}
