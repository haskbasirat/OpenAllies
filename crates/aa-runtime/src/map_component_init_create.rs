//! Fresh component+4 dispatch on the same staged ActorStore and queue owners.
//! Leaves/ordering come from the native-backed component_virtuals table. A
//! positive specialized child is a required boundary, never a true placeholder.
use crate::{actor_world::{ObjectKey,RuntimeComponent},catalog::Catalog,restoration::Staged};
use aa_actors::{ActorId,components::ComponentSlot as Slot,component_virtuals};
use aa_numeric::Precision;
use std::sync::Arc;
type Result<T>=std::result::Result<T,String>;
fn err(e:impl std::fmt::Display)->String{e.to_string()}
pub fn initialize(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,slot:Slot,
    params:&crate::map_actor_creation::Params,_pc:Precision)->Result<bool>{
    let id=staged.actors.id(key).ok_or("fresh component InitCreate GTID absent")?;
    let object=staged.actors.object(key).map_err(err)?;
    let component=object.components().map_err(err)?.get(slot).ok_or("fresh component InitCreate actual slot absent")?;
    if component.slot()!=slot{return Err("fresh component InitCreate payload/slot differs".into());}
    let mask=object.definition.component_mask.ok_or("fresh component InitCreate actual mask absent")?;
    let kind=aa_actors::init_create::factory_kind(slot,mask).ok_or("fresh component InitCreate Mind class absent")?;
    let leaf=component_virtuals::leaves(kind).init_create;
    match leaf {
        //Native438AC0 reads no component/source fields and returns AL=1.
        0x438ac0=>Ok(true),
        0x563fd0=>{
            let owner=staged.owners.get(crate::scheduler_owners::Target::Component{object:key,slot:slot as u8}).map_err(err)?;
            staged.queues.add_periodic(1,owner,0,&mut staged.rng).map_err(err)?;
            Ok(true)
        }
        0x562310=>{
            if object.components().map_err(err)?.state.construction(){
                return Err("fresh562310 state21 positive561E50 health consequences required".into());
            }
            let supply=catalog.settings.as_ref().ok_or("fresh561B40 actual Settings owner absent")?
                .unit_supply_enabled().map_err(err)?.0;
            staged.actors.initialize_ordinary_body(key,Some(aa_actors::leaf_init::CreationRecord{
                word40:params.word_40,word48:params.word_48}),supply,&staged.owners,&mut staged.queues,&mut staged.rng).map_err(err)
        }
        0x57db90=>{
            //Both Performer and Defender use57DB90, which reads only the
            //current owner's Properties and581420(attribute0) on its list.
            if let Some(p)=object.components().map_err(err)?.get(Slot::Properties){
                let RuntimeComponent::Properties(p)=p else{return Err("fresh581420 actual Properties payload absent".into());};
                if p.owner.0!=id.raw(){return Err("fresh581420 Properties owner differs".into());}
                aa_body::property_refresh::require_empty(p).map_err(err)?;
            }
            Ok(true)
        }
        0x56f250=>staged.actors.initialize_ground_support(key,Slot::Economy,&staged.owners,&mut staged.queues,&mut staged.rng).map_err(err),
        0x568750=>crate::map_denizen_init_create::initialize(staged,catalog,key,params),
        0x575ac0=>{
            let RuntimeComponent::MobileDeployment(v)=component else{return Err("575AC0 actual MobileDeployment payload differs".into());};
            if v.header.owner!=id{return Err("575AC0 actual MobileDeployment owner differs".into());}
            //Native source+534 precedes both state guards.
            let binding=actual_binding(staged,catalog,key)?;
            let data=catalog.native_mobile_deployment_data(binding.precision)?;
            let forward=data.definition(&binding.definition.proof.id).map_err(err)?.unpack_to.is_some();
            let state=staged.actors.object(key).map_err(err)?.components().map_err(err)?.state;
            let enabled=!forward && state.low&(1<<26)==0 && state.low&(1<<25)==0;
            crate::map_actor_creation::set_state(&mut staged.actors,key,34,enabled)?;
            Ok(true)
        }
        0x574610=>hq(staged,catalog,key,id),
        //575AC0,574610 and other positive services retain their exact named
        //entry until their actual source/owner effects are attached.
        _=>Err(format!("fresh component{slot:?} InitCreate{leaf:x} concrete owner is required (class{kind:?})")),
    }
}
pub(crate) fn actual_binding(staged:&Staged,catalog:&Catalog<'_>,key:ObjectKey)->Result<Arc<crate::catalog::ActorBindings>>{
    catalog.require_source_owner()?;
    let binding=staged.actors.entity(key).map_err(err)?.bindings.as_ref().ok_or("fresh component actual source allocation absent")?;
    let bank=staged.definitions.combat_source()?;
    if bank.source_precision()!=binding.precision||bank.source_identities()!=catalog.actors.sources.as_slice(){return Err("fresh component actual source Bank differs".into());}
    let expected=catalog.actor_bindings(&binding.definition.proof.id,binding.precision)?;
    if !Arc::ptr_eq(binding,&expected){return Err("fresh component actual source Catalog allocation differs".into());}
    Ok(Arc::clone(binding))
}
fn kingdom(staged:&Staged,id:ActorId)->Result<&aa_mapworld::special_state::SpecialShell>{
    let key=staged.actors.key(id).ok_or("574610 current Kingdom GTID is stale")?;
    let entity=staged.actors.entity(key).map_err(err)?;
    if entity.runtime.as_ref().map(|r|r.definition.native_class)!=Some(4){return Err("574610 current owner is not actual Kingdom class4".into());}
    let crate::actor_world::Source::Special(s)=&entity.source else{return Err("574610 current Kingdom derived owner absent".into());};
    s.derived.as_ref().ok_or("574610 current Kingdom derived fields absent".into())
}
fn kingdom_mut(staged:&mut Staged,id:ActorId)->Result<&mut aa_mapworld::special_state::SpecialShell>{
    let key=staged.actors.key(id).ok_or("4E2640 current Kingdom GTID is stale")?;
    let entity=staged.actors.entity_mut(key).map_err(err)?;
    if entity.runtime.as_ref().map(|r|r.definition.native_class)!=Some(4){return Err("4E2640 current owner is not actual Kingdom class4".into());}
    let crate::actor_world::Source::Special(s)=&mut entity.source else{return Err("4E2640 current Kingdom derived owner absent".into());};
    s.derived.as_mut().ok_or("4E2640 current Kingdom derived fields absent".into())
}
fn nation(staged:&Staged,id:ActorId)->Result<Option<&aa_mapworld::restore::NamedDefinition>>{
    let s=kingdom(staged,id)?;
    match s.references.get(&0x234){Some(v)=>Ok(v.as_ref()),None if s.words.get(&0x234)==Some(&0)=>Ok(None),_=>Err("574610 actual Kingdom234 Nation is unbound".into())}
}
fn badge(staged:&Staged,key:ObjectKey,id:ActorId)->Result<i32>{
    match staged.actors.component(id,Slot::Hq).map_err(err)?{
        Some(RuntimeComponent::Hq(v)) if v.header.owner==id && staged.actors.key(id)==Some(key)=>Ok(v.badge_index),
        _=>Err("574610 actual HQ payload/owner differs".into()),
    }
}
fn hq(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,id:ActorId)->Result<bool>{
    //57462C captures current virtual108 owner once, before reading HQ+10.
    let owner=staged.actors.ordinary_owner(id).map_err(err)?;
    if badge(staged,key,id)?==-1 {
        if let Some(owner)=owner {
            if nation(staged,owner)?.is_some(){
                let index=allocate_badge(staged,catalog,owner)?;
                match staged.actors.component_mut(id,Slot::Hq).map_err(err)?{
                    Some(RuntimeComponent::Hq(v)) if v.header.owner==id=>v.badge_index=index,
                    _=>return Err("57465D current HQ payload differs after badge allocation".into()),
                }
            }
        }
    }
    if let Some(owner)=owner {
        if nation(staged,owner)?.is_some() && badge(staged,key,id)?==-1{
            return Err("57467F positive hq_error_no_badge CString/localization/GameEvent owner required".into());
        }
    }
    Ok(true)
}
fn allocate_badge(staged:&mut Staged,catalog:&Catalog<'_>,id:ActorId)->Result<i32>{
    catalog.require_source_owner()?;
    let bank=staged.definitions.combat_source()?;
    if bank.source_identities()!=catalog.actors.sources.as_slice(){return Err("4E2640 actual source Bank differs".into());}
    let pc=bank.source_precision();
    let n=nation(staged,id)?.ok_or("4E2640 actual Nation234 is null")?;
    let registry=catalog.actors.native_data_registry("Nation").ok_or("4E2640 actual Nation registry absent")?;
    let entry=registry.find(&n.id).ok_or("4E2640 canonical Nation identity absent")?;
    if !entry.ids.eq_ignore_ascii_case(&n.id)||!entry.reloads.is_empty(){return Err("4E2640 canonical Nation/reload identity unbound".into());}
    if let Some(index)=n.index{
        if registry.entries.get(index as usize).map(|e|e.ids.as_str())!=Some(entry.ids.as_str()){return Err("4E2640 Nation registration ordinal differs".into());}
    }
    let node=catalog.actors.native_data_node(entry).map_err(err)?;
    match node.field("IDS",pc).map_err(err)?{
        Some(aa_rules::native::TypedValue::String{value,..}) if value.eq_ignore_ascii_case(&entry.ids)=>(),
        _=>return Err("4E2640 native Nation stored IDS differs".into()),
    }
    let count=catalog.actors.native_nation_badges(&n.id,pc).and_then(|v|v.count()).map_err(err)?;
    if count as i32<=0{return Ok(-1);}
    //The same SyncA/SyncS/RNG getter, including failure before a random draw.
    let drawn=crate::configure_world::random_word(catalog,&mut staged.rng)?%count;
    let s=kingdom_mut(staged,id)?;
    let index=crate::configure_world_nations::choose_badge(count,&s.nation_badge_indices,drawn);
    if index!=-1{
        s.nation_badge_indices.try_reserve(1).map_err(err)?;
        s.nation_badge_indices.insert(0,index as u32);s.words.remove(&0x36c);s.words.remove(&0x370);
    }
    Ok(index)
}
