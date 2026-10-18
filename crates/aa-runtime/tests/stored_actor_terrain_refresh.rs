//! Explicit direct leaf laboratory: real map/source/constructor owners and
//! moved diagnostic actor boxes. This does not fabricate initialized mission state.
use aa_runtime::{catalog::Catalog,restoration::{Attempt,LoadBindings},actor_world::RuntimeComponent,
 combat_source_receipt::VerifiedStartupReceipt,combat_source_completion as completion,
 settings::Settings,localization::Localization,restore_lifecycle::{ScopedManagedLoad,TaskKind}};
use aa_actors::components::ComponentSlot as Slot;
use aa_numeric::{Binary,Precision};
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
use serde_json::{Value,json};
//Compile and execute the exact pure production kernel/native fixture without
//a second whole-runtime unit linker. The held module reexports these same gates.
#[path="../src/actor_terrain_fields.rs"]mod actual_field_kernel;
#[test]
#[ignore="requires AA_GAME_ROOT actual Kursk terrain and completed source receipt"]
fn stored_terrain_refresh_uses_actual_f8_box_map_source_and_same_paired_owners(){
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
 let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();let text=Localization::load_v14(&mut sources,Limits::default()).unwrap();
 let source=ActorCatalog::load_v14(&mut sources).unwrap();let resources=std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
 let bytes=std::fs::read(game.join("reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP")).unwrap();let map=aa_mapworld::MapArchive::from_bytes(bytes).unwrap();
 let inventory=aa_mapworld::WorldInventory::decode(&map,u64::MAX).unwrap();let mut variables=None;
 let inactive=ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
 let attempt=Attempt::load_with_source_setup(&map,&source,settings.clone(),text,Precision::Double53,u64::MAX,aa_mapworld::restore::RestoreMode::InspectPartial,
  LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources.clone())},|catalog|{
   let receipt=VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut sources)?;
   completion::install(catalog,&mut sources,receipt.context(),receipt.csv(),receipt.globals())?;completion::compute(catalog)?;
   let mut vars=aa_runtime::load_view_variables::State::reset_from_fresh_source(catalog,aa_runtime::load_view_variables::NativeOverrides::ControlledNone)?;
   let rules=aa_runtime::load_world_rules::Owner::load(catalog,&inventory)?;vars.publish_loaded_rules(catalog,&rules)?;variables=Some(vars);Ok(())
  }).unwrap();
 let terrain=aa_runtime::battle_terrain::BattleTerrain::load_with_sources(&map,&mut sources,&source,&settings,Precision::Double53).unwrap();
 let mut staged=attempt.into_staged().unwrap();let mut catalog=Catalog::new(&source);catalog.settings=Some(settings);catalog.native_load_mode=Some(1);catalog.bind_resource_catalog(resources).unwrap();
 assert!(catalog.definition_bank.borrow().combat_source().is_err());let mut variables=variables.unwrap();
 let key=staged.actors.identities().iter().find_map(|entry|{
  let object=staged.actors.object(entry.value).ok()?;let actor=object.actor.as_ref()?;
  if actor.components.present(Slot::Organization)||!actor.components.present(Slot::Properties){return None}
  if !matches!(actor.components.get(Slot::Properties),Some(RuntimeComponent::Properties(p))if p.effects.is_empty()){return None}
  match actor.components.get(Slot::Element){None=>(),Some(RuntimeComponent::Element(e))if !e.flag_1c||e.organization.is_none()=>(),_=>return None}
  let pose=staged.actors.staged_placement(entry.value).ok()?;
  let tile=[0,1].map(|i|Binary::from_f32_bits(pose.position_bits[i]).ok()?.truncate_i32().ok()).map(|v|v.map(|v|v>>terrain.bound.layout.coordinate_shift));
  let tile=[tile[0]?,tile[1]?];let definition=terrain.terrain_definition_at(tile[0],tile[1]).ok()?;
  let registration=source.native_data_registry("Terrain")?.entries.iter().find(|e|e.ids.eq_ignore_ascii_case(&definition.id))?;
  source.native_data_node(registration).ok()?.children("Property").ok()?.is_empty().then_some((entry.id,entry.value,tile,definition.id.clone()))
 }).expect("real Kursk Properties Actor and actual empty Terrain Property list");
 let(owner,key,tile,terrain_id)=key;let address=staged.actors.object(key).unwrap()as *const _ as usize;
 let actor_address=staged.actors.object(key).unwrap().actor.as_ref().unwrap()as *const _ as usize;
 let definition=staged.actors.object(key).unwrap().definition.proof.clone();
 //The diagnostic loader omits Object Init. These explicit native gate inputs
 //and cached coordinates come from the real saved pose; no load parity claim.
 let p=staged.actors.staged_placement_mut(key).unwrap();p.cached_tile=Some(tile);p.flags&=!((1<<29)|(1<<12));
 let actor=staged.actors.object_mut(key).unwrap().actor.as_mut().unwrap();
 actor.components.state.low&=!((1<<24)|(1<<25)|(1<<26));actor.components.state.high&=!0x14;
 let mut rows=vec![];
 let prior=source.native_data_registry("Terrain").unwrap().entries.iter().find_map(|entry|{
  (!source.native_data_node(entry).ok()?.children("Property").ok()?.is_empty()).then_some(entry.ids.clone())
 });
 for pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
  staged.actors.object_mut(key).unwrap().actor.as_mut().unwrap().words.insert(0xF8,0);
  //This is the real native grid/tile constructor over this same bound map,
  //with no invented tile flags. No actor membership/adoption is claimed.
  let load=aa_runtime::load_spatial::LoadSpatial::new(&terrain.bound,pc).unwrap();
  let mut adoption=aa_runtime::spatial_world::SpatialWorld::adopt_loaded(load.into_parts(),&mut staged.actors);
  assert!(adoption.error.is_none()&&adoption.pending.is_empty()&&adoption.completed.is_empty());
  let(arena,mut actors)=staged.actors.split_minds();
  let mut context=aa_runtime::stored_actor_terrain_refresh::Context{actors:&mut actors,arena,bank:&mut staged.definitions,spatial:&mut adoption.world,
   terrain:&terrain,variables:&mut variables,clock:&staged.clock,inventory:&staged.inventory};
  aa_runtime::stored_actor_terrain_refresh::refresh_on_owners(&catalog,&mut context,key,pc).unwrap();
  let token=context.actors.object(key).unwrap().actor.as_ref().unwrap().words[&0xF8];assert_ne!(token,0);
  assert_eq!(context.bank.require_usable(token).unwrap().key.name,terrain_id.to_ascii_lowercase());
  //Equality bypasses all Property callbacks on the same actual F8 cell.
  aa_runtime::stored_actor_terrain_refresh::refresh_on_owners(&catalog,&mut context,key,pc).unwrap();
  assert_eq!(context.actors.object(key).unwrap().actor.as_ref().unwrap().words[&0xF8],token);
  assert_eq!(context.actors.object(key).unwrap()as *const _ as usize,address);
  assert_eq!(context.actors.object(key).unwrap().actor.as_ref().unwrap()as *const _ as usize,actor_address);
  //If an actual positive Terrain source list exists, the actual empty
  //Properties owner removes no instance without compiling unrelated effects.
  let mut old_list_probe=None;
  if let Some(prior)=&prior{
   let old=context.bank.intern("TerrainData",prior).unwrap();
   context.actors.object_mut(key).unwrap().actor.as_mut().unwrap().words.insert(0xF8,old);
   let before=context.actors.property_allocations.instances().len();
   aa_runtime::stored_actor_terrain_refresh::refresh_on_owners(&catalog,&mut context,key,pc).unwrap();
   assert_eq!(context.actors.property_allocations.instances().len(),before);
   assert_eq!(context.actors.object(key).unwrap().actor.as_ref().unwrap().words[&0xF8],token);
   old_list_probe=Some(prior.clone());
  }
  rows.push(json!({"owner":owner.raw(),"live_precision":pc,"source_precision":"Double53","tile":tile,"terrain_id":terrain_id,"token":token,"same_allocation":true,"actual_prior_source_list":old_list_probe}));
 }
 let output=json!({"scope":"Actual Kursk source/terrain and moved diagnostic Actor box; explicit active gates/cached tile seeded from actual saved pose. Native real tile constructor has no registered actors, and all placement reads use same staged owner. Empty source Property list proven; no positive Property/Organization recursion, natural Init, mission or battle claim.","definition":definition,"rows":rows});
 if let Some(path)=std::env::var_os("AA_STORED_TERRAIN_CAPTURE"){serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&output).unwrap();}
 else{assert_eq!(output,serde_json::from_slice::<Value>(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-stored-actor-terrain-refresh/actual-stored-terrain-v2.json")).unwrap()).unwrap());}
}
