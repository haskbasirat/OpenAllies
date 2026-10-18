//! Same-owner stored4D4130/579EB0, sharing the held transition kernel.
//! The canonical component table/F8, source Bank, instance State, current
//! variables, Arena and published spatial owner are borrowed in place.
use crate::{actor_world::{ActorAccess,ObjectKey,RuntimeComponent},catalog::Catalog,definitions::Bank,
    load_actor_terrain_refresh::{self as source,Property},restoration::Staged};
use aa_actors::{ActorId,actor_creation::{Error,Result},components::ComponentSlot as Slot,overlap_list};
use aa_numeric::Precision;
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}
pub struct Context<'a,'store>{
    pub actors:&'a mut ActorAccess<'store>,pub arena:&'a mut aa_mind::Arena,pub bank:&'a mut Bank,
    pub spatial:&'a mut crate::spatial_world::SpatialWorld,pub terrain:&'a crate::battle_terrain::BattleTerrain,
    pub variables:&'a mut crate::load_view_variables::State,pub clock:&'a aa_simulation::NativeClock,
    pub inventory:&'a aa_mapworld::WorldInventory,
}
struct Bound<'a,'context,'store,'source>{catalog:&'a Catalog<'source>,context:&'a mut Context<'context,'store>,
    key:ObjectKey,owner:ActorId,source_pc:Precision,pc:Precision,path:&'a mut Vec<ActorId>}
impl Bound<'_,'_,'_,'_>{
    fn object(&self,key:ObjectKey)->Result<&crate::actor_world::RuntimeObject>{self.context.actors.object(key).map_err(error)}
    fn word(&self,key:ObjectKey)->Result<u32>{self.object(key)?.actor.as_ref().ok_or(Error::NullDereference("stored4D4130 Actor"))?
        .words.get(&0xF8).copied().ok_or_else(||error("stored4D4130 actual initialized F8 absent"))}
    fn present(&self,slot:Slot)->Result<bool>{match self.context.actors.component(self.owner,slot).map_err(error)?{
        None=>Ok(false),Some(RuntimeComponent::Organization(v))if slot==Slot::Organization&&v.header.owner==self.owner=>Ok(true),
        Some(RuntimeComponent::Properties(v))if slot==Slot::Properties&&v.owner.0==self.owner.raw()=>Ok(true),
        _=>Err(error("stored4D4130 actual component payload/owner differs")),
    }}
    fn parent(&self,id:ActorId)->Result<Option<u32>>{
        let key=self.context.actors.key(id).ok_or_else(||error("stored570560 actual parent generation absent"))?;
        let object=self.object(key)?;
        crate::live_object_init::Source::bind_in_bank(self.catalog,self.context.bank,&object.definition)?;
        let actor=object.actor.as_ref().ok_or(Error::NullDereference("stored570560 parent Actor"))?;
        let flags=self.context.actors.placement(key,self.context.spatial).map_err(error)?.flags;
        if flags&(1<<29)!=0||!overlap_list::eligible(actor.components.state.low,actor.components.state.high){return Ok(None)}
        self.word(key).map(Some)
    }
    fn check_property(&self,p:&Property)->Result<()>{
        if p.source_precision()!=self.source_pc||!std::sync::Arc::ptr_eq(p.manager(),&self.catalog.layout_data(self.source_pc).map_err(error)?){
            return Err(error("stored terrain Property source manager/PC differs"))
        }
        Ok(())
    }
}
impl crate::actor_terrain_fields::Fields for Bound<'_,'_,'_,'_>{
    type Property=Property;
    fn flags_state(&self)->Result<(u32,u32,u32)>{
        let state=self.object(self.key)?.components().map_err(error)?.state;
        let flags=self.context.actors.placement(self.key,self.context.spatial).map_err(error)?.flags;
        Ok((flags,state.low,state.high))
    }
    fn current(&self)->Result<u32>{self.word(self.key)}
    fn select(&mut self)->Result<u32>{
        match self.context.actors.component(self.owner,Slot::Element).map_err(error)?{
            None=>(),Some(RuntimeComponent::Element(v))if v.header.owner==self.owner=>{
                if v.flag_1c&&let Some(parent)=v.organization&&let Some(token)=self.parent(parent)?{return Ok(token)}
            },_=>return Err(error("stored570560 actual Element payload/owner differs")),
        }
        let tile=self.context.actors.placement(self.key,self.context.spatial).map_err(error)?.cached_tile
            .ok_or_else(||error("stored4D4130 actual cached tile absent"))?;
        let id=source::terrain_id_at(self.catalog,self.context.terrain,tile,self.source_pc)?;
        self.context.bank.intern("TerrainData",&id).map_err(error)
    }
    fn store(&mut self,token:u32)->Result<()>{
        self.context.actors.object_mut(self.key).map_err(error)?.actor.as_mut().ok_or(Error::NullDereference("stored4D4130 Actor store"))?.words.insert(0xF8,token);Ok(())
    }
    fn organization(&self)->Result<bool>{self.present(Slot::Organization)}
    fn refresh_organization(&mut self)->Result<()>{
        let mut slot=0usize;
        loop{
            let id=match self.context.actors.component(self.owner,Slot::Organization).map_err(error)?{
                Some(RuntimeComponent::Organization(v))if v.header.owner==self.owner=>{
                    if v.members.len()>i32::MAX as usize{return Err(error("stored4364C0 native signed member count overflow"))}
                    v.members.get(slot).copied()
                },_=>return Err(error("stored579EB0 actual Organization owner changed")),
            };
            let Some(id)=id else{return Ok(())};slot=slot.checked_add(1).ok_or_else(||error("stored579EB0 member index overflow"))?;
            if id.is_null(){continue}
            if self.path.contains(&id){return Err(error("stored579EB0 native recursive cycle requires reentrant owner service"))}
            let key=self.context.actors.key(id).ok_or(Error::NullDereference("stored579EB0 actual member generation"))?;
            match self.context.actors.component(id,Slot::Element).map_err(error)?{
                Some(RuntimeComponent::Element(v))if v.header.owner==id=>(),None=>return Err(Error::NullDereference("570550 member Element+4 owner")),
                _=>return Err(error("stored570550 actual member Element payload/owner differs")),
            }
            run(self.catalog,self.context,key,self.pc,self.path)?;
        }
    }
    fn properties(&self)->Result<bool>{self.present(Slot::Properties)}
    fn property_list(&self,token:u32)->Result<Vec<Property>>{source::properties_in_bank(self.catalog,self.context.bank,token,self.source_pc)}
    fn remove(&mut self,p:&Property)->Result<()>{
        //The linked metadata was resolved by this Bound's property_list.
        //The shared removal entry reads the actual effect count before any
        //definition/instance compilation, preserving native empty-owner order.
        crate::stored_state_properties::remove_reference(self.catalog,self.context.actors,self.context.bank,self.context.variables,self.owner,Some(p.reference()),self.owner,self.pc).map_err(error)
    }
    fn applies(&mut self,p:&Property)->Result<bool>{
        self.check_property(p)?;
        crate::staged_property_admission::applicable_on_split_owners(self.catalog,self.context.actors,self.context.bank,self.owner,p.reference()).map_err(error)
    }
    fn add(&mut self,p:&Property,kind:u32)->Result<()>{
        self.check_property(p)?;
        crate::stored_state_properties::add(self.catalog,self.context.actors,self.context.arena,self.context.bank,self.context.spatial,
            self.context.variables,self.context.clock,self.context.inventory,self.owner,p.reference(),kind,Some(self.owner),self.pc).map_err(error)
    }
    fn null_diagnostic(&mut self)->Result<()>{Err(Error::Unsupported("stored4D4211 actual null TerrainData diagnostic sink required"))}
}
fn run(catalog:&Catalog<'_>,context:&mut Context<'_,'_>,key:ObjectKey,pc:Precision,path:&mut Vec<ActorId>)->Result<()>{
    catalog.require_source_owner().map_err(error)?;
    let owner=context.actors.id(key).ok_or_else(||error("stored terrain current GTID absent"))?;
    if context.actors.key(owner)!=Some(key){return Err(error("stored terrain current generation differs"))}
    let object=context.actors.object(key).map_err(error)?;
    if !matches!(object.definition.native_class,3|4)||object.definition.native_class!=object.definition.proof.category.runtime_class
        ||object.actor.is_none()||!crate::load_spatial::same_proof(&object.thing.definition,&object.definition.proof){return Err(error("stored terrain actual Actor proof/layout differs"))}
    let source=crate::live_object_init::Source::bind_in_bank(catalog,context.bank,&object.definition)?;
    let source_pc=source.source_precision();
    if context.terrain.source_precision()!=source_pc||context.spatial.index().precision()!=pc{return Err(error("stored terrain source/live PC owner differs"))}
    path.push(owner);
    let outcome=crate::actor_terrain_fields::transition(&mut Bound{catalog,context,key,owner,source_pc,pc,path});
    path.pop();outcome
}
pub fn refresh_on_owners(catalog:&Catalog<'_>,context:&mut Context<'_,'_>,key:ObjectKey,pc:Precision)->Result<()>{run(catalog,context,key,pc,&mut Vec::new())}
pub fn refresh(catalog:&Catalog<'_>,staged:&mut Staged,key:ObjectKey,pc:Precision)->Result<()>{
    let source_pc=staged.definitions.combat_source().map_err(error)?.source_precision();
    let world=staged.load_world.as_mut().ok_or_else(||error("stored terrain actual World owner absent"))?;
    if world.source_fields.source_precision()!=source_pc||world.source_fields.receipt().sources!=catalog.actors.sources{return Err(error("stored terrain actual World source-fields differ"))}
    let crate::load_world::SpatialOwner::Adopted(spatial)=&mut world.spatial else{return Err(error("stored terrain actual published index absent"))};
    if spatial.error.is_some()||!spatial.pending.is_empty(){return Err(error("stored terrain index adoption incomplete"))}
    let(arena,mut actors)=staged.actors.split_minds();
    refresh_on_owners(catalog,&mut Context{actors:&mut actors,arena,bank:&mut staged.definitions,spatial:&mut spatial.world,
        terrain:&world.terrain,variables:&mut world.variables,clock:&staged.clock,inventory:&staged.inventory},key,pc)
}
