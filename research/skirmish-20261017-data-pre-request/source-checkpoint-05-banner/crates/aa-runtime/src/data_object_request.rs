//!Object49B9F0/Actor45DD80 over canonical application's Data allocations.
//!The source producer below retains selected request fields/real resources;
//!it does not certify unrelated ObjectData::LoadData or native global order.
//!Run source production before World changes the Resource biome context.
use super::{Data, Result, State};
use crate::{
    catalog::Catalog,
    definition_fx::ListKind,
    definitions::Bank,
    fx_graph::{self, Graph, NodeId},
    resource_depot::{Backend, Resources},
    resource_objects::{Handle, ModelPolicy, TexturePolicy},
    vworld_resource_context::Biome,
};
use aa_mapworld::restore::{RestoreDefinitions, RuntimeDefinition};
use aa_rules::{actors::ResolvedNode, native::TypedValue};
use std::sync::Arc;
#[path = "data_request_fields.rs"]
mod fields;
pub(super) struct Object {
    definition: RuntimeDefinition,
    unique: Option<bool>,
    portrait: Option<RuntimeDefinition>,
    textures: [Option<Handle>; 3],
    graph: Option<Arc<Graph>>,
    partial_graph: Option<fx_graph::Failure>,
    step: usize,
    next_reference: usize,
    complete: bool,
    preparing: bool,
    failed: Option<String>,
}
impl std::fmt::Debug for Object {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectRequestFields")
            .field("id", &self.definition.proof.id)
            .field("step", &self.step)
            .field("complete", &self.complete)
            .field("failed", &self.failed)
            .finish()
    }
}
impl Object {
    fn constructed(d: &RuntimeDefinition) -> Self {
        Self {
            definition: d.clone(),
            unique: None,
            portrait: None,
            textures: [None, None, None],
            graph: None,
            partial_graph: None,
            step: 0,
            next_reference: 0,
            complete: false,
            preparing: false,
            failed: None,
        }
    }
}
fn source_node<'a>(catalog: &'a Catalog<'_>, d: &RuntimeDefinition) -> Result<ResolvedNode<'a>> {
    let mut selected = None;
    for tag in ["Thing", "Unit", "Projectile", "VFX"] {
        for node in catalog
            .actors
            .resolved_definitions(tag)
            .map_err(|e| e.to_string())?
        {
            if node.sources().first() == Some(&d.proof.source) {
                if selected.is_some() {
                    return Err("Data request canonical source position is ambiguous".into());
                }
                selected = Some(node)
            }
        }
    }
    let node = selected.ok_or("Data request canonical registered source absent")?;
    if node.sources() != d.proof.template_chain {
        return Err("Data request actual source layers differ".into());
    }
    Ok(node)
}
fn string(node: &ResolvedNode<'_>, name: &str, pc: aa_numeric::Precision) -> Result<String> {
    match node.field(name, pc).map_err(|e| e.to_string())? {
        Some(TypedValue::String { value, .. }) if value.is_ascii() && !value.contains('\0') => {
            Ok(value)
        }
        _ => Err(format!(
            "Data request {name} requires native plain String source"
        )),
    }
}
impl State {
    ///Selected49DC40 child producers. Each retained field is published only at
    ///its source operation; failures keep previous Handle/Graph/Draft ownership.
    ///This API must run in the real source-resource interval, before WorldInit.
    pub fn prepare_object_requests<P: ModelPolicy, T: TexturePolicy>(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        resources: &mut Resources<'_>,
        policy: &mut P,
        texture_policy: &mut T,
    ) -> Result<Data> {
        let data = self.bind_object(catalog, bank, d)?;
        if self.entry(&data)?.object_requests.is_none() {
            self.entry_mut(&data)?.object_requests = Some(Object::constructed(d))
        }
        {
            let o = self.object_request_fields(&data)?;
            if o.definition.proof.source != d.proof.source || o.definition.proof.id != d.proof.id {
                return Err("Data request existing source allocation changed".into());
            }
            if let Some(error) = &o.failed {
                return Err(error.clone());
            }
            if o.complete {
                return Ok(data);
            }
            if o.preparing {
                return Err("recursive Object portrait source loader cycle".into());
            }
        }
        self.object_request_fields_mut(&data)?.preparing = true;
        let result =
            self.prepare_object_inner(catalog, bank, d, &data, resources, policy, texture_policy);
        self.object_request_fields_mut(&data)?.preparing = false;
        if let Err(error) = &result {
            self.object_request_fields_mut(&data)?.failed = Some(error.clone())
        }
        result.map(|()| data)
    }
    fn prepare_object_inner<P: ModelPolicy, T: TexturePolicy>(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        data: &Data,
        resources: &mut Resources<'_>,
        policy: &mut P,
        texture_policy: &mut T,
    ) -> Result<()> {
        let pc = super::validate_object_data(catalog, bank, d)?;
        let root = source_node(catalog, d)?;
        let view = root.child("View").map_err(|e| e.to_string())?;
        if self.object_request_fields(data)?.step == 0 {
            self.prepare_object_aspects_with_policy(catalog, bank, d, resources, texture_policy)?;
            //Absent View leaves the actual49C760 constructor fields unchanged. The
            //present branch reads the real View Bool, never a viewer/UI option.
            let unique = match &view {
                Some(v) => v
                    .field("unique", pc)
                    .map_err(|e| e.to_string())?
                    .ok_or("Object unique source absent")?
                    .boolean()
                    .map_err(|e| e.to_string())?,
                None => false,
            };
            let o = self.object_request_fields_mut(data)?;
            o.unique = Some(unique);
            o.step = 1;
        }
        let normalized = d.proof.source.path.replace('\\', "/");
        let folder = normalized.rsplit_once('/').map_or("", |(f, _)| f);
        for (index, key) in ["shadow_texture", "icon", "banner_icon"]
            .into_iter()
            .enumerate()
        {
            if self.object_request_fields(data)?.step == index + 1 {
                let handle = if let Some(v) = &view {
                    let name = string(v, key, pc)?;
                    if name.is_empty() {
                        None
                    } else {
                        let prefix = string(v, "resource_prefix", pc)?;
                        let path =
                            super::path::relative(folder, &super::path::expand(&name, &prefix)?)?;
                        resources.texture_5a77e0_with_policy(
                            catalog,
                            bank,
                            &path.encode_utf16().collect::<Vec<_>>(),
                            0,
                            texture_policy,
                        )?
                    }
                } else {
                    None
                };
                //A positive Banner replacement is refused by prepare_object_aspects at
                //its reached source child; no ordinary substitute is admitted here.
                if index == 2
                    && view.is_some()
                    && handle.is_none()
                    && !string(view.as_ref().unwrap(), key, pc)?.is_empty()
                {
                    return Err("49DC40 actual banner_icon missing diagnostic is required".into());
                }
                let o = self.object_request_fields_mut(data)?;
                o.textures[index] = handle;
                o.step += 1;
            }
        }
        if self.object_request_fields(data)?.step == 4 {
            let portrait = if let Some(v) = &view {
                let name = string(v, "portrait_ids", pc)?;
                if name.is_empty() {
                    None
                } else {
                    Some(catalog.resolve(&name.encode_utf16().collect::<Vec<_>>(), pc)?)
                }
            } else {
                None
            };
            self.object_request_fields_mut(data)?.portrait = portrait;
            self.object_request_fields_mut(data)?.step = 5;
        }
        if self.object_request_fields(data)?.step == 5 {
            //Native Find stores the canonical pointer; recursively prepare THAT same
            //definition's selected source fields, not a fabricated class2 Object.
            if let Some(portrait) = self.object_request_fields(data)?.portrait.clone() {
                self.prepare_object_requests(
                    catalog,
                    bank,
                    &portrait,
                    resources,
                    policy,
                    texture_policy,
                )?;
            }
            self.object_request_fields_mut(data)?.step = 6;
        }
        if self.object_request_fields(data)?.step == 6 {
            match Graph::compile_request_in_bank(catalog, bank, d, ListKind::Constructed) {
                Ok(graph) => self.object_request_fields_mut(data)?.graph = Some(Arc::new(graph)),
                Err(failure) => {
                    let error = failure.error.clone();
                    self.object_request_fields_mut(data)?.partial_graph = Some(failure);
                    return Err(error);
                }
            }
            self.object_request_fields_mut(data)?.step = 7;
        }
        if self.object_request_fields(data)?.step == 7 {
            let graph = self
                .object_request_fields(data)?
                .graph
                .as_ref()
                .unwrap()
                .clone();
            while self.object_request_fields(data)?.next_reference < graph.nodes().len() {
                let index = self.object_request_fields(data)?.next_reference;
                //Source compilation has completed the reference fields. Type source
                //Resource40/Audio pointers are prepared in this pre-World interval only.
                self.prepare_graph_node(
                    catalog,
                    bank,
                    &graph,
                    graph.node_id(index)?,
                    resources,
                    policy,
                )?;
                self.object_request_fields_mut(data)?.next_reference += 1;
            }
            self.object_request_fields_mut(data)?.step = 8;
        }
        self.object_request_fields_mut(data)?.complete = true;
        if d.proof.category.native_kind == 2 {
            self.prepare_actor_request_fields(catalog, bank, d, resources, policy)?;
        }
        Ok(())
    }
    ///Selected ActorData1BC source producer, retained in the SAME Data entry.
    ///Arc loans only immutable completed source metadata; no Animation/Graph is cloned.
    pub fn prepare_actor_animation_requests(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        resources: &mut Resources<'_>,
        policy: &mut impl ModelPolicy,
    ) -> Result<()> {
        let data = self.bind_object(catalog, bank, d)?;
        if d.proof.category.native_kind != 2 {
            return Err("Actor Animation request source requires native ActorData".into());
        }
        if self.entry(&data)?.actor_animations.is_none() {
            let layout = self.prepared_aspect_layout(&data)?;
            let source =
                crate::actor_animation_source::Source::from_native_constructor(&data, layout)?;
            self.entry_mut(&data)?.actor_animations = Some(Arc::new(source));
        }
        {
            let source = self.entry_mut(&data)?.actor_animations.as_mut().unwrap();
            if source.set_count().is_err() {
                Arc::get_mut(source)
                    .ok_or("Actor Animation source was shared before completion")?
                    .load(catalog, bank, d, resources)?;
            }
        }
        let source = self
            .entry(&data)?
            .actor_animations
            .as_ref()
            .unwrap()
            .clone();
        source.visit_graphs(catalog, bank, |graph| {
            for index in 0..graph.nodes().len() {
                self.prepare_graph_node(
                    catalog,
                    bank,
                    graph,
                    graph.node_id(index)?,
                    resources,
                    policy,
                )?;
            }
            Ok(())
        })
    }
    fn object_request_fields(&self, data: &Data) -> Result<&Object> {
        self.entry(data)?
            .object_requests
            .as_ref()
            .ok_or_else(|| "actual ObjectData request source fields were not prepared".into())
    }
    fn object_request_fields_mut(&mut self, data: &Data) -> Result<&mut Object> {
        self.entry_mut(data)?
            .object_requests
            .as_mut()
            .ok_or_else(|| "actual ObjectData request source fields were not prepared".into())
    }
    fn ready_object(&self, data: &Data) -> Result<&Object> {
        let o = self.object_request_fields(data)?;
        if !o.complete {
            return Err(o
                .failed
                .clone()
                .unwrap_or_else(|| "actual ObjectData request source is incomplete".into()));
        }
        Ok(o)
    }
    pub fn prepare_graph_node(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        graph: &Graph,
        id: NodeId,
        resources: &mut Resources<'_>,
        policy: &mut impl ModelPolicy,
    ) -> Result<()> {
        graph.require_in_bank(catalog, bank)?;
        match graph.node_kind(id).ok_or("FXC request node kind absent")? {
            fx_graph::FxcKind::CreateVfx => {
                let cap = graph
                    .registered_reference(id, 0x3c)?
                    .ok_or("CreateVFX actual source pointer absent")?;
                let effect =
                    crate::view_vfx_source::Effect::from_registered_reference(catalog, bank, &cap)?;
                self.bind_vfx(catalog, bank, &effect)?;
                if matches!(effect.kind(catalog, bank)?, 10 | 11 | 14 | 17) {
                    self.prepare_vfx_resource40_with_policy(
                        catalog, bank, &effect, resources, policy,
                    )?;
                }
            }
            fx_graph::FxcKind::PlaySfx => {
                if let Some(cap) = graph.registered_reference(id, 0x44)? {
                    cap.require_audio_reference(catalog, bank)?;
                    self.audio_data.prepare(catalog, bank, &cap, resources)?;
                }
            }
            fx_graph::FxcKind::StopVfx | fx_graph::FxcKind::RequestOnly { .. } => {}
        }
        Ok(())
    }
    ///Actual4288B0 requests head roots only. Immediate/chained children are not
    ///implicitly walked here; their parent runtime execution owns those calls.
    pub fn request_fx_list(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        graph: &Graph,
        roots: &[NodeId],
        resources: &mut Resources<'_>,
        backend: &mut impl Backend,
        delta: u32,
        mode: u32,
    ) -> Result<()> {
        graph.require_in_bank(catalog, bank)?;
        for &id in roots {
            match graph.node_kind(id).ok_or("FXC request node kind absent")? {
                fx_graph::FxcKind::CreateVfx => {
                    let cap = graph
                        .registered_reference(id, 0x3c)?
                        .ok_or("CreateVFX actual source pointer absent")?;
                    let effect = crate::view_vfx_source::Effect::from_registered_reference(
                        catalog, bank, &cap,
                    )?;
                    self.pre_request_vfx_with_resources(
                        catalog, bank, &effect, delta, mode, resources, backend,
                    )?;
                }
                fx_graph::FxcKind::PlaySfx => {
                    let cap = graph
                        .registered_reference(id, 0x44)?
                        .ok_or("4288B0 PlaySFX actual AudioData44 is null")?;
                    self.audio_data.request(
                        catalog,
                        bank,
                        &cap,
                        delta as i32,
                        mode,
                        resources,
                        backend,
                    )?;
                }
                fx_graph::FxcKind::StopVfx | fx_graph::FxcKind::RequestOnly { .. } => {}
            }
        }
        Ok(())
    }
    ///One dispatch over SAME Data+20, Aspect rows, Models and child resources.
    ///A missing source field fails only after the native base request store.
    pub fn pre_request_object<P: ModelPolicy, B: Backend>(
        &mut self,
        catalog: &Catalog<'_>,
        bank: &Bank,
        d: &RuntimeDefinition,
        biome: Option<&Biome>,
        resources: &mut Resources<'_>,
        policy: &mut P,
        backend: &mut B,
        delta: u32,
        mode: u32,
    ) -> Result<()> {
        let data = self.bind_object(catalog, bank, d)?;
        let mut bound = Bound {
            state: self,
            catalog,
            bank,
            definition: d,
            data,
            biome,
            resources,
            policy,
            backend,
        };
        if d.proof.category.native_kind == 2 {
            fields::actor(&mut bound, delta, mode)
        } else {
            fields::object(&mut bound, delta, mode)
        }
    }
}
struct Bound<'a, 'source, 'manager, P, B> {
    state: &'a mut State,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    definition: &'a RuntimeDefinition,
    data: Data,
    biome: Option<&'a Biome>,
    resources: &'a mut Resources<'manager>,
    policy: &'a mut P,
    backend: &'a mut B,
}
impl<P: ModelPolicy, B: Backend> fields::ObjectWorld for Bound<'_, '_, '_, P, B> {
    type Data = RuntimeDefinition;
    type Aspect = super::aspect::Aspect;
    type Model = Handle;
    fn base_request(&mut self, d: u32, m: u32) -> Result<()> {
        self.state.entry_mut(&self.data)?.counter.request(d, m);
        Ok(())
    }
    fn portrait(&mut self) -> Result<Option<RuntimeDefinition>> {
        Ok(self.state.ready_object(&self.data)?.portrait.clone())
    }
    fn request_data(&mut self, d: RuntimeDefinition, a: u32, b: u32) -> Result<()> {
        self.state.pre_request_object(
            self.catalog,
            self.bank,
            &d,
            self.biome,
            self.resources,
            self.policy,
            self.backend,
            a,
            b,
        )
    }
    fn aspect_count(&mut self) -> Result<isize> {
        self.state.aspect_request_count(&self.data)
    }
    fn aspect(&mut self, i: usize) -> Result<Self::Aspect> {
        self.state
            .object_aspect(self.catalog, self.bank, self.definition, i)
    }
    fn request_aspect(&mut self, a: Self::Aspect, d: u32, m: u32) -> Result<()> {
        self.state.request_aspect(
            self.catalog,
            self.bank,
            &a,
            self.biome,
            self.resources,
            self.policy,
            self.backend,
            d,
            m,
        )
    }
    fn flags_1c(&mut self) -> Result<u32> {
        Ok(self.definition.flags)
    }
    fn unique_fc(&mut self) -> Result<bool> {
        self.state
            .ready_object(&self.data)?
            .unique
            .ok_or_else(|| "actual Object unique source absent".into())
    }
    fn current_aspect_model(&mut self, i: usize) -> Result<Handle> {
        self.state.aspect_current_model(&self.data, i)
    }
    fn or_model_flags(&mut self, m: Handle, f: u32) -> Result<()> {
        self.resources.data_model_flags(&m, f)
    }
    fn request_texture(&mut self, o: u32, d: u32, m: u32) -> Result<()> {
        let i = match o {
            0xec => 0,
            0xf0 => 1,
            0x138 => 2,
            _ => return Err("Object texture field offset unsupported".into()),
        };
        if let Some(h) = self.state.ready_object(&self.data)?.textures[i].clone() {
            self.resources.request_10(&h, d as i32, m, self.backend)?;
        }
        Ok(())
    }
    fn request_banner(&mut self, delta: u32, mode: u32) -> Result<()> {
        self.state.ready_object(&self.data)?;
        if let Some(reference) = self.state.entry(&self.data)?.banner.as_ref() {
            self.state.request_banner(
                self.catalog,
                self.bank,
                reference,
                self.resources,
                self.backend,
                delta,
                mode,
            )?;
        }
        Ok(())
    }
    fn request_view_fx(&mut self, i: usize, d: u32, m: u32) -> Result<()> {
        let graph = self
            .state
            .ready_object(&self.data)?
            .graph
            .as_ref()
            .ok_or("actual Object six FX lists absent")?
            .clone();
        self.state.request_fx_list(
            self.catalog,
            self.bank,
            &graph,
            graph.view_list_roots(ListKind::ALL[i])?,
            self.resources,
            self.backend,
            d,
            m,
        )
    }
}
impl<P: ModelPolicy, B: Backend> fields::ActorWorld for Bound<'_, '_, '_, P, B> {
    type AnimationSet = usize;
    type Animation = crate::actor_animation_source::AnimationHandle;
    type Ability = super::actor_request::Ability;
    fn mask_174(&mut self) -> Result<u32> {
        self.definition
            .component_mask
            .ok_or_else(|| "ActorData actual component mask absent".into())
    }
    fn mark_all_aspect_models(&mut self, f: u32) -> Result<()> {
        for i in 0..self.state.aspect_request_count(&self.data)?.max(0) as usize {
            self.resources
                .data_model_flags(&self.state.aspect_current_model(&self.data, i)?, f)?;
        }
        Ok(())
    }
    fn animation_set_count(&mut self) -> Result<isize> {
        let source = self
            .state
            .entry(&self.data)?
            .actor_animations
            .as_ref()
            .ok_or("actual ActorData1BC source owner unprepared")?;
        isize::try_from(source.set_count()?)
            .map_err(|_| "Actor Animation count outside native domain".into())
    }
    fn animation_set(&mut self, i: usize) -> Result<Option<usize>> {
        Ok(self
            .state
            .entry(&self.data)?
            .actor_animations
            .as_ref()
            .ok_or("Actor Animation source absent")?
            .set_len(i)?
            .map(|_| i))
    }
    fn animation_count(&mut self, s: &usize) -> Result<isize> {
        Ok(self
            .state
            .entry(&self.data)?
            .actor_animations
            .as_ref()
            .ok_or("Actor Animation source absent")?
            .set_len(*s)?
            .ok_or("current Actor Animation set is null")? as isize)
    }
    fn animation(&mut self, s: &usize, i: usize) -> Result<Self::Animation> {
        self.state
            .entry(&self.data)?
            .actor_animations
            .as_ref()
            .ok_or("Actor Animation source absent")?
            .animation(*s, i)
    }
    fn request_animation(&mut self, h: Self::Animation, d: u32) -> Result<()> {
        let source = self
            .state
            .entry(&self.data)?
            .actor_animations
            .as_ref()
            .ok_or("Actor Animation source absent")?
            .clone();
        let mut fx = AnimationFx {
            state: self.state,
            catalog: self.catalog,
            bank: self.bank,
        };
        source.request_animation(
            self.catalog,
            self.bank,
            &h,
            d,
            self.resources,
            self.backend,
            &mut fx,
        )
    }
    fn request_actor_fx(&mut self, o: u32, d: u32, m: u32) -> Result<()> {
        self.state.actor_fx_request(
            self.catalog,
            self.bank,
            &self.data,
            o,
            self.resources,
            self.backend,
            d,
            m,
        )
    }
    fn ability_count(&mut self, secondary: bool) -> Result<isize> {
        self.state.actor_ability_count(&self.data, secondary)
    }
    fn ability(&mut self, secondary: bool, index: usize) -> Result<Self::Ability> {
        self.state.actor_ability(&self.data, secondary, index)
    }
    fn request_ability(&mut self, ability: Self::Ability, d: u32, m: u32) -> Result<()> {
        self.state.request_actor_ability(
            self.catalog,
            self.bank,
            ability,
            self.resources,
            self.backend,
            d,
            m,
        )
    }
    fn extra_ability_2a4(&mut self) -> Result<Option<Self::Ability>> {
        self.state.actor_extra_ability_2a4(&self.data)
    }
    fn settlement_data_534(&mut self) -> Result<Option<RuntimeDefinition>> {
        self.state.actor_settlement_534(&self.data)
    }
}
struct AnimationFx<'a, 'source> {
    state: &'a mut State,
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
}
impl crate::actor_animation_source::FxRequests for AnimationFx<'_, '_> {
    fn request_list(
        &mut self,
        g: &Graph,
        roots: &[NodeId],
        d: u32,
        m: u32,
        r: &mut Resources<'_>,
        b: &mut impl Backend,
    ) -> Result<()> {
        self.state
            .request_fx_list(self.catalog, self.bank, g, roots, r, b, d, m)
    }
}

#[cfg(test)]
#[path = "data_object_request_tests.rs"]
mod tests;
