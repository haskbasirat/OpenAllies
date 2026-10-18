//!Pre-static Kingdom4F53D0/4E4130 destruction over the live restore owners.
//!No ordinary ActorStore/class3 substitution is admitted here.
use aa_actors::ActorId;
use aa_mapworld::restore::RestoredRecord;
use std::collections::BTreeMap;
use aa_actors::{IdentityRegistry,components::ComponentSlot,loading::LoadKind};
use aa_mapworld::{restore::{WorldEntityRuntime,RestoredComponent},special_state::SpecialShell};
use aa_numeric::Precision;
use crate::{catalog::Catalog,load_kingdom_init_world::KingdomRegistryState};
type Result<T>=std::result::Result<T,String>;

///Portable lifetime directory only. An allocation moves here exactly at native
///GTID retirement and remains owned through a later destructor error.
pub(crate) struct State {retired:BTreeMap<ActorId,RestoredRecord>}
impl State {pub(crate) fn new()->Self {Self {retired:BTreeMap::new()}}}

pub(crate) struct Context<'a> {
    pub(crate) state:&'a mut State,
    pub(crate) revision:&'a crate::world_revision::ConstructedWorldRevision,
    pub(crate) spatial:&'a mut crate::load_world::SpatialOwner,
    pub(crate) objects:&'a mut crate::world_object_list::WorldObjectList<ActorId>,
    pub(crate) source_fields:&'a aa_rules::native_source_requirements::NativeSourceRequirements,
    pub(crate) sai_map:&'a crate::load_sai_world::Map,
    pub(crate) presentation:&'a crate::fx_manager::State<crate::fx_graph::Command>,
    pub(crate) dispatcher:&'a Option<aa_world::DispatcherState>,
    pub(crate) updatable_lifetime:&'a crate::updatable_lifetime::State,
}
fn runtime(registry:&IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&WorldEntityRuntime> {
    match &registry.get(id).ok_or("Kingdom removal current GTID is stale")?.value {
        RestoredRecord::Kingdom(v) if v.source.serialized_id==id.raw()=>v.runtime.as_ref().ok_or("Kingdom removal actual runtime absent".into()),
        _=>Err("Kingdom removal requires the current registered Kingdom allocation".into()),
    }
}
fn runtime_mut(registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&mut WorldEntityRuntime> {
    match registry.value_mut(id).ok_or("Kingdom removal mutable GTID is stale")? {
        RestoredRecord::Kingdom(v) if v.source.serialized_id==id.raw()=>v.runtime.as_mut().ok_or("Kingdom removal actual runtime absent".into()),
        _=>Err("Kingdom removal requires actual mutable Kingdom allocation".into()),
    }
}
fn special(r:&WorldEntityRuntime)->Result<&SpecialShell> {r.derived.as_ref().ok_or("Kingdom removal derived storage absent".into())}
fn special_mut(r:&mut WorldEntityRuntime)->Result<&mut SpecialShell> {r.derived.as_mut().ok_or("Kingdom removal mutable derived storage absent".into())}
fn word(s:&SpecialShell,offset:u32)->Result<u32>{s.words.get(&offset).copied().ok_or_else(||format!("Kingdom removal actual derived+{offset:X} unbound"))}
fn actor_word(r:&WorldEntityRuntime,offset:u32)->Result<u32>{r.base.actor.as_ref().ok_or("Kingdom removal actual Actor absent")?.words.get(&offset).copied().ok_or_else(||format!("Kingdom removal actual Actor+{offset:X} unbound"))}
fn canonical(catalog:&Catalog<'_>,registry:&IdentityRegistry<RestoredRecord>,id:ActorId)->Result<()> {
    let r=runtime(registry,id)?;
    let source=crate::live_object_init::Source::bind(catalog,&r.definition).map_err(|e|e.to_string())?;
    if r.definition.native_class!=4 || r.base.kind!=LoadKind::Actor || r.base.object.is_none() || r.base.actor.is_none()
        || !crate::load_spatial::same_proof(&r.base.thing.definition,&r.definition.proof)
        || r.definition.component_mask!=Some(catalog.canonical_component_mask(&r.definition.proof.id,source.source_precision())?) {
        return Err("Kingdom removal canonical class4/source/constructor/mask differs".into());
    }
    special(r)?;Ok(())
}
impl Context<'_> {
    fn source_byte(&self,catalog:&Catalog<'_>,r:&WorldEntityRuntime,offset:u32)->Result<u8> {
        let source=crate::live_object_init::Source::bind(catalog,&r.definition).map_err(|e|e.to_string())?;
        let d=self.source_fields.definition(&r.definition.proof.id).map_err(|e|e.to_string())?;
        if self.source_fields.receipt().sources!=catalog.actors.sources || self.source_fields.source_precision()!=source.source_precision()
            || d.token().source()!=&r.definition.proof.source || d.native_kind()!=r.definition.proof.category.native_kind {
            return Err("Kingdom removal actual source field receipt/PC differs".into());
        }
        self.source_fields.byte(&r.definition.proof.id,offset).map_err(|e|e.to_string())
    }
    fn f0(&mut self,catalog:&Catalog<'_>,registry:&mut IdentityRegistry<RestoredRecord>,kingdoms:&mut KingdomRegistryState,id:ActorId,pc:Precision)->Result<()> {
        let s=special(runtime(registry,id)?)?;
        let major=word(s,0x18c)? as i32;
        if word(s,0x190)?==0 && major>=0 {
            if kingdoms.majors().get(major as usize)!=Some(&Some(id)) {return Err("4E4130 actual major index is not this Kingdom".into());}
            catalog.saved_kingdom_remove_sai(id,major)?;
        }
        crate::configure_world::clear_team_for_kingdom_removal(registry,id)?;
        if word(special(runtime(registry,id)?)?,0x184)?!=0 {return Err("50B250 actual Kingdom180 scheduled backlink cleanup required".into());}
        let lists=special(runtime(registry,id)?)?.actor_lists.as_ref().ok_or("4E4130 actual11 class lists absent")?;
        if lists.by_class.len()!=11 {return Err("4E4130 actual class-list layout differs".into());}
        for list in &lists.by_class {if !list.is_empty(){return Err("4E4130 actual current class-list head4F5490 destruction required".into());}}
        let s=special(runtime(registry,id)?)?;
        let color=match s.references.get(&0x1e8) {
            Some(Some(color))=>{
                let entries=&catalog.actors.native_data_registry("KingdomColor").ok_or("4F09C0 Color source registry absent")?.entries;
                let mut found=entries.iter().enumerate().filter(|(_,v)|v.ids.eq_ignore_ascii_case(&color.id));
                let (index,entry)=found.next().ok_or("4F09C0 actual Color identity absent")?;
                if found.next().is_some()||!entry.reloads.is_empty()||color.index.is_some_and(|v|v!=index as u32){return Err("4F09C0 actual Color ordinal ambiguous".into());}
                Some(u32::try_from(index).map_err(|e|e.to_string())?)
            }
            Some(None)=>None,
            None if s.words.get(&0x1e8)==Some(&0)=>None,
            _=>return Err("4F09C0 actual Color reference unbound".into()),
        };
        kingdoms.unregister_4f09c0(id,special_mut(runtime_mut(registry,id)?)?,color)?;
        catalog.load_session.ok_or("76C7C0 actual Session scope unbound")?
            .clear_kingdom_references_in_created_world(id,self.revision)?;
        let s=special_mut(runtime_mut(registry,id)?)?;
        let instances=s.vectors.get_mut(&0x318).ok_or("4E4130 actual318 instance vector absent")?;
        if instances.values.iter().any(|&v|v!=0){return Err("4E4130 positive318 instance allocations required".into());}
        instances.values.clear();
        crate::actor_uninit::run(&mut RestoringUninit {context:self,catalog,registry,id,pc})
    }
}
impl crate::configure_world::KingdomRemoval for Context<'_> {
    fn remove(&mut self,catalog:&Catalog<'_>,registry:&mut IdentityRegistry<RestoredRecord>,kingdoms:&mut KingdomRegistryState,
        registration:&mut crate::live_registration::State,id:ActorId,pc:Precision)->Result<()> {
        canonical(catalog,registry,id)?;
        if self.state.retired.contains_key(&id){return Err("Kingdom removal allocation already retired".into());}
        //4F5370: Kingdom inherits B0/B4 true and BC/C0 false. Source34 and
        //the reached City slot still come from the actual source/allocation.
        if self.source_byte(catalog,runtime(registry,id)?,0x34)?!=0 {registration.remove_available_city(id);}
        else if runtime(registry,id)?.base.actor.as_ref().unwrap().components.present(ComponentSlot::City) {
            return Err("4F4C40 positive Kingdom City/158 availability required".into());
        }
        let phase=self.dispatcher.as_ref().ok_or("4F53D0 actual WorldD8 owner absent")?.phase;
        if phase!=6 {
            let r=runtime(registry,id)?;
            struct Fields<'a>{r:&'a WorldEntityRuntime}
            impl crate::live_object_init::MemoryFields for Fields<'_> {
                fn phase(&mut self)->aa_actors::actor_creation::Result<u32>{Err(aa_actors::actor_creation::Error::Unsupported("4EA350 must not read SessionF8"))}
                fn mask(&mut self)->aa_actors::actor_creation::Result<u32>{self.r.definition.component_mask.ok_or(aa_actors::actor_creation::Error::Unsupported("4EA350 source174 absent"))}
                fn flags(&mut self)->aa_actors::actor_creation::Result<u32>{Ok(self.r.base.thing.flags)}
                fn announce(&mut self)->aa_actors::actor_creation::Result<u8>{Err(aa_actors::actor_creation::Error::Unsupported("4EA350 reached source16D service required"))}
            }
            if crate::live_object_init::potentially_visible(4,&mut Fields {r}).map_err(|e|e.to_string())? {
                return Err("4EC290 positive Kingdom memory service required".into());
            }
            if r.base.object.as_ref().unwrap().memory.count()>0 {return Err("4F53D0 retained Kingdom F0(1)/World110 path required".into());}
        }
        self.f0(catalog,registry,kingdoms,id,pc)?;
        if self.presentation.attached_view().is_some(){return Err("50C1B0 actual Kingdom attached-view unlink required".into());}
        self.e0(catalog,id)?;
        //The real scalar-deleting destructor completed. No live identity or
        //index owns this allocation now; only this directory can reclaim it.
        self.state.retired.remove(&id).ok_or("4E6BC0 completed allocation disappeared")?;
        Ok(())
    }
}
impl Context<'_> {
    fn e0(&mut self,catalog:&Catalog<'_>,id:ActorId)->Result<()> {
        let record=self.state.retired.get_mut(&id).ok_or("4E57C0 allocation has not reached4EFEC0")?;
        let RestoredRecord::Kingdom(record)=record else{return Err("4E57C0 retired allocation class differs".into())};
        let r=record.runtime.as_mut().ok_or("4E57C0 retired actual runtime absent")?;
        let s=r.derived.as_mut().ok_or("4E57C0 retired actual Kingdom storage absent")?;
        kingdom_owned_destructor_prefix(s)?;
        //50B7A0 occurs after all Kingdom fields, before the shared Actor dtor.
        let target=aa_mapworld::restore_components::UpdatableTarget::Kingdom {owner:id.raw()};
        let scheduler=crate::scheduler_owners::Target::Restoring(target);
        let mut owners=catalog.load_owners.try_borrow_mut().map_err(|e|e.to_string())?;
        let owner=match owners.get(scheduler){Ok(v)=>Some(v),Err(crate::scheduler_owners::Error::Missing)=>None,Err(e)=>return Err(e.to_string())};
        let mut queue=catalog.load_queues.try_borrow_mut().map_err(|e|e.to_string())?;
        let queue=queue.as_deref_mut().ok_or("50B7A0 actual queue owner absent")?;
        if let Some(owner)=owner {queue.cancel_owner(owner).map_err(|e|e.to_string())?;}
        if self.updatable_lifetime.saved_table_present() {
            let mut table=catalog.updatables.try_borrow_mut().map_err(|e|e.to_string())?;
            let table=table.as_mut().ok_or("50B7A0 actual saved table absent")?;
            self.updatable_lifetime.require_live_table_count(table.len())?;
            table.remove(&target);
        }
        if self.updatable_lifetime.save_hash_present(){return Err("50B7A0 positive save-enumeration hash removal required".into());}
        if owner.is_some(){owners.retire(scheduler,queue).map_err(|e|e.to_string())?;}
        crate::map_replacement_drain::actor_owned_destructor(
            r.base.actor.as_mut().ok_or("4D1B80 actual Actor allocation absent")?,
            &r.base.object.as_ref().ok_or("4D1B80 actual Object allocation absent")?.memory,
            &mut r.base.thing.name_utf16,self.sai_map.lifecycle())
    }
}
///4E57C0 owned-field release up to the embedded Updatable. Native frees do
///not dispatch gameplay through these empty/native-owned containers. Positive
///unbound pointees stop at their exact reached ownership boundary.
fn kingdom_owned_destructor_prefix(s:&mut SpecialShell)->Result<()> {
    fn scalar_zero(s:&SpecialShell,off:u32)->Result<()> {
        if word(s,off)?!=0 {Err(format!("4E57C0 positive raw Kingdom{off:X} allocation owner required"))}else{Ok(())}
    }
    fn vector(s:&mut SpecialShell,off:u32,owned_pointees:bool)->Result<()> {
        let v=s.vectors.get_mut(&off).ok_or_else(||format!("4E57C0 actual vector{off:X} absent"))?;
        if owned_pointees&&v.values.iter().any(|&p|p!=0){return Err(format!("4E57C0 positive vector{off:X} pointee release required"));}
        v.values.clear();v.capacity_hint=0;Ok(())
    }
    fn string(s:&mut SpecialShell,off:u32)->Result<()> {
        s.strings_utf16.get_mut(&off).ok_or_else(||format!("4E57C0 actual CString{off:X} absent"))?.clear();Ok(())
    }
    //300 pointees are freed first; the backing vector survives until later.
    {let v=s.vectors.get_mut(&0x300).ok_or("4E57C0 actual300 vector absent")?;
     if v.values.iter().any(|&p|p!=0){return Err("4E57C0 positive300 record allocation release required".into());}v.values.clear();}
    scalar_zero(s,0x338)?;
    scalar_zero(s,0x32c)?;
    if let Some(strategic)=s.strategic.take(){drop(strategic);}else{scalar_zero(s,0x330)?;}
    scalar_zero(s,0x340)?;
    if let Some(tactical)=s.tactical.as_mut() {
        //96BE33 destroys handle tables4..0, then definition tables4..0.
        //4A6AD0 only frees linked nodes; keys/values are borrowed identities.
        for table in tactical.handles.iter_mut().rev(){table.count=0;table.buckets.clear();}
        for table in tactical.definitions.iter_mut().rev(){table.count=0;table.buckets.clear();}
        s.tactical=None;
    }else{scalar_zero(s,0x344)?;}
    {let v=s.vectors.get_mut(&0x34c).ok_or("4E57C0 actual34C vector absent")?;
     if v.values.iter().any(|&p|p!=0){return Err("4E3ED0 positive34C record release required".into());}v.values.clear();}
    if let Some(commander)=s.commander.as_mut() {
        //4CE310 never dereferences summoned IDs and never destroys Actors.
        for ability in &mut commander.abilities {ability.summoned_ids.clear();ability.word28=0;}
        commander.abilities.clear();s.commander=None;
    }else{scalar_zero(s,0x230)?;}
    s.nation_badge_indices.clear();s.definition_counts=None;
    vector(s,0x34c,true)?;string(s,0x348)?;
    for off in [0x338,0x324]{scalar_zero(s,off)?;}
    vector(s,0x318,true)?;
    //4E59xx frees only30C list nodes, never the borrowed Actor values.
    s.actor_lists.as_mut().ok_or("4E57C0 actual Kingdom30C list owner absent")?.city_ownership.clear();
    vector(s,0x300,true)?;
    let lists=s.actor_lists.as_mut().ok_or("4E57C0 actual class/flare lists absent")?;
    lists.flares.clear();
    for off in [0x2e8,0x2dc,0x2d0,0x2c4]{vector(s,off,false)?;}
    let lists=s.actor_lists.as_mut().unwrap();
    if lists.by_class.len()!=11{return Err("4E57C0 actual11 class lists differ".into());}
    for list in lists.by_class.iter_mut().rev(){list.clear();}
    s.supply_sources.clear();
    for off in [0x258,0x24c]{vector(s,off,false)?;}
    string(s,0x248)?;string(s,0x240)?;
    //4E29B0 frees table VALUE allocations directly, then hash nodes and
    //singles backing storage; there are no RecruitAi/removal callbacks.
    s.recruit_items.clear();
    s.indexed_words.get_mut(&0x204).ok_or("4E29B0 actual recruitment buckets absent")?.clear();
    vector(s,0x214,false)?;
    for off in [0x1dc,0x1d0,0x1c4,0x1b8,0x1ac,0x1a0]{vector(s,off,false)?;}
    string(s,0x198)?;string(s,0x194)?;
    Ok(())
}
struct RestoringUninit<'a,'world,'source> {
    context:&'a mut Context<'world>,catalog:&'a Catalog<'source>,registry:&'a mut IdentityRegistry<RestoredRecord>,id:ActorId,pc:Precision,
}
impl RestoringUninit<'_,'_,'_> {
    fn actor(&self)->Result<&aa_actors::loading::ActorShell<RestoredComponent>> {runtime(self.registry,self.id)?.base.actor.as_ref().ok_or("Kingdom actual Actor absent".into())}
    fn require_zero(&self,offset:u32,name:&str)->Result<()> {if actor_word(runtime(self.registry,self.id)?,offset)?!=0 {Err(format!("4D7580 positive{name} required"))}else{Ok(())}}
    fn require_not_live(&self,offset:u32,name:&str)->Result<()> {
        let other=ActorId::from_raw(actor_word(runtime(self.registry,self.id)?,offset)?);
        if self.registry.get(other).is_some(){Err(format!("4D7580 live{name} required"))}else{Ok(())}
    }
}
impl crate::actor_uninit::World for RestoringUninit<'_,'_,'_> {
    fn or_flags(&mut self,bits:u32)->Result<()> {runtime_mut(self.registry,self.id)?.base.thing.flags|=bits;Ok(())}
    fn timed_list(&mut self)->Result<()> {self.require_zero(0x130,"timed-list4D37E0")}
    fn allocation_144(&mut self)->Result<()> {self.require_zero(0x144,"144allocation")}
    fn observer_transition(&mut self)->Result<()> {if self.catalog.saved_observer_active()? {Err("4D7580 active7A0A40 transition required".into())}else{Ok(())}}
    fn set_category_140(&mut self)->Result<()> {*runtime_mut(self.registry,self.id)?.base.actor.as_mut().unwrap().words.get_mut(&0x140).ok_or("4D7580 actual140 absent")?=5;Ok(())}
    fn companion_158(&mut self)->Result<()> {self.require_not_live(0x158,"158companion12C")}
    fn zones(&mut self)->Result<()> {
        for zone in self.actor()?.zones_148.zones().slots.iter().flatten(){if zone.radius_bits!=aa_zones::zone::INACTIVE {return Err("4D7580 actual514CE0 activeZone uninit required".into());}}
        Ok(())
    }
    fn state_bit25(&mut self)->Result<bool>{Ok(self.actor()?.components.state.low&(1<<25)!=0)}
    fn linked_160(&mut self)->Result<()> {self.require_zero(0x160,"160owner4D19F0")}
    fn linked_164(&mut self)->Result<()> {self.require_not_live(0x164,"164owner4DFF30")}
    fn mind(&mut self)->Result<()> {if self.actor()?.components.present(ComponentSlot::Mind){Err("4D7580 actual Kingdom Mind unlink required".into())}else{Ok(())}}
    fn element(&mut self)->Result<()> {if self.actor()?.components.present(ComponentSlot::Element){Err("4D7580 actual Kingdom Element unlink required".into())}else{Ok(())}}
    fn owner(&mut self)->Result<()> {self.require_zero(0xe8,"E8Kingdom owner4E44E0")}
    fn linked_168(&mut self)->Result<()> {self.require_not_live(0x168,"168actor4F5490")}
    fn structure(&mut self)->Result<()> {if self.actor()?.components.present(ComponentSlot::Structure){Err("4D7580 actual Kingdom Structure589CC0 required".into())}else{Ok(())}}
    fn component(&mut self,slot:ComponentSlot)->Result<()> {
        if !self.actor()?.components.present(slot){return Ok(());}
        if slot!=ComponentSlot::Properties{return Err(format!("4CF390 actual Kingdom{slot:?} Uninit/destructor required"));}
        let target=aa_mapworld::restore_components::UpdatableTarget::Component {owner:self.id.raw(),slot:slot as u8};
        let scheduler=crate::scheduler_owners::Target::Restoring(target.clone());
        let mut owners=self.catalog.load_owners.try_borrow_mut().map_err(|e|e.to_string())?;
        let owner=match owners.get(scheduler){Ok(v)=>Some(v),Err(crate::scheduler_owners::Error::Missing)=>None,Err(e)=>return Err(e.to_string())};
        let mut queue=self.catalog.load_queues.try_borrow_mut().map_err(|e|e.to_string())?;
        let queue=queue.as_deref_mut().ok_or("5812A0 actual load queue owner absent")?;
        let mut table=self.catalog.updatables.try_borrow_mut().map_err(|e|e.to_string())?;
        let table=table.as_mut().ok_or("50B7A0 actual saved Updatable table absent")?;
        let actor=runtime_mut(self.registry,self.id)?.base.actor.as_mut().unwrap();
        let Some(RestoredComponent::Properties(properties))=actor.components.get_mut(slot) else{return Err("Kingdom actual Properties slot class differs".into())};
        if properties.owner.0!=self.id.raw(){return Err("Kingdom actual Properties owner differs".into());}
        let lifetime=self.context.updatable_lifetime;
        crate::map_replacement_drain::properties_owned_cleanup(properties,owner,scheduler,&mut owners,queue,lifetime,||{
            lifetime.require_live_table_count(table.len())?;table.remove(&target);Ok(())
        })?;
        actor.components.remove(slot);Ok(())
    }
    fn base_object(&mut self)->Result<()> {
        let r=runtime(self.registry,self.id)?;
        self.context.objects.remove_first(self.id,r.base.thing.flags).map_err(|e|e.to_string())?;
        let object=r.base.object.as_ref().ok_or("4EA6F0 actual Object absent")?;
        if self.registry.get(ActorId::from_raw(object.words_50_to_54[0])).is_some(){return Err("4EA6F0 actual liveObject50 recursive removal required".into());}
        if (object.byte_4c as i8)>0{return Err("4EA6F0 actual signedObject4C zone-grid remove required".into());}
        if r.base.thing.flags&2==0{return Err("4EA6F0 actual destroy FX42A640 required".into());}
        let crate::load_world::SpatialOwner::Loading(Some(load))=&mut self.context.spatial else{return Err("4EE560 pre-static sameLoadSpatial owner absent".into());};
        let r=runtime_mut(self.registry,self.id)?;
        load.detach_for_destruction(self.catalog,self.id,&mut r.base,&r.definition,self.pc).map_err(|e|e.to_string())?;
        //Actual4EFEC0 point. The same allocation remains in a persistent
        //directory until E0 completes; a later failure never drops it.
        if self.context.state.retired.contains_key(&self.id){return Err("4EFEC0 duplicate retired allocation".into());}
        let retired=self.registry.remove(self.id).map_err(|e|e.to_string())?;
        self.context.state.retired.insert(self.id,retired.value);
        Ok(())
    }
}

///Read-only diagnostic of the actual stopped graph. Debug text preserves
///typed owned state without constructing another mutable runtime owner.
pub fn diagnostic_snapshot(attempt:&crate::restoration::Attempt,id:ActorId)->Result<serde_json::Value> {
    let world=attempt.load_world.as_ref().ok_or("Kingdom removal actual World absent")?;
    let current=attempt.registry.get(id).map(|v|&v.value);
    let retired=world.kingdom_removal.retired.get(&id);
    let Some(record)=current.or(retired) else {
        return Ok(serde_json::json!({"id":id.raw(),"registered":false,"retained_retired":false,
            "world_kingdoms":&world.kingdoms,"world_registration":format!("{:?}",world.registration),
            "actual_world48":world.mission_fields.started48(),"actual_mode44":world.mission_fields.mode44(),
            "retired_kingdoms":world.kingdom_removal.retired.keys().map(|id|id.raw()).collect::<Vec<_>>()}));
    };
    let RestoredRecord::Kingdom(kingdom)=record else{return Err("Kingdom removal snapshot needs actual class4".into())};
    let runtime=kingdom.runtime.as_ref().ok_or("Kingdom removal actual allocation absent")?;
    let shell=runtime.derived.as_ref().ok_or("Kingdom removal actual derived allocation absent")?;
    let team=shell.identity_references.get(&0x1ec).and_then(|v|*v);
    let team_debug=team.map(|team|attempt.registry.get(team).map(|v|format!("{:?}",v.value)));
    let spatial=match &world.spatial {
        crate::load_world::SpatialOwner::Loading(Some(load))=>format!("{:?}",load.bindings().iter().filter(|v|v.id==id).collect::<Vec<_>>()),
        crate::load_world::SpatialOwner::Loading(None)=>"loading owner absent".into(),
        crate::load_world::SpatialOwner::Adopted(_)=>"already adopted".into(),
    };
    Ok(serde_json::json!({"id":id.raw(),"registered":current.is_some(),"retained_retired":retired.is_some(),"base":format!("{:?}",runtime.base),"definition":format!("{:?}",runtime.definition),
        "derived":format!("{shell:?}"),"team":team.map(ActorId::raw),"team_record":team_debug,
        "world_kingdoms":&world.kingdoms,"world_registration":format!("{:?}",world.registration),
        "world_objects":format!("{:?}",world.objects.nodes()),"load_spatial":spatial,
        "sai_map":format!("{:?}",world.sai_map.lifecycle()),"map_mode":world.map_mode.as_ref(),
        "saved_world48":attempt.inventory.world.word_48,"actual_world48":world.mission_fields.started48(),
        "actual_mode44":world.mission_fields.mode44(),"observer_active":attempt.inactive_load.as_ref().map(|v|v.observer_active()),
        "queues":attempt.queues.snapshot(),"retired_kingdoms":world.kingdom_removal.retired.keys().map(|id|id.raw()).collect::<Vec<_>>()}))
}

#[cfg(test)]
#[path="configure_kingdom_removal_tests.rs"]
mod tests;
