//! Concrete creation-placement children over the one in-progress load world.
//!
//! The query policy is read-only. The index lease owns native query-link/flag
//! writes and commits them back to every actual shell before discarding fields.
use crate::{
    battle_terrain::BattleTerrain,
    catalog::Catalog,
    live_object_init::Record,
    live_placement, live_required_placement,
    load_spatial::{HeldView, LoadSpatial, QueryBindings},
    load_world_globals::SessionPhase,
};
use aa_actors::{ActorId, components::ComponentSlot, loading::LoadShell};
use aa_mapworld::{
    bootstrap::DefinitionProof,
    restore::{RestoredComponent, RestoredRecord, RuntimeDefinition},
};
use aa_numeric::Precision;
use aa_rules::native_source_requirements::NativeSourceRequirements;
use aa_spatial::{
    collision::{self, Descriptor, Error as CollisionError},
    creation_placement::Context,
    gworld_placement::GMapView,
    index::{ActorKey, SpatialIndex},
    index_world::CollisionPolicy,
    loaded_object_placement,
    query::{QueryGeometry, QueryPolicy},
    required_placement, source_exclusion,
};
use aa_zones::{query::ZoneActors, tile::ZoneMap};

fn fail(e: impl std::fmt::Display) -> CollisionError {
    CollisionError::World(e.to_string())
}
fn required<T>(name: &str) -> collision::Result<T> {
    Err(fail(format!("load placement requires actual {name} owner")))
}
fn spatial(e: impl std::fmt::Display) -> aa_spatial::Error {
    aa_spatial::Error::World(e.to_string())
}

/// Actual current and previously registered saved shells; no pose/owner mirror.
struct Actors<'a, 'c, 'r, 's> {
    catalog: &'c Catalog<'a>,
    record: &'r Record<'s>,
    fields: &'r NativeSourceRequirements,
    extra: Option<HeldView<'r>>,
}
impl Actors<'_, '_, '_, '_> {
    fn source_pc(&self) -> collision::Result<Precision> {
        self.catalog.require_source_owner().map_err(fail)?;
        let bank = self.catalog.definition_bank.try_borrow().map_err(fail)?;
        let source = bank.combat_source().map_err(fail)?;
        if source.source_identities() != self.catalog.actors.sources.as_slice()
            || self.fields.receipt().sources != self.catalog.actors.sources
            || source.source_precision() != self.fields.source_precision()
        {
            return Err(fail("load collision source owner/FP receipt differs"));
        }
        Ok(source.source_precision())
    }
    fn id(&self, token: u32) -> collision::Result<String> {
        self.source_pc()?;
        let bank = self.catalog.definition_bank.try_borrow().map_err(fail)?;
        let entry = bank.entry(token).map_err(fail)?;
        if entry.key.domain != "actordefinition" {
            return Err(fail("load collision definition domain differs"));
        }
        self.catalog
            .require_untranslated_actor_identity(&entry.key.name)
            .map_err(fail)?;
        Ok(entry.key.name.clone())
    }
    fn definition_flags(&self, token: u32) -> collision::Result<u32> {
        let id = self.id(token)?;
        let category = self.catalog.actors.native_category(&id).map_err(fail)?;
        if category.registry_index == 31 {
            return self.catalog.verified_placeholder_flags(&id,self.fields).map_err(fail);
        }
        if !matches!(category.native_kind, 1 | 2 | 20 | 21) {
            return required("completed derived/Placeholder source flags");
        }
        let flags = self
            .catalog
            .actors
            .native_final_flags(&id, self.source_pc()?)
            .map_err(fail)?;
        if category.native_kind != 2 && flags & (1 << 23) != 0 {
            return required("non-Actor ownable source flag verifier");
        }
        Ok(flags)
    }
    fn fields(&self, id: &str) -> collision::Result<&NativeSourceRequirements> {
        self.source_pc()?;
        let canonical = self
            .catalog
            .actors
            .find(id)
            .ok_or_else(|| fail("collision canonical source absent"))?;
        let token = self.fields.definition(id).map_err(fail)?.token();
        if token.source() != &canonical.source {
            return Err(fail("collision source token provenance differs"));
        }
        Ok(self.fields)
    }
    fn shell(
        &self,
        id: ActorId,
    ) -> collision::Result<(
        &LoadShell<DefinitionProof, RestoredComponent>,
        &RuntimeDefinition,
    )> {
        let (shell, definition) = if id == self.record.owner {
            (&*self.record.shell, self.record.definition)
        } else if let Some(extra)=self.extra.filter(|extra|extra.owner==id) {
            (extra.shell,extra.definition)
        } else {
            let entry = self
                .record
                .registry
                .get(id)
                .ok_or_else(|| fail("collision actor generation is stale"))?;
            match &entry.value {
                RestoredRecord::Actor(v) | RestoredRecord::Object(v) | RestoredRecord::Thing(v) => {
                    (&v.shell, &v.definition)
                }
                RestoredRecord::Kingdom(v) => {
                    let v = v
                        .runtime
                        .as_ref()
                        .ok_or_else(|| fail("collision derived runtime absent"))?;
                    (&v.base, &v.definition)
                }
                RestoredRecord::Team(_) => return required("Team collision runtime layout"),
                RestoredRecord::Loading => return required("other concurrently held record"),
            }
        };
        let proof = &shell.thing.definition;
        self.catalog
            .require_untranslated_actor_identity(&proof.id)
            .map_err(fail)?;
        let canonical = self
            .catalog
            .actors
            .find(&proof.id)
            .ok_or_else(|| fail("collision canonical definition missing"))?;
        let category = self
            .catalog
            .actors
            .native_category(&proof.id)
            .map_err(fail)?;
        if canonical.source != proof.source
            || canonical.template_chain != proof.template_chain
            || category != proof.category
            || !crate::load_spatial::same_proof(proof, &definition.proof)
            || definition.native_class != category.runtime_class
            || !crate::load_spatial::ordinary_layout(
                definition.native_class,
                shell.kind,
                shell.object.is_some(),
                shell.actor.is_some(),
            )
        {
            return Err(fail("load collision actual shell/source/storage differs"));
        }
        Ok((shell, definition))
    }
    fn owner(&self, id: ActorId) -> collision::Result<Option<u32>> {
        let (shell, definition) = self.shell(id)?;
        match definition.native_class {
            2 | 6 | 7 | 9 => Ok(None), // actual4C9FA0 Object virtual108
            3 | 4 => {
                let raw = *shell
                    .actor
                    .as_ref()
                    .ok_or_else(|| fail("collision Actor absent"))?
                    .words
                    .get(&0xe8)
                    .ok_or_else(|| fail("collision Actor+E8 absent"))?;
                if raw == 0 {
                    return Ok(None);
                }
                let target = ActorId::from_raw(raw);
                let (_, owner) = self.shell(target)?;
                if owner.native_class != 4 {
                    return Err(fail("collision owner is not an actual Kingdom"));
                }
                Ok(Some(raw))
            }
            _ => required("runtime virtual108 dispatch"),
        }
    }
    fn parent(&self, id: ActorId, slot: u32) -> collision::Result<Option<ActorId>> {
        let (shell, definition) = self.shell(id)?;
        if matches!(definition.native_class, 2 | 6 | 7 | 9) && slot == 0x100 {
            return Ok(None);
        }
        if !matches!(definition.native_class, 3 | 4) {
            return required("runtime parent dispatch");
        }
        let actor = shell
            .actor
            .as_ref()
            .ok_or_else(|| fail("parent Actor absent"))?;
        if let Some(component) = actor.components.get(ComponentSlot::Structure) {
            let RestoredComponent::Structure(value) = component else {
                return Err(fail("Structure slot payload differs"));
            };
            if value.header.owner != id {
                return Err(fail("Structure owner differs"));
            }
            if value.object_handle_10 != 0 {
                return required("positive Structure+10 parent pointer binding");
            }
        }
        //4CED60 reads Wall only after Structure+10 is null;4CE9A0's
        //4CE980->588990 false path returns null when Structure+10 is null.
        match slot {
            0x15c => Ok(None),
            0x100 if !actor.components.present(ComponentSlot::Wall) => Ok(None),
            0x100 => required("58CB30 Wall parent"),
            _ => required("runtime parent virtual slot"),
        }
    }
    fn moving(&self, id: ActorId) -> collision::Result<bool> {
        let (shell, _) = self.shell(id)?;
        let actor = shell
            .actor
            .as_ref()
            .ok_or_else(|| fail("moving predicate Actor absent"))?;
        let Some(RestoredComponent::Mind(mind)) = actor.components.get(ComponentSlot::Mind) else {
            return required("moving predicate concrete Mind");
        };
        let parts = mind
            .state
            .as_any()
            .downcast_ref::<crate::catalog::RestoredMindParts>()
            .ok_or_else(|| fail("moving predicate Mind owner type differs"))?;
        if parts
            .mind
            .as_ref()
            .ok_or_else(|| fail("moving predicate Mind core held elsewhere"))?
            .actor
            != id.raw()
        {
            return Err(fail("moving predicate Mind owner differs"));
        }
        let offset = match parts.installed.kind {
            aa_mind::construct::Kind::Element => 0x7c,
            aa_mind::construct::Kind::Regiment => 0x68,
            aa_mind::construct::Kind::Aircraft => 0x34,
            aa_mind::construct::Kind::Base => return required("abstract Mind virtual64"),
        };
        let state = parts
            .installed
            .state_bindings
            .get(&offset)
            .and_then(|s| parts.states.get(s))
            .ok_or_else(|| fail("moving predicate embedded Move state absent"))?;
        if state.parent != parts.installed.mind_id {
            return Err(fail("moving predicate state parent differs"));
        }
        Ok(state.status == 2)
    }
    fn derived(
        &self,
        id: ActorId,
        class: u32,
    ) -> collision::Result<&aa_mapworld::special_state::SpecialShell> {
        if id == self.record.owner || self.extra.is_some_and(|extra|extra.owner==id) {
            return required("held current derived Kingdom/Team fields");
        }
        let entry = self
            .record
            .registry
            .get(id)
            .ok_or_else(|| fail("relation actor generation is stale"))?;
        let runtime = match &entry.value {
            RestoredRecord::Kingdom(v) if class == 4 => v.runtime.as_ref(),
            RestoredRecord::Team(v) if class == 5 => v.runtime.as_ref(),
            _ => return Err(fail("relation actor has different actual class")),
        }
        .ok_or_else(|| fail("relation derived runtime absent"))?;
        let canonical = self
            .catalog
            .actors
            .find(&runtime.definition.proof.id)
            .ok_or_else(|| fail("relation canonical source absent"))?;
        let category = self
            .catalog
            .actors
            .native_category(&canonical.id)
            .map_err(fail)?;
        if runtime.definition.native_class != class
            || category.runtime_class != class
            || canonical.source != runtime.definition.proof.source
            || canonical.template_chain != runtime.definition.proof.template_chain
            || !crate::load_spatial::same_proof(
                &runtime.base.thing.definition,
                &runtime.definition.proof,
            )
            || runtime.definition.proof.category != category
            || (class == 5
                && (runtime.base.kind != aa_actors::loading::LoadKind::Thing
                    || runtime.base.object.is_some()
                    || runtime.base.actor.is_some()))
            || (class == 4
                && !crate::load_spatial::ordinary_layout(
                    class,
                    runtime.base.kind,
                    runtime.base.object.is_some(),
                    runtime.base.actor.is_some(),
                ))
        {
            return Err(fail("relation actual definition/storage proof differs"));
        }
        runtime
            .derived
            .as_ref()
            .ok_or_else(|| fail("relation derived allocation absent"))
    }
    fn kingdom_team(&self, id: ActorId) -> collision::Result<Option<ActorId>> {
        self.derived(id, 4)?
            .identity_references
            .get(&0x1ec)
            .copied()
            .ok_or_else(|| fail("actual resolved Kingdom1EC team owner absent"))
    }
    fn vector(&self, id: ActorId, class: u32, offset: u32, slot: u32) -> collision::Result<u32> {
        self.derived(id, class)?
            .vectors
            .get(&offset)
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or_else(|| {
                fail(format!(
                    "actual relation vector+{offset:X} slot{slot} absent"
                ))
            })
    }
    /// Existing4E52D0/4E10F0 read order over current derived owners. Null-target
    /// globals remain required; saved vector contents never infer diplomacy.
    fn relation(&self, kingdom: ActorId, other: Option<u32>) -> collision::Result<u32> {
        if other == Some(kingdom.raw()) {
            return Ok(0);
        }
        let other = ActorId::from_raw(
            other.ok_or_else(|| fail("relation requires actual orphan globalB28158"))?,
        );
        if let Some(team) = self.kingdom_team(kingdom)? {
            if let Some(other_team) = self.kingdom_team(other)? {
                let slot = *self
                    .derived(other_team, 5)?
                    .words
                    .get(&0x38)
                    .ok_or_else(|| fail("Team38 slot absent"))?;
                return self.vector(team, 5, 0x58, slot);
            }
            let slot = *self
                .derived(other, 4)?
                .words
                .get(&0x188)
                .ok_or_else(|| fail("Kingdom188 slot absent"))?;
            return self.vector(team, 5, 0x4c, slot);
        }
        if let Some(team) = self.kingdom_team(other)? {
            let slot = *self
                .derived(team, 5)?
                .words
                .get(&0x38)
                .ok_or_else(|| fail("Team38 slot absent"))?;
            return self.vector(kingdom, 4, 0x1dc, slot);
        }
        let slot = *self
            .derived(other, 4)?
            .words
            .get(&0x188)
            .ok_or_else(|| fail("Kingdom188 slot absent"))?;
        self.vector(kingdom, 4, 0x1d0, slot)
    }
    fn actor_relation(&self, id: ActorId, other: Option<u32>) -> collision::Result<u32> {
        let owner = self.owner(id)?;
        if owner == other {
            return Ok(0);
        }
        self.relation(
            ActorId::from_raw(
                owner.ok_or_else(|| fail("Actor relation requires actual orphan globalB28158"))?,
            ),
            other,
        )
    }
    ///4D1120, preserving reached component/field order. Native+98 is the
    ///Settlement slot (14), and+D4 the City slot (29); component names must
    ///not be inferred from older decompiler labels.
    fn capture_eligible(&self, id: ActorId) -> collision::Result<bool> {
        let (shell, _) = self.shell(id)?;
        let actor = shell
            .actor
            .as_ref()
            .ok_or_else(|| fail("4D1120 Actor absent"))?;
        if actor.components.present(ComponentSlot::Settlement) {
            return required("4CEDA0 Settlement+14 delegate");
        }
        if actor.components.state.low & 0x200000 != 0 {
            return Ok(false);
        }
        if let Some(component) = actor.components.get(ComponentSlot::Properties) {
            let RestoredComponent::Properties(value) = component else {
                return Err(fail("4D1120 Properties payload differs"));
            };
            if value.owner.0 != id.raw() {
                return Err(fail("4D1120 Properties owner differs"));
            }
            if value.modifier.flags & 0x400 != 0 {
                return Ok(false);
            }
        }
        if let Some(component) = actor.components.get(ComponentSlot::Structure) {
            let RestoredComponent::Structure(value) = component else {
                return Err(fail("4D1120 Structure payload differs"));
            };
            if value.header.owner != id {
                return Err(fail("4D1120 Structure owner differs"));
            }
            if self
                .record
                .registry
                .get(ActorId::from_raw(value.word_34))
                .is_some()
            {
                return Ok(false);
            }
        }
        if actor.components.present(ComponentSlot::City) {
            return Ok(false);
        }
        let id = &shell.thing.definition.id;
        Ok(self.fields(id)?.byte(id, 0x290).map_err(fail)? != 0)
    }
}

/// Shared concrete policy for creation placement and load overlap refresh.
/// It never mutates Record, descriptors or the index through host services.
pub(crate) struct Policy<'a, 'c, 'r, 's, 'b> {
    actors: Actors<'a, 'c, 'r, 's>,
    bindings: QueryBindings<'b>,
}
impl<'a, 'c, 'r, 's, 'b> Policy<'a, 'c, 'r, 's, 'b> {
    pub(crate) fn new(
        catalog: &'c Catalog<'a>,
        record: &'r Record<'s>,
        fields: &'r NativeSourceRequirements,
        bindings: QueryBindings<'b>,
    ) -> Self {
        Self {
            actors: Actors {
                catalog,
                record,
                fields,
                extra:None,
            },
            bindings,
        }
    }
    pub(crate) fn with_extra(mut self, extra:HeldView<'r>) -> Self {
        self.actors.extra=Some(extra);
        self
    }
    fn actor(&self, key: ActorKey) -> collision::Result<ActorId> {
        let id = self.bindings.id(key).map_err(fail)?;
        let (shell, definition) = self.actors.shell(id)?;
        if self.bindings.native_class(key).map_err(fail)? != definition.native_class
            || !crate::load_spatial::same_proof(
                self.bindings.proof(key).map_err(fail)?,
                &shell.thing.definition,
            )
        {
            return Err(fail("query lease source differs from actual shell"));
        }
        Ok(id)
    }
    fn actor_key(&self, raw: u32) -> collision::Result<ActorKey> {
        self.bindings.key(ActorId::from_raw(raw)).map_err(fail)
    }
}
struct Exclusion<'p, 'i, 'a, 'c, 'r, 's, 'b> {
    policy: &'p Policy<'a, 'c, 'r, 's, 'b>,
    index: &'i SpatialIndex,
    source: ActorId,
}
impl source_exclusion::World for Exclusion<'_, '_, '_, '_, '_, '_, '_> {
    type Actor = ActorId;
    type Mind = ActorId;
    type AcceptanceDispatch = u32;
    type Owner = Option<u32>;
    fn actor(&self, id: ActorId) -> collision::Result<source_exclusion::Actor> {
        let (shell, _) = self.policy.actors.shell(id)?;
        let key = self.policy.bindings.key(id).map_err(fail)?;
        let member = self.index.member(key).map_err(fail)?;
        let actor = shell
            .actor
            .as_ref()
            .ok_or_else(|| fail("source-exclusion Actor absent"))?;
        Ok(source_exclusion::Actor {
            definition: member.actor.definition,
            reference: id.raw(),
            flags: member.actor.flags,
            position: member.actor.position,
            linked_50: shell
                .object
                .as_ref()
                .ok_or_else(|| fail("source-exclusion Object absent"))?
                .words_50_to_54[0],
            linked_168: *actor
                .words
                .get(&0x168)
                .ok_or_else(|| fail("source-exclusion Actor168 absent"))?,
            has_element: actor.components.present(ComponentSlot::Element),
            has_wall: actor.components.present(ComponentSlot::Wall),
        })
    }
    fn definition(&self, token: u32) -> collision::Result<source_exclusion::Definition> {
        let a = &self.policy.actors;
        let id = a.id(token)?;
        let fields = a.fields(&id)?;
        let required_object = fields
            .reference(&id, 0x4e0)
            .map_err(fail)?
            .map(|id| {
                a.catalog
                    .definition_bank
                    .try_borrow_mut()
                    .map_err(fail)?
                    .intern("ActorDefinition", id)
                    .map_err(fail)
            })
            .transpose()?;
        //462B18 initializes gate0; the existing source projection binds the
        //actual optional WallComponent writer at the Bank's source precision.
        let gate = a
            .catalog
            .actors
            .native_collision_definition(&id, a.source_pc()?)
            .map_err(fail)?
            .gate;
        Ok(source_exclusion::Definition {
            required_object,
            settlement_wall: fields.byte(&id, 0x1a8).map_err(fail)? != 0,
            gate,
        })
    }
    fn resolve(&self, raw: u32) -> collision::Result<Option<ActorId>> {
        let id = ActorId::from_raw(raw);
        Ok(self.policy.actors.record.registry.get(id).map(|_| id))
    }
    fn exclusion_head(&self, id: ActorId) -> collision::Result<Option<u32>> {
        let (shell, _) = self.policy.actors.shell(id)?;
        let actor = shell
            .actor
            .as_ref()
            .ok_or_else(|| fail("source-exclusion Actor absent"))?;
        if actor.words.contains_key(&0x110) || actor.words.contains_key(&0x114) {
            return Err(fail("obsolete raw overlap-list authority"));
        }
        actor
            .overlap_list_110
            .head()
            .map(|n| u32::try_from(n.raw()).map_err(fail))
            .transpose()
    }
    fn exclusion_node(&self, key: u32) -> collision::Result<source_exclusion::ExclusionNode> {
        let (shell, _) = self.policy.actors.shell(self.source)?;
        let actor = shell
            .actor
            .as_ref()
            .ok_or_else(|| fail("source-exclusion Actor absent"))?;
        let node = actor
            .overlap_list_110
            .node_by_raw(key as usize)
            .map_err(fail)?;
        Ok(source_exclusion::ExclusionNode {
            reference: node.entry.reference,
            next: node
                .next
                .map(|n| u32::try_from(n.raw()).map_err(fail))
                .transpose()?,
        })
    }
    fn actor_category(&mut self, id: ActorId) -> collision::Result<bool> {
        Ok(matches!(
            self.policy.actors.shell(id)?.1.native_class,
            3 | 4
        ))
    }
    fn acceptance_dispatch(&self, id: ActorId) -> collision::Result<u32> {
        Ok(self.policy.actors.shell(id)?.1.native_class)
    }
    fn owner(&mut self, id: ActorId) -> collision::Result<Option<u32>> {
        self.policy.actors.owner(id)
    }
    fn accepts_owner(
        &mut self,
        dispatch: u32,
        id: ActorId,
        owner: Option<u32>,
    ) -> collision::Result<bool> {
        match dispatch {
            3 => source_exclusion::ordinary_accepts_owner(
                self.policy.actors.owner(id)?,
                owner,
                || self.policy.actors.actor_relation(id, owner),
            ),
            4 if owner == Some(id.raw()) => Ok(true),
            4 => Ok(self.policy.actors.relation(id, owner)? == 1),
            _ => required("candidate virtual138 class"),
        }
    }
    fn organization(&self, id: ActorId) -> collision::Result<Option<ActorId>> {
        let (shell, _) = self.policy.actors.shell(id)?;
        match shell
            .actor
            .as_ref()
            .and_then(|a| a.components.get(ComponentSlot::Element))
        {
            Some(RestoredComponent::Element(value)) if value.header.owner == id => {
                Ok(value.organization)
            }
            _ => Err(fail(
                "source-exclusion actual Element owner absent/different",
            )),
        }
    }
    fn mind(&self, id: ActorId) -> collision::Result<Option<ActorId>> {
        let (shell, _) = self.policy.actors.shell(id)?;
        Ok(shell
            .actor
            .as_ref()
            .ok_or_else(|| fail("Mind Actor absent"))?
            .components
            .present(ComponentSlot::Mind)
            .then_some(id))
    }
    fn mind_ready_64(&mut self, id: ActorId) -> collision::Result<bool> {
        self.policy.actors.moving(id)
    }
    fn wall_allows_source(&mut self, _: ActorId, _: ActorId) -> collision::Result<bool> {
        required("58CBC0 Wall state")
    }
    fn parent(&mut self, id: ActorId) -> collision::Result<Option<ActorId>> {
        self.policy.actors.parent(id, 0x100)
    }
    fn parent_collision_required(
        &mut self,
        _: ActorId,
        _: Option<ActorId>,
    ) -> collision::Result<bool> {
        required("4D36F0 parent collision")
    }
}
impl QueryPolicy for Policy<'_, '_, '_, '_, '_> {
    fn definition_has_object_data(&self, token: u32) -> collision::Result<bool> {
        let id = self.actors.id(token)?;
        match self
            .actors
            .catalog
            .actors
            .native_category(&id)
            .map_err(fail)?
            .native_kind
        {
            1 | 2 | 20 | 21 => Ok(true),
            0 => Ok(false),
            _ => required("definition virtual30 class"),
        }
    }
}
impl CollisionPolicy for Policy<'_, '_, '_, '_, '_> {
    fn definition_flags(&self, d: u32) -> collision::Result<u32> {
        self.actors.definition_flags(d)
    }
    fn definition_flag_1a8(&self, d: u32) -> collision::Result<bool> {
        let id = self.actors.id(d)?;
        Ok(self.actors.fields(&id)?.byte(&id, 0x1a8).map_err(fail)? != 0)
    }
    fn custom_filter(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<bool> {
        required("custom collision function")
    }
    fn source_excludes(
        &mut self,
        index: &mut SpatialIndex,
        source: ActorKey,
        actor: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<bool> {
        let source_id = self.actor(source)?;
        let actor_id = self.actor(actor)?;
        if source_id == actor_id {
            return Ok(true);
        }
        let (shell, definition) = self.actors.shell(source_id)?;
        let reference = shell
            .object
            .as_ref()
            .ok_or_else(|| fail("source Object+50 absent"))?
            .words_50_to_54[0];
        if reference != 0 && index.resolve_reference(reference) == Some(actor) {
            return Ok(true);
        }
        if matches!(definition.native_class, 2 | 6 | 7 | 9) {
            return Ok(false);
        } //4EB7D0
        if matches!(self.actors.shell(actor_id)?.1.native_class, 2 | 6 | 7 | 9) {
            //4D5600 compares Object candidates before virtual+B4. Do not
            //manufacture Actor+168/component storage for an ordinary Object.
            let data = shell
                .actor
                .as_ref()
                .ok_or_else(|| fail("source Actor absent"))?;
            if data.words.contains_key(&0x110) || data.words.contains_key(&0x114) {
                return Err(fail("obsolete raw overlap-list authority"));
            }
            let mut node = data.overlap_list_110.head();
            let mut seen = std::collections::BTreeSet::new();
            while let Some(id) = node {
                if !seen.insert(id.raw()) {
                    return Err(fail("cyclic source overlap list"));
                }
                let value = data.overlap_list_110.node_by_raw(id.raw()).map_err(fail)?;
                if value.entry.reference == actor_id.raw() {
                    return Ok(true);
                }
                node = value.next;
            }
            let source_member = index.member(source).map_err(fail)?;
            let candidate = index.member(actor).map_err(fail)?;
            let id = self.actors.id(source_member.actor.definition)?;
            let required = self
                .actors
                .fields(&id)?
                .reference(&id, 0x4e0)
                .map_err(fail)?;
            let candidate_id = self.actors.id(candidate.actor.definition)?;
            if required.is_some_and(|id| id.eq_ignore_ascii_case(&candidate_id))
                && source_member
                    .actor
                    .position
                    .into_iter()
                    .zip(candidate.actor.position)
                    .all(|(a, b)| f32::from_bits(a) == f32::from_bits(b))
            {
                return Ok(true);
            }
            return Ok(false); //actual Object+B4 false
        }
        source_exclusion::excludes(
            source_id,
            actor_id,
            &mut Exclusion {
                policy: self,
                index,
                source: source_id,
            },
        )
    }
    fn allegiance(
        &mut self,
        _: &mut SpatialIndex,
        a: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<Option<u32>> {
        self.actors.owner(self.actor(a)?)
    }
    fn hostility_dispatch(&self, context: u32) -> collision::Result<u32> {
        let (_, definition) = self.actors.shell(ActorId::from_raw(context))?;
        if definition.native_class != 4 {
            return required("non-Kingdom hostility vtable");
        }
        Ok(4)
    }
    fn hostile(
        &mut self,
        _: &mut SpatialIndex,
        dispatch: u32,
        context: Option<u32>,
        other: Option<u32>,
        _: &mut Descriptor,
    ) -> collision::Result<bool> {
        if dispatch != 4 {
            return required("hostility dispatch");
        }
        let context = context.ok_or_else(|| fail("hostility receiver became null"))?;
        if other == Some(context) {
            return Ok(false);
        } //4E52D0 self->0;4E52A0 relation==3
        Ok(self.actors.relation(ActorId::from_raw(context), other)? == 3)
    }
    fn moving_exclusion(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<bool> {
        required("4CEDE0 actual Mind/Organization state")
    }
    fn outer_actor(
        &mut self,
        _: &mut SpatialIndex,
        a: ActorKey,
        _: &mut Descriptor,
    ) -> collision::Result<Option<u32>> {
        self.actors
            .parent(self.actor(a)?, 0x100)?
            .map(|id| self.actor_key(id.raw()))
            .transpose()
    }
}

/// One synchronous creation-placement child context; all objects below are
/// borrowed from the same concrete load owner and actual Session.
pub struct Children<'a, 'c, 'w> {
    pub catalog: &'c Catalog<'a>,
    pub map: GMapView,
    pub geometry: &'w QueryGeometry,
    pub terrain: &'w BattleTerrain,
    pub spatial: &'w mut LoadSpatial,
    pub zones: &'w ZoneMap,
    pub fields: &'w NativeSourceRequirements,
    pub phase: &'w dyn SessionPhase,
}
macro_rules! required_zone_services {
    () => {
        fn owner(&mut self, _: u32) -> aa_zones::Result<u32> {
            Err(aa_zones::Error::World(
                "placement actual zone owner is not yet bound".into(),
            ))
        }
        fn allied(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
            Err(aa_zones::Error::World(
                "placement actual zone alliance is not yet bound".into(),
            ))
        }
        fn enemy(&mut self, _: u32, _: u32) -> aa_zones::Result<bool> {
            Err(aa_zones::Error::World(
                "placement actual zone hostility is not yet bound".into(),
            ))
        }
        fn zone_slot(&mut self, _: u32, _: u32) -> aa_zones::Result<u32> {
            Err(aa_zones::Error::World(
                "placement actual zone slot is not yet bound".into(),
            ))
        }
        fn word4(&mut self, _: u32) -> aa_zones::Result<u32> {
            Err(aa_zones::Error::World(
                "placement actual zone source is not yet bound".into(),
            ))
        }
        fn component_b0(&mut self, _: u32) -> aa_zones::Result<u32> {
            Err(aa_zones::Error::World(
                "placement actual zone component is not yet bound".into(),
            ))
        }
        fn reveal_mask(&mut self, _: u32) -> aa_zones::Result<Option<u32>> {
            Err(aa_zones::Error::World(
                "placement actual reveal mask is not yet bound".into(),
            ))
        }
    };
}
impl ZoneActors for Children<'_, '_, '_> {
    required_zone_services!();
}
struct Required<'a, 'c, 'w> {
    catalog: &'c Catalog<'a>,
    fields: &'w NativeSourceRequirements,
    phase: &'w dyn SessionPhase,
}
impl ZoneActors for Required<'_, '_, '_> {
    required_zone_services!();
}
impl live_required_placement::Services for Required<'_, '_, '_> {
    fn actor_link(
        &mut self,
        record: &mut Record<'_>,
        actor: ActorId,
        slot: u32,
    ) -> aa_spatial::Result<Option<ActorId>> {
        Actors {
            catalog: self.catalog,
            record,
            fields: self.fields,
            extra: None,
        }
        .parent(actor, slot)
        .map_err(spatial)
    }
    fn mobile_4d1120(
        &mut self,
        record: &mut Record<'_>,
        actor: ActorId,
    ) -> aa_spatial::Result<bool> {
        Actors {
            catalog: self.catalog,
            record,
            fields: self.fields,
            extra: None,
        }
        .capture_eligible(actor)
        .map_err(spatial)
    }
    fn enemy_140(
        &mut self,
        record: &mut Record<'_>,
        actor: ActorId,
        owner: u32,
    ) -> aa_spatial::Result<bool> {
        Ok(Actors {
            catalog: self.catalog,
            record,
            fields: self.fields,
            extra: None,
        }
        .actor_relation(actor, (owner != 0).then_some(owner))
        .map_err(spatial)?
            == 3)
    }
    fn zone_actor(&mut self, _: u32) -> aa_spatial::Result<ActorId> {
        Err(spatial("actual zone actor owner not yet bound"))
    }
    fn game_mode_e4(&mut self) -> aa_spatial::Result<u32> {
        self.phase.application_state().map_err(spatial)
    }
    fn current_thread_is_simulation(&mut self) -> aa_spatial::Result<bool> {
        Err(spatial(
            "actual current-thread/Sim thread ID owner not yet bound",
        ))
    }
    fn global_mode_b28158(&mut self) -> aa_spatial::Result<u32> {
        Err(spatial("actual globalB28158 owner not yet bound"))
    }
}
impl live_placement::Services for Children<'_, '_, '_> {
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
    ) -> aa_spatial::Result<u32> {
        live_required_placement::check(
            self.catalog,
            record,
            &live_required_placement::MapFacts {
                map: self.map,
                geometry: self.geometry,
                index: self.spatial.index(),
                zones: self.zones,
                terrain: self.terrain,
                source_fields: Some(self.fields),
            },
            &mut Required {
                catalog: self.catalog,
                fields: self.fields,
                phase: self.phase,
            },
            required_placement::Arguments {
                position,
                definition: Some(definition),
                owner,
                other,
                argument_7,
                argument_8,
            },
            p,
        )
    }
    fn zone_actor(&mut self, _: u32) -> aa_spatial::Result<ActorId> {
        Err(spatial("actual zone actor owner not yet bound"))
    }
    fn structure_at(
        &mut self,
        _: &mut Record<'_>,
        _: [u32; 2],
    ) -> aa_spatial::Result<Option<ActorId>> {
        Err(spatial("actual4F2220 structure owner not yet bound"))
    }
    fn collision(
        &mut self,
        record: &mut Record<'_>,
        context: &mut Context<u32, ActorId>,
        original_definition: u32,
        _: Precision,
    ) -> aa_spatial::Result<u32> {
        self.spatial
            .with_readonly_query(self.catalog, record, |index, bindings, current| {
                let prepared = live_placement::prepare_collision(context, |id| {
                    bindings.key(id).map_err(spatial)
                })?;
                let mut policy = Policy::new(self.catalog, current, self.fields, bindings);
                live_placement::collision(index, self.geometry, &prepared, &mut policy, || {
                    let id = Actors {
                        catalog: self.catalog,
                        record: current,
                        fields: self.fields,
                        extra: None,
                    }
                    .id(original_definition)
                    .map_err(spatial)?;
                    let category = self.catalog.actors.native_category(&id).map_err(spatial)?;
                    match category.native_kind {
                        1 | 20 | 21 => Ok(false),
                        2 => Ok(true),
                        _ => Err(spatial("actual definition virtual38 class unavailable")),
                    }
                })
            })
            .map_err(spatial)
    }
    fn footprint(
        &mut self,
        _: &mut Record<'_>,
        _: &loaded_object_placement::Descriptor<u32, ()>,
        _: Precision,
    ) -> aa_spatial::Result<u32> {
        Err(spatial("actual4F0B70 footprint owner not yet bound"))
    }
}
