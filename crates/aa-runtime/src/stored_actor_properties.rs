//! Native4D4CF0 build-target list refresh on the same live ActorStore nodes.
//! This is distinct from numeric PropertiesComponent effects. Native clear,
//! free, allocation and each link/value store commit immediately; member and
//! direct Element parent reads use current generation-checked actor storage.
use crate::{actor_world::{ActorStore,ObjectKey,RuntimeComponent},catalog::Catalog,definitions::Bank};
use aa_actors::{ActorId,components::ComponentSlot as Slot,loading::ActorShell};
use aa_body::{Error,Result,actor_property_list::{self,NodeId,State,World}};
use std::collections::BTreeSet;
fn error(e:impl std::fmt::Display)->Error{Error::Callback(e.to_string())}
#[derive(Clone,Copy)]
enum Node{Owned(ActorId,NodeId),Source(ActorId,usize)}
struct Bound<'a,'source>{catalog:&'a Catalog<'source>,actors:&'a mut ActorStore,bank:&'a Bank,parents:BTreeSet<ActorId>}
impl Bound<'_,'_>{
    fn key(&self,id:ActorId)->Result<ObjectKey>{self.actors.key(id).ok_or_else(||error("4D4CF0 current stored Actor generation absent"))}
    fn shell(&self,id:ActorId)->Result<&ActorShell<RuntimeComponent>>{
        let o=self.actors.object(self.key(id)?).map_err(error)?;
        if !matches!(o.definition.native_class,3|4){return Err(error("4D4CF0 current stored allocation is not Actor storage"))}
        o.actor.as_ref().ok_or_else(||error("4D4CF0 current Actor shell absent"))
    }
    fn shell_mut(&mut self,id:ActorId)->Result<&mut ActorShell<RuntimeComponent>>{
        let key=self.key(id)?;let o=self.actors.object_mut(key).map_err(error)?;
        if !matches!(o.definition.native_class,3|4){return Err(error("4D4CF0 current stored allocation is not Actor storage"))}
        o.actor.as_mut().ok_or_else(||error("4D4CF0 current Actor shell absent"))
    }
    fn list(&self,id:ActorId)->Result<&State<String>>{Ok(&self.shell(id)?.actor_list_108)}
    fn list_mut(&mut self,id:ActorId)->Result<&mut State<String>>{Ok(&mut self.shell_mut(id)?.actor_list_108)}
    fn local(actor:ActorId,node:Option<Node>)->Result<Option<NodeId>>{
        node.map(|n|match n{Node::Owned(owner,n)if owner==actor=>Ok(n),_=>Err(error("4D4CF0 stored link crosses actor allocation"))}).transpose()
    }
    fn source_pc(&self,actor:ActorId)->Result<aa_numeric::Precision>{
        self.catalog.require_source_owner().map_err(error)?;
        let receipt=self.bank.combat_source().map_err(error)?;receipt.completed_means().map_err(error)?;
        if receipt.source_identities()!=self.catalog.actors.sources.as_slice(){return Err(error("4D4CF0 moved source Bank universe differs"))}
        let pc=receipt.source_precision();let o=self.actors.object(self.key(actor)?).map_err(error)?;
        let actual=self.catalog.actor_bindings(&o.definition.proof.id,pc).map_err(error)?;
        if !crate::load_spatial::same_proof(&o.thing.definition,&o.definition.proof)
            ||!crate::load_spatial::same_proof(&actual.definition.proof,&o.definition.proof)
            ||actual.definition.component_mask!=o.definition.component_mask{
            return Err(error("4D4CF0 current stored ActorData source proof differs"))
        }
        Ok(pc)
    }
    fn source_id(&self,actor:ActorId)->Result<&str>{Ok(&self.actors.object(self.key(actor)?).map_err(error)?.definition.proof.id)}
    fn org(&self,id:ActorId)->Result<&aa_actors::ground_states::OrganizationState<String>>{
        match self.shell(id)?.components.get(Slot::Organization){
            Some(RuntimeComponent::Organization(v))if v.header.owner==id=>Ok(v),
            _=>Err(error("4D4CF0 actual stored Organization payload/owner differs")),
        }
    }
}
impl World for Bound<'_,'_>{
    type Actor=ActorId;type Node=Node;type Definition=ActorId;type Organization=ActorId;type Element=ActorId;type Value=String;
    fn actor_head(&mut self,a:ActorId)->Result<Option<Node>>{Ok(self.list(a)?.head().map(|n|Node::Owned(a,n)))}
    fn actor_tail(&mut self,a:ActorId)->Result<Option<Node>>{Ok(self.list(a)?.tail().map(|n|Node::Owned(a,n)))}
    fn store_actor_head(&mut self,a:ActorId,n:Option<Node>)->Result<()>{let n=Self::local(a,n)?;self.list_mut(a)?.set_head(n);Ok(())}
    fn store_actor_tail(&mut self,a:ActorId,n:Option<Node>)->Result<()>{let n=Self::local(a,n)?;self.list_mut(a)?.set_tail(n);Ok(())}
    fn node_value(&mut self,n:Node)->Result<String>{match n{
        Node::Owned(a,n)=>Ok(self.list(a)?.value(n)?.clone()),
        Node::Source(a,index)=>{
            let source=self.catalog.native_builder_data(self.source_pc(a)?).map_err(error)?;
            source.definition(self.source_id(a)?).map_err(error)?.ids.get(index).cloned().ok_or_else(||error("ActorData+504 stored source value outside actual list"))
        }
    }}
    fn node_next(&mut self,n:Node)->Result<Option<Node>>{match n{
        Node::Owned(a,n)=>Ok(self.list(a)?.next(n)?.map(|n|Node::Owned(a,n))),
        Node::Source(a,index)=>{
            let source=self.catalog.native_builder_data(self.source_pc(a)?).map_err(error)?;
            let ids=&source.definition(self.source_id(a)?).map_err(error)?.ids;
            if index>=ids.len(){return Err(error("ActorData+504 stored source next outside actual list"))}
            Ok((index+1<ids.len()).then_some(Node::Source(a,index+1)))
        }
    }}
    fn store_node_value(&mut self,n:Node,v:String)->Result<()>{match n{Node::Owned(a,n)=>self.list_mut(a)?.set_value(n,v),Node::Source(..)=>Err(error("ActorData+504 immutable source value store"))}}
    fn store_node_next(&mut self,n:Node,next:Option<Node>)->Result<()>{match n{Node::Owned(a,n)=>{let next=Self::local(a,next)?;self.list_mut(a)?.set_next(n,next)},Node::Source(..)=>Err(error("ActorData+504 immutable source next store"))}}
    fn allocate_node(&mut self,a:ActorId)->Result<Node>{Ok(Node::Owned(a,self.list_mut(a)?.allocate()?))}
    fn free_node(&mut self,a:ActorId,n:Node)->Result<()>{match n{Node::Owned(owner,n)if owner==a=>self.list_mut(a)?.free(n),_=>Err(error("4D4CF0 stored free crosses actor allocation"))}}
    fn has_source_component_a8(&mut self,a:ActorId)->Result<bool>{match self.shell(a)?.components.get(Slot::Builder){None=>Ok(false),Some(RuntimeComponent::Builder(_))=>Ok(true),_=>Err(error("4D4CF0 stored Builder payload differs"))}}
    fn definition(&mut self,a:ActorId)->Result<ActorId>{self.source_id(a)?;Ok(a)}
    fn definition_flags(&mut self,a:ActorId)->Result<u32>{self.catalog.canonical_component_mask(self.source_id(a)?,self.source_pc(a)?).map_err(error)}
    fn definition_head_504(&mut self,a:ActorId)->Result<Option<Node>>{
        let source=self.catalog.native_builder_data(self.source_pc(a)?).map_err(error)?;
        Ok((!source.definition(self.source_id(a)?).map_err(error)?.ids.is_empty()).then_some(Node::Source(a,0)))
    }
    fn organization(&mut self,a:ActorId)->Result<Option<ActorId>>{match self.shell(a)?.components.get(Slot::Organization){None=>Ok(None),Some(RuntimeComponent::Organization(_))=>Ok(Some(a)),_=>Err(error("4D4CF0 stored Organization payload differs"))}}
    fn organization_member_count(&mut self,a:ActorId)->Result<i32>{i32::try_from(self.org(a)?.members.len()).map_err(error)}
    fn organization_member(&mut self,a:ActorId,index:i32)->Result<Option<ActorId>>{
        let index=usize::try_from(index).map_err(error)?;let member=*self.org(a)?.members.get(index).ok_or_else(||error("4D4CF0 stored member slot outside actual array"))?;
        Ok((!member.is_null()).then_some(member))
    }
    fn element(&mut self,a:ActorId)->Result<Option<ActorId>>{match self.shell(a)?.components.get(Slot::Element){None=>Ok(None),Some(RuntimeComponent::Element(_))=>Ok(Some(a)),_=>Err(error("4D4E50 stored Element payload differs"))}}
    fn element_parent(&mut self,a:ActorId)->Result<Option<ActorId>>{
        let parent=match self.shell(a)?.components.get(Slot::Element){Some(RuntimeComponent::Element(v))if v.header.owner==a=>v.organization,_=>return Err(error("4D4E50 stored Element payload/owner differs"))};
        if let Some(parent)=parent&& !self.parents.insert(parent){return Err(error("4D4E50 stored Element parent cycle (portable invalid-graph refusal)"))}
        Ok(parent)
    }
}
///Refresh the real current allocation, preserving every native partial store.
///Source Bank completion is demanded only when native reaches Builder source.
pub fn refresh(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&Bank,key:ObjectKey)->std::result::Result<(),String>{
    let owner=actors.id(key).ok_or("4D4CF0 current stored GTID absent")?;
    actor_property_list::refresh(&mut Bound{catalog,actors,bank,parents:[owner].into_iter().collect()},owner).map_err(|e|e.to_string())
}
