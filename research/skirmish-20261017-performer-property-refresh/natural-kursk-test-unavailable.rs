//! Concrete57F1E0 with actual restored allocations and source requirements.
//! Positive attachment phases are explicit test inputs built in the real typed
//! allocation owner; this does not claim full581630 admission/consequences.
use aa_runtime::{catalog::Catalog,live_object_init::Record,load_performer_property_refresh::{self as gate,Context}};
use aa_actors::components::ComponentSlot;
use aa_mapworld::restore::{RestoredRecord,RestoredComponent};
use aa_numeric::Precision;
use aa_body::F32;
use serde_json::{Value,json};
#[path="../../aa-client/src/runtime_fp.rs"]mod actual_fp;
fn requirements(catalog:&Catalog<'_>,handle:u32,pc:Precision)->Option<Vec<aa_rules::abilities::DefinitionIdentity>>{
 let name={let arena=catalog.ability_arena.borrow();let token=arena.get(handle).ok()?.local_definition?;catalog.definition_bank.borrow().entry(token).ok()?.key.name.clone()};
 let(id,index)=name.rsplit_once("/ability/")?;let index=index.parse::<usize>().ok()?;
 let binding=catalog.actor_bindings(id,pc).ok()?;Some(binding.performer.as_ref()?.abilities().get(index)?.required_properties.clone())
}
fn enabled(catalog:&Catalog<'_>,handles:&[u32])->Vec<bool>{let arena=catalog.ability_arena.borrow();handles.iter().map(|h|arena.get(*h).unwrap().enabled).collect()}
#[test]
#[ignore="requires AA_GAME_ROOT original Kursk actual source owners and current thread CRT"]
fn actual_required_property_ability_bytes_use_same_allocations_all_pc(){
 use aa_runtime::{combat_source_completion as completion,combat_source_receipt::VerifiedStartupReceipt,localization::Localization,
  restoration::{Attempt,LoadBindings},restore_lifecycle::{ScopedManagedLoad,TaskKind},settings::Settings};
 use aa_mapworld::restore::RestoreMode;
 use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits,sha256},resources::ResourceCatalog,tgi::Limits,native::layout_data::PropertyRef};
 use std::sync::Arc;
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
 let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();let text=Localization::load_v14(&mut sources,Limits::default()).unwrap();
 let actors=ActorCatalog::load_v14(&mut sources).unwrap();let resources=Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
 let bytes=std::fs::read(game.join("reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP")).unwrap();let map=aa_mapworld::MapArchive::from_bytes(bytes.clone()).unwrap();
 let inactive=ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
 let mut attempt=Attempt::load_with_source_setup(&map,&actors,settings.clone(),text,Precision::Double53,u64::MAX,RestoreMode::InspectPartial,
  LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources.clone())},|catalog|{
   let proof=VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut sources)?;
   completion::install(catalog,&mut sources,proof.context(),proof.csv(),proof.globals())?;completion::compute(catalog)
  }).unwrap();
 let mut catalog=Catalog::new(&actors);catalog.settings=Some(settings);catalog.native_load_mode=Some(1);catalog.bind_resource_catalog(resources).unwrap();
 *catalog.definition_bank.borrow_mut()=attempt.definitions;*catalog.ability_arena.borrow_mut()=attempt.abilities;
 let pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
 let mut variables=aa_runtime::load_view_variables::State::reset_from_fresh_source(&catalog,aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
 let inventory=aa_mapworld::WorldInventory::decode(&map,u64::MAX).unwrap();let rules=aa_runtime::load_world_rules::Owner::load(&catalog,&inventory).unwrap();variables.publish_loaded_rules(&catalog,&rules).unwrap();
 let ids=attempt.registry.iter().filter_map(|entry|{
  let RestoredRecord::Actor(v)=&entry.value else{return None};let actor=v.shell.actor.as_ref()?;
  let Some(RestoredComponent::Performer(p))=actor.components.get(ComponentSlot::Performer)else{return None};
  let Some(RestoredComponent::Properties(props))=actor.components.get(ComponentSlot::Properties)else{return None};
  if !props.effects.is_empty(){return None}
  p.state.abilities.iter().any(|h|requirements(&catalog,*h,pc).is_some_and(|r|!r.is_empty())).then_some(entry.id)
 }).take(2).collect::<Vec<_>>();assert!(!ids.is_empty(),"actual restored required-property Performer needed");
 println!("actual required-property Performer candidates: {ids:?}");
 let mut properties=aa_runtime::load_property_instances::State::from_world_constructor();properties.bind_registered_sources(&catalog).unwrap();
 let mut rows=vec![];let mut changed=0;let mut successful=0;let mut allocated=0;
 for id in ids{
  let mut held=std::mem::replace(attempt.registry.value_mut(id).unwrap(),RestoredRecord::Loading);
  let RestoredRecord::Actor(entity)=&mut held else{unreachable!()};let load_failure=entity.failure.as_ref().map(|e|format!("{e:?}"));
  let mut record=Record{owner:id,shell:&mut entity.shell,definition:&entity.definition,registry:&mut attempt.registry};
  let(handles,address,before_value)=match record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Performer){Some(RestoredComponent::Performer(p))=>(p.state.abilities.clone(),p as *const _ as usize,p.word60),_=>unreachable!()};
  let before_enabled=enabled(&catalog,&handles);
  let source=handles.iter().map(|h|requirements(&catalog,*h,pc)).collect::<Vec<_>>();
  let mut tokens=vec![];
  for defs in source.iter().flatten(){for d in defs{
   let reference=catalog.layout_data(pc).unwrap().find_property(&d.id,None).unwrap();assert!(matches!(reference,PropertyRef::Registered(_)));
   let token=properties.definition_for_reference(reference).unwrap();if !tokens.contains(&token){tokens.push(token)}
  }}
  let created=F32(attempt.clock.begin.0);let mut added=vec![];
  for token in &tokens{
   let handle=properties.reserve_handle().unwrap();let instance=aa_body::property_effects::Instance::new(handle,*token,0,created,Some(id.raw()));
   assert!(properties.instances_mut().insert(handle,instance).is_none());added.push(handle);allocated+=1;
  }
  for phase in ["missing_actual_empty_effects","explicit_present_disabled_instances","explicit_present_enabled_instances"]{
   let present=phase!="missing_actual_empty_effects";
   if let Some(RestoredComponent::Properties(p))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Properties){assert!(p.effects.is_empty());if present{p.effects.extend(&added)}}else{unreachable!()}
   for handle in &added{properties.instances_mut().get_mut(handle).unwrap().disabled=phase=="explicit_present_disabled_instances";}
   for live_pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
    {let mut arena=catalog.ability_arena.borrow_mut();for (h,req)in handles.iter().zip(&source){if req.as_ref().is_some_and(|r|!r.is_empty()){arena.get_mut(*h).unwrap().enabled=!present}}}
    let before=enabled(&catalog,&handles);
    let result=gate::refresh(&catalog,&mut record,&mut Context{variables:&mut variables,properties:&properties,inventory:&inventory,current_environment:actual_fp::sample},live_pc);
    let after=enabled(&catalog,&handles);let mut expected=before.clone();
    for (i,req)in source.iter().enumerate(){if req.as_ref().is_some_and(|r|!r.is_empty()){expected[i]=present;}}
    assert_eq!(after,expected,"{id:?} {phase} {result:?}");assert_ne!(before,after);changed+=1;if result.is_ok(){successful+=1}
    let Some(RestoredComponent::Performer(p))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Performer)else{unreachable!()};assert_eq!(p as *const _ as usize,address);
    let values=source.iter().map(|defs|defs.as_ref().map(|defs|defs.iter().map(|d|{let reference=catalog.layout_data(pc).unwrap().find_property(&d.id,None).unwrap();let token=properties.definition_for_reference(reference).unwrap();tokens.iter().position(|t|*t==token).unwrap()}).collect::<Vec<_>>())).collect::<Vec<_>>();
    let native_input=json!({"abilities":values.iter().zip(&before).map(|(required,enabled)|json!({"required":required,"enabled":enabled})).collect::<Vec<_>>(),"properties":if present{tokens.iter().enumerate().map(|(i,_)|json!({"definition":i,"disabled":phase=="explicit_present_disabled_instances","source":id.raw()})).collect::<Vec<_>>()}else{vec![]}});
    rows.push(json!({"owner":id.raw(),"definition":record.definition.proof.id,"load_failure":load_failure,"phase":phase,"precision":format!("{live_pc:?}"),"input":native_input,"enabled_after":after,"result":result.map_err(|e|e.to_string()),"value":p.word60,"same_performer_allocation":true}));p.word60=before_value;
   }
   if let Some(RestoredComponent::Properties(p))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Properties){p.effects.clear();}
  }
  {let mut arena=catalog.ability_arena.borrow_mut();for(h,b)in handles.iter().zip(before_enabled){arena.get_mut(*h).unwrap().enabled=b;}}
  drop(record);*attempt.registry.value_mut(id).unwrap()=held;
 }
 assert!(changed>0&&allocated>0);
 let out=json!({"scope":"Actual diagnostic-restored owners/real source requirements and AbilityArena. Positive phase manually constructs real typed instances and attaches explicit membership as gate test input; not full581630 admission/ActorInit/battle. Full bound actions/value Result is retained, not coerced to success.","map_sha256":sha256(&bytes),"rows":rows,"changed_prefixes":changed,"successful_full_refreshes":successful,"constructed_instance_count":allocated});
 if let Some(path)=std::env::var_os("AA_PERFORMER_PROPERTY_CAPTURE"){serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&out).unwrap();}
 else{let expected:Value=serde_json::from_slice(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-performer-property-refresh/actual-property-refresh.json")).unwrap()).unwrap();assert_eq!(out,expected);}
}
