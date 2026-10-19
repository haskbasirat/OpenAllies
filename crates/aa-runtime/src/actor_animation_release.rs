//!Actor Animation vf4 45EAC0 (release) / 45EB10 (ready) over the SAME
//!source-owned Animation that 45EA60 requests. Kernel/proof:
//!data_release_kernels.
use super::*;
use crate::data_pre_request::release_kernels::{self as k, ResourceRelease};
///The caller's 428900/428950 over one contextual FX list's head roots.
pub(crate) trait FxLists: ResourceRelease {
    fn release_list(&mut self, graph: &Graph, roots: &[NodeId], delta: u32) -> Result<()>;
    fn list_ready(&mut self, graph: &Graph, roots: &[NodeId]) -> Result<bool>;
}
struct Bound<'a, 'c, S> {
    animation: &'a core::Animation<Handle, FxOwner>,
    catalog: &'a Catalog<'c>,
    bank: &'a Bank,
    services: &'a mut S,
}
fn graph<'g>(animation: &'g core::Animation<Handle, FxOwner>, catalog: &Catalog<'_>, bank: &Bank) -> Result<&'g Graph> {
    let context = animation.fx.as_ref().ok_or("Actor Animation FX context absent")?;
    let graph = context.graph.as_ref().ok_or("Actor Animation contextual Graph is unfinished")?;
    graph.require_in_bank(catalog, bank)?;
    Ok(graph)
}
impl<S: FxLists> k::AnimationRelease for Bound<'_, '_, S> {
    type Resource = Handle;
    type List = usize;
    fn resource(&mut self) -> Result<Option<Handle>> {
        Ok(self.animation.resource.clone())
    }
    fn release(&mut self, r: &Handle, delta: u32) -> Result<()> {
        self.services.release_14(r, delta)
    }
    fn loaded(&mut self, r: &Handle) -> Result<bool> {
        self.services.loaded(r)
    }
    fn lists(&mut self) -> Result<Vec<usize>> {
        let mut keys = Vec::new();
        for key in &self.animation.lists {
            keys.push(key.ok_or("Actor Animation list head absent")?);
        }
        for cue in &self.animation.timed {
            keys.push(cue.list.ok_or("Actor Animation timed list absent")?);
        }
        Ok(keys)
    }
    fn release_list(&mut self, key: &usize, delta: u32) -> Result<()> {
        let graph = graph(self.animation, self.catalog, self.bank)?;
        let roots = graph.animation_list_roots(*key)?;
        if roots.is_empty() { Ok(()) } else { self.services.release_list(graph, roots, delta) }
    }
    fn list_ready(&mut self, key: &usize) -> Result<bool> {
        let graph = graph(self.animation, self.catalog, self.bank)?;
        let roots = graph.animation_list_roots(*key)?;
        if roots.is_empty() { Ok(true) } else { self.services.list_ready(graph, roots) }
    }
}
impl Source {
    fn completed_animation(&self, catalog: &Catalog<'_>, bank: &Bank, handle: &AnimationHandle)
        -> Result<&core::Animation<Handle, FxOwner>> {
        self.ready()?;
        self.require(catalog, bank)?;
        if !self.data.same_allocation(&handle.data) {
            return Err("Animation handle belongs to another Data".into());
        }
        let animation = self.sets.get(handle.set).and_then(Option::as_ref).and_then(|set| set.rows.get(handle.index))
            .ok_or("Animation handle outside retained source owner")?;
        if !animation.complete {
            return Err("Actor Animation source is incomplete".into());
        }
        Ok(animation)
    }
    ///45EAC0
    pub(crate) fn release_animation_with(&self, catalog: &Catalog<'_>, bank: &Bank, handle: &AnimationHandle,
        delta: u32, services: &mut impl FxLists) -> Result<()> {
        let animation = self.completed_animation(catalog, bank, handle)?;
        k::animation_release(&mut Bound { animation, catalog, bank, services }, delta)
    }
    ///45EB10
    pub(crate) fn animation_ready_with(&self, catalog: &Catalog<'_>, bank: &Bank, handle: &AnimationHandle,
        services: &mut impl FxLists) -> Result<bool> {
        let animation = self.completed_animation(catalog, bank, handle)?;
        k::animation_ready(&mut Bound { animation, catalog, bank, services })
    }
}
