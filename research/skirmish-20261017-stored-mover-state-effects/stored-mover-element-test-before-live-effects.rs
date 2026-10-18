//! Source component-constructor lab in one diagnostic-loaded/moved world.
//! It does not bypass or claim the complete Actor assembly/mission startup.
use aa_runtime::{catalog::Catalog,restoration::{Attempt,LoadBindings},actor_world::RuntimeComponent,
 combat_source_receipt::VerifiedStartupReceipt,combat_source_completion as completion,
 settings::Settings,localization::Localization,restore_lifecycle::{ScopedManagedLoad,TaskKind},scheduler_owners::Target};
use aa_actors::{ActorId,components::ComponentSlot as Slot,creation::{ActorCreationRecord,CreationRecord,ThingDefinition}};
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
use serde_json::{Value,json};
#[test]
#[ignore="requires AA_GAME_ROOT; actual component-constructor lab, not full Actor/mission initialization"]
fn actual_stored_mover_element_keep_sources_allocations_queues_and_error_prefix(){
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
 let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();let text=Localization::load_v14(&mut sources,Limits::default()).unwrap();
 let source=ActorCatalog::load_v14(&mut sources).unwrap();let resources=std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
 let bytes=std::fs::read(game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP")).unwrap();let map=aa_mapworld::MapArchive::from_bytes(bytes.clone()).unwrap();
 let inactive=ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
 let attempt=Attempt::load_with_source_setup(&map,&source,settings.clone(),text,Precision::Double53,u64::MAX,aa_mapworld::restore::RestoreMode::InspectPartial,
  LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources.clone())},|catalog|{
   let receipt=VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut sources)?;
   completion::install(catalog,&mut sources,receipt.context(),receipt.csv(),receipt.globals())?;completion::compute(catalog)
  }).unwrap();
 let mut staged=attempt.into_staged().unwrap();let mut catalog=Catalog::new(&source);catalog.settings=Some(settings);catalog.native_load_mode=Some(1);catalog.bind_resource_catalog(resources).unwrap();
 assert!(catalog.definition_bank.borrow().combat_source().is_err());let pc=staged.definitions.combat_source().unwrap().source_precision();assert_eq!(pc,Precision::Double53);

 let live=Precision::Double53;let source_pc=staged.definitions.combat_source().unwrap().source_precision();
 let binding=catalog.actor_bindings("usa_infantry",source_pc).unwrap();let mask=binding.definition.component_mask.unwrap();
 assert_ne!(mask&(1<<Slot::Mover.index()),0);assert_ne!(mask&(1<<Slot::Element.index()),0);
 let key=staged.actors.construct_new_bound(&catalog,binding.clone(),ActorCreationRecord{base:CreationRecord{
  definition:ThingDefinition{key:binding.definition.proof.id.clone(),flags:binding.definition.flags},
  position_bits:[0;3],direction_bits:[0,(-1f32).to_bits()],name_utf16:vec![],saved_id:ActorId::NULL},
  variant:0,owner_word:0,word_30:0,request_nonblocking:false},staged.clock.begin.0,&mut staged.rng,source_pc,live).unwrap();
 let owner=staged.actors.initialize_object(key,&mut staged.rng,live).unwrap();
 //This explicit laboratory invokes the real component constructor, then the
 //same owner-label registration used by GroundAssembly. Other source slots
 //are deliberately unassembled and no full Actor Init success is claimed.
 for slot in [Slot::Mover,Slot::Element,Slot::Properties]{
  if mask&(1<<slot.index())!=0{
   staged.actors.construct_ground(key,slot,aa_body::F32(staged.clock.begin.0),live).unwrap();
   staged.owners.register(Target::Component{object:key,slot:slot.index()as u8}).unwrap();
  }
 }
 let object_address=staged.actors.object(key).unwrap()as *const _ as usize;
 let mover_address=match staged.actors.component(owner,Slot::Mover).unwrap(){Some(RuntimeComponent::Mover(v))=>&**v as *const _ as usize,_=>panic!()};
 let element_address=match staged.actors.component(owner,Slot::Element).unwrap(){Some(RuntimeComponent::Element(v))=>&**v as *const _ as usize,_=>panic!()};
 let mover_owner=staged.owners.get(Target::Component{object:key,slot:3}).unwrap();
 let element_owner=staged.owners.get(Target::Component{object:key,slot:8}).unwrap();
 let state_initial=staged.actors.object(key).unwrap().components().unwrap().state;
 let flags_initial=staged.actors.staged_placement(key).unwrap().flags;
 let version_initial=staged.report.world_version;let mut rows=vec![];
 assert!(version_initial>=22);let seed=staged.rng.state;
 assert!(aa_runtime::stored_mover_init_world::init_existing(&catalog,&mut staged,key,live).unwrap());
 assert_eq!(seed,staged.rng.state);assert!(staged.queues.owner_head(mover_owner).is_none());
 assert_eq!(staged.actors.object(key).unwrap().components().unwrap().state,state_initial);
 rows.push(json!({"phase":"genuine fresh Mover+56zero","rng_before":seed,"rng_after":staged.rng.state,"complete":true}));
 assert!(aa_runtime::stored_element_init_world::init_existing(&catalog,&mut staged,key,live).unwrap());
 assert_eq!(seed,staged.rng.state);assert!(staged.queues.owner_head(element_owner).is_none());
 let source_value=aa_rules::native::actor_morale::actor_morale_source(&source,"usa_infantry",source_pc).unwrap().resupply_rate.unwrap().bits;
 let rate=match staged.actors.component(owner,Slot::Element).unwrap(){Some(RuntimeComponent::Element(v))=>v.word_20,_=>panic!()};
 assert_eq!(rate,source_value);rows.push(json!({"phase":"genuine fresh Element currentversion","word20":rate,"complete":true}));
 //Explicit old-version input probes use the same retained word and queues.
 //They are not assertions that this map naturally starts at WorldVersion21.
 staged.report.world_version=21;
 let mut expected=staged.rng.clone();expected.next_u32();
 assert!(aa_runtime::stored_element_init_world::init_existing(&catalog,&mut staged,key,live).unwrap());
 assert_eq!(staged.rng.state,expected.state);assert!(staged.queues.has(element_owner,1,0));
 let seed=staged.rng.state;assert!(aa_runtime::stored_element_init_world::init_existing(&catalog,&mut staged,key,live).unwrap());
 assert_eq!(staged.rng.state,seed);rows.push(json!({"phase":"explicit WorldVersion21 Element actualqueue1 and duplicate-head skip","word20":rate,"rng_after":seed,"complete":true}));
 if let Some(RuntimeComponent::Mover(v))=staged.actors.component_mut(owner,Slot::Mover).unwrap(){v.bytes.insert(0x56,1);}
 staged.actors.staged_placement_mut(key).unwrap().flags=flags_initial|0x80000000;
 let seed=staged.rng.state;let mut expected=staged.rng.clone();expected.next_u32();
 assert!(aa_runtime::stored_mover_init_world::init_existing(&catalog,&mut staged,key,live).unwrap());
 assert_eq!(staged.rng.state,expected.state);assert!(staged.queues.has(mover_owner,0,0));
 assert_eq!(staged.actors.object(key).unwrap().components().unwrap().state.low,state_initial.low|(1<<24));
 assert_eq!(staged.actors.staged_placement(key).unwrap().flags,flags_initial|0x80001000);
 let seed_after=staged.rng.state;assert!(aa_runtime::stored_mover_init_world::init_existing(&catalog,&mut staged,key,live).unwrap());
 assert_eq!(staged.rng.state,seed_after);
 rows.push(json!({"phase":"explicit falling+flag31 oldversion actualqueue0 and state24 suffix","rng_before":seed,"rng_after":seed_after,"flags":staged.actors.staged_placement(key).unwrap().flags,"complete":true}));
 //A live state-entry refusal retains the preceding state24 bit, and must not
 //commit the later bit12 flag suffix. The already-existing queue stays intact.
 staged.report.world_version=22;
 staged.actors.object_mut(key).unwrap().components_mut().unwrap().state=state_initial;
 staged.actors.staged_placement_mut(key).unwrap().flags=flags_initial&!0x80001000;
 let seed=staged.rng.state;let error=aa_runtime::stored_mover_init_world::init_existing(&catalog,&mut staged,key,live).unwrap_err();
 assert!(error.to_string().contains("4D4A90"));assert_eq!(staged.rng.state,seed);
 assert_eq!(staged.actors.object(key).unwrap().components().unwrap().state.low,state_initial.low|(1<<24));
 assert_eq!(staged.actors.staged_placement(key).unwrap().flags,flags_initial&!0x80001000);
 rows.push(json!({"phase":"explicit live falling required4D4A90 prefix","state_low":staged.actors.object(key).unwrap().components().unwrap().state.low,"flags":staged.actors.staged_placement(key).unwrap().flags,"required":"4D4A90"}));
 aa_runtime::stored_mover_init_world::after_init(&catalog,&staged.actors,&staged.definitions,key).unwrap();
 aa_runtime::stored_element_init_world::after_init(&catalog,&staged.actors,&staged.definitions,key).unwrap();
 assert_eq!(staged.actors.object(key).unwrap()as *const _ as usize,object_address);
 assert_eq!(match staged.actors.component(owner,Slot::Mover).unwrap(){Some(RuntimeComponent::Mover(v))=>&**v as *const _ as usize,_=>panic!()},mover_address);
 assert_eq!(match staged.actors.component(owner,Slot::Element).unwrap(){Some(RuntimeComponent::Element(v))=>&**v as *const _ as usize,_=>panic!()},element_address);
 assert!(catalog.definition_bank.borrow().combat_source().is_err());staged.report.world_version=version_initial;
 let result=json!({"scope":"actual source usa_infantry component-constructor lab in diagnostic tutorial02 Staged; source/liveDouble53; same boxes/Bank/queue/RNG owners. Explicit WorldVersion/falling/flag inputs separated from genuine constructor branches; no full assembly, initialized mission or battle claim.","rows":rows});
 if let Some(path)=std::env::var_os("AA_STORED_MOVER_ELEMENT_CAPTURE"){serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&result).unwrap();}
 else{let expected:Value=serde_json::from_slice(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-stored-mover-element/actual-lab.json")).unwrap()).unwrap();assert_eq!(result,expected);}
}
