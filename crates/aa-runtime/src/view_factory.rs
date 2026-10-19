//!Concrete4F0750 reads over the SAME live World/ActorStore/Kingdom/FX owners.
//!The client Directory supplies actual allocation/derived Init/publication;
//!no source metadata or uploaded model is treated as successful derived Init.
use crate::{actor_world::ObjectKey,live_world_frame::ViewBindingParts};
use aa_actors::ActorId;
pub use crate::view_factory_order::{Report,Stage,State};
type Result<T>=std::result::Result<T,String>;
pub trait Services {
    type View:Copy;
    fn begin_diagnostic(&mut self)->Result<()>;
    fn attach_view(&mut self,world:&mut ViewBindingParts<'_>)->Result<()>;
    fn source_view_type(&mut self,world:&ViewBindingParts<'_>,thing:ActorId)->Result<u32>;
    fn construct_view(&mut self,world:&mut ViewBindingParts<'_>,thing:ActorId)->Result<Option<Self::View>>;
    fn initialize_view(&mut self,world:&mut ViewBindingParts<'_>,view:Self::View)->Result<bool>;
    fn destroy_view(&mut self,world:&mut ViewBindingParts<'_>,view:Self::View,free:u32)->Result<()>;
    fn view_flags(&mut self,world:&ViewBindingParts<'_>,view:Self::View)->Result<u32>;
    fn prepend_update_view(&mut self,world:&mut ViewBindingParts<'_>,view:Self::View)->Result<()>;
    ///Only the real private Directory publication receipt may write Thing+C.
    fn publish_view(&mut self,world:&mut ViewBindingParts<'_>,thing:ActorId,view:Option<Self::View>)->Result<()>;
    fn end_diagnostic(&mut self,existing:i32,attempted_missing:i32)->Result<()>;
}
struct Current<'a,'world,S>{world:ViewBindingParts<'world>,services:&'a mut S}
impl<S:Services> Current<'_,'_,S>{
    fn key(&self,id:ActorId)->Result<ObjectKey>{
        let key=self.world.actors.key(id).ok_or("view traversal current GTID is stale")?;
        self.world.actors.object(key).map_err(|e|e.to_string())?;Ok(key)
    }
}
impl<S:Services> crate::view_factory_order::Fields for Current<'_,'_,S>{
    type Thing=ActorId;type View=S::View;
    fn begin_diagnostic(&mut self)->Result<()>{self.services.begin_diagnostic()}
    fn attach_view(&mut self)->Result<()>{self.services.attach_view(&mut self.world)}
    fn kingdom_count(&mut self)->Result<i32>{i32::try_from(self.world.kingdoms.all().len()).map_err(|_|"actual World14C does not fit native signed count".into())}
    fn kingdom(&mut self,index:i32)->Result<Option<ActorId>>{
        self.world.kingdoms.all().get(usize::try_from(index).map_err(|_|"negative World148 slot")?)
            .copied().ok_or_else(||"actual World148 slot is outside its current14C".into())
    }
    fn world_phase(&mut self)->Result<u32>{Ok(*self.world.world_phase)}
    fn attached_view_present(&mut self)->Result<bool>{Ok(self.world.presentation.attached_view().is_some())}
    fn source_view_type(&mut self,id:ActorId)->Result<u32>{self.key(id)?;self.services.source_view_type(&self.world,id)}
    fn construct_view(&mut self,id:ActorId)->Result<Option<Self::View>>{self.key(id)?;self.services.construct_view(&mut self.world,id)}
    fn initialize_view(&mut self,view:Self::View)->Result<bool>{self.services.initialize_view(&mut self.world,view)}
    fn destroy_view(&mut self,view:Self::View,free:u32)->Result<()>{self.services.destroy_view(&mut self.world,view,free)}
    fn view_flags(&mut self,view:Self::View)->Result<u32>{self.services.view_flags(&self.world,view)}
    fn prepend_update_view(&mut self,view:Self::View)->Result<()>{self.services.prepend_update_view(&mut self.world,view)}
    fn publish_view(&mut self,id:ActorId,view:Option<Self::View>)->Result<()>{self.key(id)?;self.services.publish_view(&mut self.world,id,view)}
    fn next_thing(&mut self,cursor:&mut u32)->Result<Option<ActorId>>{
        let first=if *cursor==0{1}else{(*cursor&0xffff)+1};
        //Entries sit at their own slot, ascending: the first at or after `first`.
        let found=self.world.actors.identities().iter_from(first as usize).next();
        let id=found.map(|entry|entry.id);*cursor=id.map_or(0,ActorId::raw);Ok(id)
    }
    fn thing_has_view(&mut self,id:ActorId)->Result<bool>{
        Ok(self.world.actors.object(self.key(id)?).map_err(|e|e.to_string())?.thing.word_c!=0)
    }
    fn end_diagnostic(&mut self,existing:i32,attempted_missing:i32)->Result<()>{self.services.end_diagnostic(existing,attempted_missing)}
}
pub fn run<S:Services>(state:&mut State<ActorId>,world:ViewBindingParts<'_>,services:&mut S)->Result<Report>{
    state.run(&mut Current{world,services})
}
