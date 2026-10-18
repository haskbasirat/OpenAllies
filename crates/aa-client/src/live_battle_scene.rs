//! Read-only presentation bridge from the adopted native world. This module
//! does not advance simulation or derive transforms from extracted placements.
//! A view owner must supply the actual visibility and replacement-color result;
//! there is deliberately no all-visible or guessed local-player fallback.
use super::{ActorVisual, LiveScene, LiveUpdateReport};
use crate::{gpu::SceneRenderer, inspection::SceneObjects};
use aa_actors::ActorId;
use aa_mapworld::{SourceIdentity, bootstrap::DefinitionProof};
use aa_runtime::{
    actor_world::{ActorStore, ActorStoreWitness, ObjectKey, PlacementState, RuntimeObject},
    restoration::Staged,
    spatial_world::{SourceGeometry, SpatialWorld, SpatialWorldWitness},
};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, sync::Arc};
#[path = "live_battle_visibility.rs"]
pub mod native;
#[path = "rendered_map_source.rs"]
mod source;
pub(crate) use source::RenderedMapSource;

#[derive(Clone, Copy, Debug)]
pub struct Appearance {
    pub visible: bool,
    pub team_color: Option<[f32; 3]>,
}

#[cfg(test)]
#[path = "live_battle_scene_tests.rs"]
mod tests;

/// Reads the current view's native state at collection time. A missing local
/// player, Kingdom, fog variable, or positive component service must be an error.
pub trait Presentation {
    fn appearance(
        &self,
        actors: &ActorStore,
        spatial: &SpatialWorld,
        actor: ActorId,
        object: &RuntimeObject,
        placement: PlacementState,
    ) -> Result<Appearance>;
}

struct Binding {
    key: ObjectKey,
    /// Retains immutable source identity only, never mutable actor fields.
    geometry: Arc<SourceGeometry>,
    geometry_token: u32,
    proof: DefinitionProof,
    native_class: u32,
    definition_flags: u32,
    component_mask: Option<u32>,
    variant: i32,
    stance:Option<u32>,
    construction:bool,
    object_index: usize,
}

/// Immutable source/identity bindings only. Current pose, scale, fog and color
/// stay in their actual owners and are read again for every frame.
pub struct BattleScene {
    map_source: SourceIdentity,
    actor_owner: ActorStoreWitness,
    spatial_owner: SpatialWorldWitness,
    bindings: BTreeMap<ActorId, Binding>,
}

fn same_proof(a: &DefinitionProof, b: &DefinitionProof) -> bool {
    a.id.eq_ignore_ascii_case(&b.id)
        && a.source == b.source
        && a.template_chain == b.template_chain
        && a.category == b.category
        && a.base_flag_inputs == b.base_flag_inputs
}

fn spatial(staged: &Staged) -> Result<&SpatialWorld> {
    staged
        .load_world
        .as_ref()
        .context("native battle presentation requires the actual managed World")?
        .live_spatial()
        .map_err(anyhow::Error::msg)
}

impl BattleScene {
    pub fn from_live_world(world:&aa_runtime::live_world_frame::World<'_>,live:&LiveScene,objects:&SceneObjects)->Result<Self> {
        let p=world.presentation_parts().map_err(anyhow::Error::msg)?;
        Self::require_map_source(live,&p.inventory.map_source)?;
        Self::from_owners(p.actors,p.spatial,p.inventory.map_source.clone(),live,objects)
    }
    pub fn collect_live_world(&self,world:&aa_runtime::live_world_frame::World<'_>,presentation:&impl Presentation)->Result<Vec<ActorVisual>> {
        let p=world.presentation_parts().map_err(anyhow::Error::msg)?;
        ensure!(self.map_source==p.inventory.map_source,"native battle presentation belongs to another map source");
        self.collect_owners(p.actors,p.spatial,presentation)
    }
    pub fn apply_live_world(&self,world:&aa_runtime::live_world_frame::World<'_>,presentation:&impl Presentation,
        live:&mut LiveScene,renderer:&mut SceneRenderer,objects:&mut SceneObjects,sequence:u64)->Result<LiveUpdateReport> {
        self.require_live_bindings(live,objects)?;
        let snapshot=self.collect_live_world(world,presentation)?;
        live.apply(renderer,objects,sequence,&snapshot)
    }
    /// This is a presentation handoff, not battle-readiness certification.
    /// The managed world must have completed the same native index adoption.
    pub fn from_staged(staged: &Staged, live: &LiveScene, objects: &SceneObjects) -> Result<Self> {
        Self::require_map_source(live, &staged.inventory.map_source)?;
        let spatial = spatial(staged)?;
        Self::from_owners(
            &staged.actors,
            spatial,
            staged.inventory.map_source.clone(),
            live,
            objects,
        )
    }

    fn require_map_source(live: &LiveScene, source: &SourceIdentity) -> Result<()> {
        live.rendered_map_source
            .as_ref()
            .context("native battle requires a certified rendered RMP source")?
            .require_identity(source)
    }

    fn from_owners(
        actors: &ActorStore,
        spatial: &SpatialWorld,
        map_source: SourceIdentity,
        live: &LiveScene,
        objects: &SceneObjects,
    ) -> Result<Self> {
        ensure!(
            live.ambiguous_actor_ids().is_empty(),
            "native battle presentation has ambiguous rendered GTIDs"
        );
        let mut bindings = BTreeMap::new();
        for (&raw, render_binding) in &live.bindings {
            let id = ActorId::from_raw(raw);
            let key = actors
                .key(id)
                .context("rendered GTID is absent from the current native store")?;
            actors.entity(key)?;
            let runtime = actors.object(key)?;
            let fields = runtime
                .object
                .as_ref()
                .context("rendered native actor has no Object allocation")?;
            let picked = objects
                .objects
                .get(render_binding.object)
                .context("native render binding has no picking object")?;
            ensure!(
                picked.instance_id == raw
                    && picked
                        .thing_id
                        .eq_ignore_ascii_case(&runtime.definition.proof.id),
                "rendered model identity differs from the native definition"
            );
            ensure!(
                same_proof(&runtime.thing.definition, &runtime.definition.proof),
                "native Thing and held definition have different source proofs"
            );
            // Refuse unpublished/foreign placement even if ActorStore could
            // still return a staged transform for a diagnostic entity.
            let actor_key = spatial.key(key)?;
            let geometry_token = spatial.index().member(actor_key)?.actor.definition;
            let geometry = spatial.source_geometry(geometry_token)?;
            ensure!(
                geometry.definition == geometry_token
                    && geometry
                        .source
                        .id
                        .eq_ignore_ascii_case(&runtime.definition.proof.id)
                    && geometry.source.source == runtime.definition.proof.source
                    && geometry.definition_flags == runtime.definition.flags,
                "native spatial geometry has another canonical source"
            );
            ensure!(
                spatial.object(actor_key)? == key
                    && spatial.index().resolve_reference(raw) == Some(actor_key),
                "native render actor is not published in this adopted index"
            );
            actors.placement(key, spatial)?;
            bindings.insert(
                id,
                Binding {
                    key,
                    geometry: spatial.source_geometry_owner(geometry_token)?,
                    geometry_token,
                    proof: runtime.definition.proof.clone(),
                    native_class: runtime.definition.native_class,
                    definition_flags: runtime.definition.flags,
                    component_mask: runtime.definition.component_mask,
                    variant: fields.variant,
                    stance:match runtime.actor.as_ref() {Some(a)=>Some(*a.words.get(&0x15c)
                        .context("native model binding has an unbound stance selector")?),None=>None},
                    construction:runtime.actor.as_ref().is_some_and(|a|a.components.state.construction()),
                    object_index: render_binding.object,
                },
            );
        }
        Ok(Self {
            map_source,
            actor_owner: actors.identity_witness(),
            spatial_owner: spatial.identity_witness(),
            bindings,
        })
    }

    pub fn actor_count(&self) -> usize {
        self.bindings.len()
    }

    /// Collect the entire snapshot before touching GPU or picking state.
    pub fn collect(
        &self,
        staged: &Staged,
        presentation: &impl Presentation,
    ) -> Result<Vec<ActorVisual>> {
        ensure!(
            self.map_source == staged.inventory.map_source,
            "native battle presentation belongs to another map source"
        );
        let spatial = spatial(staged)?;
        self.collect_owners(&staged.actors, spatial, presentation)
    }

    fn collect_owners(
        &self,
        actors: &ActorStore,
        spatial: &SpatialWorld,
        presentation: &impl Presentation,
    ) -> Result<Vec<ActorVisual>> {
        ensure!(
            self.actor_owner.is_for(actors),
            "native render binding belongs to another actor allocation owner"
        );
        ensure!(
            self.spatial_owner.is_for(spatial),
            "native render binding belongs to another spatial owner"
        );
        let mut result = Vec::new();
        result.try_reserve_exact(self.bindings.len())?;
        for (&id, binding) in &self.bindings {
            ensure!(
                actors.key(id) == Some(binding.key),
                "native render GTID was removed, replaced or changed generation"
            );
            actors.entity(binding.key)?;
            let object = actors.object(binding.key)?;
            ensure!(
                same_proof(&binding.proof, &object.definition.proof)
                    && same_proof(&object.thing.definition, &binding.proof)
                    && object.definition.native_class == binding.native_class
                    && object.definition.flags == binding.definition_flags
                    && object.definition.component_mask == binding.component_mask,
                "native render source or class changed without model rebinding"
            );
            let fields = object
                .object
                .as_ref()
                .context("native render Object allocation disappeared")?;
            ensure!(
                fields.variant == binding.variant,
                "native Object variant changed; source-resolved model rebinding is required"
            );
            ensure!(object.actor.as_ref().map(|a|a.words.get(&0x15c).copied())==binding.stance.map(Some)
                && object.actor.as_ref().is_some_and(|a|a.components.state.construction())==binding.construction,
                "native stance/construction selector changed; model rebinding is required");
            let key = spatial.key(binding.key)?;
            let token = spatial.index().member(key)?.actor.definition;
            ensure!(
                token == binding.geometry_token
                    && Arc::ptr_eq(&spatial.source_geometry_owner(token)?, &binding.geometry),
                "native render binding belongs to another spatial owner"
            );
            ensure!(
                spatial.index().resolve_reference(id.raw()) == Some(key),
                "native render spatial generation is stale"
            );
            let placement = actors.placement(binding.key, spatial)?;
            let appearance = presentation.appearance(actors, spatial, id, object, placement)?;
            let visual = ActorVisual {
                actor_id: id.raw(),
                native_position: placement.position_bits.map(f32::from_bits),
                native_facing: placement.facing_bits.map(f32::from_bits),
                scale: f32::from_bits(fields.scale_bits),
                visible: appearance.visible,
                team_color: appearance.team_color,
            };
            // Reuse the renderer's existing finite/invertibility validation,
            // while the actual snapshot is still unpublished.
            visual.instance()?;
            result.push(visual);
        }
        Ok(result)
    }

    pub fn apply(
        &self,
        staged: &Staged,
        presentation: &impl Presentation,
        live: &mut LiveScene,
        renderer: &mut SceneRenderer,
        objects: &mut SceneObjects,
        sequence: u64,
    ) -> Result<LiveUpdateReport> {
        self.require_live_bindings(live,objects)?;
        let actors = self.collect(staged, presentation)?;
        live.apply(renderer, objects, sequence, &actors)
    }
    fn require_live_bindings(&self,live:&LiveScene,objects:&SceneObjects)->Result<()> {
        Self::require_map_source(live, &self.map_source)?;
        for (&id, binding) in &self.bindings {
            ensure!(
                live.bindings
                    .get(&id.raw())
                    .is_some_and(|b| b.object == binding.object_index)
                    && objects
                        .objects
                        .get(binding.object_index)
                        .is_some_and(|o| o.instance_id == id.raw()
                            && o.thing_id.eq_ignore_ascii_case(&binding.proof.id)),
                "native battle presentation bindings changed scene"
            );
        }
        Ok(())
    }
    pub(crate) fn validate_live_bindings(&self,live:&LiveScene,objects:&SceneObjects)->Result<()> {
        self.require_live_bindings(live,objects)
    }
}
