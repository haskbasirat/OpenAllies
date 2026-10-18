//! Actual50C220 gates for a fresh Thing before an attached VWorld exists.
//! Reuses the native publication kernel. A positive attached-view callback
//! needs the actual client factory and is never acknowledged by a null stub.
use crate::{actor_world::{ActorStore, ObjectKey}, load_world::World, view_factory_order::{self, Fields, Result}};
#[derive(Clone, Copy)] enum UnboundView {}
struct Current<'a> { world: &'a World }
fn required<T>() -> Result<T> { Err("fresh50C220 positive actual client View factory loan is required".into()) }
pub(crate) fn on_added(world: &World, actors: &ActorStore, thing: ObjectKey) -> Result<()> {
    actors.id(thing).ok_or("fresh50C220 actual Thing generation absent")?;
    view_factory_order::on_thing_added(&mut Current { world }, Some(thing))
}
impl Fields for Current<'_> {
    type Thing = ObjectKey;
    type View = UnboundView;
    fn world_phase(&mut self) -> Result<u32> {
        self.world.dispatcher.as_ref().map(|owner| owner.phase)
            .ok_or_else(|| "fresh50C220 actual WorldD8 owner absent".into())
    }
    fn attached_view_present(&mut self) -> Result<bool> { Ok(self.world.presentation.attached_view().is_some()) }
    fn source_view_type(&mut self, _: ObjectKey) -> Result<u32> { required() }
    fn construct_view(&mut self, _: ObjectKey) -> Result<Option<UnboundView>> { required() }
    fn initialize_view(&mut self, view: UnboundView) -> Result<bool> { match view {} }
    fn destroy_view(&mut self, view: UnboundView, _: u32) -> Result<()> { match view {} }
    fn view_flags(&mut self, view: UnboundView) -> Result<u32> { match view {} }
    fn prepend_update_view(&mut self, view: UnboundView) -> Result<()> { match view {} }
    fn publish_view(&mut self, _: ObjectKey, view: Option<UnboundView>) -> Result<()> {
        match view { Some(view) => match view {}, None => required() }
    }
    //4F0750 traversal-only methods cannot be reached by50C220.
    fn begin_diagnostic(&mut self) -> Result<()> { required() }
    fn attach_view(&mut self) -> Result<()> { required() }
    fn kingdom_count(&mut self) -> Result<i32> { required() }
    fn kingdom(&mut self, _: i32) -> Result<Option<ObjectKey>> { required() }
    fn next_thing(&mut self, _: &mut u32) -> Result<Option<ObjectKey>> { required() }
    fn thing_has_view(&mut self, _: ObjectKey) -> Result<bool> { required() }
    fn end_diagnostic(&mut self, _: i32, _: i32) -> Result<()> { required() }
}
