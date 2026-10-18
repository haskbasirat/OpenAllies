//! Actual moved source Bank/components, direct leaf calls. No natural mission
//! or initialized Performer values are supplied by this diagnostic regression.
use aa_runtime::{catalog::Catalog,restoration::{Attempt,LoadBindings},actor_world::{ActorStore,ObjectKey,RuntimeComponent},
 combat_source_receipt::VerifiedStartupReceipt,combat_source_completion as completion,
 settings::Settings,localization::Localization,restore_lifecycle::{ScopedManagedLoad,TaskKind}};
use aa_actors::{ActorId,components::ComponentSlot as Slot};
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
use serde_json::{Value,json};
fn key(actors:&ActorStore,id:u32)->ObjectKey{actors.key(ActorId::from_raw(id)).unwrap()}
fn source_value(bank:&aa_runtime::definitions::Bank,id:&str)->u32{
 bank.combat_source().unwrap().completed_values(bank.token("ActorDefinition",id).unwrap()).unwrap().value_1e4.0
}
fn values(actors:&ActorStore,owner:ActorId,slot:Slot)->([u32;2],usize){match actors.component(owner,slot).unwrap(){
 Some(RuntimeComponent::Organization(v))=>([v.words[&0x68],v.words[&0x6c]],v.as_ref()as *const _ as usize),
 Some(RuntimeComponent::Denizen(v))=>([v.words[&0x10],v.words[&0x14]],v.as_ref()as *const _ as usize),_=>panic!(),
}}
#[test]
#[ignore="requires AA_GAME_ROOT original tutorial02 and completed source receipt"]
fn stored_denizen_and_real_hq_organization_keep_actual_boxes_members_sources_and_error_prefixes(){
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
 let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();let text=Localization::load_v14(&mut sources,Limits::default()).unwrap();
 let source=ActorCatalog::load_v14(&mut sources).unwrap();let resources=std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
 let bytes=std::fs::read(game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP")).unwrap();let map=aa_mapworld::MapArchive::from_bytes(bytes).unwrap();
 let inactive=ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
 let attempt=Attempt::load_with_source_setup(&map,&source,settings.clone(),text,Precision::Double53,u64::MAX,aa_mapworld::restore::RestoreMode::InspectPartial,
  LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources.clone())},|catalog|{
   let receipt=VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut sources)?;
   completion::install(catalog,&mut sources,receipt.context(),receipt.csv(),receipt.globals())?;completion::compute(catalog)
  }).unwrap();
 let mut staged=attempt.into_staged().unwrap();let mut catalog=Catalog::new(&source);catalog.settings=Some(settings);catalog.native_load_mode=Some(1);catalog.bind_resource_catalog(resources).unwrap();
 assert!(catalog.definition_bank.borrow().combat_source().is_err());
 let source_pc=staged.definitions.combat_source().unwrap().source_precision();assert_eq!(source_pc,Precision::Double53);
 let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-actor-value-refresh/actual-denizen-value.json")).unwrap();
 let denizen=ActorId::from_raw(fixture["owner"].as_u64().unwrap()as u32);let dk=key(&staged.actors,denizen.raw());
 let organization=ActorId::from_raw(fixture["organization"]["owner"].as_u64().unwrap()as u32);let ok=key(&staged.actors,organization.raw());
 assert_eq!(staged.actors.object(dk).unwrap().definition.proof.id,"usa_airfield");
 assert_eq!(staged.actors.object(ok).unwrap().definition.proof.id,"regiment_usa_hq_infantry_unpacked");
 let gbr=catalog.actor_bindings("gbr_corps",source_pc).unwrap();
 assert_eq!(gbr.definition.component_mask.unwrap()&(1<<Slot::Organization.index()),0,"gbr_corps is an HQ Actor, not an Organization");
 assert_ne!(staged.actors.object(ok).unwrap().definition.component_mask.unwrap()&(1<<Slot::Organization.index()),0);
 //Prove the same native-fixture inputs before using its expected outputs.
 let Some(RuntimeComponent::Denizen(d))=staged.actors.component(denizen,Slot::Denizen).unwrap()else{panic!()};
 let slots=d.inventory.iter().map(|s|{
  assert_eq!(s.word_00,0);let mask=catalog.canonical_component_mask(&s.target,source_pc).unwrap();
  let health=if mask&1!=0{catalog.actors.native_ground_definition(&s.target,source_pc).unwrap().attributes.body.unwrap().health.bits}else{0};
  let value=if mask&0x20!=0{source_value(&staged.definitions,&s.target)}else{0};
  json!({"occupant":Value::Null,"slot_value":s.value_08.0,"target":{"flags":mask,"health":health,"value":value}})
 }).collect::<Vec<_>>();assert_eq!(json!(slots),fixture["rows"][0]["input"]["slots"]);
 let Some(RuntimeComponent::Organization(o))=staged.actors.component(organization,Slot::Organization).unwrap()else{panic!()};
 let info=aa_mapworld::restore::RestoreDefinitions::organization_layout_info(&catalog,&o.layout.definition,source_pc).unwrap();
 let members=o.members.iter().map(|id|{
  if id.is_null(){return Value::Null}
  let performer=match staged.actors.component(*id,Slot::Performer).unwrap(){None=>Value::Null,Some(RuntimeComponent::Performer(p))=>{
   let body=match staged.actors.component(*id,Slot::Body).unwrap(){None=>Value::Null,Some(RuntimeComponent::Body(b))=>json!([b.body.health.0,b.body.maximum.0]),_=>panic!()};
   json!({"owner":p.state.owner,"value":p.word60,"body":body})},_=>panic!()};
  json!({"id":id.raw(),"performer":performer})
 }).collect::<Vec<_>>();
 let parameters=o.layout.parameters.iter().map(|p|p.as_ref().map(|id|{
  let mask=catalog.canonical_component_mask(id,source_pc).unwrap();
  json!({"id":id,"flags":mask,"value":if mask&0x20!=0{source_value(&staged.definitions,id)}else{0}})
 })).collect::<Vec<_>>();
 let expected_input=&fixture["organization"]["rows"][0]["input"];
 assert_eq!(json!(members),expected_input["members"]);assert_eq!(json!(parameters),expected_input["parameters"]);
 assert_eq!(json!(info.slot_groups),expected_input["groups"]);assert_eq!(json!(o.layout.definition),expected_input["layout"]);
 let object_addresses=[staged.actors.object(dk).unwrap()as *const _ as usize,staged.actors.object(ok).unwrap()as *const _ as usize];
 let component_addresses=[values(&staged.actors,denizen,Slot::Denizen).1,values(&staged.actors,organization,Slot::Organization).1];
 let mut rows=vec![];
 for pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
  aa_runtime::stored_actor_value_refresh::refresh_denizen(&catalog,&mut staged.actors,&staged.definitions,dk,pc).unwrap();
  aa_runtime::stored_actor_value_refresh::refresh_organization(&catalog,&mut staged.actors,&staged.definitions,ok,pc).unwrap();
  let denizen_values=values(&staged.actors,denizen,Slot::Denizen).0;let organization_values=values(&staged.actors,organization,Slot::Organization).0;
  let expected=fixture["rows"].as_array().unwrap().iter().find(|r|r["precision"]==format!("{pc:?}")).unwrap();
  assert_eq!(denizen_values,[expected["output"]["current"].as_u64().unwrap()as u32,expected["output"]["maximum"].as_u64().unwrap()as u32]);
  let expected=fixture["organization"]["rows"].as_array().unwrap().iter().find(|r|r["precision"]==format!("{pc:?}")).unwrap();
  assert_eq!(organization_values,[expected["output"]["current"].as_u64().unwrap()as u32,expected["output"]["maximum"].as_u64().unwrap()as u32]);
  //Borrowed paired entry uses the same canonical allocations without taking
  //the Arena. Constructor diagnostic boxes remain diagnostic boxes.
  let(_,mut access)=staged.actors.split_minds();
  aa_runtime::stored_actor_value_refresh::refresh_denizen_on_owners(&catalog,&mut access,&staged.definitions,dk,pc).unwrap();
  aa_runtime::stored_actor_value_refresh::refresh_organization_on_owners(&catalog,&mut access,&staged.definitions,ok,pc).unwrap();drop(access);
  assert_eq!(values(&staged.actors,denizen,Slot::Denizen),(denizen_values,component_addresses[0]));
  assert_eq!(values(&staged.actors,organization,Slot::Organization),(organization_values,component_addresses[1]));
  rows.push(json!({"live_precision":pc,"denizen":denizen_values,"organization":organization_values,"same_allocations":true}));
 }
 let old_target=match staged.actors.component_mut(denizen,Slot::Denizen).unwrap(){Some(RuntimeComponent::Denizen(d))=>std::mem::replace(&mut d.inventory[0].target,"missing_stored_target_fault_probe".into()),_=>panic!()};
 let denizen_error=aa_runtime::stored_actor_value_refresh::refresh_denizen(&catalog,&mut staged.actors,&staged.definitions,dk,Precision::Double53).unwrap_err();
 assert_eq!(values(&staged.actors,denizen,Slot::Denizen),([0,0],component_addresses[0]));
 match staged.actors.component_mut(denizen,Slot::Denizen).unwrap(){Some(RuntimeComponent::Denizen(d))=>d.inventory[0].target=old_target,_=>panic!()}
 let old_layout=match staged.actors.component_mut(organization,Slot::Organization).unwrap(){Some(RuntimeComponent::Organization(o))=>{assert!(o.members[0].is_null());std::mem::replace(&mut o.layout.definition,"missing_stored_layout_fault_probe".into())},_=>panic!()};
 let organization_error=aa_runtime::stored_actor_value_refresh::refresh_organization(&catalog,&mut staged.actors,&staged.definitions,ok,Precision::Double53).unwrap_err();
 assert_eq!(values(&staged.actors,organization,Slot::Organization),([0,0],component_addresses[1]));
 match staged.actors.component_mut(organization,Slot::Organization).unwrap(){Some(RuntimeComponent::Organization(o))=>o.layout.definition=old_layout,_=>panic!()}
 assert_eq!([staged.actors.object(dk).unwrap()as *const _ as usize,staged.actors.object(ok).unwrap()as *const _ as usize],object_addresses);
 assert!(catalog.definition_bank.borrow().combat_source().is_err());
 let output=json!({"scope":"Same diagnostic tutorial02 actual usa_airfield Denizen and regiment_usa_hq_infantry_unpacked Organization allocations moved once with source Bank. Native-fixture input equality and all three live PC outputs; Store/split entries and source/layout error prefixes. gbr_corps source factory explicitly lacks Organization. Not natural fresh initialization or battle.","rows":rows,"denizen_error":denizen_error.to_string(),"organization_error":organization_error.to_string()});
 if let Some(path)=std::env::var_os("AA_STORED_ACTOR_VALUE_CAPTURE"){serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&output).unwrap();}
 else{assert_eq!(output,serde_json::from_slice::<Value>(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-stored-actor-value-refresh/actual-stored-values.json")).unwrap()).unwrap());}
}
