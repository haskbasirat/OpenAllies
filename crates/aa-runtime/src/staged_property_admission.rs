//!4CEF10/5035E0 over the same live ActorStore and moved source Bank.
//! Required-property lookup retains disabled duplicates; native Organization
//! recursion and vacant-slot source fallback reuse the proved shared kernel.
use crate::{actor_world::{ActorStore,ObjectKey,RuntimeComponent},catalog::Catalog,
    definitions::Bank,load_property_instances::State,restoration::Staged};
use aa_actors::{ActorId,components::ComponentSlot as Slot,state_effects::{self as native,Applicability}};
use aa_body::{credit::Object,property_api,property_admission};
use aa_numeric::Precision;
use aa_rules::native::layout_data::PropertyRef;
use std::cell::RefCell;
#[path="replacement_property_reference.rs"]
mod captured;
type Result<T> = std::result::Result<T,String>;
fn error(e:impl std::fmt::Display)->native::Error{native::Error::World(e.to_string())}
fn body(e:impl std::fmt::Display)->aa_body::Error{aa_body::Error::Callback(e.to_string())}

trait ReadActors{
    fn property_actor(&self,id:ActorId)->native::Result<&aa_actors::loading::ActorShell<RuntimeComponent>>;
}
impl ReadActors for ActorStore{
    fn property_actor(&self,id:ActorId)->native::Result<&aa_actors::loading::ActorShell<RuntimeComponent>>{
        let key=self.key(id).ok_or_else(||error("5035E0 current actor generation absent"))?;
        let object=self.object(key).map_err(error)?;
        if !matches!(object.definition.native_class,3|4){return Err(error("5035E0 current allocation is not proven Actor storage"))}
        object.actor.as_ref().ok_or_else(||error("5035E0 actual Actor allocation absent"))
    }
}
impl ReadActors for crate::actor_world::ActorAccess<'_>{
    fn property_actor(&self,id:ActorId)->native::Result<&aa_actors::loading::ActorShell<RuntimeComponent>>{
        let key=self.key(id).ok_or_else(||error("5035E0 current actor generation absent"))?;
        let object=self.object(key).map_err(error)?;
        if !matches!(object.definition.native_class,3|4){return Err(error("5035E0 current allocation is not proven Actor storage"))}
        object.actor.as_ref().ok_or_else(||error("5035E0 actual Actor allocation absent"))
    }
}
struct Current<'a,'source,A:ReadActors>{catalog:&'a Catalog<'source>,actors:&'a A,state:&'a State,
    bank:RefCell<&'a mut Bank>,source_pc:Precision}
impl<A:ReadActors> Current<'_,'_,A>{
    fn actor(&self,id:ActorId)->native::Result<&aa_actors::loading::ActorShell<RuntimeComponent>>{
        self.actors.property_actor(id)
    }
    fn properties(&self,id:ActorId)->native::Result<Option<&aa_body::properties::PropertiesState>>{
        match self.actor(id)?.components.get(Slot::Properties){
            None=>Ok(None),Some(RuntimeComponent::Properties(v))if v.owner.0==id.raw()=>Ok(Some(v)),
            _=>Err(error("5035E0 actual Properties payload/owner differs")),
        }
    }
    fn organization(&self,id:ActorId)->native::Result<Option<&aa_actors::ground_states::OrganizationState<String>>>{
        match self.actor(id)?.components.get(Slot::Organization){
            None=>Ok(None),Some(RuntimeComponent::Organization(v))if v.header.owner==id=>Ok(Some(v)),
            _=>Err(error("5035E0 actual Organization payload/owner differs")),
        }
    }
    fn definition_id(&self,definition:Object)->native::Result<String>{
        let bank=self.bank.borrow();let entry=bank.entry(definition.0).map_err(error)?;
        if entry.key.domain!="actordata"{return Err(error("503390 definition token belongs to another source domain"))}
        Ok(entry.key.name.clone())
    }
}
impl<A:ReadActors> Applicability for Current<'_,'_,A>{
    fn has_properties(&self,a:ActorId)->native::Result<bool>{Ok(self.properties(a)?.is_some())}
    fn property_requirements(&self,p:Object)->native::Result<Vec<Object>>{Ok(self.state.facts(p).map_err(error)?.required().to_vec())}
    fn actor_has_property(&self,a:ActorId,p:Object)->native::Result<bool>{
        let Some(properties)=self.properties(a)?else{return Ok(false)};
        native::property_present(properties,self.state.instances(),p)
    }
    fn organization_count(&self,a:ActorId)->native::Result<Option<usize>>{Ok(self.organization(a)?.map(|o|o.members.len()))}
    fn organization_member(&self,a:ActorId,index:usize)->native::Result<Option<ActorId>>{
        let org=self.organization(a)?.ok_or_else(||error("5035E0 Organization disappeared"))?;
        let id=*org.members.get(index).ok_or_else(||error("5035E0 current slot outside member array"))?;
        Ok((!id.is_null()).then_some(id))
    }
    fn vacant_member_definition(&self,a:ActorId,index:usize)->native::Result<Option<Object>>{
        let org=self.organization(a)?.ok_or_else(||error("5035E0 Organization disappeared"))?;
        let data=self.catalog.layout_data(self.source_pc).map_err(error)?;
        let layout=data.find_layout(&org.layout.definition).ok_or_else(||error("5035E0 current LayoutData source absent"))?;
        let group=*layout.slot_groups.get(index).ok_or_else(||error("5035E0 slot-group source absent"))?;
        let id=org.layout.parameters.get(group as usize).ok_or_else(||error("5035E0 expanded parameter absent"))?;
        id.as_ref().map(|id|{
            self.catalog.require_untranslated_actor_identity(id).map_err(error)?;
            self.bank.borrow_mut().intern("ActorData",id).map(Object).map_err(error)
        }).transpose()
    }
    fn definition_component_mask(&self,d:Object)->native::Result<u32>{
        self.catalog.canonical_component_mask(&self.definition_id(d)?,self.source_pc).map_err(error)
    }
    fn definition_properties(&self,d:Object)->native::Result<Vec<Object>>{
        let data=self.catalog.layout_data(self.source_pc).map_err(error)?;
        aa_rules::native::layout_data::native_actor_properties(self.catalog.actors,&data,&self.definition_id(d)?,self.source_pc)
            .map_err(error)?.into_iter().map(|p|self.state.definition_for_reference(p).map_err(error)).collect()
    }
}

///The owner binding is the actual mutable stored arena and source Bank. Native
///read-only eligibility may intern only portable immutable source identities;
///no Property instance or gameplay component is created by this predicate.
pub fn applicable_on_owners(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&mut Bank,
    key:ObjectKey,property:PropertyRef)->Result<bool>{
    catalog.require_source_owner()?;
    let receipt=bank.combat_source()?;receipt.completed_means()?;
    if receipt.source_identities()!=catalog.actors.sources.as_slice(){return Err("stored5035E0 source Bank universe differs".into())}
    let source_pc=receipt.source_precision();
    let owner=actors.id(key).ok_or("5035E0 current stored GTID absent")?;
    let object=actors.object(key).map_err(|e|e.to_string())?;
    crate::live_object_init::Source::bind_in_bank(catalog,bank,&object.definition).map_err(|e|e.to_string())?;
    if !matches!(object.definition.native_class,3|4)
        ||!crate::load_spatial::same_proof(&object.thing.definition,&object.definition.proof){
        return Err("5035E0 actual stored Actor/source allocation differs".into())
    }
    actors.property_allocations.bind_in_bank(catalog,bank)?;
    let state=&actors.property_allocations;let definition=state.definition_for_reference(property)?;
    native::applicable(&Current{catalog,actors,state,bank:RefCell::new(bank),source_pc},owner,definition).map_err(|e|e.to_string())
}

///Same read-only eligibility kernel while a synchronous caller already owns
///the paired Mind Arena loan. It does not construct a second ActorStore.
pub(crate) fn applicable_on_split_owners(catalog:&Catalog<'_>,actors:&mut crate::actor_world::ActorAccess<'_>,bank:&mut Bank,
    owner:ActorId,property:PropertyRef)->Result<bool>{
    catalog.require_source_owner()?;
    let receipt=bank.combat_source()?;receipt.completed_means()?;
    if receipt.source_identities()!=catalog.actors.sources.as_slice(){return Err("stored5035E0 source Bank universe differs".into())}
    let source_pc=receipt.source_precision();
    actors.property_actor(owner).map_err(|e|e.to_string())?;
    actors.property_allocations.bind_in_bank(catalog,bank)?;
    let state=&*actors.property_allocations;let definition=state.definition_for_reference(property)?;
    native::applicable(&Current{catalog,actors:&*actors,state,bank:RefCell::new(bank),source_pc},owner,definition).map_err(|e|e.to_string())
}

enum PropertyInput<'a>{Reference(PropertyRef),Captured(&'a crate::map_replacement_queue::DefinitionRef)}
struct Admission<'a,'source>{staged:&'a mut Staged,catalog:&'a Catalog<'source>,key:ObjectKey,
    owner:ActorId,reference:PropertyInput<'a>,symbol:Object,pc:Precision}
impl Admission<'_,'_>{
    fn check(&self,a:Object)->aa_body::Result<()>{
        if a.0!=self.owner.raw()||self.staged.actors.id(self.key)!=Some(self.owner){return Err(body("4CEF10 current stored actor identity changed"))}
        Ok(())
    }
    fn reference(&mut self)->Result<PropertyRef>{
        match &self.reference {
            PropertyInput::Reference(reference)=>Ok(*reference),
            PropertyInput::Captured(definition)=>{
                let token=captured::token(&self.staged.definitions,definition)?;
                let state=&mut self.staged.actors.property_allocations;
                state.bind_in_bank(self.catalog,&mut self.staged.definitions)?;
                let facts=state.facts(token)?;
                if !facts.id().eq_ignore_ascii_case(&definition.canonical_ids){
                    return Err("4CEF10 captured Property and actual source facts differ".into());
                }
                Ok(facts.reference())
            }
        }
    }
}
impl property_admission::World for Admission<'_,'_>{
    fn property_component(&self,a:Object)->aa_body::Result<Option<Object>>{
        self.check(a)?;let actor=self.staged.actors.object(self.key).map_err(body)?.actor.as_ref().ok_or_else(||body("4CEF10 actual Actor allocation absent"))?;
        match actor.components.get(Slot::Properties){None=>Ok(None),Some(RuntimeComponent::Properties(v))if v.owner.0==self.owner.raw()=>Ok(Some(a)),_=>Err(body("4CEF10 actual Properties payload/owner differs"))}
    }
    fn component_98_present(&self,a:Object)->aa_body::Result<bool>{
        self.check(a)?;Ok(self.staged.actors.object(self.key).map_err(body)?.actor.as_ref().ok_or_else(||body("4CEF10 Actor absent"))?.components.present(Slot::Settlement))
    }
    fn applicable(&mut self,a:Object,p:Object)->aa_body::Result<bool>{
        self.check(a)?;if p!=self.symbol{return Err(body("4CEF10 transported Property identity changed"))}
        let reference=self.reference().map_err(body)?;
        applicable_on_owners(self.catalog,&mut self.staged.actors,&mut self.staged.definitions,self.key,reference).map_err(body)
    }
    fn create_and_insert(&mut self,component:Object,p:Object,kind:u32,source:Option<Object>)->aa_body::Result<()>{
        self.check(component)?;if p!=self.symbol{return Err(body("5817A0 transported Property identity changed"))}
        let reference=self.reference().map_err(body)?;
        crate::staged_property_instances::add(self.staged,self.catalog,self.key,reference,kind,source.map(|id|ActorId::from_raw(id.0)),self.pc).map_err(body)
    }
}
///Native4CEF10, retaining its null-property, missing-Properties and kind8/10
///Settlement gates. The PropertyRef is an opaque source identity transported
///through the wrapper; its real Bank token is resolved only by the reached
///eligibility/allocation child. No ordinal is used as a gameplay token.
pub fn apply(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,property:Option<PropertyRef>,
    kind:u32,source:Option<ActorId>,pc:Precision)->Result<bool>{
    let Some(reference)=property else{return Ok(false)};
    let owner=staged.actors.id(key).ok_or("4CEF10 current stored GTID absent")?;
    let symbol=Object(reference.handle().map_err(|e|e.to_string())?);
    property_api::apply_property(Object(owner.raw()),Some(symbol),kind,source.map(|a|Object(a.raw())),
        &mut Admission{staged,catalog,key,owner,reference:PropertyInput::Reference(reference),symbol,pc}).map_err(|e|e.to_string())
}

///4FBB32 passes each captured Property, kind2 and the SAME new Actor as source.
///The opaque queue token is resolved only in the reached native child: null
///Property and absent Properties never demand a completed source arena.
pub(crate) fn apply_captured(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,
    property:Option<&crate::map_replacement_queue::DefinitionRef>,pc:Precision)->Result<bool>{
    let Some(definition)=property else{return Ok(false)};
    let owner=staged.actors.id(key).ok_or("4CEF10 current stored GTID absent")?;
    let symbol=Object(definition.token);
    property_api::apply_property(Object(owner.raw()),Some(symbol),2,Some(Object(owner.raw())),
        &mut Admission{staged,catalog,key,owner,reference:PropertyInput::Captured(definition),symbol,pc})
        .map_err(|e|e.to_string())
}
