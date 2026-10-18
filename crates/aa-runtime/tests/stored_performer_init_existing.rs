//! Actual diagnostic-loaded allocations moved once to ActorStore, then the
//! stored57E000 binding. This is a focused leaf test, not fresh mission startup.
use aa_runtime::{catalog::Catalog,restoration::{Attempt,LoadBindings},stored_performer_init_existing as init,
 actor_world::{ActorStore,RuntimeComponent,Spatial,PlacementState},combat_source_receipt::VerifiedStartupReceipt,
 combat_source_completion as completion,settings::Settings,localization::Localization,
 restore_lifecycle::{ScopedManagedLoad,TaskKind}};
use aa_actors::{ActorId,components::ComponentSlot as Slot};
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
use serde_json::{Value,json};
struct RequiredSpatial;
impl Spatial for RequiredSpatial{
 fn placement(&self,_:ActorId)->aa_runtime::actor_world::Result<PlacementState>{Err(aa_runtime::actor_world::Error::Missing("actual published placement not provided in bounded staged test"))}
 fn publish(&mut self,_:ActorId,_:PlacementState)->aa_runtime::actor_world::Result<()>{panic!("read-only action initialization never publishes a placement")}
}
fn limits(catalog:&Catalog<'_>,bank:&aa_runtime::definitions::Bank,local:u32,pc:Precision)->[i32;2]{
 let name=&bank.entry(local).unwrap().key.name;let(id,index)=name.rsplit_once("/ability/").expect("actual primary local source");
 let binding=catalog.actor_bindings(id,pc).unwrap();let ability=&binding.performer.as_ref().unwrap().abilities()[index.parse::<usize>().unwrap()];
 [ability.authored.fields["required_experience_min"].integer().unwrap(),ability.authored.fields["required_experience_max"].integer().unwrap()]
}
fn performer(actors:&ActorStore,key:aa_runtime::actor_world::ObjectKey)->&aa_combat::constructors::OwnedPerformer{
 match actors.object(key).unwrap().components().unwrap().get(Slot::Performer){Some(RuntimeComponent::Performer(p))=>p,_=>panic!("actual Performer allocation")}
}
#[test]
#[ignore="requires AA_GAME_ROOT original tutorial02 and verified source startup context"]
fn actual_stored_performer_init_keeps_moved_bank_component_and_ability_owners(){
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
 let ids=staged.actors.identities().iter().filter_map(|e|{
  let object=staged.actors.object(e.value).ok()?;
  (matches!(object.definition.proof.id.as_str(),"usa_corps"|"usa_battleship")&&object.components().ok()?.present(Slot::Performer)).then_some((e.id,e.value))
 }).collect::<Vec<_>>();
 assert!(ids.iter().any(|(id,_)|id.raw()==4097));let mut rows=vec![];let mut positive=0;
 for(owner,key)in ids{
  let handles=performer(&staged.actors,key).state.abilities.clone();let address=std::ptr::from_ref(performer(&staged.actors,key))as usize;
  let original=handles.iter().map(|h|staged.actors.abilities[h].enabled).collect::<Vec<_>>();if !handles.is_empty(){positive+=1;}
  for phase in ["original_saved_bytes","explicit_inverted_rank_allowed_bytes"]{
   for live in [Precision::Single24,Precision::Double53,Precision::Extended64]{
    for(handle,initial)in handles.iter().zip(&original){staged.actors.abilities.get_mut(handle).unwrap().enabled=*initial;}
    if phase.starts_with("explicit"){
     for handle in &handles{let ability=&staged.actors.abilities[handle];if let Some(local)=ability.local_definition{
      if let Some(allowed)=aa_combat::performer::rank_allows(limits(&catalog,&staged.definitions,local,pc),0){staged.actors.abilities.get_mut(handle).unwrap().enabled=!allowed;}
     }}
    }
    let before=handles.iter().map(|h|{let a=&staged.actors.abilities[h];let bound=a.local_definition.map(|l|limits(&catalog,&staged.definitions,l,pc));json!({"local":bound.is_some(),"minimum":bound.map_or(0,|b|b[0]),"maximum":bound.map_or(0,|b|b[1]),"enabled":a.enabled})}).collect::<Vec<_>>();
    let ability_addresses=handles.iter().map(|h|std::ptr::from_ref(&staged.actors.abilities[h])as usize).collect::<Vec<_>>();
    let outcome=init::init_existing(&catalog,&mut staged.actors,&mut staged.definitions,key,&staged.inventory,&RequiredSpatial,live);
    let after=handles.iter().map(|h|staged.actors.abilities[h].enabled).collect::<Vec<_>>();
    let expected=before.iter().map(|a|if a["local"].as_bool().unwrap(){aa_combat::performer::rank_allows([a["minimum"].as_i64().unwrap()as i32,a["maximum"].as_i64().unwrap()as i32],0).unwrap_or(a["enabled"].as_bool().unwrap())}else{a["enabled"].as_bool().unwrap()}).collect::<Vec<_>>();
    assert_eq!(after,expected);let changed=before.iter().zip(&after).any(|(a,b)|a["enabled"].as_bool().unwrap()!=*b);
    if !changed{assert!(outcome.as_ref().is_ok_and(|v|*v),"{outcome:?}");}
    assert_eq!(std::ptr::from_ref(performer(&staged.actors,key))as usize,address);
    assert_eq!(handles.iter().map(|h|std::ptr::from_ref(&staged.actors.abilities[h])as usize).collect::<Vec<_>>(),ability_addresses);
    init::after_init(&catalog,&staged.actors,&staged.definitions,key).unwrap();
    rows.push(json!({"owner":owner.raw(),"phase":phase,"precision":live,"input":{"kind":"init","abilities":before},"enabled_after":after,"changed":changed,"result":outcome,"same_allocations":true}));
   }
  }
  for(handle,enabled)in handles.iter().zip(original){staged.actors.abilities.get_mut(handle).unwrap().enabled=enabled;}
 }
 assert!(positive>0);let out=json!({"scope":"Actual diagnostic tutorial02 allocations moved once to ActorStore; sourceBankDouble53 fixed,all3livePC; original saved bytes and explicit inverted-rank bytes. Full4D1350 Result retained, no completed fresh allocation/mission/battle claim.","map_sha256":aa_rules::sources::sha256(&bytes),"positive_ability_owners":positive,"rows":rows});
 if let Some(path)=std::env::var_os("AA_STORED_PERFORMER_INIT_CAPTURE"){serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&out).unwrap();}
 else{let expected:Value=serde_json::from_slice(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-fresh-actor-init/actual-stored-performer-init.json")).unwrap()).unwrap();assert_eq!(out,expected);}
}
