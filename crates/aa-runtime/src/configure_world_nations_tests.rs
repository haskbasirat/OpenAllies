use super::*;
use serde_json::{Value,json};
fn fixture(name:&str)->Value {serde_json::from_slice(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-configure-nations").join(name)).unwrap()).unwrap()}
fn pc(value:&Value)->Precision {match value.as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>unreachable!()}}
fn named(id:&str)->NamedDefinition {NamedDefinition{id:id.into(),index:None}}
#[test]
fn actual_native_faction_filter_and_rng_consumption_match() {
 let f=fixture("native-configure-nations.json");let mut count=0;
 for row in f["rows"].as_array().unwrap() {
  let c=&row["input"];if c["kind"]!="random"||c["commander"].is_number()||!c["old_nation"].is_number()||c.get("allowed").is_some(){continue}
  let nation=c["old_nation"].as_u64().unwrap()as usize;
  let allowed=[vec![0,1],vec![2],vec![0,2]];
  let list=allowed[nation].iter().map(|v|named(&v.to_string())).collect();
  let mut rng=NativeRng::new(c["seed"].as_u64().unwrap()as u32);let mut visited=vec![];
  let selected=pick_enabled(list,|v|{let index=v.id.parse::<usize>().unwrap();visited.push(index);Ok(c["enabled"][index].as_u64().unwrap()!=0)},||rng.next_u32()).unwrap();
  assert_eq!(json!(visited),json!(allowed[nation]));
  assert_eq!(json!(selected.map(|v|v.id.parse::<u32>().unwrap())),row["output"]["commander"],"{row}");
  assert_eq!(json!(rng.state),row["output"]["seed_after"]);
  let logs=row["output"]["events"].as_array().unwrap().iter().filter(|v|v[0]=="log").collect::<Vec<_>>();
  assert_eq!(logs.len(),2);assert_eq!(logs[0][1],"A");assert_eq!(logs[1][1],"S");count+=1;
 }
 assert_eq!(count,81);
 let mut calls=0;assert!(pick_enabled(vec![named("first"),named("second")],|_|Err("required public field".into()),||{calls+=1;7}).is_err());assert_eq!(calls,0);
}
#[test]
fn actual_native_hq_allocator_lowest_free_and_duplicate_order_match() {
 let f=fixture("native-configure-nations.json");let mut rows=0;
 for row in f["rows"].as_array().unwrap() {
  let c=&row["input"];let Some(hqs)=c.get("hqs")else{continue};
  let mut taken=c["taken"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect::<Vec<_>>();
  let mut rng=NativeRng::new(c["seed"].as_u64().unwrap()as u32);let slots=c["slots"].as_u64().unwrap()as u32;let mut result=vec![];
  for hq in hqs.as_array().unwrap() {
   let old=hq.as_i64().unwrap()as i32;
   if old!=-1 {if let Some(at)=taken.iter().position(|v|*v==old as u32){taken.remove(at);}}
   let next=if slots==0{-1}else{choose_badge(slots,&taken,rng.next_u32()%slots)};
   if next!=-1 {taken.insert(0,next as u32)}result.push(next as u32);
  }
  assert_eq!(json!(taken),row["output"]["taken"],"{row}");assert_eq!(json!(result),row["output"]["hqs"]);assert_eq!(json!(rng.state),row["output"]["seed_after"]);rows+=1;
 }
 assert_eq!(rows,108);
}
#[test]
fn original_source_getters_token_order_boolean_defaults_and_unknown_refs_match() {
 use aa_rules::{actors::native_load::{load_files,NativeLoadReport},sources::{ResolvedFile,Origin,sha256},tgi::Limits};
 let f=fixture("native-nation-source.json");assert_eq!(f["rows"].as_array().unwrap().len(),81);
 let file=|path:&str,text:String|ResolvedFile{logical_path:path.into(),origin:Origin::V14Loose,sha256:sha256(text.as_bytes()),bytes:text.into_bytes()};
 for row in f["rows"].as_array().unwrap(){
  let c=&row["input"];let precision=pc(&row["precision"]);
  let template="[Template Nation]\n{string IDS\n string factions = NULL\n flag selectable = true\n}\n[Template Faction]\n{string IDS\n bool public = true\n}\n";
  let subject=if c["kind"]=="nation" {format!("[Nation template=Nation]\n{{IDS = subject\n{}{} }}\n",c["tokens"].as_str().map(|v|format!("factions = {v}\n")).unwrap_or_default(),c["selectable"].as_bool().map(|v|format!("selectable = {v}\n")).unwrap_or_default())}
   else{format!("[Faction template=Faction]\n{{IDS = subject\n{} }}\n",c["public"].as_bool().map(|v|format!("public = {v}\n")).unwrap_or_default())};
  let fs="[Faction template=Faction]\n IDS = first\n[Faction template=Faction]\n IDS = second\n[Faction template=Faction]\n IDS = third\n";
  let actors=load_files(vec![file("templates.tgi",template.into()),file("factions.tgi",fs.into()),file("subject.tgi",subject)],Limits::default(),&mut NativeLoadReport::default());let catalog=Catalog::new(&actors);
  if c["kind"]=="nation" {
   let found=factions(&catalog,&named("subject"),precision);
   if row["output"]["throw"].is_null(){assert_eq!(json!(found.unwrap().iter().map(|v|&v.id).collect::<Vec<_>>()),row["output"]["factions"],"{row}");assert_eq!(selectable(&catalog,&named("subject"),precision).unwrap(),row["output"]["selectable"]==1);}
   else{assert!(found.is_err(),"{row}")}
  }else{assert_eq!(enabled(&catalog,&named("subject"),precision).unwrap(),row["output"]["public"]==1);}
 }
}
#[test]
#[ignore="requires actual AA_GAME_ROOT and verified source Bank; bounded source/owner integration, no full startup claim"]
fn actual_firefight_registered_kingdom_random_sources_use_same_bank_rng_and_logs() {
 use crate::{combat_source_completion as completion,combat_source_receipt::VerifiedStartupReceipt,localization::Localization,settings::Settings,restoration::{Attempt,LoadBindings}};
 use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
 use aa_orders::net::sync::Synchronizer;use std::cell::RefCell;
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();let text=Localization::load_v14(&mut sources,Limits::default()).unwrap();let actors=ActorCatalog::load_v14(&mut sources).unwrap();
 let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(game.join("reconstruction/assets/original/Data/Maps/DeveloperMaps/(3v3)Firefight.RMP")).unwrap()).unwrap();
 let mut attempt=Attempt::load_with_source_setup(&map,&actors,settings,text,Precision::Double53,u64::MAX,aa_mapworld::restore::RestoreMode::StopOnError,LoadBindings{native_load_mode:Some(1),..LoadBindings::default()},|catalog|{
  let proof=VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut sources)?;completion::install(catalog,&mut sources,proof.context(),proof.csv(),proof.globals())?;completion::compute(catalog)
 }).unwrap();
 let a=RefCell::new(Synchronizer::new(1024));let s=RefCell::new(Synchronizer::new(1024));a.borrow_mut().reset(0,-1);s.borrow_mut().reset(0,-1);
 let mut gate_a=||Ok(true);let mut gate_s=||Ok(true);
 let mut catalog=Catalog::new(&actors);catalog.definition_bank=RefCell::new(std::mem::take(&mut attempt.definitions));
 catalog.bind_configuration_sync(crate::live_clocked::SyncBinding::Shared{owner:&a,gate:&mut gate_a}).unwrap();catalog.bind_configuration_sync_s(crate::live_clocked::SyncBinding::Shared{owner:&s,gate:&mut gate_s}).unwrap();
 let ids=attempt.registry.iter().filter_map(|e|matches!(e.value,RestoredRecord::Kingdom(_)).then_some(e.id)).collect::<Vec<_>>();assert!(!ids.is_empty());
 let mut rng=NativeRng::new(0);let mut output=vec![];let source_pc=source_pc(&catalog).unwrap();
 let source_rows=actors.native_data_registry("Nation").unwrap().entries.iter().map(|e|{let n=named(&e.ids);json!({"id":e.ids,"selectable":selectable(&catalog,&n,source_pc).unwrap(),"factions":factions(&catalog,&n,source_pc).unwrap().iter().map(|f|json!({"id":f.id,"enabled":enabled(&catalog,f,source_pc).unwrap()})).collect::<Vec<_>>()})}).collect::<Vec<_>>();
 for id in ids {
  assert!(nation(shell(&attempt.registry,id).unwrap()).unwrap().is_none());assert!(commander_is_null(shell(&attempt.registry,id).unwrap()).unwrap());
  assert!(shell(&attempt.registry,id).unwrap().vectors[&0x2dc].values.is_empty());
  let before=shell(&attempt.registry,id).unwrap()as*const _ as usize;let seed=rng.state;
  resolve_random(&catalog,&mut attempt.registry,id,&mut rng).unwrap();assert_eq!(shell(&attempt.registry,id).unwrap()as*const _ as usize,before);
  let n=nation(shell(&attempt.registry,id).unwrap()).unwrap().unwrap();assert!(selectable(&catalog,n,source_pc).unwrap());
  let commander=commander_faction(&catalog,shell(&attempt.registry,id).unwrap()).unwrap();let list=factions(&catalog,n,source_pc).unwrap();assert!(commander.as_ref().is_some_and(|c|list.iter().any(|v|v.id.eq_ignore_ascii_case(c))));
  output.push(json!({"id":id.raw(),"seed_before":seed,"seed_after":rng.state,"nation":n.id,"commander":commander}));
 }
 assert_eq!(a.borrow().end(),s.borrow().end());
 //The current valid source/owner receipt is required only after the Nation
 //store reaches an existing nonnull Commander. Refusal retains that store.
 let id=attempt.registry.iter().find(|e|matches!(e.value,RestoredRecord::Kingdom(_))).unwrap().id;
 let retained=commander_faction(&catalog,shell(&attempt.registry,id).unwrap()).unwrap().unwrap();
 let invalid_catalog=Catalog::new(&actors);let seed=rng.state;
 assert!(set_nation(&invalid_catalog,&mut attempt.registry,id,Some(named("nation_usa")),&mut rng).is_err());
 assert_eq!(nation(shell(&attempt.registry,id).unwrap()).unwrap().unwrap().id,"nation_usa");
 assert_eq!(commander_faction(&catalog,shell(&attempt.registry,id).unwrap()).unwrap(),Some(retained));assert_eq!(rng.state,seed);
 //Actual source compatibility keeps an allowed Commander and drops the same
 //allocation when changing to a Nation whose list excludes its definition.
 let pc=source_pc;let all=actors.native_data_registry("Nation").unwrap();
 let faction=commander_faction(&catalog,shell(&attempt.registry,id).unwrap()).unwrap().unwrap();
 let permitted=all.entries.iter().find(|n|factions(&catalog,&named(&n.ids),pc).unwrap().iter().any(|v|v.id.eq_ignore_ascii_case(&faction))).unwrap();
 set_nation(&catalog,&mut attempt.registry,id,Some(named(&permitted.ids)),&mut rng).unwrap();assert_eq!(commander_faction(&catalog,shell(&attempt.registry,id).unwrap()).unwrap(),Some(faction.clone()));
 let denied=all.entries.iter().find(|n|!factions(&catalog,&named(&n.ids),pc).unwrap().iter().any(|v|v.id.eq_ignore_ascii_case(&faction))).unwrap();
 set_nation(&catalog,&mut attempt.registry,id,Some(named(&denied.ids)),&mut rng).unwrap();assert!(commander_is_null(shell(&attempt.registry,id).unwrap()).unwrap());assert_eq!(rng.state,seed);
 if let Some(path)=std::env::var_os("AA_CONFIGURE_NATIONS_CAPTURE"){std::fs::write(path,serde_json::to_vec_pretty(&json!({"source_rows":source_rows,"kingdoms":output,"logsA":a.borrow().end(),"logsS":s.borrow().end()})).unwrap()).unwrap();}
}

