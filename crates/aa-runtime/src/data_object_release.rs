//!Data vf18 release (ObjectData 49BB00 / ActorData 45DF40) and vf1C ready
//!(49BBB0 / 45EF20) over the SAME canonical Data allocations and source-owned
//!children that 49B9F0/45DD80 request. Runtime callers: WorldView 43EC60
//!polls its request queue with VObject vf74 (45B140 tail-jumps Data vf1C) and
//!then 45B720 releases (delta 1) and clears VObject+54 bit28; the VObject
//!uninit 45BCA0 does the same release for a still-flagged view.
//!Every Resource vf14 (5AA550) goes through the caller's ReleaseServices;
//!readiness reads only Resource flags+8 bit0.
use super::*;
use super::release_fields::{self, ActorRelease, ObjectRelease};
use crate::data_pre_request::release_kernels::{self as k, ResourceRelease};
use crate::view_vfx_source::Effect;

///Resource vf14 owner.
pub trait ReleaseServices {
    ///5AA550: delta1 InterlockedDecrement(+4), otherwise a locked subtract;
    ///at zero 5AEE70 ReleaseResource aborts the load task or unloads (5AC3D0).
    fn release_14(&mut self, resources: &mut Resources<'_>, handle: &Handle, delta: u32) -> Result<()>;
}
///Explicit stop until the ResourceManager owner provides 5AA550/5AEE70.
pub struct RequiredReleaseServices;
impl ReleaseServices for RequiredReleaseServices {
    fn release_14(&mut self, _: &mut Resources<'_>, handle: &Handle, _: u32) -> Result<()> {
        Err(format!(
            "5AA550 Resource vf14 release of {} requires the ResourceManager owner's 5AA550/5AEE70 port",
            String::from_utf16_lossy(handle.logical_ids())
        ))
    }
}
struct Res<'a, 'm, S> {
    resources: &'a mut Resources<'m>,
    services: &'a mut S,
}
impl<S: ReleaseServices> ResourceRelease for Res<'_, '_, S> {
    fn release_14(&mut self, handle: &Handle, delta: u32) -> Result<()> {
        self.services.release_14(self.resources, handle, delta)
    }
    fn loaded(&mut self, handle: &Handle) -> Result<bool> {
        Ok(self.resources.flags_08(handle)? & 1 != 0)
    }
}
impl State {
    ///Data vf18(delta): 49BB00 for ObjectData, 45DF40 for ActorData.
    pub fn release_object<S: ReleaseServices>(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        resources: &mut Resources<'_>,
        services: &mut S,
        delta: u32,
    ) -> Result<()> {
        self.release_inner(catalog, bank, d, &mut Res { resources, services }, delta)
    }
    ///Data vf1C(): 49BBB0 for ObjectData, 45EF20 for ActorData.
    pub fn object_ready(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        resources: &mut Resources<'_>,
    ) -> Result<bool> {
        self.ready_inner(catalog, bank, d, &mut Res { resources, services: &mut RequiredReleaseServices })
    }
    fn release_inner<S: ReleaseServices>(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        res: &mut Res<'_, '_, S>,
        delta: u32,
    ) -> Result<()> {
        let data = self.bind_object(catalog, bank, d)?;
        let mut w = Released { state: self, catalog, bank, definition: d, data, res };
        if d.proof.category.native_kind == 2 {
            release_fields::actor_release(&mut w, delta)
        } else {
            release_fields::object_release(&mut w, delta)
        }
    }
    fn ready_inner<S: ReleaseServices>(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        res: &mut Res<'_, '_, S>,
    ) -> Result<bool> {
        let data = self.bind_object(catalog, bank, d)?;
        let mut w = Released { state: self, catalog, bank, definition: d, data, res };
        if d.proof.category.native_kind == 2 {
            release_fields::actor_ready(&mut w)
        } else {
            release_fields::object_ready(&mut w)
        }
    }
}
struct Released<'a, 'r, 'source, 'manager, S> {
    state: &'a mut State,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    definition: &'a RuntimeDefinition,
    data: Data,
    res: &'a mut Res<'r, 'manager, S>,
}
impl<S: ReleaseServices> Released<'_, '_, '_, '_, S> {
    fn release_roots(&mut self, graph: &Graph, roots: &[NodeId], delta: u32) -> Result<()> {
        graph.require_in_bank(self.catalog, self.bank)?;
        k::fx_list_release(&mut Nodes { w: self, graph }, roots, delta)
    }
    fn roots_ready(&mut self, graph: &Graph, roots: &[NodeId]) -> Result<bool> {
        graph.require_in_bank(self.catalog, self.bank)?;
        k::fx_list_ready(&mut Nodes { w: self, graph }, roots)
    }
    fn texture(&self, offset: u32) -> Result<Option<Handle>> {
        let i = match offset {
            0xec => 0,
            0xf0 => 1,
            0x138 => 2,
            _ => return Err("Object texture field offset unsupported".into()),
        };
        Ok(self.state.ready_object(&self.data)?.textures[i].clone())
    }
    fn view_graph(&self) -> Result<Arc<Graph>> {
        self.state
            .ready_object(&self.data)?
            .graph
            .as_ref()
            .cloned()
            .ok_or_else(|| "actual Object six FX lists absent".into())
    }
    fn animations(&self) -> Result<Arc<crate::actor_animation_source::Source>> {
        self.state
            .entry(&self.data)?
            .actor_animations
            .as_ref()
            .cloned()
            .ok_or_else(|| "Actor Animation source absent".into())
    }
}
struct Nodes<'x, 'a, 'r, 's, 'm, S> {
    w: &'x mut Released<'a, 'r, 's, 'm, S>,
    graph: &'x Graph,
}
impl<S: ReleaseServices> Nodes<'_, '_, '_, '_, '_, S> {
    fn effect(&self, id: NodeId) -> Result<Effect> {
        let cap = self
            .graph
            .registered_reference(id, 0x3c)?
            .ok_or("428900/428950 CreateVFX +3C is null (native null dereference)")?;
        Effect::from_registered_reference(self.w.catalog, self.w.bank, &cap)
    }
    fn vfx_release(&mut self, id: NodeId, delta: u32) -> Result<()> {
        let effect = self.effect(id)?;
        let w = &mut *self.w;
        let data = w.state.bind_vfx(w.catalog, w.bank, &effect)?;
        let class = k::vfx_resource_class(effect.kind(w.catalog, w.bank)?)?;
        let mut v = Vfx { state: &mut *w.state, catalog: w.catalog, bank: w.bank, effect: &effect, data, res: &mut *w.res };
        k::vfx_release(&mut v, class, delta)
    }
    fn vfx_ready_at(&mut self, id: NodeId) -> Result<bool> {
        let effect = self.effect(id)?;
        let w = &mut *self.w;
        let data = w.state.bind_vfx(w.catalog, w.bank, &effect)?;
        let class = k::vfx_resource_class(effect.kind(w.catalog, w.bank)?)?;
        let mut v = Vfx { state: &mut *w.state, catalog: w.catalog, bank: w.bank, effect: &effect, data, res: &mut *w.res };
        k::vfx_ready(&mut v, class)
    }
}
fn audio_reference<'g>(graph: &'g Graph, id: NodeId, catalog: &Catalog<'_>, bank: &Bank) -> Result<crate::fx_graph::RegisteredReference<'g>> {
    let cap = graph
        .registered_reference(id, 0x44)?
        .ok_or("428900/428950 PlaySFX +44 is null (native null dereference)")?;
    cap.require_audio_reference(catalog, bank)?;
    Ok(cap)
}
impl<S: ReleaseServices> k::FxNodes for Nodes<'_, '_, '_, '_, '_, S> {
    type Node = NodeId;
    fn creates_vfx(&mut self, id: &NodeId) -> Result<bool> {
        Ok(matches!(self.graph.node_kind(*id).ok_or("FXC node kind absent")?, fx_graph::FxcKind::CreateVfx))
    }
    fn plays_sfx(&mut self, id: &NodeId) -> Result<bool> {
        Ok(matches!(self.graph.node_kind(*id).ok_or("FXC node kind absent")?, fx_graph::FxcKind::PlaySfx))
    }
    fn release_vfx(&mut self, id: &NodeId, delta: u32) -> Result<()> {
        self.vfx_release(*id, delta)
    }
    fn vfx_ready(&mut self, id: &NodeId) -> Result<bool> {
        self.vfx_ready_at(*id)
    }
    ///AudioData vf18 46B260.
    fn release_sfx(&mut self, id: &NodeId, delta: u32) -> Result<()> {
        let graph = self.graph;
        let w = &mut *self.w;
        let cap = audio_reference(graph, *id, w.catalog, w.bank)?;
        let lists = w.state.audio_data.prepared_sound_lists(w.catalog, w.bank, &cap)?;
        let res = &mut *w.res;
        k::audio_release(lists, delta, |h, d| res.release_14(h, d))
    }
    ///AudioData vf1C 46B060.
    fn sfx_ready(&mut self, id: &NodeId) -> Result<bool> {
        let graph = self.graph;
        let w = &mut *self.w;
        let cap = audio_reference(graph, *id, w.catalog, w.bank)?;
        let lists = w.state.audio_data.prepared_sound_lists(w.catalog, w.bank, &cap)?;
        let res = &mut *w.res;
        k::audio_ready(lists, |h| res.loaded(h))
    }
}
struct Vfx<'x, 'r, 'source, 'manager, 'e, S> {
    state: &'x mut State,
    catalog: &'x Catalog<'source>,
    bank: &'x Bank,
    effect: &'e Effect,
    data: Data,
    res: &'x mut Res<'r, 'manager, S>,
}
impl<S: ReleaseServices> k::VfxRelease for Vfx<'_, '_, '_, '_, '_, S> {
    type Resource = Handle;
    fn release_base(&mut self, delta: u32) -> Result<()> {
        self.state.release_base(&self.data, delta)
    }
    fn requested(&mut self) -> Result<bool> {
        self.state.requested_base(&self.data)
    }
    fn resource40(&mut self) -> Result<Option<Handle>> {
        self.state.vfx_resource40(self.catalog, self.bank, self.effect)
    }
    fn release(&mut self, h: &Handle, delta: u32) -> Result<()> {
        self.res.release_14(h, delta)
    }
    fn loaded(&mut self, h: &Handle) -> Result<bool> {
        self.res.loaded(h)
    }
}
impl<S: ReleaseServices> ObjectRelease for Released<'_, '_, '_, '_, S> {
    type Data = RuntimeDefinition;
    type Aspect = super::super::aspect::Aspect;
    fn release_base(&mut self, delta: u32) -> Result<()> {
        self.state.release_base(&self.data, delta)
    }
    fn requested_base(&mut self) -> Result<bool> {
        self.state.requested_base(&self.data)
    }
    fn portrait(&mut self) -> Result<Option<RuntimeDefinition>> {
        Ok(self.state.ready_object(&self.data)?.portrait.clone())
    }
    fn release_data(&mut self, d: RuntimeDefinition, delta: u32) -> Result<()> {
        self.state.release_inner(self.catalog, self.bank, &d, self.res, delta)
    }
    fn data_ready(&mut self, d: RuntimeDefinition) -> Result<bool> {
        self.state.ready_inner(self.catalog, self.bank, &d, self.res)
    }
    fn release_banner(&mut self, delta: u32) -> Result<()> {
        self.state.ready_object(&self.data)?;
        if let Some(r) = self.state.entry(&self.data)?.banner.clone() {
            self.state.release_banner(self.catalog, self.bank, &r, delta, self.res)?;
        }
        Ok(())
    }
    fn banner_ready(&mut self) -> Result<bool> {
        self.state.ready_object(&self.data)?;
        match self.state.entry(&self.data)?.banner.clone() {
            Some(r) => self.state.banner_ready(self.catalog, self.bank, &r, self.res),
            None => Ok(true),
        }
    }
    fn aspect_count(&mut self) -> Result<isize> {
        self.state.aspect_request_count(&self.data)
    }
    fn aspect(&mut self, i: usize) -> Result<Self::Aspect> {
        self.state.object_aspect(self.catalog, self.bank, self.definition, i)
    }
    fn release_aspect(&mut self, a: Self::Aspect, delta: u32) -> Result<()> {
        self.state.release_aspect(self.catalog, self.bank, &a, delta, self.res)
    }
    fn aspect_ready(&mut self, a: Self::Aspect) -> Result<bool> {
        self.state.aspect_ready(self.catalog, self.bank, &a, self.res)
    }
    fn release_texture(&mut self, offset: u32, delta: u32) -> Result<()> {
        if let Some(h) = self.texture(offset)? {
            self.res.release_14(&h, delta)?;
        }
        Ok(())
    }
    fn texture_ready(&mut self, offset: u32) -> Result<bool> {
        match self.texture(offset)? {
            Some(h) => self.res.loaded(&h),
            None => Ok(true),
        }
    }
    fn release_view_fx(&mut self, i: usize, delta: u32) -> Result<()> {
        let graph = self.view_graph()?;
        self.release_roots(&graph, graph.view_list_roots(ListKind::ALL[i])?, delta)
    }
    fn view_fx_ready(&mut self, i: usize) -> Result<bool> {
        let graph = self.view_graph()?;
        self.roots_ready(&graph, graph.view_list_roots(ListKind::ALL[i])?)
    }
}
///A source Animation's 428900/428950 calls through the SAME release owner.
struct AnimationLists<'x, 'a, 'r, 's, 'm, S>(&'x mut Released<'a, 'r, 's, 'm, S>);
impl<S: ReleaseServices> ResourceRelease for AnimationLists<'_, '_, '_, '_, '_, S> {
    fn release_14(&mut self, h: &Handle, delta: u32) -> Result<()> {
        self.0.res.release_14(h, delta)
    }
    fn loaded(&mut self, h: &Handle) -> Result<bool> {
        self.0.res.loaded(h)
    }
}
impl<S: ReleaseServices> crate::actor_animation_source::release::FxLists for AnimationLists<'_, '_, '_, '_, '_, S> {
    fn release_list(&mut self, graph: &Graph, roots: &[NodeId], delta: u32) -> Result<()> {
        self.0.release_roots(graph, roots, delta)
    }
    fn list_ready(&mut self, graph: &Graph, roots: &[NodeId]) -> Result<bool> {
        self.0.roots_ready(graph, roots)
    }
}
impl<S: ReleaseServices> ActorRelease for Released<'_, '_, '_, '_, S> {
    type AnimationSet = usize;
    type Animation = crate::actor_animation_source::AnimationHandle;
    type Ability = super::super::actor_request::Ability;
    fn mask_174(&mut self) -> Result<u32> {
        self.definition.component_mask.ok_or_else(|| "ActorData actual component mask absent".into())
    }
    fn animation_set_count(&mut self) -> Result<isize> {
        isize::try_from(self.animations()?.set_count()?).map_err(|_| "Actor Animation count outside native domain".into())
    }
    fn animation_set(&mut self, i: usize) -> Result<Option<usize>> {
        Ok(self.animations()?.set_len(i)?.map(|_| i))
    }
    fn animation_count(&mut self, s: &usize) -> Result<isize> {
        Ok(self.animations()?.set_len(*s)?.ok_or("current Actor Animation set is null")? as isize)
    }
    fn animation(&mut self, s: &usize, i: usize) -> Result<Self::Animation> {
        self.animations()?.animation(*s, i)
    }
    fn release_animation(&mut self, h: Self::Animation, delta: u32) -> Result<()> {
        let source = self.animations()?;
        let (catalog, bank) = (self.catalog, self.bank);
        source.release_animation_with(catalog, bank, &h, delta, &mut AnimationLists(self))
    }
    fn animation_ready(&mut self, h: Self::Animation) -> Result<bool> {
        let source = self.animations()?;
        let (catalog, bank) = (self.catalog, self.bank);
        source.animation_ready_with(catalog, bank, &h, &mut AnimationLists(self))
    }
    fn release_actor_fx(&mut self, offset: u32, delta: u32) -> Result<()> {
        if let Some(graph) = self.state.actor_fx_graph(&self.data, offset)? {
            self.release_roots(&graph, graph.actor_pair_roots(offset)?, delta)?;
        }
        Ok(())
    }
    fn actor_fx_ready(&mut self, offset: u32) -> Result<bool> {
        match self.state.actor_fx_graph(&self.data, offset)? {
            Some(graph) => self.roots_ready(&graph, graph.actor_pair_roots(offset)?),
            None => Ok(true),
        }
    }
    fn ability_count(&mut self, secondary: bool) -> Result<isize> {
        self.state.actor_ability_count(&self.data, secondary)
    }
    fn ability(&mut self, secondary: bool, index: usize) -> Result<Self::Ability> {
        self.state.actor_ability(&self.data, secondary, index)
    }
    fn release_ability(&mut self, a: Self::Ability, delta: u32) -> Result<()> {
        let graph = self.state.actor_ability_graph(self.catalog, self.bank, &a)?;
        k::ability_release(delta, |offset, delta| self.release_roots(&graph, graph.actor_request_roots(offset)?, delta))
    }
    fn ability_ready(&mut self, a: Self::Ability) -> Result<bool> {
        let graph = self.state.actor_ability_graph(self.catalog, self.bank, &a)?;
        k::ability_ready(|offset| self.roots_ready(&graph, graph.actor_request_roots(offset)?))
    }
    fn extra_ability_2a4(&mut self) -> Result<Option<Self::Ability>> {
        self.state.actor_extra_ability_2a4(&self.data)
    }
    fn deployment_data_534(&mut self) -> Result<Option<RuntimeDefinition>> {
        self.state.actor_deployment_534(&self.data)
    }
}

#[cfg(test)]
#[path = "data_object_release_tests.rs"]
mod tests;
