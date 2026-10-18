//! Independent current-source transport input capture. No runtime profiles or
//! World are created. The production LoadedRestrictionGroups must replay these
//! captured references separately before acceptance.
use aa_mapworld::{MapArchive, restriction_groups::{LocalizedStorage, RestrictionGroups}};
use aa_rules::{actors::ActorCatalog, sources::{SourceLimits,V14Sources},native::layout_data::PropertyRef};
use aa_numeric::Precision;
use serde_json::json;
use std::{collections::BTreeMap,path::{Path,PathBuf}};
fn walk(at:&Path,root:&Path,out:&mut Vec<String>){for e in std::fs::read_dir(at).unwrap(){let p=e.unwrap().path();if p.is_dir(){walk(&p,root,out)}else if p.extension().is_some_and(|e|e.eq_ignore_ascii_case("rmp")){out.push(p.strip_prefix(root).unwrap().to_string_lossy().replace('\\',"/"));}}}
fn main(){
 let root=PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut sources=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
 let actors=ActorCatalog::load_v14(&mut sources).unwrap();
 let properties=actors.native_layout_data(Precision::Double53).unwrap();
 let maps=root.join("reconstruction/assets/original/Data/Maps");let mut names=Vec::new();walk(&maps,&maps,&mut names);names.sort();assert_eq!(names.len(),80);
 let mut rows=Vec::new();
 for name in names {
  let map=MapArchive::from_bytes(std::fs::read(maps.join(&name)).unwrap()).unwrap();let row=map.directory().iter().find(|r|r.name=="RESTRICTION_GROUPS").unwrap();
  let chunk=map.decode_chunk("RESTRICTION_GROUPS",row.decoded_bytes).unwrap();let input=RestrictionGroups::decode(&chunk,LocalizedStorage::KeyOnly).unwrap();
  let mut lookups:BTreeMap<&str,BTreeMap<String,Option<String>>>=BTreeMap::from([("actor",BTreeMap::new()),("property",BTreeMap::new())]);
  for profile in input.profiles(){
   for (kind,list) in [("actor",&profile.allowed_actors),("property",&profile.allowed_properties),("actor",profile.restricted_actors.as_ref().unwrap()),("property",profile.restricted_properties.as_ref().unwrap())]{
    for u in list {let id=String::from_utf16(u).unwrap();let result=if kind=="actor"{
     if actors.find(&id).is_some() && matches!(actors.native_category(&id).unwrap().native_kind,2|19) {Some(actors.find(&id).unwrap().id.clone())}else{None}
    }else{properties.find_property(&id,None).map(|r|{let PropertyRef::Registered(i)=r else{panic!("embedded Property Find")};properties.properties[i as usize].ids.clone()})};
    lookups.get_mut(kind).unwrap().insert(id.to_ascii_lowercase(),result);
   }}
  }
  rows.push(json!({"map":name,"payload_sha256":input.payload_source().sha256,"lookups":lookups}));
 }
 let receipt=json!({"schema":1,"scope":"Independent current typed source transport inputs from native-order ActorCatalog and shared NativeLayoutData; no runtime restriction/world completion claim.","source_files":actors.sources,"maps":rows});
 let bytes=serde_json::to_vec_pretty(&receipt).unwrap();std::fs::write("research/skirmish-20261016-restriction-groups/source-lookups.json",bytes).unwrap();
 println!("captured actual current typed references for80 maps");
}
