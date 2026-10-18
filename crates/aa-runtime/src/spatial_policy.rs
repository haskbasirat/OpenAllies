//! Ordinary Actor collision policy over canonical live storage. Bind only for
//! the duration of a query; no pose, ownership, component or Mind copy survives.
//! Derived dispatch and unresolved pointer lists return errors on the branch
//! that needs them. An absent binding is never a clear-world answer.
use crate::{
    actor_world::{ActiveComponent, ActorAccess, ObjectKey, RuntimeComponent, RuntimeObject},
    catalog::Catalog,
    relations::Relations,
    spatial_world::SpatialBindings,
};
use aa_actors::{ActorId, components::ComponentSlot};
use aa_mind::Arena;
use aa_numeric::Precision;
use aa_spatial::{
    collision::{Descriptor, Error, Result},
    index::{ActorKey, SpatialIndex},
    index_world::CollisionPolicy,
    moving_state,
    query::QueryPolicy,
    source_exclusion::{self, Definition},
};
use std::collections::BTreeMap;

fn fail(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
#[derive(Clone, Debug)]
struct DefinitionEntry {
    id: String,
    source_sha256: String,
    flags: u32,
    exclusion: Definition,
    proof: aa_rules::collision_definition::CollisionDefinitionInputs,
}
/// Immutable source projections sharing Catalog's ActorDefinition token domain.
#[derive(Default)]
pub struct CollisionDefinitions {
    entries: BTreeMap<u32, DefinitionEntry>,
}
impl CollisionDefinitions {
    pub fn prepare(catalog: &Catalog<'_>, ids: &[&str], precision: Precision) -> Result<Self> {
        let mut entries = BTreeMap::new();
        for id in ids {
            let category = catalog.actors.native_category(id).map_err(fail)?;
            if category.native_kind != 2 || category.runtime_class != 3 {
                return Err(fail(
                    "collision source projection requires ordinary Actor class3",
                ));
            }
            let proof = catalog
                .actors
                .native_collision_definition(id, precision)
                .map_err(fail)?;
            let source = catalog
                .actors
                .find(&proof.id)
                .ok_or_else(|| fail("collision source identity missing"))?;
            let flags = catalog
                .actors
                .native_final_flags(&proof.id, precision)
                .map_err(fail)?;
            let mut bank = catalog.definition_bank.borrow_mut();
            let token = bank.intern("ActorDefinition", &proof.id).map_err(fail)?;
            let required_object = proof
                .required_object
                .as_ref()
                .map(|d| bank.intern("ActorDefinition", &d.id).map_err(fail))
                .transpose()?;
            entries.insert(
                token,
                DefinitionEntry {
                    id: proof.id.clone(),
                    source_sha256: source.source.sha256.clone(),
                    flags,
                    exclusion: Definition {
                        required_object,
                        settlement_wall: proof.settlement_wall,
                        gate: proof.gate,
                    },
                    proof,
                },
            );
        }
        Ok(Self { entries })
    }
    pub fn source_proof(
        &self,
        token: u32,
    ) -> Result<&aa_rules::collision_definition::CollisionDefinitionInputs> {
        Ok(&self.entry(token)?.proof)
    }
    fn entry(&self, token: u32) -> Result<&DefinitionEntry> {
        self.entries
            .get(&token)
            .ok_or_else(|| fail("collision definition is not source-bound"))
    }
}
#[derive(Clone, Copy)]
pub struct PolicyInputs<'a> {
    pub definitions: &'a CollisionDefinitions,
    pub relations: &'a Relations,
}
impl PolicyInputs<'_> {
    pub fn bind<'a, 'store>(
        &'a self,
        actors: &'a ActorAccess<'store>,
        arena: &'a Arena,
        bindings: SpatialBindings<'a>,
    ) -> SpatialCollisionPolicy<'a, 'store> {
        SpatialCollisionPolicy {
            actors,
            arena,
            bindings,
            definitions: self.definitions,
            relations: self.relations,
        }
    }
}
pub struct SpatialCollisionPolicy<'a, 'store> {
    actors: &'a ActorAccess<'store>,
    arena: &'a Arena,
    bindings: SpatialBindings<'a>,
    definitions: &'a CollisionDefinitions,
    relations: &'a Relations,
}
impl SpatialCollisionPolicy<'_, '_> {
    fn object(&self, key: ObjectKey) -> Result<&RuntimeObject> {
        let actor = self.actors.object(key).map_err(fail)?;
        if actor.definition.native_class != 3 || actor.definition.proof.category.native_kind != 2 {
            return Err(fail("collision requires ordinary class3 Actor dispatch"));
        }
        actor.components().map_err(fail)?;
        Ok(actor)
    }
    fn id(&self, key: ObjectKey) -> Result<ActorId> {
        self.actors
            .id(key)
            .ok_or_else(|| fail("collision actor has no live GTID"))
    }
    fn object_key(&self, id: ActorId) -> Result<ObjectKey> {
        self.actors
            .key(id)
            .ok_or_else(|| fail("collision actor GTID is stale"))
    }
    fn member<'a>(
        &self,
        index: &'a SpatialIndex,
        object: ObjectKey,
    ) -> Result<&'a aa_spatial::index::Member> {
        let runtime = self.object(object)?;
        let member = index
            .member(self.bindings.key(object).map_err(fail)?)
            .map_err(fail)?;
        if member.reference != self.id(object)?.raw() {
            return Err(fail("spatial actor generation mismatch"));
        }
        let entry = self.definitions.entry(member.actor.definition)?;
        if entry.id != runtime.definition.proof.id
            || entry.source_sha256 != runtime.definition.proof.source.sha256
        {
            return Err(fail("collision definition differs from actor source"));
        }
        Ok(member)
    }
    fn owner(&self, object: ObjectKey) -> Result<Option<ActorId>> {
        self.object(object)?;
        self.actors.ordinary_owner(self.id(object)?).map_err(fail)
    }
    fn word(&self, object: ObjectKey, offset: u32) -> Result<u32> {
        self.object(object)?
            .actor
            .as_ref()
            .and_then(|v| v.words.get(&offset))
            .copied()
            .ok_or_else(|| fail(format!("collision requires initialized Actor+{offset:x}")))
    }
    fn parent(&self, object: ObjectKey) -> Result<Option<ObjectKey>> {
        let components = self.object(object)?.components().map_err(fail)?;
        if components.get(ComponentSlot::Structure).is_some()
            || components.get(ComponentSlot::Wall).is_some()
        {
            return Err(fail(
                "ordinary virtual100 requires canonical Structure/Wall parent state",
            ));
        }
        Ok(None)
    }
}
struct ExclusionView<'a, 'policy, 'store> {
    policy: &'a SpatialCollisionPolicy<'policy, 'store>,
    index: &'a SpatialIndex,
    source: ObjectKey,
}
impl source_exclusion::World for ExclusionView<'_, '_, '_> {
    type Actor = ObjectKey;
    type Mind = ActorId;
    type AcceptanceDispatch = ();
    type Owner = Option<ActorId>;
    fn actor(&self, object: ObjectKey) -> Result<source_exclusion::Actor> {
        let p = self.policy;
        let runtime = p.object(object)?;
        let member = p.member(self.index, object)?;
        let components = runtime.components().map_err(fail)?;
        Ok(source_exclusion::Actor {
            definition: member.actor.definition,
            reference: member.reference,
            flags: member.actor.flags,
            position: member.actor.position,
            linked_50: runtime
                .object
                .as_ref()
                .ok_or_else(|| fail("Object fields missing"))?
                .words_50_to_54[0],
            linked_168: p.word(object, 0x168)?,
            has_element: components.get(ComponentSlot::Element).is_some(),
            has_wall: components.get(ComponentSlot::Wall).is_some(),
        })
    }
    fn definition(&self, token: u32) -> Result<Definition> {
        Ok(self.policy.definitions.entry(token)?.exclusion)
    }
    fn resolve(&self, reference: u32) -> Result<Option<ObjectKey>> {
        Ok(self.policy.actors.key(ActorId::from_raw(reference)))
    }
    fn exclusion_head(&self, object: ObjectKey) -> Result<Option<u32>> {
        if object != self.source {
            return Err(fail("overlap node owner differs from source actor"));
        }
        let actor = self
            .policy
            .object(object)?
            .actor
            .as_ref()
            .ok_or_else(|| fail("overlap source Actor shell missing"))?;
        if actor.words.contains_key(&0x110) || actor.words.contains_key(&0x114) {
            return Err(fail("obsolete raw exclusion list head/tail authority"));
        }
        actor
            .overlap_list_110
            .head()
            .map(|id| u32::try_from(id.raw()).map_err(fail))
            .transpose()
    }
    fn exclusion_node(&self, node: u32) -> Result<source_exclusion::ExclusionNode> {
        let actor = self
            .policy
            .object(self.source)?
            .actor
            .as_ref()
            .ok_or_else(|| fail("overlap source Actor shell missing"))?;
        let value = actor
            .overlap_list_110
            .node_by_raw(node as usize)
            .map_err(fail)?;
        Ok(source_exclusion::ExclusionNode {
            reference: value.entry.reference,
            next: value
                .next
                .map(|id| u32::try_from(id.raw()).map_err(fail))
                .transpose()?,
        })
    }
    fn actor_category(&mut self, object: ObjectKey) -> Result<bool> {
        self.policy.object(object)?;
        Ok(true)
    }
    fn acceptance_dispatch(&self, object: ObjectKey) -> Result<()> {
        self.policy.object(object)?;
        Ok(())
    }
    fn owner(&mut self, object: ObjectKey) -> Result<Option<ActorId>> {
        self.policy.owner(object)
    }
    fn accepts_owner(&mut self, _: (), object: ObjectKey, owner: Option<ActorId>) -> Result<bool> {
        let current = self.policy.owner(object)?;
        source_exclusion::ordinary_accepts_owner(current, owner, || {
            self.policy
                .relations
                .actor_relation(current, owner)
                .map_err(fail)
        })
    }
    fn organization(&self, object: ObjectKey) -> Result<Option<ObjectKey>> {
        match self
            .policy
            .actors
            .component(self.policy.id(object)?, ComponentSlot::Element)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Element(element)) => element
                .organization
                .map(|id| self.policy.object_key(id))
                .transpose(),
            _ => Err(fail(
                "source-exclusion requires canonical Element component",
            )),
        }
    }
    fn mind(&self, object: ObjectKey) -> Result<Option<ActorId>> {
        let id = self.policy.id(object)?;
        match self
            .policy
            .actors
            .component(id, ComponentSlot::Mind)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Mind(_)) => Ok(Some(id)),
            None => Ok(None),
            _ => Err(fail("Mind component payload mismatch")),
        }
    }
    fn mind_ready_64(&mut self, actor: ActorId) -> Result<bool> {
        crate::mind_world::moving(self.policy.actors, self.policy.arena, actor).map_err(fail)
    }
    fn wall_allows_source(&mut self, _: ObjectKey, _: ObjectKey) -> Result<bool> {
        Err(fail("native58CBC0 needs canonical Wall state"))
    }
    fn parent(&mut self, object: ObjectKey) -> Result<Option<ObjectKey>> {
        self.policy.parent(object)
    }
    fn parent_collision_required(&mut self, _: ObjectKey, _: Option<ObjectKey>) -> Result<bool> {
        Err(fail("native4D36F0 parent collision branch is unbound"))
    }
}

struct MovingView<'a, 'policy, 'store> {
    policy: &'a SpatialCollisionPolicy<'policy, 'store>,
    index: &'a SpatialIndex,
}
fn moving_error(e: impl std::fmt::Display) -> moving_state::Error {
    moving_state::Error::World(e.to_string())
}
impl moving_state::World for MovingView<'_, '_, '_> {
    fn actor(&self, key: u32) -> moving_state::Result<moving_state::Actor> {
        let p = self.policy;
        let object = p.bindings.object(key).map_err(moving_error)?;
        let actor = p.object(object).map_err(moving_error)?;
        let components = actor.components().map_err(moving_error)?;
        let organization = components.get(ComponentSlot::Organization).is_some();
        let mover = components.get(ComponentSlot::Mover).is_some();
        // Active Mover equality is only read in the native no-Organization
        // branch. These role-local tokens model that equality; the component
        // payload remains in ActorAccess and is never copied into this view.
        let active = match (organization, actor.active_component) {
            (true, _) => None,
            (false, ActiveComponent::UnresolvedNativeWord(_)) => {
                return Err(moving_error("active component pointer is unresolved"));
            }
            (false, ActiveComponent::Slot(ComponentSlot::Mover)) => Some(key),
            _ => None,
        };
        Ok(moving_state::Actor {
            component7c: organization.then_some(key),
            component6c: mover.then_some(key),
            active_component_e4: active,
            active_state70: components.get(ComponentSlot::Mind).map(|_| key),
            position: p
                .member(self.index, object)
                .map_err(moving_error)?
                .actor
                .position,
        })
    }
    fn component(&self, key: u32) -> moving_state::Result<moving_state::Component> {
        let p = self.policy;
        let object = p.bindings.object(key).map_err(moving_error)?;
        let actor = p.id(object).map_err(moving_error)?;
        let Some(RuntimeComponent::Organization(value)) = p
            .actors
            .component(actor, ComponentSlot::Organization)
            .map_err(moving_error)?
        else {
            return Err(moving_error("Organization payload missing"));
        };
        if value.header.owner != actor {
            return Err(moving_error("Organization owner mismatch"));
        }
        let selected = *value
            .words
            .get(&0x40)
            .ok_or_else(|| moving_error("Organization selected-member field missing"))?;
        if selected != 0 {
            return Err(moving_error(
                "Organization selected-member pointer is not canonical yet",
            ));
        }
        Ok(moving_state::Component {
            owner: key,
            linked_actor40: None,
            comparison_position: value.position_xy.map(|v| v.0),
        })
    }
    fn state_kind74(&self, key: u32) -> moving_state::Result<u32> {
        let p = self.policy;
        let actor = p
            .id(p.bindings.object(key).map_err(moving_error)?)
            .map_err(moving_error)?;
        let Some(RuntimeComponent::Mind(value)) = p
            .actors
            .component(actor, ComponentSlot::Mind)
            .map_err(moving_error)?
        else {
            return Err(moving_error("Mind payload missing"));
        };
        if p.arena.mind(value.mind_id).map_err(moving_error)?.actor != actor.raw() {
            return Err(moving_error("Mind owner mismatch"));
        }
        //4CEDE0 reads literal Mind+74, not a virtual function or stack head.
        if let Some(id) = value.state_bindings.get(&0x68) {
            let state = p.arena.state(*id).map_err(moving_error)?;
            if state.parent != value.mind_id {
                return Err(moving_error("embedded state parent mismatch"));
            }
            Ok(state.status)
        } else {
            value
                .words
                .get(&0x74)
                .copied()
                .ok_or_else(|| moving_error("Mind+74 field has no initialized binding"))
        }
    }
}
impl QueryPolicy for SpatialCollisionPolicy<'_, '_> {
    fn definition_has_object_data(&self, definition: u32) -> Result<bool> {
        self.definitions.entry(definition)?;
        Ok(true)
    }
}
impl CollisionPolicy for SpatialCollisionPolicy<'_, '_> {
    fn definition_flags(&self, definition: u32) -> Result<u32> {
        Ok(self.definitions.entry(definition)?.flags)
    }
    fn definition_flag_1a8(&self, definition: u32) -> Result<bool> {
        Ok(self
            .definitions
            .entry(definition)?
            .exclusion
            .settlement_wall)
    }
    fn custom_filter(
        &mut self,
        _: &mut SpatialIndex,
        _: ActorKey,
        _: &mut Descriptor,
    ) -> Result<bool> {
        Err(fail(
            "query custom-filter function has no bound implementation",
        ))
    }
    fn source_excludes(
        &mut self,
        index: &mut SpatialIndex,
        source: ActorKey,
        actor: ActorKey,
        _: &mut Descriptor,
    ) -> Result<bool> {
        let source = self.bindings.object(source).map_err(fail)?;
        let actor = self.bindings.object(actor).map_err(fail)?;
        self.object(source)?;
        source_exclusion::excludes(
            source,
            actor,
            &mut ExclusionView {
                policy: self,
                index,
                source,
            },
        )
    }
    fn allegiance(
        &mut self,
        _: &mut SpatialIndex,
        actor: ActorKey,
        _: &mut Descriptor,
    ) -> Result<Option<u32>> {
        Ok(self
            .owner(self.bindings.object(actor).map_err(fail)?)?
            .map(ActorId::raw))
    }
    fn hostility_dispatch(&self, context: u32) -> Result<u32> {
        let id = ActorId::from_raw(context);
        let object = self.actors.object(self.object_key(id)?).map_err(fail)?;
        if object.definition.native_class != 4 || !self.relations.kingdoms.contains_key(&id) {
            return Err(fail("hostility context needs a bound Kingdom"));
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
    ) -> Result<bool> {
        if dispatch != 4 {
            return Err(fail("unsupported hostility dispatch"));
        }
        let context = ActorId::from_raw(context.ok_or_else(|| fail("hostility receiver is null"))?);
        Ok(Relations::hostile(
            self.relations
                .kingdom_relation(context, other.map(ActorId::from_raw))
                .map_err(fail)?,
        ))
    }
    fn moving_exclusion(
        &mut self,
        index: &mut SpatialIndex,
        actor: ActorKey,
        _: &mut Descriptor,
    ) -> Result<bool> {
        moving_state::excluded(
            actor,
            &MovingView {
                policy: self,
                index,
            },
        )
        .map_err(fail)
    }
    fn outer_actor(
        &mut self,
        _: &mut SpatialIndex,
        actor: ActorKey,
        _: &mut Descriptor,
    ) -> Result<Option<u32>> {
        self.parent(self.bindings.object(actor).map_err(fail)?)?
            .map(|v| self.bindings.key(v).map_err(fail))
            .transpose()
    }
}
