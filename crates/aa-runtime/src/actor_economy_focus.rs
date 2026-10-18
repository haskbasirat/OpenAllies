//! Borrow-only Economy focus, including overlays for synchronous parent refresh.
//! Building the temporary index borrows entity fields; no canonical entity or
//! component is removed, taken, cloned, or replaced. Its O(n) borrowed index is
//! a current integration cost, not game iteration or simulation ordering.
use super::*;
use aa_actors::components::{OtherComponents, StateBits};
use aa_actors::ground_states::EconomyState;

pub struct ActorParts<'a> {
    pub components: OtherComponents<'a, RuntimeComponent>,
    pub words: &'a mut BTreeMap<u32, u32>,
    pub string_dc_utf16: &'a mut Vec<u16>,
    pub string_e0_utf16: &'a mut Vec<u16>,
}
pub struct RuntimeParts<'a> {
    pub kind: LoadKind,
    pub active_component: &'a mut ActiveComponent,
    pub definition: &'a RuntimeDefinition,
    pub thing: &'a mut ThingCore,
    placement: &'a mut PlacementOwner,
    pub object: &'a mut Option<ObjectFields>,
    pub actor: Option<ActorParts<'a>>,
}
impl<'a> RuntimeParts<'a> {
    pub fn components(&self) -> Result<&OtherComponents<'_, RuntimeComponent>> {
        Ok(&self
            .actor
            .as_ref()
            .ok_or(Error::Missing("Actor component table"))?
            .components)
    }
    pub fn components_mut(&mut self) -> Result<&mut OtherComponents<'a, RuntimeComponent>> {
        Ok(&mut self
            .actor
            .as_mut()
            .ok_or(Error::Missing("Actor component table"))?
            .components)
    }
    pub fn state_bits(&self) -> Result<StateBits> {
        Ok(*self.components()?.state)
    }
    fn reborrow(&mut self) -> RuntimeParts<'_> {
        RuntimeParts {
            kind: self.kind,
            active_component: &mut *self.active_component,
            definition: self.definition,
            thing: &mut *self.thing,
            placement: &mut *self.placement,
            object: &mut *self.object,
            actor: self.actor.as_mut().map(|a| ActorParts {
                components: a.components.reborrow(),
                words: &mut *a.words,
                string_dc_utf16: &mut *a.string_dc_utf16,
                string_e0_utf16: &mut *a.string_e0_utf16,
            }),
        }
    }
}
pub struct EntityParts<'a> {
    pub runtime: Option<RuntimeParts<'a>>,
    pub source: &'a mut Source,
    pub bindings: &'a Option<Arc<ActorBindings>>,
}
impl EntityParts<'_> {
    fn reborrow(&mut self) -> EntityParts<'_> {
        EntityParts {
            runtime: self.runtime.as_mut().map(RuntimeParts::reborrow),
            source: &mut *self.source,
            bindings: self.bindings,
        }
    }
}

pub struct EconomyAccess<'a> {
    identities: &'a IdentityRegistry<ObjectKey>,
    entities: BTreeMap<ObjectKey, EntityParts<'a>>,
    economies: BTreeMap<ObjectKey, &'a mut EconomyState>,
    pub property_instances: &'a mut aa_body::property_effects::Instances,
    pub abilities: &'a mut BTreeMap<u32, aa_combat::ability::Ability>,
    pub animations: &'a AnimationBank,
}
impl ActorStore {
    pub fn economy_access(&mut self) -> Result<EconomyAccess<'_>> {
        make_economy_access(&self.identities,&mut self.entities,self.property_allocations.instances_mut(),&mut self.abilities,&self.animations)
    }
}
impl ActorAccess<'_>{
    ///Same borrowed component focus while the original Mind arena remains
    ///loaned to its synchronous caller. No Entity or component is extracted.
    pub fn economy_access(&mut self)->Result<EconomyAccess<'_>>{
        make_economy_access(self.identities,&mut *self.entities,self.property_allocations.instances_mut(),&mut *self.abilities,self.animations)
    }
}
fn make_economy_access<'a>(identities:&'a IdentityRegistry<ObjectKey>,arena:&'a mut BTreeMap<ObjectKey,Box<Entity>>,
    property_instances:&'a mut aa_body::property_effects::Instances,abilities:&'a mut BTreeMap<u32,aa_combat::ability::Ability>,animations:&'a AnimationBank)->Result<EconomyAccess<'a>>{
        let mut entities = BTreeMap::new();
        let mut economies = BTreeMap::new();
        for (&key, entity) in arena.iter_mut() {
            let Entity {
                runtime,
                source,
                bindings,
            } = &mut **entity;
            let runtime = if let Some(runtime) = runtime {
                let RuntimeObject {
                    kind,
                    active_component,
                    definition,
                    thing,
                    placement,
                    object,
                    actor,
                    ..
                } = runtime;
                let actor = if let Some(actor) = actor {
                    let ActorShell {
                        components,
                        string_dc_utf16,
                        string_e0_utf16,
                        words,
                        ..
                    } = actor;
                    let (component, components) = components.split_slot(ComponentSlot::Economy);
                    if let Some(component) = component {
                        let RuntimeComponent::Economy(state) = component else {
                            return Err(Error::Missing("Economy slot payload"));
                        };
                        economies.insert(key, &mut **state);
                    }
                    Some(ActorParts {
                        components,
                        words,
                        string_dc_utf16,
                        string_e0_utf16,
                    })
                } else {
                    None
                };
                Some(RuntimeParts {
                    kind: *kind,
                    active_component,
                    definition,
                    thing,
                    placement,
                    object,
                    actor,
                })
            } else {
                None
            };
            entities.insert(
                key,
                EntityParts {
                    runtime,
                    source,
                    bindings,
                },
            );
        }
        Ok(EconomyAccess {
            identities,
            entities,
            economies,
            property_instances,
            abilities,
            animations,
        })
}
impl<'a> EconomyAccess<'a> {
    pub fn key(&self, id: ActorId) -> Option<ObjectKey> {
        self.identities.get(id).map(|e| e.value)
    }
    pub fn id(&self, key: ObjectKey) -> Option<ActorId> {
        self.identities
            .iter()
            .find(|e| e.value == key)
            .map(|e| e.id)
    }
    pub fn entity(&self, key: ObjectKey) -> Result<&EntityParts<'_>> {
        self.entities
            .get(&key)
            .ok_or(Error::Missing("object borrowed identity"))
    }
    pub fn entity_mut(&mut self, key: ObjectKey) -> Result<&mut EntityParts<'a>> {
        self.entities
            .get_mut(&key)
            .ok_or(Error::Missing("object borrowed identity"))
    }
    pub fn object(&self, key: ObjectKey) -> Result<&RuntimeParts<'_>> {
        self.entity(key)?
            .runtime
            .as_ref()
            .ok_or(Error::Missing("runtime object"))
    }
    pub fn object_mut(&mut self, key: ObjectKey) -> Result<&mut RuntimeParts<'a>> {
        self.entity_mut(key)?
            .runtime
            .as_mut()
            .ok_or(Error::Missing("runtime object"))
    }
    pub fn component(&self, id: ActorId, slot: ComponentSlot) -> Result<Option<&RuntimeComponent>> {
        if slot == ComponentSlot::Economy {
            return Err(Error::Missing(
                "use canonical economy getter or focused callback argument",
            ));
        }
        let key = self.key(id).ok_or(Error::Missing("actor GTID"))?;
        Ok(self.object(key)?.components()?.get(slot))
    }
    pub fn component_mut(
        &mut self,
        id: ActorId,
        slot: ComponentSlot,
    ) -> Result<Option<&mut RuntimeComponent>> {
        if slot == ComponentSlot::Economy {
            return Err(Error::Missing(
                "use canonical economy getter or focused callback argument",
            ));
        }
        let key = self.key(id).ok_or(Error::Missing("actor GTID"))?;
        Ok(self.object_mut(key)?.components_mut()?.get_mut(slot))
    }
    pub fn economy(&self, key: ObjectKey) -> Result<&EconomyState> {
        self.economies.get(&key).map(|v| &**v).ok_or(Error::Missing(
            "focused or absent Economy; current state must be supplied by callback",
        ))
    }
    pub fn economy_mut(&mut self, key: ObjectKey) -> Result<&mut EconomyState> {
        self.economies
            .get_mut(&key)
            .map(|v| &mut **v)
            .ok_or(Error::Missing(
                "focused or absent Economy; current state must be supplied by callback",
            ))
    }
    /// Removes ONLY a borrow-index entry, never the canonical component Option.
    /// Native component presence remains true in RuntimeParts::components().
    pub fn take_focus(&mut self, key: ObjectKey) -> Result<&'a mut EconomyState> {
        self.economies
            .remove(&key)
            .ok_or(Error::Missing("focus Economy absent or already borrowed"))
    }
    pub fn placement(&self, key: ObjectKey, spatial: &impl Spatial) -> Result<PlacementState> {
        match *self.object(key)?.placement {
            PlacementOwner::Staged(p) => Ok(p),
            PlacementOwner::Spatial => {
                spatial.placement(self.id(key).ok_or(Error::Missing("published GTID"))?)
            }
        }
    }
    /// Reborrows every field and adds the current callback's component to the
    /// child overlay before selecting another focus (usually its parent). Parent
    /// value scans therefore read the exact already-mutated child, not a snapshot.
    pub fn reborrow_with<'b>(
        &'b mut self,
        key: ObjectKey,
        current: &'b mut EconomyState,
    ) -> Result<EconomyAccess<'b>> {
        if self.economies.contains_key(&key) {
            return Err(Error::Occupied);
        }
        if !self
            .object(key)?
            .components()?
            .present(ComponentSlot::Economy)
        {
            return Err(Error::Missing("overlay actor Economy presence"));
        }
        let mut economies: BTreeMap<_, _> = self
            .economies
            .iter_mut()
            .map(|(&k, v)| (k, &mut **v))
            .collect();
        economies.insert(key, current);
        Ok(EconomyAccess {
            identities: self.identities,
            entities: self
                .entities
                .iter_mut()
                .map(|(&k, v)| (k, v.reborrow()))
                .collect(),
            economies,
            property_instances: &mut *self.property_instances,
            abilities: &mut *self.abilities,
            animations: self.animations,
        })
    }
}
