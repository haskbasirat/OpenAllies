//! Borrowed Property callback storage, shared by held Load and staged actors.
//! Every projection points at the original component box. Neither shell,
//! component table, Ability arena, identity registry nor definition Bank moves.
use crate::{actor_world::{Entity,ObjectKey,RuntimeComponent},catalog::Catalog,definitions::Bank};
use aa_actors::{ActorId,IdentityRegistry,components::{ComponentSlot as Slot,OtherComponents},
    ground_states::{BodyState,DetectionState,ElementState,OrganizationState}};
use aa_mapworld::restore::{RestoredComponent,RestoredRecord,RuntimeDefinition};
use aa_numeric::Precision;
use std::collections::BTreeMap;
pub enum ComponentRef<'a>{
    Body(&'a BodyState),Performer(&'a aa_combat::constructors::OwnedPerformer),
    Defender(&'a aa_actors::defender_state::DefenderState),Supply(&'a aa_actors::supply_state::SupplyState),
    Detection(&'a DetectionState),Organization(&'a OrganizationState<String>),Element(&'a ElementState<String>),Other(Slot),
}
pub enum ComponentMut<'a>{
    Body(&'a mut BodyState),Performer(&'a mut aa_combat::constructors::OwnedPerformer),
    Defender(&'a mut aa_actors::defender_state::DefenderState),Supply(&'a mut aa_actors::supply_state::SupplyState),
    Detection(&'a mut DetectionState),Organization(&'a mut OrganizationState<String>),Element(&'a mut ElementState<String>),Other(Slot),
}
fn project<M>(v:&aa_actors::owned::Component<String,M,aa_combat::constructors::OwnedPerformer>)->ComponentRef<'_>{
    use aa_actors::owned::Component as C;
    match v {C::Body(v)=>ComponentRef::Body(v),C::Performer(v)=>ComponentRef::Performer(v),C::Defender(v)=>ComponentRef::Defender(v),
        C::Supply(v)=>ComponentRef::Supply(v),C::Detection(v)=>ComponentRef::Detection(v),C::Organization(v)=>ComponentRef::Organization(v),
        C::Element(v)=>ComponentRef::Element(v),v=>ComponentRef::Other(v.slot())}
}
fn project_mut<M>(v:&mut aa_actors::owned::Component<String,M,aa_combat::constructors::OwnedPerformer>)->ComponentMut<'_>{
    use aa_actors::owned::Component as C;
    match v {C::Body(v)=>ComponentMut::Body(v),C::Performer(v)=>ComponentMut::Performer(v),C::Defender(v)=>ComponentMut::Defender(v),
        C::Supply(v)=>ComponentMut::Supply(v),C::Detection(v)=>ComponentMut::Detection(v),C::Organization(v)=>ComponentMut::Organization(v),
        C::Element(v)=>ComponentMut::Element(v),v=>ComponentMut::Other(v.slot())}
}
pub enum Components<'a>{Saved(OtherComponents<'a,RestoredComponent>),Stored(OtherComponents<'a,RuntimeComponent>)}
impl Components<'_>{
    pub fn present(&self,s:Slot)->bool{match self{Self::Saved(v)=>v.present(s),Self::Stored(v)=>v.present(s)}}
    pub fn get(&self,s:Slot)->Option<ComponentRef<'_>>{match self{Self::Saved(v)=>v.get(s).map(project),Self::Stored(v)=>v.get(s).map(project)}}
    pub fn get_mut(&mut self,s:Slot)->Option<ComponentMut<'_>>{match self{Self::Saved(v)=>v.get_mut(s).map(project_mut),Self::Stored(v)=>v.get_mut(s).map(project_mut)}}
}
pub enum Registry<'a>{
    Saved(&'a mut IdentityRegistry<RestoredRecord>),
    Stored{identities:&'a IdentityRegistry<ObjectKey>,others:BTreeMap<ObjectKey,&'a Entity>},
}
impl Registry<'_>{
    pub fn contains(&self,id:ActorId)->bool{match self{Self::Saved(v)=>v.get(id).is_some(),Self::Stored{identities,..}=>identities.get(id).is_some()}}
    fn linked_body(&self,id:ActorId,current:ActorId,components:&Components<'_>)->Result<bool,String>{
        match self{
            Self::Saved(v)=>{
                let Some(entry)=v.get(id)else{return Ok(false)};
                let actor=match &entry.value{
                    RestoredRecord::Actor(e)=>e.shell.actor.as_ref(),
                    RestoredRecord::Kingdom(e)=>e.runtime.as_ref().and_then(|r|r.base.actor.as_ref()),
                    RestoredRecord::Loading=>return Err("561D40 reached held linked Body allocation is required".into()),
                    _=>None,
                };Ok(actor.is_some_and(|a|a.components.present(Slot::Body)))
            }
            Self::Stored{identities,others}=>{
                let Some(entry)=identities.get(id)else{return Ok(false)};
                if id==current{return Ok(components.present(Slot::Body));}
                let entity=others.get(&entry.value).ok_or("561D40 current linked Entity loan absent")?;
                let object=entity.runtime.as_ref().ok_or("561D40 linked runtime allocation absent")?;
                if !matches!(object.definition.native_class,3|4){return Err("561D40 linked ordinary Actor dispatch is required".into());}
                Ok(object.actor.as_ref().ok_or("561D40 linked Actor storage absent")?.components.present(Slot::Body))
            }
        }
    }
}
enum SourceBank<'a>{Catalog,Stored(&'a Bank)}
enum Abilities<'a>{Catalog,Stored(&'a mut BTreeMap<u32,aa_combat::ability::Ability>)}
pub struct Fields<'a>{
    pub definition:&'a RuntimeDefinition,pub precision:Precision,pub owner:ActorId,
    pub words:&'a mut BTreeMap<u32,u32>,pub components:Components<'a>,pub registry:Registry<'a>,
    bank:SourceBank<'a>,abilities:Abilities<'a>,
}
impl<'a> Fields<'a>{
    pub fn saved(definition:&'a RuntimeDefinition,precision:Precision,owner:ActorId,
        words:&'a mut BTreeMap<u32,u32>,components:OtherComponents<'a,RestoredComponent>,
        registry:&'a mut IdentityRegistry<RestoredRecord>)->Self{
        Self{definition,precision,owner,words,components:Components::Saved(components),registry:Registry::Saved(registry),bank:SourceBank::Catalog,abilities:Abilities::Catalog}
    }
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn stored(definition:&'a RuntimeDefinition,precision:Precision,owner:ActorId,
        words:&'a mut BTreeMap<u32,u32>,components:OtherComponents<'a,RuntimeComponent>,
        identities:&'a IdentityRegistry<ObjectKey>,others:BTreeMap<ObjectKey,&'a Entity>,bank:&'a Bank,
        abilities:&'a mut BTreeMap<u32,aa_combat::ability::Ability>)->Self{
        Self{definition,precision,owner,words,components:Components::Stored(components),registry:Registry::Stored{identities,others},bank:SourceBank::Stored(bank),abilities:Abilities::Stored(abilities)}
    }
    pub(crate) fn source_precision(&self,catalog:&Catalog<'_>)->Result<Precision,String>{
        catalog.require_source_owner()?;
        let read=|bank:&Bank|{
            let source=bank.combat_source()?;
            if source.source_identities()!=catalog.actors.sources.as_slice(){return Err("property source Bank differs".into());}
            Ok(source.source_precision())
        };
        match self.bank{
            SourceBank::Catalog=>{let bank=catalog.definition_bank.try_borrow().map_err(|e|e.to_string())?;read(&bank)},
            SourceBank::Stored(bank)=>read(bank),
        }
    }
    pub(crate) fn linked_has_body(&self,id:ActorId)->Result<bool,String>{self.registry.linked_body(id,self.owner,&self.components)}
    pub(crate) fn validate_variables(&self,catalog:&Catalog<'_>,variables:&crate::load_view_variables::State)->Result<(),String>{
        match self.bank{SourceBank::Catalog=>variables.validate_catalog(catalog),SourceBank::Stored(bank)=>variables.validate_in_bank(catalog,bank)}
    }
    pub(crate) fn local_ability(&self,catalog:&Catalog<'_>,handle:u32,pc:Precision)->Result<aa_combat::combat_value::AbilityLocal,String>{
        match &self.abilities{
            Abilities::Catalog=>crate::load_performer_value_refresh::local_ability(catalog,handle,pc).map_err(|e|e.to_string()),
            Abilities::Stored(values)=>{
                let SourceBank::Stored(bank)=self.bank else{return Err("stored Ability has no same moved Bank".into());};
                let ability=values.get(&handle).ok_or("numeric actual Ability allocation absent")?;
                crate::load_performer_value_refresh::local_ability_in_bank(catalog,bank,ability,pc).map_err(|e|e.to_string())
            }
        }
    }
    pub(crate) fn set_ability_power(&mut self,catalog:&Catalog<'_>,handle:u32,power:aa_body::F32)->Result<(),String>{
        match &mut self.abilities{
            Abilities::Catalog=>catalog.ability_arena.try_borrow_mut().map_err(|e|e.to_string())?.get_mut(handle)?.power=power,
            Abilities::Stored(values)=>values.get_mut(&handle).ok_or("numeric actual Ability allocation absent")?.power=power,
        }Ok(())
    }
}
