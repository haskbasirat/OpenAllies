//! Native4D1350 after staging over the same ActorStore and moved source Bank.
//! This is a field/service binding of the existing command-mask kernels. It
//! does not convert live allocations to saved shells or reuse constructor-only
//! absence assumptions. Earlier F0/F4 writes survive every required child.
use crate::{actor_world::{ActorStore,ActorAccess,ObjectKey,RuntimeComponent,Spatial},catalog::Catalog,definitions::Bank};
use aa_actors::{ActorId,ability_flags as flags,components::ComponentSlot as Slot,
    action_publication,construction_actions as construction,structure_actions as structure,
    recruit_actions as recruit,research_actions as research,denizen_actions as denizen,
    mobile_deployment_actions as mobile};
use aa_numeric::Precision;
use aa_rules::native::{layout_data::OrganizationComponentData,recruit_data};

fn error(e:impl std::fmt::Display)->flags::Error{flags::Error::World(e.to_string())}
fn required<T>(what:&'static str)->flags::Result<T>{Err(flags::Error::Missing(what))}

///The actual spatial provider is borrowed only on a reached Thing+8 read.
///A still-unpublished actor uses its own staged placement without querying it.
pub fn rebuild<S:Spatial>(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&mut Bank,
    key:ObjectKey,inventory:&aa_mapworld::WorldInventory,spatial:&S,live_pc:Precision,
)->Result<(),String>{
    rebuild_with_variables(catalog,actors,bank,key,inventory,spatial,None,live_pc)
}
///Actual World callers lend their same current SVar owner. An omitted owner
///is an explicit unbound service and errors only when native reads that cell.
#[allow(clippy::too_many_arguments)]
pub fn rebuild_with_variables<S:Spatial>(catalog:&Catalog<'_>,actors:&mut ActorStore,bank:&mut Bank,
    key:ObjectKey,inventory:&aa_mapworld::WorldInventory,spatial:&S,
    variables:Option<&crate::load_view_variables::State>,live_pc:Precision,
)->Result<(),String>{
    let (arena,mut actors)=actors.split_minds();
    rebuild_on_owners(catalog,&mut actors,arena,bank,key,inventory,spatial,variables,live_pc)
}
///Synchronous Mind state callbacks already hold these disjoint loans. Reborrow
///the SAME entity/component, ability and Mind storage without extracting an
///arena or restarting4D1350 through a second coordinator.
///`actors` and`arena` must be the paired current loans of one ActorStore;
///the caller must keep its active Mind core installed during this callback.
#[allow(clippy::too_many_arguments)]
pub fn rebuild_on_owners<S:Spatial>(catalog:&Catalog<'_>,actors:&mut ActorAccess<'_>,
    arena:&mut aa_mind::Arena,bank:&mut Bank,key:ObjectKey,inventory:&aa_mapworld::WorldInventory,
    spatial:&S,variables:Option<&crate::load_view_variables::State>,live_pc:Precision,
)->Result<(),String>{
    on_owners(catalog,actors,arena,bank,key,inventory,spatial,variables,live_pc,|world,owner|flags::rebuild(world,owner))
}
///Native57B390 is only the Organization contributor. In particular, it must
///not invoke the full4D1350 callback sequence during57C870/579DD0.
#[allow(clippy::too_many_arguments)]
pub fn organization_on_owners<S:Spatial>(catalog:&Catalog<'_>,actors:&mut ActorAccess<'_>,
    arena:&mut aa_mind::Arena,bank:&mut Bank,key:ObjectKey,inventory:&aa_mapworld::WorldInventory,
    spatial:&S,variables:Option<&crate::load_view_variables::State>,live_pc:Precision,
)->Result<(),String>{
    on_owners(catalog,actors,arena,bank,key,inventory,spatial,variables,live_pc,|world,owner|flags::organization(world,owner))
}
#[allow(clippy::too_many_arguments)]
fn on_owners<S:Spatial,T>(catalog:&Catalog<'_>,actors:&mut ActorAccess<'_>,
    arena:&mut aa_mind::Arena,bank:&mut Bank,key:ObjectKey,inventory:&aa_mapworld::WorldInventory,
    spatial:&S,variables:Option<&crate::load_view_variables::State>,live_pc:Precision,
    call:impl FnOnce(&mut Bound<'_,'_,'_,S>,ActorId)->flags::Result<T>,
)->Result<T,String>{
    catalog.require_source_owner()?;
    let source=bank.combat_source()?;
    source.completed_means()?;
    if source.source_identities()!=catalog.actors.sources.as_slice(){return Err("stored4D1350 source Bank universe differs".into())}
    let source_pc=source.source_precision();
    let owner=actors.id(key).ok_or("stored4D1350 actual GTID absent")?;
    let mut world=Bound{catalog,actors,arena,bank,inventory,spatial,variables,source_pc,live_pc};
    world.definition(owner).map_err(|e|e.to_string())?;
    call(&mut world,owner).map_err(|e|e.to_string())
}

struct Bound<'a,'store,'source,S>{catalog:&'a Catalog<'source>,actors:&'a mut ActorAccess<'store>,
    arena:&'a mut aa_mind::Arena,
    bank:&'a mut Bank,inventory:&'a aa_mapworld::WorldInventory,spatial:&'a S,
    variables:Option<&'a crate::load_view_variables::State>,
    source_pc:Precision,live_pc:Precision}
impl<S:Spatial> Bound<'_,'_,'_,S>{
    fn key(&self,a:ActorId)->flags::Result<ObjectKey>{self.actors.key(a).ok_or(flags::Error::Missing("stored action current GTID"))}
    fn object(&self,a:ActorId)->flags::Result<&crate::actor_world::RuntimeObject>{self.actors.object(self.key(a)?).map_err(error)}
    fn shell(&self,a:ActorId)->flags::Result<&aa_actors::loading::ActorShell<RuntimeComponent>>{
        self.object(a)?.actor.as_ref().ok_or(flags::Error::Missing("stored action actual Actor allocation"))
    }
    fn component(&self,a:ActorId,s:Slot)->flags::Result<Option<&RuntimeComponent>>{Ok(self.shell(a)?.components.get(s))}
    fn word(&self,a:ActorId,o:u32)->flags::Result<u32>{self.shell(a)?.words.get(&o).copied().ok_or_else(||error(format!("stored Actor+{o:X} is uninitialized")))}
    fn definition(&self,a:ActorId)->flags::Result<&aa_mapworld::restore::RuntimeDefinition>{
        let object=self.object(a)?;
        let d=&object.definition;
        let expected=self.catalog.actor_bindings(&d.proof.id,self.source_pc).map_err(error)?;
        if object.kind!=aa_actors::loading::LoadKind::Actor || d.native_class!=3
            || !crate::load_spatial::same_proof(&object.thing.definition,&d.proof)
            || !crate::load_spatial::same_proof(&expected.definition.proof,&d.proof)
            ||d.component_mask!=expected.definition.component_mask||d.flags!=expected.definition.flags{
            return required("stored4D1350 ordinary Actor/source proof differs")
        }
        //The same actual source owner is required; Catalog's post-staging Bank
        //is intentionally empty and is never temporarily reinstalled here.
        if self.bank.combat_source().map_err(error)?.source_precision()!=self.source_pc{
            return required("stored4D1350 source precision changed")
        }
        Ok(d)
    }
    fn state_type(&self,i:u8)->flags::Result<u32>{
        let d=aa_actors::state::DESCRIPTORS.get(usize::from(i)).ok_or(flags::Error::Missing("stored action ActorState index"))?;
        recruit_data::actor_state_type(self.catalog.actors,d.name,self.source_pc).map_err(error)
    }
    fn structure_parent(&self,a:ActorId)->flags::Result<Option<Option<ActorId>>>{match self.component(a,Slot::Structure)?{
        None=>Ok(None),Some(RuntimeComponent::Structure(v))if v.header.owner==a=>Ok(Some((v.object_handle_10!=0).then_some(ActorId::from_raw(v.object_handle_10)))),
        _=>required("stored Structure owner/payload"),
    }}
    fn wall_reference(&self,a:ActorId)->flags::Result<Option<Option<ActorId>>>{match self.component(a,Slot::Wall)?{
        None=>Ok(None),Some(_)=>required("stored Wall+10 payload"),
    }}
    fn settlement_reference(&self,a:ActorId)->flags::Result<Option<Option<ActorId>>>{match self.component(a,Slot::Settlement)?{
        None=>Ok(None),Some(_)=>required("stored Settlement+14 payload"),
    }}
    fn properties_flags(&self,a:ActorId)->flags::Result<Option<u32>>{match self.component(a,Slot::Properties)?{
        None=>Ok(None),Some(RuntimeComponent::Properties(v))if v.owner.0==a.raw()=>Ok(Some(v.modifier.flags)),
        _=>required("stored Properties owner/payload"),
    }}
    fn targets(&self,a:ActorId)->flags::Result<Vec<String>>{
        recruit_data::actor_recruit_targets(self.catalog.actors,&self.definition(a)?.proof.id).map(|v|v.ids).map_err(error)
    }
    fn org(&self,a:ActorId)->flags::Result<&aa_actors::ground_states::OrganizationState<String>>{match self.component(a,Slot::Organization)?{
        Some(RuntimeComponent::Organization(v))if v.header.owner==a=>Ok(v),_=>required("stored Organization owner/payload"),
    }}
    fn org_word(&self,a:ActorId,o:u32)->flags::Result<u32>{self.org(a)?.words.get(&o).copied().ok_or_else(||error(format!("stored Organization+{o:X} uninitialized")))}
}

impl<S:Spatial> flags::World for Bound<'_,'_,'_,S>{
    fn actor_fields(&self,a:ActorId)->flags::Result<flags::ActorFields>{
        let state=self.shell(a)?.components.state;
        Ok(flags::ActorFields{thing_flags:self.actors.placement(self.key(a)?,self.spatial).map_err(error)?.flags,
            state_bits:u64::from(state.low)|(u64::from(state.high)<<32),action_bits:u64::from(self.word(a,0xf0)?)|(u64::from(self.word(a,0xf4)?)<<32)})
    }
    fn set_action_bits(&mut self,a:ActorId,bits:u64)->flags::Result<()>{
        self.word(a,0xf0)?;self.word(a,0xf4)?;
        self.actors.set_action_bits(a,bits).map_err(error)
    }
    fn has_component(&self,a:ActorId,s:Slot)->flags::Result<bool>{Ok(self.component(a,s)?.is_some())}
    fn element(&self,a:ActorId)->flags::Result<flags::ElementView>{match self.component(a,Slot::Element)?{
        Some(RuntimeComponent::Element(v))if v.header.owner==a=>Ok(flags::ElementView{owner:v.header.owner,definition_present:v.element_definition.is_some(),organization:v.organization,ready:v.flag_1c}),
        _=>required("stored Element owner/payload"),
    }}
    fn organization_member_count(&self,a:ActorId)->flags::Result<usize>{Ok(self.org(a)?.members.len())}
    fn organization_member(&self,a:ActorId,i:usize)->flags::Result<Option<ActorId>>{
        let m=*self.org(a)?.members.get(i).ok_or(flags::Error::Missing("stored Organization member ordinal"))?;
        Ok((!m.is_null()).then_some(m))
    }
    fn organization_member_44(&self,a:ActorId)->flags::Result<Option<ActorId>>{Ok((self.org_word(a,0x44)?!=0).then(||ActorId::from_raw(self.org_word(a,0x44).expect("same immutable field"))))}
    fn organization_reference_d0(&self,a:ActorId)->flags::Result<ActorId>{Ok(ActorId::from_raw(self.org_word(a,0xd0)?))}
    fn valid_reference(&self,a:ActorId)->flags::Result<bool>{Ok(self.actors.key(a).is_some())}
    fn organization_group_count(&self,_:ActorId)->flags::Result<usize>{required("stored Organization578A30 current source-layout binding")}
    fn organization_attack_group(&self,_:ActorId,_:usize)->flags::Result<flags::AttackGroup>{required("stored Organization578A30 current local Ability bindings")}
    fn owner(&mut self,a:ActorId)->flags::Result<Option<ActorId>>{
        self.definition(a)?;let w=self.word(a,0xe8)?;Ok((w!=0).then_some(ActorId::from_raw(w)))
    }
    fn kingdom_word_218(&self,_:ActorId)->flags::Result<u32>{required("stored Kingdom actual recruit-items vector owner")}
    fn state_resupply(&self,i:u8)->flags::Result<bool>{
        let d=aa_actors::state::DESCRIPTORS.get(usize::from(i)).ok_or(flags::Error::Missing("stored ActorState resupply index"))?;
        self.catalog.actors.native_actor_state(d.name,self.source_pc).map_err(error)?.mind.ok_or(flags::Error::Missing("stored ActorState Mind fields"))?.resupply.boolean().map_err(error)
    }
    fn owner_can_recruit(&mut self,_:ActorId,_:ActorId)->flags::Result<bool>{required("stored owner4E1430 recruitment eligibility")}
    fn owner_recruit_count(&mut self,_:ActorId,_:ActorId)->flags::Result<i32>{required("stored owner4E1E40 recruit count")}
    fn action_available(&mut self,a:ActorId,i:u32)->flags::Result<bool>{flags::available(flags::World::actor_fields(self,a)?.action_bits,i)}
    fn other_contributor(&mut self,a:ActorId,s:Slot)->flags::Result<()>{match s{
        Slot::Recruit=>recruit::contribute(self,a),Slot::Research=>research::contribute(self,a),
        Slot::Structure=>structure::contribute(self,a),Slot::Denizen=>denizen::contribute(self,a),
        Slot::Construction=>construction::contribute(self,a),Slot::MobileDeployment=>mobile::contribute(self,a,self.live_pc),
        Slot::Settlement=>required("stored Settlement585AE0 contributor"),Slot::Flare=>required("stored Flare virtual38 contributor"),
        _=>required("unexpected stored action contributor"),
    }}
    fn mind_contributor(&mut self,a:ActorId)->flags::Result<()>{
        //Split loans keep the SAME Arena and component allocations in place.
        //Only the existing Mind virtual contributor runs, not another4D1350.
        let mut spatial=ReadOnlySpatial(self.spatial);
        let mut services=MindServices{source_pc:self.source_pc,variables:self.variables,bank:&*self.bank,catalog:self.catalog};
        let mut world=crate::mind_actions::World{context:crate::mind_actions::Context{
            actors:&mut *self.actors,arena:&mut *self.arena,spatial:&mut spatial,definitions:&*self.bank,catalog:self.catalog,precision:self.live_pc,
        },services:&mut services};
        flags::World::mind_contributor(&mut world,a)
    }
    fn changed(&mut self,a:ActorId)->flags::Result<()>{action_publication::changed(a,self)}
}

impl<S:Spatial> action_publication::World for Bound<'_,'_,'_,S>{
    fn element_organization(&self,a:ActorId)->flags::Result<Option<ActorId>>{match self.component(a,Slot::Element)?{
        None=>Ok(None),Some(RuntimeComponent::Element(v))if v.header.owner==a=>Ok(v.organization),_=>required("stored Element publication owner/payload"),
    }}
    fn organization_owner(&self,a:ActorId)->flags::Result<ActorId>{Ok(self.org(a)?.header.owner)}
    fn rebuild_actions(&mut self,a:ActorId)->flags::Result<()>{flags::rebuild(self,a)}
    fn owner(&mut self,a:ActorId)->flags::Result<Option<ActorId>>{flags::World::owner(self,a)}
    fn object_byte_4c(&self,a:ActorId)->flags::Result<u8>{Ok(self.object(a)?.object.as_ref().ok_or(flags::Error::Missing("stored Object+4C allocation"))?.byte_4c)}
    fn refresh_owner_views(&mut self,_:Option<ActorId>)->flags::Result<()>{required("stored76C7F0 actual owner-view publication")}
    fn renderer_flags(&self,a:ActorId)->flags::Result<Option<u32>>{
        if self.object(a)?.thing.word_c==0{Ok(None)}else{required("stored renderer link+54 current flags")}
    }
    fn refresh_selected_view(&mut self,_:ActorId)->flags::Result<()>{required("stored50C8E0 selected-view publication")}
}

impl<S:Spatial> construction::World for Bound<'_,'_,'_,S>{
    fn fields(&self,a:ActorId)->flags::Result<flags::ActorFields>{flags::World::actor_fields(self,a)}
    fn set_action_bits(&mut self,a:ActorId,b:u64)->flags::Result<()>{flags::World::set_action_bits(self,a,b)}
    fn definition(&self,a:ActorId)->flags::Result<construction::Definition>{
        match self.component(a,Slot::Construction)?{Some(RuntimeComponent::Construction(v))if v.header.owner==a=>(),_=>return required("stored Construction owner/payload")}
        let v=self.catalog.actors.native_construction_flags(&self.definition(a)?.proof.id,self.source_pc).map_err(error)?.ok_or(flags::Error::Missing("stored Construction ActorData fields"))?;
        Ok(construction::Definition{can_sell:v.can_sell,can_raze:v.can_raze,upgrade_count:v.upgrade_count})
    }
    fn structure_parent(&self,a:ActorId)->flags::Result<Option<Option<ActorId>>>{self.structure_parent(a)}
    fn wall_reference(&self,a:ActorId)->flags::Result<Option<Option<ActorId>>>{self.wall_reference(a)}
    fn settlement_reference(&self,a:ActorId)->flags::Result<Option<Option<ActorId>>>{self.settlement_reference(a)}
    fn properties_flags(&self,a:ActorId)->flags::Result<Option<u32>>{self.properties_flags(a)}
    fn allow_sell_under_siege(&mut self)->flags::Result<bool>{
        let rules=self.catalog.actors.native_data_registry("WorldRule").ok_or(flags::Error::Missing("stored Construction WorldRule registry"))?;
        if rules.entries.iter().any(|r|r.ids.eq_ignore_ascii_case("AllowSellUnderSiege")){return required("stored AllowSellUnderSiege current WorldRule override")}
        self.catalog.settings.as_ref().ok_or(flags::Error::Missing("stored effective source Settings"))?.allow_sell_under_siege().map(|v|v.0).map_err(error)
    }
    fn settlement_upgrades_ready(&mut self,_:ActorId)->flags::Result<bool>{required("stored585F00 Settlement upgrade readiness")}
}
impl<S:Spatial> structure::World for Bound<'_,'_,'_,S>{
    fn structure_owner(&self,a:ActorId)->flags::Result<ActorId>{match self.component(a,Slot::Structure)?{Some(RuntimeComponent::Structure(v))if v.header.owner==a=>Ok(v.header.owner),_=>required("stored Structure owner/payload")}}
    fn structure_reference(&self,a:ActorId)->flags::Result<Option<structure::StructureReference>>{match self.component(a,Slot::Structure)?{
        None=>Ok(None),Some(RuntimeComponent::Structure(v))if v.header.owner==a=>Ok(Some(structure::StructureReference{owner:v.header.owner,parent:(v.object_handle_10!=0).then_some(ActorId::from_raw(v.object_handle_10))})),_=>required("stored Structure reference owner/payload"),
    }}
    fn settlement_reference(&self,a:ActorId)->flags::Result<Option<Option<ActorId>>>{self.settlement_reference(a)}
    fn action_bits(&self,a:ActorId)->flags::Result<u64>{Ok(flags::World::actor_fields(self,a)?.action_bits)}
    fn set_action_bits(&mut self,a:ActorId,b:u64)->flags::Result<()>{flags::World::set_action_bits(self,a,b)}
    fn queue_nonempty(&self,a:ActorId)->flags::Result<bool>{match self.component(a,Slot::Structure)?{Some(RuntimeComponent::Structure(v))if v.header.owner==a=>Ok(!v.queue_14.is_empty()),_=>required("stored Structure queue owner/payload")}}
    fn rebuild_actions(&mut self,a:ActorId)->flags::Result<()>{flags::rebuild(self,a)}
}
impl<S:Spatial> research::World for Bound<'_,'_,'_,S>{
    fn research_owner(&self,a:ActorId)->flags::Result<ActorId>{match self.component(a,Slot::Research)?{Some(RuntimeComponent::Research(v))if v.header.owner==a=>Ok(v.header.owner),_=>required("stored Research owner/payload")}}
    fn state_bits(&self,a:ActorId)->flags::Result<u64>{Ok(flags::World::actor_fields(self,a)?.state_bits)}
    fn state_type(&self,i:u8)->flags::Result<u32>{self.state_type(i)}
    fn research_items_present(&self,a:ActorId)->flags::Result<bool>{self.definition(a)?.ground.research_items.as_ref().map(|v|!v.is_empty()).ok_or(flags::Error::Missing("stored completed ActorData558 ResearchItems"))}
    fn action_bits(&self,a:ActorId)->flags::Result<u64>{Ok(flags::World::actor_fields(self,a)?.action_bits)}
    fn set_action_bits(&mut self,a:ActorId,b:u64)->flags::Result<()>{flags::World::set_action_bits(self,a,b)}
}
impl<S:Spatial> recruit::World for Bound<'_,'_,'_,S>{
    type Cursor=(ActorId,usize);type Definition=String;type Layout=Option<OrganizationComponentData>;
    fn recruit_owner(&self,a:ActorId)->flags::Result<ActorId>{match self.component(a,Slot::Recruit)?{Some(RuntimeComponent::Recruit(v))if v.header.owner==a=>Ok(v.header.owner),_=>required("stored Recruit owner/payload")}}
    fn fields(&self,a:ActorId)->flags::Result<flags::ActorFields>{flags::World::actor_fields(self,a)}
    fn set_action_bits(&mut self,a:ActorId,b:u64)->flags::Result<()>{flags::World::set_action_bits(self,a,b)}
    fn state_type(&self,i:u8)->flags::Result<u32>{self.state_type(i)}
    fn settlement_reference(&self,a:ActorId)->flags::Result<Option<Option<ActorId>>>{self.settlement_reference(a)}
    fn recruit_enabled(&self,a:ActorId,location:bool)->flags::Result<u8>{match self.component(a,Slot::Recruit)?{Some(RuntimeComponent::Recruit(v))if v.header.owner==a=>Ok(if location{v.byte_10}else{v.byte_11}),_=>required("stored Recruit enable owner/payload")}}
    fn recruit_list_head(&self,a:ActorId)->flags::Result<Option<Self::Cursor>>{Ok((!self.targets(a)?.is_empty()).then_some((a,0)))}
    fn recruit_entry(&self,c:&Self::Cursor)->flags::Result<recruit::Entry<String,Self::Layout>>{
        let id=self.targets(c.0)?.get(c.1).cloned().ok_or(flags::Error::Missing("stored Recruit native node ordinal"))?;
        let location_class=recruit_data::actor_transport_class(self.catalog.actors,&id).map_err(error)?;
        let data=self.catalog.layout_data(self.source_pc).map_err(error)?;
        let default_layout=data.organization_component(self.catalog.actors,&id,self.source_pc).map_err(error)?;
        Ok(recruit::Entry{definition:id,location_class,default_layout})
    }
    fn next_recruit_entry(&self,c:&Self::Cursor)->flags::Result<Option<Self::Cursor>>{
        let next=c.1.checked_add(1).ok_or(flags::Error::Missing("stored Recruit cursor overflow"))?;
        Ok((next<self.targets(c.0)?.len()).then_some((c.0,next)))
    }
    fn eligibility(&mut self,recruit:ActorId,definition:&String,layout:&Self::Layout)->flags::Result<u32>{
        crate::stored_recruit_eligibility::eligibility(self.catalog,self.actors,self.bank,recruit,definition,layout,self.source_pc)
    }
}
impl<S:Spatial> denizen::World for Bound<'_,'_,'_,S>{
    fn denizen_owner(&self,a:ActorId)->flags::Result<ActorId>{match self.component(a,Slot::Denizen)?{Some(RuntimeComponent::Denizen(v))if v.header.owner==a=>Ok(v.header.owner),_=>required("stored Denizen owner/payload")}}
    fn fields(&self,a:ActorId)->flags::Result<flags::ActorFields>{flags::World::actor_fields(self,a)}
    fn set_action_bits(&mut self,a:ActorId,b:u64)->flags::Result<()>{flags::World::set_action_bits(self,a,b)}
    fn denizen_byte(&self,a:ActorId,o:u32)->flags::Result<u8>{match self.component(a,Slot::Denizen)?{
        Some(RuntimeComponent::Denizen(v))if v.header.owner==a=>match o{0x20=>Ok(v.byte_20),0x21=>Ok(v.byte_21),0x38|0x3c=>v.words.get(&o).map(|v|*v as u8).ok_or(flags::Error::Missing("stored Denizen current byte uninitialized")),_=>required("stored Denizen byte offset")},_=>required("stored Denizen owner/payload"),
    }}
    fn settlement_present(&self,a:ActorId)->flags::Result<bool>{Ok(self.component(a,Slot::Settlement)?.is_some())}
    fn structure_parent(&self,a:ActorId)->flags::Result<Option<Option<ActorId>>>{self.structure_parent(a)}
    fn wall_reference(&self,a:ActorId)->flags::Result<Option<Option<ActorId>>>{self.wall_reference(a)}
}
impl<S:Spatial> Bound<'_,'_,'_,S>{
    fn mobile_source(&self,a:ActorId)->flags::Result<(std::sync::Arc<aa_rules::native_mobile_deployment::NativeMobileDeploymentData>,String)>{
        match self.component(a,Slot::MobileDeployment)?{Some(RuntimeComponent::MobileDeployment(v))if v.header.owner==a=>(),_=>return required("stored MobileDeployment owner/payload")}
        let id=self.definition(a)?.proof.id.clone();let data=self.catalog.native_mobile_deployment_data(self.source_pc).map_err(error)?;
        if !data.definition(&id).map_err(error)?.mobile{return required("stored MobileDeployment ActorData component")}
        Ok((data,id))
    }
}
impl<S:Spatial> mobile::World for Bound<'_,'_,'_,S>{
    fn fields(&self,a:ActorId)->flags::Result<flags::ActorFields>{flags::World::actor_fields(self,a)}
    fn set_action_bits(&mut self,a:ActorId,b:u64)->flags::Result<()>{flags::World::set_action_bits(self,a,b)}
    fn has_pack_reference(&self,a:ActorId)->flags::Result<bool>{let(d,id)=self.mobile_source(a)?;Ok(d.definition(&id).map_err(error)?.pack_to.is_some())}
    fn has_unpack_reference(&self,a:ActorId)->flags::Result<bool>{let(d,id)=self.mobile_source(a)?;Ok(d.definition(&id).map_err(error)?.unpack_to.is_some())}
    fn pack_health_min_bits(&self,a:ActorId)->flags::Result<u32>{let(d,id)=self.mobile_source(a)?;Ok(d.definition(&id).map_err(error)?.pack_health_min_bits)}
    fn properties_flags(&self,a:ActorId)->flags::Result<Option<u32>>{self.properties_flags(a)}
    fn body_health(&self,a:ActorId)->flags::Result<Option<mobile::BodyHealth>>{match self.component(a,Slot::Body)?{
        None=>Ok(None),Some(RuntimeComponent::Body(v))if v.header.owner==a=>Ok(Some(mobile::BodyHealth{health_bits:v.body.health.0,maximum_bits:v.body.maximum.0})),_=>required("stored Body owner/payload"),
    }}
    fn has_mind(&self,a:ActorId)->flags::Result<bool>{flags::World::has_component(self,a,Slot::Mind)}
}

struct ReadOnlySpatial<'a,S>(&'a S);
impl<S:Spatial> Spatial for ReadOnlySpatial<'_,S>{
    fn placement(&self,id:ActorId)->crate::actor_world::Result<crate::actor_world::PlacementState>{self.0.placement(id)}
    fn publish(&mut self,_:ActorId,_:crate::actor_world::PlacementState)->crate::actor_world::Result<()>{
        Err(crate::actor_world::Error::Missing("Mind action contribution cannot publish placement"))
    }
}
fn mind_error(e:impl std::fmt::Display)->aa_mind::Error{aa_mind::Error::World(e.to_string())}
fn mind_required<T>(s:&str)->aa_mind::Result<T>{Err(mind_error(s))}
struct MindServices<'a,'source>{source_pc:Precision,variables:Option<&'a crate::load_view_variables::State>,
    bank:&'a Bank,catalog:&'a Catalog<'source>}
impl<P:Spatial> crate::mind_actions::Services<P> for MindServices<'_,'_>{
    fn source_precision(&self,_:Precision)->aa_mind::Result<Precision>{
        let source=self.bank.combat_source().map_err(mind_error)?;
        if source.source_identities()!=self.catalog.actors.sources.as_slice(){return mind_required("stored Mind source Bank universe differs")}
        Ok(source.source_precision())
    }
    fn setting_cache108(&self)->aa_mind::Result<bool>{
        let v=self.variables.ok_or_else(||mind_error("stored Mind FogOfWar requires actual current World.variables"))?;
        v.validate_in_bank(self.catalog,self.bank).map_err(mind_error)?;
        v.require_published().map_err(mind_error)?;
        Ok(v.fog_enabled())
    }
    fn setting_cached8(&self)->aa_mind::Result<u32>{mind_required("stored Mind MoraleRetreatPenalty requires actual current SVar cell")}
    fn state_resupply(&self,_:u8)->aa_mind::Result<bool>{mind_required("Mind-only contribution cannot dispatch Organization resupply")}
    fn organization_member_44(&self,_:ActorId,_:&crate::actor_world::ActorAccess<'_>)->aa_mind::Result<Option<ActorId>>{mind_required("Mind-only contribution cannot dispatch Organization member44")}
    fn organization_groups(&self,_:ActorId,_:&crate::actor_world::ActorAccess<'_>)->aa_mind::Result<Vec<flags::AttackGroup>>{mind_required("Mind-only contribution cannot dispatch Organization layout groups")}
    fn owner_can_recruit(&mut self,_:ActorId,_:ActorId,_:&mut crate::mind_actions::Context<'_,'_,'_,P>)->aa_mind::Result<bool>{mind_required("Mind-only contribution cannot dispatch owner recruit eligibility")}
    fn owner_recruit_count(&mut self,_:ActorId,_:ActorId,_:&mut crate::mind_actions::Context<'_,'_,'_,P>)->aa_mind::Result<i32>{mind_required("Mind-only contribution cannot dispatch owner recruit count")}
    fn mobile_deployment(&mut self,a:ActorId,entry:u32,c:&mut crate::mind_actions::Context<'_,'_,'_,P>)->aa_mind::Result<bool>{
        let pc=c.precision;let fields=MindMobile{context:c,source_pc:self.source_pc};
        match entry{0x575bb0=>mobile::can_unpack(&fields,a,true).map_err(mind_error),
            0x575c00=>mobile::can_pack(&fields,a,true,pc).map_err(mind_error),
            _=>mind_required("stored Mind unexpected MobileDeployment predicate"),}
    }
    fn other_contributor(&mut self,_:ActorId,_:Slot,_:&mut crate::mind_actions::Context<'_,'_,'_,P>)->aa_mind::Result<()>{mind_required("Mind-only entry cannot dispatch another action contributor")}
    fn refresh_owner_views(&mut self,_:Option<ActorId>,_:&mut crate::mind_actions::Context<'_,'_,'_,P>)->aa_mind::Result<()>{mind_required("Mind-only contribution cannot publish owner views")}
    fn renderer_flags(&self,_:ActorId,_:&crate::actor_world::ActorAccess<'_>)->aa_mind::Result<Option<u32>>{mind_required("Mind-only contribution cannot publish renderer flags")}
    fn refresh_selected_view(&mut self,_:ActorId,_:&mut crate::mind_actions::Context<'_,'_,'_,P>)->aa_mind::Result<()>{mind_required("Mind-only contribution cannot publish selected views")}
}
struct MindMobile<'a,'b,'store,'source,P:Spatial>{context:&'a mut crate::mind_actions::Context<'b,'store,'source,P>,source_pc:Precision}
impl<P:Spatial> MindMobile<'_,'_,'_,'_,P>{
    fn component(&self,a:ActorId,s:Slot)->flags::Result<Option<&RuntimeComponent>>{self.context.actors.component(a,s).map_err(error)}
    fn source(&self,a:ActorId)->flags::Result<(std::sync::Arc<aa_rules::native_mobile_deployment::NativeMobileDeploymentData>,String)>{
        match self.component(a,Slot::MobileDeployment)?{Some(RuntimeComponent::MobileDeployment(v))if v.header.owner==a=>(),_=>return required("stored Mind MobileDeployment owner/payload")}
        let key=self.context.actors.key(a).ok_or(flags::Error::Missing("stored Mind MobileDeployment GTID"))?;
        let object=self.context.actors.object(key).map_err(error)?;
        if object.definition.native_class!=3{return required("stored Mind MobileDeployment ordinary Actor dispatch")}
        let id=object.definition.proof.id.clone();let data=self.context.catalog.native_mobile_deployment_data(self.source_pc).map_err(error)?;
        if !data.definition(&id).map_err(error)?.mobile{return required("stored Mind MobileDeployment source component")}
        Ok((data,id))
    }
}
impl<P:Spatial> mobile::World for MindMobile<'_,'_,'_,'_,P>{
    fn fields(&self,a:ActorId)->flags::Result<flags::ActorFields>{self.context.actors.action_fields(a,self.context.spatial).map_err(error)}
    fn set_action_bits(&mut self,a:ActorId,b:u64)->flags::Result<()>{self.context.actors.set_action_bits(a,b).map_err(error)}
    fn has_pack_reference(&self,a:ActorId)->flags::Result<bool>{let(d,id)=self.source(a)?;Ok(d.definition(&id).map_err(error)?.pack_to.is_some())}
    fn has_unpack_reference(&self,a:ActorId)->flags::Result<bool>{let(d,id)=self.source(a)?;Ok(d.definition(&id).map_err(error)?.unpack_to.is_some())}
    fn pack_health_min_bits(&self,a:ActorId)->flags::Result<u32>{let(d,id)=self.source(a)?;Ok(d.definition(&id).map_err(error)?.pack_health_min_bits)}
    fn properties_flags(&self,a:ActorId)->flags::Result<Option<u32>>{match self.component(a,Slot::Properties)?{
        None=>Ok(None),Some(RuntimeComponent::Properties(v))if v.owner.0==a.raw()=>Ok(Some(v.modifier.flags)),_=>required("stored Mind Properties owner/payload"),
    }}
    fn body_health(&self,a:ActorId)->flags::Result<Option<mobile::BodyHealth>>{match self.component(a,Slot::Body)?{
        None=>Ok(None),Some(RuntimeComponent::Body(v))if v.header.owner==a=>Ok(Some(mobile::BodyHealth{health_bits:v.body.health.0,maximum_bits:v.body.maximum.0})),_=>required("stored Mind Body owner/payload"),
    }}
    fn has_mind(&self,a:ActorId)->flags::Result<bool>{Ok(self.component(a,Slot::Mind)?.is_some())}
}
