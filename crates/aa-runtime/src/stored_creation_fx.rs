//! Reached42A590/429080 on the actual fresh Actor, source graph and World's
//! pending50C800 owner. The common constructor retains native getter order;
//! no resource-request-only graph or source-count placeholder may execute.
use crate::{
    actor_world::{ActorStore,ObjectKey,Source}, catalog::Catalog, definition_fx::ListKind,
    fx_graph::{self,Command,CreationInputs,ExistingInputs,Graph}, fx_manager,
    restoration::Staged, spatial_world::SpatialWorld,
};
use aa_actors::ActorId;
use std::sync::Arc;
type Result<T> = std::result::Result<T,String>;
fn error(e:impl std::fmt::Display)->String { e.to_string() }

pub(crate) fn publish(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,kind:ListKind)->Result<()> {
    let owner=staged.actors.id(key).ok_or("fresh42A590 actual Actor generation absent")?;
    let object=staged.actors.object(key).map_err(error)?;
    if object.definition.native_class!=3 || object.object.is_none() || object.actor.is_none()
        || !matches!(&staged.actors.entity(key).map_err(error)?.source,Source::New{..}) {
        return Err("fresh42A590 requires the same newly constructed ordinary Actor".into());
    }
    let graph=match catalog.prepared_view_graph(&staged.definitions,&object.definition,kind)?{
        Some(graph)=>graph,
        None=>Arc::new(Graph::compile_in_bank(catalog,&staged.definitions,&object.definition,kind).map_err(|failure|failure.error)?),
    };
    graph.require_in_bank(catalog,&staged.definitions)?;
    //The graph's full-field guard belongs to the actual common constructor.
    //Compile failures cannot produce a graph or a successful command token.
    let world=staged.load_world.as_mut().ok_or("fresh42A590 actual World absent")?;
    let crate::load_world::SpatialOwner::Adopted(spatial)=&world.spatial else {
        return Err("fresh42A590 actual published spatial owner absent".into());
    };
    if spatial.error.is_some() || !spatial.pending.is_empty() {
        return Err("fresh42A590 actual spatial adoption is incomplete".into());
    }
    let command=fx_graph::construct_existing_list(graph,kind,&mut Inputs {
        catalog,actors:&staged.actors,spatial:&spatial.world,clock:&staged.clock,
        key,owner,presentation:&world.presentation,
    },0)?;
    if let Some(command)=command { world.presentation.submit(Some(command),0,&mut Presentation)?; }
    Ok(())
}
struct Inputs<'a,'source> {
    catalog:&'a Catalog<'source>,actors:&'a ActorStore,spatial:&'a SpatialWorld,
    clock:&'a aa_simulation::NativeClock,key:ObjectKey,owner:ActorId,
    presentation:&'a fx_manager::State<Command>,
}
impl Inputs<'_,'_> {
    fn object(&self)->Result<&crate::actor_world::RuntimeObject> {
        if self.actors.id(self.key)!=Some(self.owner) { return Err("fresh FX current allocation/generation changed".into()); }
        self.actors.object(self.key).map_err(error)
    }
}
impl CreationInputs for Inputs<'_,'_> {
    fn muted(&mut self)->Result<bool> { Ok(self.presentation.muted()) }
    fn creator_kingdom(&mut self)->Result<Option<ActorId>> {
        //428F40: ordinary Actor+A8=false/B4=true, then actual virtual108.
        let raw=self.object()?.actor.as_ref().and_then(|actor|actor.words.get(&0xe8)).copied()
            .ok_or("fresh FX actual ActorE8 is unwritten")?;
        if raw==0 { return Ok(None); }
        let owner=ActorId::from_raw(raw);
        let key=self.actors.key(owner).ok_or("fresh FX creator owner generation absent")?;
        self.actors.object(key).map_err(error)?; //virtual108 must return an actual Thing, not a codec-only slot.
        if self.actors.id(key)!=Some(owner) { return Err("fresh FX creator Thing identity differs".into()); }
        Ok(Some(owner))
    }
    fn current_position(&mut self)->Result<[u32;3]> {
        self.object()?;
        Ok(self.actors.placement(self.key,self.spatial).map_err(error)?.position_bits)
    }
    fn current_thing(&mut self)->Result<Option<ActorId>> { self.object()?;Ok(Some(self.owner)) }
    fn current_os_thread_id(&mut self)->Result<u32> {
        let mut execution=self.catalog.load_execution.try_borrow_mut().map_err(error)?;
        execution.as_deref_mut().ok_or("fresh FX requires the actual current execution OS service")?.current_os_thread_id()
    }
    fn application_os_thread_id(&mut self)->Result<u32> {
        let execution=self.catalog.load_execution.try_borrow().map_err(error)?;
        Ok(execution.as_deref().ok_or("fresh FX requires the actual retained Application OS identity")?.application_os_thread_id())
    }
    fn world_time(&mut self)->Result<u32> { Ok(self.clock.begin.0) }
    fn view_time(&mut self)->Result<u32> { Err("fresh FX on the Application OS thread requires actual VWorld8C clock loan".into()) }
}
impl ExistingInputs for Inputs<'_,'_> {
    fn source_definition_attachment(&mut self)->Result<()> {
        //42916C..429194 dereferences actual Thing+0C, then View+54/vfBC.
        //Immutable ObjectData flags cannot replace this mutable View owner.
        if self.object()?.thing.word_c==0 { return Ok(()); }
        Err("fresh FX positive Thing0C requires actual current View54/vfBC/45C740 subscription owner".into())
    }
}
struct Presentation;
impl fx_manager::Effects<Command> for Presentation {
    fn deliver(&mut self,_:u64,_:Option<Command>)->Result<()> {
        Err("fresh FX requires actual attached View43B870 delivery".into())
    }
    fn destroy(&mut self,_:Command)->Result<()> {
        Err("fresh FX muted submission requires actual428350 command destruction".into())
    }
}

#[cfg(test)]
#[path = "stored_creation_fx_tests.rs"]
mod tests;
