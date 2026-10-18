use aa_rules::{actors::ActorCatalog,sources::{ResolvedFile,Origin,sha256},tgi::Limits};
use aa_numeric::Precision;
use serde_json::Value;
fn catalog(count:usize)->ActorCatalog {
 let mut text=String::from("[Thing]\n{\nstring IDS = CityFixture\nstring category = ACTOR\n[CityComponent]\n{\n[Layout]\n{\n[list Object]\n{}\n");
 for i in 0..count {text.push_str(&format!("[Object]\n{{\nstring object_ids = Target{i}\npoint2 offset = {i}.125, -{i}.25\npoint2 orientation = 0.6,-0.8\n}}\n"));}
 text.push_str("}\n}\n}\n");let bytes=text.into_bytes();
 ActorCatalog::from_files(vec![ResolvedFile{logical_path:"city-layout-source-laboratory.tgi".into(),origin:Origin::V14Loose,sha256:sha256(&bytes),bytes}],Limits::default())
}
#[test]
fn native_deferred_rows_reverse_source_and_preserve_all_point_words(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-object-ground/native-city-layout-source.json")).unwrap();
 assert_eq!(fixture["rows"].as_array().unwrap().len(),18);
 for row in fixture["rows"].as_array().unwrap(){let source=catalog(row["input"]["count"].as_u64().unwrap()as usize);
  let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};
  let layouts=source.native_city_layouts("CityFixture",pc).unwrap().unwrap();assert_eq!(layouts.len(),1);
  let expected=row["output"]["rows"].as_array().unwrap();assert_eq!(layouts[0].len(),expected.len());
  for(actual,expected)in layouts[0].iter().zip(expected){assert_eq!(actual.definition_ids,expected["definition_ids"].as_str().unwrap());
   for i in 0..2 {assert_eq!(actual.offset[i],expected["offset"][i].as_u64().unwrap()as u32);assert_eq!(actual.orientation[i],expected["orientation"][i].as_u64().unwrap()as u32);}}
 }
 assert!(catalog(0).native_city_layouts("CityFixture",Precision::Double53).is_err());
}
#[test]
#[ignore="requires AA_GAME_ROOT original/v14 sources"]
fn actual_city_source_inventory_has_typed_layouts_and_canonical_objects(){
 use aa_rules::sources::{V14Sources,SourceLimits};
 let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let catalog=ActorCatalog::load_v14(&mut files).unwrap();let mut cities=0;let mut layouts=0;let mut rows=0;let mut actor_rows=0;let mut object_rows=0;
 for actor in &catalog.actors {for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
  let Some(current)=catalog.native_city_layouts(&actor.id,pc).unwrap_or_else(|e|panic!("{} {pc:?}: {e}",actor.id))else{continue};
  if pc==Precision::Double53 {cities+=1;layouts+=current.len();}
  for layout in current {for row in layout {let kind=catalog.native_category(&row.definition_ids).unwrap_or_else(|e|panic!("{} -> {}: {e}",actor.id,row.definition_ids)).native_kind;
   if pc==Precision::Double53 {rows+=1;if kind==2{actor_rows+=1}else{object_rows+=1}}}}
 }}
 assert!(cities>0);println!("actual_source_cities={cities} layouts={layouts} rows={rows} actor_rows={actor_rows} object_rows={object_rows}");
}
