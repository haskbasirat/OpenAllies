//! Actual4CFC80/563BA0 over the same staged Character, active-component word,
//! AnimationBank, deferred queue and world RNG. Positive unbound effects fail
//! at their reached callback, retaining earlier native writes.
use crate::{
    actor_world::{self, ActiveComponent, ActorAccess, ObjectKey, RuntimeComponent},
    catalog::{ActorBindings, Catalog},
    restoration::Staged,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_animation::runtime::{self, Animation, Character, Effect, borrowed};
use aa_body::F32;
use aa_numeric::Precision;
use std::sync::Arc;

fn fail(e: impl std::fmt::Display) -> runtime::Error {
    runtime::Error::Service(format!("map Character stance: {e}"))
}
type Result<T> = runtime::Result<T>;

/// No detached Character or copied ActorStore is created. Ordinary class3
/// source identity and arithmetic/source PCs remain distinct requirements.
pub fn default_stance(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    pc: Precision,
) -> std::result::Result<(), String> {
    let (_, mut actors) = staged.actors.split_minds();
    let (id, binding) = bind(catalog, &actors, &staged.definitions, key)?;
    let world = staged
        .load_world
        .as_mut()
        .ok_or("4CFC80 same actual World absent")?;
    let crate::load_world::SpatialOwner::Adopted(spatial) = &mut world.spatial else {
        return Err("4CFC80 actual adopted spatial owner absent".into());
    };
    run_bound(
        catalog,
        &mut actors,
        &mut spatial.world,
        &mut staged.queues,
        &staged.owners,
        &mut staged.rng,
        &staged.clock,
        &staged.definitions,
        key,
        id,
        binding,
        pc,
    )
}

///Same4CFC80/563BA0 algorithm while its caller already lends the real Mind
///arena and ActorAccess separately. No Character/ActorStore extraction or
///clone is used; repeated native calls remain repeated calls on those owners.
#[allow(clippy::too_many_arguments)]
pub fn default_stance_on_owners(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    spatial: &mut crate::spatial_world::SpatialWorld,
    queues: &mut aa_world::queues::Queues,
    owners: &crate::scheduler_owners::Owners,
    random: &mut aa_simulation::NativeRng,
    clock: &aa_simulation::NativeClock,
    bank: &crate::definitions::Bank,
    key: ObjectKey,
    pc: Precision,
) -> std::result::Result<(), String> {
    let (id, binding) = bind(catalog, actors, bank, key)?;
    run_bound(
        catalog, actors, spatial, queues, owners, random, clock, bank, key, id, binding, pc,
    )
}

fn bind(
    catalog: &Catalog<'_>,
    actors: &ActorAccess<'_>,
    bank: &crate::definitions::Bank,
    key: ObjectKey,
) -> std::result::Result<(ActorId, Arc<ActorBindings>), String> {
    catalog.require_source_owner()?;
    let id = actors.id(key).ok_or("4CFC80 current Actor GTID absent")?;
    let binding = Arc::clone(
        actors
            .entity(key)
            .map_err(|e| e.to_string())?
            .bindings
            .as_ref()
            .ok_or("4CFC80 actual source allocation absent")?,
    );
    let source = bank.combat_source()?;
    if source.source_identities() != catalog.actors.sources.as_slice()
        || source.source_precision() != binding.precision
    {
        return Err("4CFC80 source Bank/Catalog/Actor binding differ".into());
    }
    let canonical = catalog.actor_bindings(&binding.definition.proof.id, binding.precision)?;
    if !Arc::ptr_eq(&binding, &canonical)
        || actors
            .object(key)
            .map_err(|e| e.to_string())?
            .definition
            .native_class
            != 3
    {
        return Err("4CFC80 actual ordinary Actor source identity differs".into());
    }
    Ok((id, binding))
}

#[allow(clippy::too_many_arguments)]
fn run_bound(
    catalog: &Catalog<'_>,
    actors: &mut ActorAccess<'_>,
    spatial: &mut crate::spatial_world::SpatialWorld,
    queues: &mut aa_world::queues::Queues,
    owners: &crate::scheduler_owners::Owners,
    random: &mut aa_simulation::NativeRng,
    clock: &aa_simulation::NativeClock,
    bank: &crate::definitions::Bank,
    key: ObjectKey,
    id: ActorId,
    binding: Arc<ActorBindings>,
    pc: Precision,
) -> std::result::Result<(), String> {
    if queues.precision != pc || spatial.index().precision() != pc {
        return Err("4CFC80 actual live arithmetic PC differs".into());
    }
    let mut w = Bound {
        actors,
        spatial,
        queues,
        owners,
        random,
        clock,
        bank,
        catalog,
        binding,
        key,
        id,
    };
    //4CFC80 dispatch uses the actual current component table.
    if w.actors
        .object(key)
        .map_err(|e| e.to_string())?
        .components()
        .map_err(|e| e.to_string())?
        .present(Slot::Character)
    {
        borrowed::default_stance(&mut w, pc).map_err(|e| e.to_string())
    } else {
        let old = active(w.actors, key).map_err(|e| e.to_string())?;
        w.actors
            .object_mut(key)
            .map_err(|e| e.to_string())?
            .active_component = ActiveComponent::None;
        if let Some(old) = old {
            w.deactivate(old).map_err(|e| e.to_string())?;
        }
        if let Some(new) = active(w.actors, key).map_err(|e| e.to_string())? {
            w.enter(new).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

fn active(actors: &ActorAccess<'_>, key: ObjectKey) -> Result<Option<Slot>> {
    match actors.object(key).map_err(fail)?.active_component {
        ActiveComponent::None => Ok(None),
        ActiveComponent::Slot(s) => Ok(Some(s)),
        ActiveComponent::UnresolvedNativeWord(_) => {
            Err(fail("actual active component is unresolved"))
        }
    }
}
struct Bound<'a, 'store, 'source> {
    actors: &'a mut ActorAccess<'store>,
    spatial: &'a mut crate::spatial_world::SpatialWorld,
    queues: &'a mut aa_world::queues::Queues,
    owners: &'a crate::scheduler_owners::Owners,
    random: &'a mut aa_simulation::NativeRng,
    clock: &'a aa_simulation::NativeClock,
    bank: &'a crate::definitions::Bank,
    catalog: &'a Catalog<'source>,
    binding: Arc<ActorBindings>,
    key: ObjectKey,
    id: ActorId,
}
#[cfg(test)]
#[path = "map_character_stance_tests.rs"]
mod tests;
impl Bound<'_, '_, '_> {
    fn owner(&self) -> Result<u64> {
        self.owners
            .get(crate::scheduler_owners::Target::Component {
                object: self.key,
                slot: Slot::Character as u8,
            })
            .map_err(fail)
    }
    fn set(&self) -> Result<u32> {
        let object = self.actors.object(self.key).map_err(fail)?;
        let actor = object
            .actor
            .as_ref()
            .ok_or_else(|| fail("actual Actor shell absent"))?;
        let stance = actor
            .words
            .get(&0x15c)
            .copied()
            .ok_or_else(|| fail("actual Actor+15C is unbound"))?;
        if stance != u32::MAX {
            return Err(fail(
                "4D0720/4D0750 positive current state-view owner is required",
            ));
        }
        let variant = object
            .object
            .as_ref()
            .ok_or_else(|| fail("actual Object view owner absent"))?
            .variant;
        let character = self
            .binding
            .character
            .as_ref()
            .ok_or_else(|| fail("source Character definition absent"))?;
        let requested = if variant == -1 {
            u32::try_from(character.aspect_ids.len()).map_err(fail)?
        } else {
            u32::try_from(variant).map_err(fail)?
        };
        actor_world::effective_animation_set(character, requested).map_err(fail)
    }
    fn cancel(&mut self) -> Result<()> {
        let owner = self.owner()?;
        self.queues.cancel_owner(owner).map_err(fail)
    }
    fn deactivate(&mut self, slot: Slot) -> Result<()> {
        if slot == Slot::Character {
            self.cancel()
        } else {
            Err(fail(format!(
                "4CFC80 reached {slot:?} virtual40 owner is required after active-pointer clear"
            )))
        }
    }
    fn enter(&mut self, slot: Slot) -> Result<()> {
        if slot == Slot::Character {
            self.cancel()
        } else {
            Err(fail(format!(
                "4CFC80 reached {slot:?} virtual3C owner is required"
            )))
        }
    }
    fn animation_empty_effect(&self, key: &str, effect: Effect) -> Result<bool> {
        let animation = self.actors.animations.get(key).map_err(fail)?;
        if !animation
            .actor_id
            .eq_ignore_ascii_case(&self.binding.definition.proof.id)
        {
            return Err(fail("Animation belongs to another actual Actor source"));
        }
        let actor = self
            .catalog
            .actors
            .resolved_actor(&animation.actor_id)
            .map_err(fail)?;
        let character = actor
            .child("CharacterComponent")
            .map_err(fail)?
            .ok_or_else(|| fail("Animation Character source absent"))?;
        let mut nodes = character.children("Animation").map_err(fail)?;
        for set in character.children("AspectAnimationSet").map_err(fail)? {
            nodes.extend(set.children("Animation").map_err(fail)?);
        }
        let matching: Vec<_> = nodes
            .into_iter()
            .filter(|n| n.sources() == animation.source.sources)
            .collect();
        if matching.len() != 1 {
            return Err(fail("Animation exact source-node ownership is ambiguous"));
        }
        let field = match effect {
            Effect::Start => "WhenStart",
            Effect::Use => "WhenDo",
            Effect::End => "WhenEnd",
            Effect::Cue(_) => return Err(fail("Animation cue FX compiler is required")),
        };
        Ok(matching[0].children(field).map_err(fail)?.is_empty())
    }
}
impl borrowed::World<String> for Bound<'_, '_, '_> {
    fn character(&self) -> Result<&Character<String>> {
        match self
            .actors
            .component(self.id, Slot::Character)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Character(c)) if c.header.owner == self.id => Ok(c),
            _ => Err(fail("actual Character payload/owner differs")),
        }
    }
    fn character_mut(&mut self) -> Result<&mut Character<String>> {
        match self
            .actors
            .component_mut(self.id, Slot::Character)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Character(c)) if c.header.owner == self.id => Ok(c),
            _ => Err(fail("actual Character payload/owner differs")),
        }
    }
    fn scheduled_head(&self) -> Result<Option<u64>> {
        Ok(self.queues.owner_head(self.owner()?).map(|h| h.raw()))
    }
    fn animation(&self, h: String) -> Result<Animation> {
        Ok(self
            .actors
            .animations
            .get(&h)
            .map_err(fail)?
            .source
            .timing
            .clone())
    }
    fn looping_type(&self, k: u32) -> Result<bool> {
        runtime::canonical_looping_type(k)
    }
    fn now(&self) -> Result<F32> {
        Ok(F32(self.clock.begin.0))
    }
    fn remap_variant(&mut self, _: i32, _: String) -> Result<String> {
        Err(fail(
            "563770 positive pending variant/source remap owner is required",
        ))
    }
    fn apply_variant(&mut self, _: i32) -> Result<()> {
        Err(fail(
            "4CFD90 actual stance property transition owner is required",
        ))
    }
    fn movement_speed(&self) -> Result<Option<F32>> {
        match self.actors.component(self.id, Slot::Mover).map_err(fail)? {
            None => Ok(None),
            Some(RuntimeComponent::Mover(m)) if m.header.owner == self.id => m
                .words
                .get(&0x20)
                .copied()
                .map(|x| Some(F32(x)))
                .ok_or_else(|| fail("actual Mover+20 speed unbound")),
            _ => Err(fail("actual Mover payload/owner differs")),
        }
    }
    fn visual_scale(&mut self) -> Result<F32> {
        Err(fail(
            "563770 positive Actor virtualF4 visual scale owner is required",
        ))
    }
    fn effect(&mut self, h: String, e: Effect) -> Result<()> {
        if self.animation_empty_effect(&h, e)? {
            Ok(())
        } else {
            Err(fail(format!(
                "42A590 actual Animation {e:?} FX compiler/presentation owner is required"
            )))
        }
    }
    fn cancel_events(&mut self) -> Result<()> {
        self.cancel()
    }
    fn schedule_after(&mut self, d: F32, e: u32) -> Result<()> {
        let owner = self.owner()?;
        self.queues.add_after(3, owner, e, d.0).map_err(fail)?;
        Ok(())
    }
}
impl borrowed::StanceWorld<String> for Bound<'_, '_, '_> {
    fn active_component_is_character(&self) -> Result<bool> {
        Ok(active(self.actors, self.key)? == Some(Slot::Character))
    }
    fn activate(&mut self) -> Result<bool> {
        let mut context = crate::movement_world::ActivationContext {
            actors: self.actors,
            spatial: self.spatial,
            queues: self.queues,
            owners: self.owners,
        };
        crate::movement_world::activate_component(
            self.id,
            Some(Slot::Character),
            &mut context,
            &mut CharacterActivation,
        )
        .map_err(fail)
    }
    fn performer(&self) -> Result<Option<u32>> {
        match self
            .actors
            .component(self.id, Slot::Performer)
            .map_err(fail)?
        {
            None => Ok(None),
            Some(RuntimeComponent::Performer(p)) if p.state.owner == self.id.raw() => {
                Ok(Some(Slot::Performer as u32))
            }
            _ => Err(fail("actual Performer payload/owner differs")),
        }
    }
    fn component_is_active(&self, p: u32) -> Result<bool> {
        if p != Slot::Performer as u32 {
            return Err(fail("foreign component identity in stance query"));
        }
        Ok(active(self.actors, self.key)? == Some(Slot::Performer))
    }
    fn actor_flag(&self, f: u32) -> Result<bool> {
        let s = self
            .actors
            .object(self.key)
            .map_err(fail)?
            .components()
            .map_err(fail)?
            .state;
        Ok((u64::from(s.low) | (u64::from(s.high) << 32)) & (1u64 << (f & 63)) != 0)
    }
    fn performer_ability(&self, p: u32) -> Result<Option<u32>> {
        if p != Slot::Performer as u32 {
            return Err(fail("foreign Performer identity in stance query"));
        }
        match self
            .actors
            .component(self.id, Slot::Performer)
            .map_err(fail)?
        {
            Some(RuntimeComponent::Performer(p)) => Ok(p.state.active_ability),
            _ => Err(fail("Performer removed during stance query")),
        }
    }
    fn model_variant(&self) -> Result<u32> {
        self.set()
    }
    fn ability_idle(&self, h: u32, set: u32) -> Result<Option<String>> {
        let a = self
            .actors
            .abilities
            .get(&h)
            .ok_or_else(|| fail("active Ability handle is absent"))?;
        let local = self
            .bank
            .entry(
                a.local_definition
                    .ok_or_else(|| fail("actual local Ability definition is null"))?,
            )
            .map_err(fail)?;
        let loaded = self
            .binding
            .performer
            .as_ref()
            .ok_or_else(|| fail("actual Performer source absent"))?;
        let index = (0..loaded.abilities().len())
            .find(|i| {
                local.key.domain == "actorlocalability"
                    && local.key.name.eq_ignore_ascii_case(&format!(
                        "{}/ability/{i}",
                        self.binding.definition.proof.id
                    ))
            })
            .ok_or_else(|| fail("active Ability local source identity differs"))?;
        let idle = &loaded.abilities()[index]
            .animations
            .get(set as usize)
            .ok_or_else(|| fail("native Ability idle set outside actual allocation"))?
            .idle;
        let animation = self
            .actors
            .animations
            .find(&self.binding.definition.proof.id, set, &idle.name)
            .map_err(fail)?
            .ok_or_else(|| fail("native Ability idle pointer absent"))?;
        if animation.source.timing.kind != 0
            || animation.source.timing.length.0 != idle.length_bits
            || animation.source.sources != idle.sources
        {
            return Err(fail("actual Ability idle Animation identity differs"));
        }
        Ok(Some(actor_world::animation_key(
            &animation.actor_id,
            animation.set,
            animation.row,
        )))
    }
    fn first_idle(&self) -> Result<Option<String>> {
        let set = self.set()?;
        let rows = self
            .binding
            .character
            .as_ref()
            .ok_or_else(|| fail("source Character absent"))?
            .for_variant(set)
            .map_err(fail)?;
        Ok(rows
            .iter()
            .position(|a| a.timing.kind == 0)
            .map(|row| actor_world::animation_key(&self.binding.definition.proof.id, set, row)))
    }
    fn idle_repetitions(&mut self) -> Result<i32> {
        //4F02B0 logs the same world seed before529C30/49B4D0 read either bound.
        crate::configure_world::log_random_seed(self.catalog, self.random).map_err(fail)?;
        let read = |name| {
            self.catalog
                .actors
                .native_field(
                    &self.binding.definition.proof.id,
                    name,
                    self.binding.precision,
                )
                .map_err(fail)?
                .ok_or_else(|| fail(format!("49B4D0 source {name} absent")))?
                .integer()
                .map_err(fail)
        };
        let min = read("View.idle_loops_min")?;
        let max = read("View.idle_loops_max")?;
        runtime::idle_repetitions(min, max, self.random)
    }
}
struct CharacterActivation;
impl crate::movement_world::ActivationEffects for CharacterActivation {
    fn permits_inactive(
        &mut self,
        _: ActorId,
        s: Slot,
        _: &mut crate::movement_world::ActivationContext<'_, '_>,
    ) -> aa_mind::Result<bool> {
        if s == Slot::Character {
            Ok(true)
        } else {
            Err(aa_mind::Error::World(
                "actual other component virtual44 is required".into(),
            ))
        }
    }
    fn enter_other(
        &mut self,
        a: ActorId,
        s: Slot,
        _: bool,
        c: &mut crate::movement_world::ActivationContext<'_, '_>,
    ) -> aa_mind::Result<()> {
        self.leave_other(a, s, false, c)
    }
    fn leave_other(
        &mut self,
        a: ActorId,
        s: Slot,
        _: bool,
        c: &mut crate::movement_world::ActivationContext<'_, '_>,
    ) -> aa_mind::Result<()> {
        if s != Slot::Character {
            return Err(aa_mind::Error::World(format!(
                "actual {s:?} activation virtual3C/40 is required"
            )));
        }
        let key = c
            .actors
            .key(a)
            .ok_or_else(|| aa_mind::Error::World("Character GTID is stale".into()))?;
        let owner = c
            .owners
            .get(crate::scheduler_owners::Target::Component {
                object: key,
                slot: s as u8,
            })
            .map_err(|e| aa_mind::Error::World(e.to_string()))?;
        c.queues
            .cancel_owner(owner)
            .map_err(|e| aa_mind::Error::World(e.to_string()))?;
        Ok(())
    }
    fn mover_effect(
        &mut self,
        _: ActorId,
        _: bool,
        _: &mut crate::movement_world::ActivationContext<'_, '_>,
    ) -> aa_mind::Result<()> {
        Err(aa_mind::Error::World(
            "actual Mover start/stop FX source owner is required".into(),
        ))
    }
}
