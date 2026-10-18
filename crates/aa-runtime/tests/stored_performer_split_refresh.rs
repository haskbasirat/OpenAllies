//! Same diagnostic-loaded allocations moved once, then paired ActorAccess/Arena
//! callbacks. This is a direct stored leaf regression, not natural startup.
use aa_runtime::{catalog::Catalog,restoration::{Attempt,LoadBindings},actor_world::RuntimeComponent,
 combat_source_receipt::VerifiedStartupReceipt,combat_source_completion as completion,
 settings::Settings,localization::Localization,restore_lifecycle::{ScopedManagedLoad,TaskKind}};
use aa_actors::components::ComponentSlot as Slot;
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
use serde_json::{json,Value};
#[path="../../aa-client/src/runtime_fp.rs"]mod actual_fp;
fn value(actors:&aa_runtime::actor_world::ActorStore,key:aa_runtime::actor_world::ObjectKey)->u32{match actors.object(key).unwrap().components().unwrap().get(Slot::Performer){Some(RuntimeComponent::Performer(v))=>v.word60,_=>panic!()}}
#[test]
#[ignore="requires AA_GAME_ROOT original tutorial02 and actual current thread FP environment"]
fn stored_and_split_performer_refresh_share_same_actual_owners_and_results(){
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
 let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();let text=Localization::load_v14(&mut sources,Limits::default()).unwrap();
 let source=ActorCatalog::load_v14(&mut sources).unwrap();let resources=std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
 let bytes=std::fs::read(game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP")).unwrap();let map=aa_mapworld::MapArchive::from_bytes(bytes.clone()).unwrap();
 let inactive=ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
 let inventory=aa_mapworld::WorldInventory::decode(&map,u64::MAX).unwrap();
 let mut variables=None;
 let attempt=Attempt::load_with_source_setup(&map,&source,settings.clone(),text,Precision::Double53,u64::MAX,aa_mapworld::restore::RestoreMode::InspectPartial,
  LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources.clone())},|catalog|{
   let receipt=VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut sources)?;
   completion::install(catalog,&mut sources,receipt.context(),receipt.csv(),receipt.globals())?;completion::compute(catalog)?;
   let mut current=aa_runtime::load_view_variables::State::reset_from_fresh_source(catalog,aa_runtime::load_view_variables::NativeOverrides::ControlledNone)?;
   let rules=aa_runtime::load_world_rules::Owner::load(catalog,&inventory)?;current.publish_loaded_rules(catalog,&rules)?;variables=Some(current);Ok(())
  }).unwrap();
 let mut staged=attempt.into_staged().unwrap();let mut catalog=Catalog::new(&source);catalog.settings=Some(settings);catalog.native_load_mode=Some(1);catalog.bind_resource_catalog(resources).unwrap();
 assert!(catalog.definition_bank.borrow().combat_source().is_err());let pc=staged.definitions.combat_source().unwrap().source_precision();assert_eq!(pc,Precision::Double53);

 let mut variables=variables.unwrap();

 let rows=staged.actors.identities().iter().filter_map(|e|{
  let object=staged.actors.object(e.value).ok()?;
  (matches!(object.definition.proof.id.as_str(),"usa_corps"|"usa_battleship")&&object.components().ok()?.present(Slot::Performer)).then_some((e.id,e.value))
 }).collect::<Vec<_>>();assert!(rows.iter().any(|(_,key)|staged.actors.object(*key).unwrap().definition.proof.id=="usa_battleship"));
 let mut output=vec![];
 for(owner,key)in rows{
  let object_address=staged.actors.object(key).unwrap()as *const _ as usize;
  let (component_address,handles)=match staged.actors.component(owner,Slot::Performer).unwrap(){Some(RuntimeComponent::Performer(p))=>(p as *const _ as usize,p.state.abilities.clone()),_=>panic!()};
  let ability_addresses=handles.iter().map(|h|&staged.actors.abilities[h]as *const _ as usize).collect::<Vec<_>>();
  let enabled=handles.iter().map(|h|staged.actors.abilities[h].enabled).collect::<Vec<_>>();
  for pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
   aa_runtime::stored_performer_refresh::refresh_on_owners(&catalog,&mut staged.actors,&mut staged.definitions,None,key,&mut variables,&staged.inventory,actual_fp::sample,pc).unwrap();
   let expected=value(&staged.actors,key);
   match staged.actors.component_mut(owner,Slot::Performer).unwrap(){Some(RuntimeComponent::Performer(p))=>p.word60=99f32.to_bits(),_=>panic!()}
   let (arena,mut actors)=staged.actors.split_minds();
   aa_runtime::stored_performer_refresh::refresh_on_split_owners(&catalog,&mut actors,arena,&mut staged.definitions,None,key,&mut variables,&staged.inventory,actual_fp::sample,pc).unwrap();
   drop(actors);
   assert_eq!(value(&staged.actors,key),expected);
   assert_eq!(staged.actors.object(key).unwrap()as *const _ as usize,object_address);
   assert_eq!(match staged.actors.component(owner,Slot::Performer).unwrap(){Some(RuntimeComponent::Performer(p))=>p as *const _ as usize,_=>panic!()},component_address);
   assert_eq!(handles.iter().map(|h|&staged.actors.abilities[h]as *const _ as usize).collect::<Vec<_>>(),ability_addresses);
   assert_eq!(handles.iter().map(|h|staged.actors.abilities[h].enabled).collect::<Vec<_>>(),enabled);
   output.push(json!({"owner":owner.raw(),"live_precision":pc,"source_precision":"Double53","abilities":handles.len(),"value":expected,"same_owners":true}));
  }
 }
 let out=json!({"scope":"same actual diagnostic tutorial02 stored Performer/Ability/Property State + paired Arena loans, current variables created once in source owner then retained. Both APIs use one field/gate kernel. Not natural fresh mission; actual rows do not force changed-enabled Actions.","rows":output});
 if let Some(path)=std::env::var_os("AA_SPLIT_PERFORMER_CAPTURE"){serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&out).unwrap();}
 else{let expected:Value=serde_json::from_slice(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-split-performer-refresh/actual-split.json")).unwrap()).unwrap();assert_eq!(out,expected);}
}
