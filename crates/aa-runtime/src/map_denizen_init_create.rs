//!Fresh568750 over the same live Denizen, immutable source arena and queues.
use crate::{actor_world::{ObjectKey,RuntimeComponent},catalog::{ActorBindings,Catalog},restoration::Staged};
use aa_actors::{ActorId,actor_creation::{Error,Result},component_virtuals::{self,DenizenCreateWorld,LeafParams},components::ComponentSlot};
use aa_numeric::Binary;
use std::sync::Arc;
fn error(e:impl std::fmt::Display)->Error{Error::World(e.to_string())}

struct Bound<'a,'source>{staged:&'a mut Staged,catalog:&'a Catalog<'source>,binding:Arc<ActorBindings>,key:ObjectKey,id:ActorId}
impl Bound<'_,'_>{
    fn denizen(&self)->Result<&aa_actors::denizen_state::DenizenState>{
        match self.staged.actors.component(self.id,ComponentSlot::Denizen).map_err(error)?{
            Some(RuntimeComponent::Denizen(d))if d.header.owner==self.id=>Ok(d),
            _=>Err(error("568750 actual Denizen allocation/owner differs")),
        }
    }
    fn denizen_mut(&mut self)->Result<&mut aa_actors::denizen_state::DenizenState>{
        match self.staged.actors.component_mut(self.id,ComponentSlot::Denizen).map_err(error)?{
            Some(RuntimeComponent::Denizen(d))if d.header.owner==self.id=>Ok(d),
            _=>Err(error("568750 current Denizen allocation/owner differs")),
        }
    }
    fn guards(&self)->Result<&aa_rules::denizen::DenizenGuards>{
        self.binding.source_ground.denizen_guards.as_ref().ok_or_else(||error("568750 actual Denizen source fields absent"))
    }
}
impl DenizenCreateWorld for Bound<'_,'_>{
    type Actor=();
    fn denizen_construction(&self,_:())->Result<bool>{Ok(self.staged.actors.object(self.key).map_err(error)?.components().map_err(error)?.state.construction())}
    fn denizen_count(&self,_:())->Result<usize>{Ok(self.denizen()?.inventory.len())}
    fn denizen_health(&mut self,_:(),index:usize)->Result<Binary>{
        let target=&self.denizen()?.inventory.get(index).ok_or_else(||error("568780 current Inventory node absent"))?.target;
        self.catalog.require_untranslated_actor_identity(target).map_err(error)?;
        let health=self.binding.definition.ground.denizen_target_health_bits.as_ref()
            .and_then(|values|values.get(target)).ok_or_else(||error("45D730 actual target ActorData284 absent"))?;
        Binary::from_f32_bits(*health).map_err(error)
    }
    fn denizen_store_health(&mut self,_:(),index:usize,bits:u32)->Result<()>{
        self.denizen_mut()?.inventory.get_mut(index).ok_or_else(||error("56878A current Inventory node absent"))?.value_08=aa_body::F32(bits);Ok(())
    }
    fn denizen_guard_range(&self,_:())->Result<u32>{Ok(self.guards()?.finalized_bits[0])}
    fn denizen_inventory_head_present(&self,_:())->Result<bool>{
        //Data35C is the authored Inventory-node head, including nodes with
        //zero/negative counts. It is not the constructed runtime slot count.
        Ok(!self.guards()?.inventory.is_empty())
    }
    fn denizen_schedule(&mut self,_:())->Result<()>{
        let owner=self.staged.owners.get(crate::scheduler_owners::Target::Component{object:self.key,slot:ComponentSlot::Denizen as u8}).map_err(error)?;
        self.staged.queues.add_periodic(1,owner,0,&mut self.staged.rng).map_err(error)?;Ok(())
    }
    fn denizen_delay(&self,_:())->Result<u32>{
        let root=self.catalog.actors.resolved_actor(&self.binding.definition.proof.id).map_err(error)?;
        let denizen=root.child("DenizenComponent").map_err(error)?.ok_or_else(||error("5687C8 actual Denizen source absent"))?;
        //4629F8 initializes3A4=0. Only a real optional Runway overrides it
        //at46759C..4675A8 using its assigned Fixed delay field.
        let Some(runway)=denizen.child("Runway").map_err(error)?else{return Ok(0)};
        match runway.field("delay",self.binding.precision).map_err(error)?{
            Some(aa_rules::native::TypedValue::Scalar(v))if v.field_type==aa_rules::native::ScalarType::Fixed=>Ok(v.bits),
            _=>Err(error("4675A8 actual Runway.delay Fixed field absent")),
        }
    }
    fn denizen_store_time(&mut self,_:(),bits:u32)->Result<()>{
        let word=self.denizen_mut()?.words.get_mut(&0x34).ok_or_else(||error("5687D2 actual initialized Denizen34 absent"))?;
        *word=bits;Ok(())
    }
}
pub fn initialize(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,params:&crate::map_actor_creation::Params)->std::result::Result<bool,String>{
    let binding=crate::map_component_init_create::actual_binding(staged,catalog,key)?;
    let id=staged.actors.id(key).ok_or("568750 actual GTID absent")?;
    let mut bound=Bound{staged,catalog,binding,key,id};
    bound.denizen().map_err(|e|e.to_string())?;
    let params=LeafParams{word_34:params.word_34,mode:params.mode,word_40:params.word_40,word_48:params.word_48};
    component_virtuals::init_create_denizen((),Some(&params),&mut bound).map_err(|e|e.to_string())
}
