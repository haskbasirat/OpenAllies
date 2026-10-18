//!5638B0 and its4CFD90 branch over the same stored Character/Actor/Properties.
//! Pending stance and completed property mutations survive reached child errors.
use crate::{
    actor_world::{self, ObjectKey, RuntimeComponent},
    catalog::{ActorBindings, Catalog},
    stored_actor_state_effects::Context,
};
use aa_actors::{ActorId, components::ComponentSlot as Slot};
use aa_animation::runtime::{self, Animation, Character, Effect, borrowed};
use aa_body::{F32, credit::Object};
use aa_numeric::Precision;
use aa_rules::native::{
    TypedValue,
    layout_data::{PropertyRef, RootPosition},
};
use std::sync::Arc;
type Result<T> = std::result::Result<T, String>;
fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn animation_error(e: impl std::fmt::Display) -> runtime::Error {
    runtime::Error::Service(e.to_string())
}

trait World {
    type Set: PartialEq;
    type Animation: Clone;
    fn requested_set(&mut self, stance: i32) -> Result<Self::Set>;
    fn current_set(&mut self) -> Result<Self::Set>;
    fn current_stance(&mut self) -> Result<i32>;
    fn stance_count(&mut self) -> Result<i32>;
    fn invalid_stance(&mut self, stance: i32, current: bool) -> Result<()>;
    fn stance_property(&mut self, stance: i32) -> Result<Option<PropertyRef>>;
    fn has_properties(&mut self) -> Result<bool>;
    fn remove_property(&mut self, property: PropertyRef) -> Result<()>;
    fn applicable(&mut self, property: PropertyRef) -> Result<bool>;
    fn add_property(&mut self, property: PropertyRef) -> Result<()>;
    fn store_stance(&mut self, stance: i32) -> Result<()>;
    fn store_pending(&mut self, stance: i32) -> Result<()>;
    fn current_animation(&mut self) -> Result<Self::Animation>;
    fn animation_allows_transition(&mut self, animation: Self::Animation) -> Result<bool>;
    fn play(&mut self, animation: Self::Animation) -> Result<()>;
}
///4CFD90 with the Character-recursion flagfalse. The native count is captured
///once after the equality guard; selected Property fields remain lazy.
fn apply_stance(world: &mut impl World, next: i32) -> Result<bool> {
    let old = world.current_stance()?;
    if old == next {
        return Ok(false);
    }
    let count = world.stance_count()?;
    if count <= next {
        world.invalid_stance(next, false)?;
        return Ok(false);
    }
    if old != -1 {
        if old < count {
            if let Some(property) = world.stance_property(old)? {
                if world.has_properties()? {
                    world.remove_property(property)?;
                }
            }
        } else {
            world.invalid_stance(old, true)?;
        }
    }
    if next != -1 {
        if let Some(property) = world.stance_property(next)? {
            if world.has_properties()? && world.applicable(property)? {
                world.add_property(property)?;
            }
        }
    }
    world.store_stance(next)?;
    Ok(true)
}
fn change(world: &mut impl World, next: i32) -> Result<bool> {
    let requested = world.requested_set(next)?;
    let current = world.current_set()?;
    if requested == current {
        return apply_stance(world, next);
    }
    world.store_pending(next)?;
    let animation = world.current_animation()?;
    if world.animation_allows_transition(animation.clone())? {
        world.play(animation)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

///The caller already lends the real Arena, complete PropertyState and World
///owners. Source-PC is read from that moved Bank; arithmetic uses live_pc.
pub(crate) fn set(
    catalog: &Catalog<'_>,
    context: &mut Context<'_, '_>,
    actor: ActorId,
    next: i32,
    live_pc: Precision,
) -> Result<bool> {
    catalog.require_source_owner()?;
    let key = context
        .actors
        .key(actor)
        .ok_or("5638B0 current Actor generation absent")?;
    let object = context.actors.object(key).map_err(error)?;
    let source =
        crate::live_object_init::Source::bind_in_bank(catalog, context.bank, &object.definition)
            .map_err(error)?;
    let binding = Arc::clone(
        context
            .actors
            .entity(key)
            .map_err(error)?
            .bindings
            .as_ref()
            .ok_or("5638B0 actual Actor source allocation absent")?,
    );
    let actual = catalog.actor_bindings(&binding.definition.proof.id, binding.precision)?;
    if !Arc::ptr_eq(&actual, &binding)
        || binding.character.is_none()
        || object.definition.native_class != 3
        || source.source_precision() != binding.precision
        || !crate::load_spatial::same_proof(&object.definition.proof, &binding.definition.proof)
    {
        return Err("5638B0 actual Character/Actor source identity differs".into());
    }
    //49DC40/464BD0 append source arrays. This binding describes one completed
    //declaration; a reloaded ObjectData requires its retained array owner.
    let mut declarations = 0usize;
    for tag in aa_rules::actors::native_load::THING_TAGS {
        for node in catalog.actors.resolved_definitions(tag).map_err(error)? {
            if let Some(TypedValue::String { value, .. }) =
                node.field("IDS", binding.precision).map_err(error)?
                && value.eq_ignore_ascii_case(&binding.definition.proof.id)
            {
                declarations += 1;
            }
        }
    }
    if declarations != 1 {
        return Err("5638B0 requires actual shared reload/stance/aspect ownership".into());
    }
    if context.queues.precision != live_pc || context.spatial.index().precision() != live_pc {
        return Err("5638B0 actual queue/spatial live PC differs".into());
    }
    change(
        &mut Bound {
            catalog,
            context,
            actor,
            key,
            binding,
            pc: live_pc,
        },
        next,
    )
}
struct Bound<'a, 'context, 'store, 'source> {
    catalog: &'a Catalog<'source>,
    context: &'a mut Context<'context, 'store>,
    actor: ActorId,
    key: ObjectKey,
    binding: Arc<ActorBindings>,
    pc: Precision,
}
impl Bound<'_, '_, '_, '_> {
    fn source(&self) -> Result<aa_rules::actors::ResolvedNode<'_>> {
        self.catalog
            .require_untranslated_actor_identity(&self.binding.definition.proof.id)?;
        self.catalog
            .actors
            .resolved_actor(&self.binding.definition.proof.id)
            .map_err(error)
    }
    fn stance(&self, index: i32) -> Result<aa_rules::actors::ResolvedNode<'_>> {
        self.source()?
            .children("Stance")
            .map_err(error)?
            .into_iter()
            .nth(usize::try_from(index).map_err(error)?)
            .ok_or("5638B0 stance outside actual source allocation".into())
    }
    fn normal_aspect(&self) -> Result<i32> {
        let source = self
            .catalog
            .actors
            .native_creation_definition(&self.binding.definition.proof.id, self.binding.precision)
            .map_err(error)?;
        if source.variations_enabled {
            return Err("5638B0 variation-mode ObjectData+118 producer required".into());
        }
        //49DC40 loads AspectNormal first and stores its just-appended index
        //to118 before the ordinary Aspect loop. The typed source requires it.
        if source.aspects.is_empty() {
            return Err("5638B0 actual AspectNormal allocation absent".into());
        }
        Ok(0)
    }
    fn aspect(&self, stance: i32) -> Result<i32> {
        if stance == -1 {
            return self.normal_aspect();
        }
        let id = text(
            self.stance(stance)?
                .field("aspect_ids", self.binding.precision)
                .map_err(error)?,
            "Stance.aspect_ids",
        )?;
        if id.is_empty() {
            return self.normal_aspect();
        }
        let character = self
            .binding
            .character
            .as_ref()
            .ok_or("5638B0 actual Character source absent")?;
        let index = character
            .aspect_ids
            .iter()
            .position(|v| v.eq_ignore_ascii_case(&id))
            .ok_or("5638B0 stance aspect absent from actual View allocation")?;
        i32::try_from(index).map_err(error)
    }
    fn effective(&self, index: i32) -> Result<u32> {
        let character = self
            .binding
            .character
            .as_ref()
            .ok_or("5638B0 actual Character source absent")?;
        let index = if index == -1 {
            u32::try_from(character.aspect_ids.len()).map_err(error)?
        } else {
            u32::try_from(index).map_err(error)?
        };
        actor_world::effective_animation_set(character, index).map_err(error)
    }
    fn word(&self, offset: u32) -> Result<u32> {
        self.context
            .actors
            .object(self.key)
            .map_err(error)?
            .actor
            .as_ref()
            .ok_or("5638B0 actual Actor absent")?
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| format!("5638B0 Actor+{offset:X} unwritten"))
    }
    fn animation_set(&self, stance: i32) -> Result<u32> {
        self.effective(self.aspect(stance)?)
    }
    fn scheduler_owner(&self) -> Result<u64> {
        self.context
            .owners
            .get(crate::scheduler_owners::Target::Component {
                object: self.key,
                slot: Slot::Character as u8,
            })
            .map_err(error)
    }
    fn property_definition(&mut self, reference: PropertyRef) -> Result<Object> {
        self.context
            .actors
            .property_allocations
            .bind_in_bank(self.catalog, self.context.bank)?;
        self.context
            .actors
            .property_allocations
            .definition_for_reference(reference)
    }
    fn source_effect_empty(&self, handle: &str, effect: Effect) -> Result<bool> {
        let animation = self.context.actors.animations.get(handle).map_err(error)?;
        if !animation
            .actor_id
            .eq_ignore_ascii_case(&self.binding.definition.proof.id)
        {
            return Err("563770 current Animation source belongs to another Actor".into());
        }
        let source = self.source()?;
        let character = source
            .child("CharacterComponent")
            .map_err(error)?
            .ok_or("563770 actual Character source absent")?;
        let mut nodes = character.children("Animation").map_err(error)?;
        for set in character.children("AspectAnimationSet").map_err(error)? {
            nodes.extend(set.children("Animation").map_err(error)?)
        }
        let matching = nodes
            .into_iter()
            .filter(|n| n.sources() == animation.source.sources)
            .collect::<Vec<_>>();
        if matching.len() != 1 {
            return Err("563770 actual Animation source identity ambiguous".into());
        }
        let field = match effect {
            Effect::Start => "WhenStart",
            Effect::Use => "WhenDo",
            Effect::End => "WhenEnd",
            Effect::Cue(_) => return Err("563770 cue FX owner required".into()),
        };
        Ok(matching[0].children(field).map_err(error)?.is_empty())
    }
}
fn text(value: Option<TypedValue>, field: &str) -> Result<String> {
    match value {
        Some(TypedValue::String { value, .. }) => Ok(value),
        _ => Err(format!("5638B0 typed {field} unavailable")),
    }
}
impl World for Bound<'_, '_, '_, '_> {
    type Set = u32;
    type Animation = String;
    fn requested_set(&mut self, stance: i32) -> Result<u32> {
        self.animation_set(stance)
    }
    fn current_set(&mut self) -> Result<u32> {
        let current = self.word(0x15c)? as i32;
        if current != -1 {
            return self.animation_set(current);
        }
        self.effective(
            self.context
                .actors
                .object(self.key)
                .map_err(error)?
                .object
                .as_ref()
                .ok_or("5638B0 Object variant absent")?
                .variant,
        )
    }
    fn current_stance(&mut self) -> Result<i32> {
        Ok(self.word(0x15c)? as i32)
    }
    fn stance_count(&mut self) -> Result<i32> {
        i32::try_from(self.source()?.children("Stance").map_err(error)?.len()).map_err(error)
    }
    fn invalid_stance(&mut self, _: i32, _: bool) -> Result<()> {
        Err("4CFD90 invalid stance requires actual404ED0 diagnostic sink".into())
    }
    fn stance_property(&mut self, stance: i32) -> Result<Option<PropertyRef>> {
        let node = self.stance(stance)?;
        let id = text(
            node.field("property_IDS", self.binding.precision)
                .map_err(error)?,
            "Stance.property_IDS",
        )?;
        if id.is_empty() {
            return Ok(None);
        }
        let source = self.source()?.sources();
        let source = source.first().ok_or("4B4640 source root absent")?;
        let file = self
            .catalog
            .actors
            .sources
            .iter()
            .position(|s| s.path.eq_ignore_ascii_case(&source.path))
            .ok_or("4B4640 source file identity absent")?;
        let layout = self.catalog.layout_data(self.binding.precision)?;
        layout
            .find_property(
                &id,
                Some(RootPosition {
                    file,
                    offset: source.span.start,
                }),
            )
            .map(Some)
            .ok_or("4B4640 immediate source Property Find failed".into())
    }
    fn has_properties(&mut self) -> Result<bool> {
        Ok(self
            .context
            .actors
            .component(self.actor, Slot::Properties)
            .map_err(error)?
            .is_some())
    }
    fn remove_property(&mut self, reference: PropertyRef) -> Result<()> {
        let definition = self.property_definition(reference)?;
        crate::stored_state_properties::remove(
            self.catalog,
            self.context.actors,
            self.context.bank,
            self.context.variables,
            self.actor,
            definition,
            self.actor,
            self.pc,
        )
    }
    fn applicable(&mut self, p: PropertyRef) -> Result<bool> {
        crate::staged_property_admission::applicable_on_split_owners(
            self.catalog,
            self.context.actors,
            self.context.bank,
            self.actor,
            p,
        )
    }
    fn add_property(&mut self, p: PropertyRef) -> Result<()> {
        crate::stored_state_properties::add(
            self.catalog,
            self.context.actors,
            self.context.arena,
            self.context.bank,
            self.context.spatial,
            self.context.variables,
            self.context.clock,
            self.context.inventory,
            self.actor,
            p,
            9,
            Some(self.actor),
            self.pc,
        )
    }
    fn store_stance(&mut self, stance: i32) -> Result<()> {
        *self
            .context
            .actors
            .object_mut(self.key)
            .map_err(error)?
            .actor
            .as_mut()
            .ok_or("4CFD90 actual Actor absent")?
            .words
            .get_mut(&0x15c)
            .ok_or("4CFD90 Actor+15C unwritten")? = stance as u32;
        Ok(())
    }
    fn store_pending(&mut self, stance: i32) -> Result<()> {
        borrowed::World::character_mut(self).map_err(error)?.variant = stance;
        Ok(())
    }
    fn current_animation(&mut self) -> Result<String> {
        borrowed::World::character(self)
            .map_err(error)?
            .animation
            .clone()
            .ok_or("5638B0 current Animation is null".into())
    }
    fn animation_allows_transition(&mut self, h: String) -> Result<bool> {
        runtime::canonical_looping_type(
            self.context
                .actors
                .animations
                .get(&h)
                .map_err(error)?
                .source
                .timing
                .kind,
        )
        .map_err(error)
    }
    fn play(&mut self, h: String) -> Result<()> {
        borrowed::play(self, h, F32::ZERO, self.pc).map_err(error)
    }
}
impl borrowed::World<String> for Bound<'_, '_, '_, '_> {
    fn character(&self) -> runtime::Result<&Character<String>> {
        match self
            .context
            .actors
            .component(self.actor, Slot::Character)
            .map_err(animation_error)?
        {
            Some(RuntimeComponent::Character(c)) if c.header.owner == self.actor => Ok(c),
            _ => Err(animation_error(
                "5638B0 actual Character payload/owner differs",
            )),
        }
    }
    fn character_mut(&mut self) -> runtime::Result<&mut Character<String>> {
        match self
            .context
            .actors
            .component_mut(self.actor, Slot::Character)
            .map_err(animation_error)?
        {
            Some(RuntimeComponent::Character(c)) if c.header.owner == self.actor => Ok(c),
            _ => Err(animation_error(
                "5638B0 actual Character payload/owner differs",
            )),
        }
    }
    fn scheduled_head(&self) -> runtime::Result<Option<u64>> {
        Ok(self
            .context
            .queues
            .owner_head(self.scheduler_owner().map_err(animation_error)?)
            .map(|v| v.raw()))
    }
    fn animation(&self, h: String) -> runtime::Result<Animation> {
        Ok(self
            .context
            .actors
            .animations
            .get(&h)
            .map_err(animation_error)?
            .source
            .timing
            .clone())
    }
    fn looping_type(&self, kind: u32) -> runtime::Result<bool> {
        runtime::canonical_looping_type(kind)
    }
    fn now(&self) -> runtime::Result<F32> {
        Ok(F32(self.context.clock.begin.0))
    }
    fn remap_variant(&mut self, variant: i32, requested: String) -> runtime::Result<String> {
        let set = self.animation_set(variant).map_err(animation_error)?;
        let old = self
            .context
            .actors
            .animations
            .get(&requested)
            .map_err(animation_error)?;
        if !old
            .actor_id
            .eq_ignore_ascii_case(&self.binding.definition.proof.id)
        {
            return Err(animation_error("563770 Animation owner differs"));
        }
        let remapped = self
            .context
            .actors
            .animations
            .find(&old.actor_id, set, &old.original_name)
            .map_err(animation_error)?
            .ok_or_else(|| {
                animation_error(
                    "563770 missing remap requires404ED0 diagnostic and native type-array fallback",
                )
            })?;
        Ok(actor_world::animation_key(
            &remapped.actor_id,
            remapped.set,
            remapped.row,
        ))
    }
    fn apply_variant(&mut self, variant: i32) -> runtime::Result<()> {
        apply_stance(self, variant)
            .map(|_| ())
            .map_err(animation_error)
    }
    fn movement_speed(&self) -> runtime::Result<Option<F32>> {
        match self
            .context
            .actors
            .component(self.actor, Slot::Mover)
            .map_err(animation_error)?
        {
            None => Ok(None),
            Some(RuntimeComponent::Mover(m)) if m.header.owner == self.actor => m
                .words
                .get(&0x20)
                .copied()
                .map(|v| Some(F32(v)))
                .ok_or_else(|| animation_error("563770 Mover+20 unbound")),
            _ => Err(animation_error("563770 actual Mover payload/owner differs")),
        }
    }
    fn visual_scale(&mut self) -> runtime::Result<F32> {
        Err(animation_error(
            "563770 actual Actor virtualF4 visual-scale owner required",
        ))
    }
    fn effect(&mut self, h: String, e: Effect) -> runtime::Result<()> {
        if self.source_effect_empty(&h, e).map_err(animation_error)? {
            Ok(())
        } else {
            Err(animation_error(
                "563770 positive Animation FX compiled graph/presentation required",
            ))
        }
    }
    fn cancel_events(&mut self) -> runtime::Result<()> {
        let owner = self.scheduler_owner().map_err(animation_error)?;
        self.context
            .queues
            .cancel_owner(owner)
            .map_err(animation_error)
    }
    fn schedule_after(&mut self, delay: F32, event: u32) -> runtime::Result<()> {
        let owner = self.scheduler_owner().map_err(animation_error)?;
        self.context
            .queues
            .add_after(3, owner, event, delay.0)
            .map(|_| ())
            .map_err(animation_error)
    }
}
#[cfg(test)]
#[path = "stored_character_view_tests.rs"]
mod tests;
