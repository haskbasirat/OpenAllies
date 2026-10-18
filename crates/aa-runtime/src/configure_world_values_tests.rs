use super::*;
fn fixture()->serde_json::Value {serde_json::from_str(include_str!("../../../research/skirmish-20261017-mission-start/native-configure-values.json")).unwrap()}
fn parameter(flags:u8,positive:bool,resources:Vec<u32>)->aa_mapworld::header::KingdomParameter {
 aa_mapworld::header::KingdomParameter {id:vec![],byte_10:0,nation:vec![],faction:vec![],color:vec![],handicap:vec![],display_key:vec![],string_0c:Some(vec![]),
 string_2c:vec![0x1234,0,0x5678],byte_29:0,byte_28:flags,string_14:vec![],localized_entries:vec![],definitions:if positive{vec![vec![42]]}else{vec![]},
 string_58:vec![0x5678,0,0x1234],properties:vec![],byte_31:0,byte_32:255-flags,resources:resources.into_iter().map(|v|(vec![],v)).collect()}
}
#[test]
fn native_scalar_tail_preserves_real_offsets_bytes_and_required_property_prefix() {
 let f=fixture();assert_eq!(f["tails"].as_array().unwrap().len(),96);
 for r in f["tails"].as_array().unwrap() {
  let input=&r["input"];let flags=input["flags"].as_u64().unwrap()as u8;
  let p=parameter(flags,input["positive"].as_bool().unwrap(),input["resources"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect());
  let mut shell=SpecialShell::kingdom(ActorId::from_raw(7),2).unwrap();
  shell.words.insert(0x238,0xaabbccdd);shell.words.insert(0x23c,0x11223344);
  shell.strings_utf16.insert(0x240,vec![0x2222]);shell.strings_utf16.insert(0x248,vec![0x4444]);
  shell.strings_utf16.insert(0x348,vec![0x7777]);
  shell.vectors.get_mut(&0x258).unwrap().replace_values(&[0x3f800000,0x40000000]).unwrap();
  let mut calls=0;let result=parameter_tail(&mut shell,&p,Some(NamedDefinition{id:"fixture-76543210".into(),index:None}),|_,_|{calls+=1;Err("required4E6640".into())});
  let out=&r["output"];assert_eq!(result.is_ok(),out["outcome"]=="ok","{r}");
  assert_eq!(calls,usize::from(input["positive"].as_bool().unwrap()));
  for (offset,key) in [(0x238,"word238"),(0x23c,"word23c")] {assert_eq!(shell.words[&offset],out[key].as_u64().unwrap()as u32,"{r}")}
  assert_eq!(shell.references[&0x244].as_ref().unwrap().id,"fixture-76543210");
  assert_eq!(shell.strings_utf16[&0x240],p.string_2c);
  assert_eq!(shell.strings_utf16[&0x248],if result.is_ok(){p.string_58}else{vec![0x4444]});
  assert_eq!(shell.strings_utf16[&0x348],[0x7777]);
  let expected:Vec<u32>=out["resources"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();
  assert_eq!(shell.vectors[&0x258].values,expected,"{r}");
 }
}
#[test]
fn native_team_first_match_removal_duplicates_and_wrapping_counters() {
 let f=fixture();assert_eq!(f["teams"].as_array().unwrap().len(),162);
 for r in f["teams"].as_array().unwrap() {
  let input=&r["input"];let old=input["old"].as_u64().unwrap()as usize;let new=input["new"].as_u64().unwrap()as usize;
  let mut shells=[SpecialShell::team().unwrap(),SpecialShell::team().unwrap()];
  for (i,s) in shells.iter_mut().enumerate() {
   s.team_members=Some(input["members"][i].as_array().unwrap().iter().map(|v|ActorId::from_raw(v.as_u64().unwrap()as u32)).collect());
   s.words.insert(0x68,if i==0{u32::MAX}else{7});
  }
  if old>0 {remove_from_team(&mut shells[old-1],ActorId::from_raw(1)).unwrap();}
  if new>0 {add_to_team(&mut shells[new-1],ActorId::from_raw(1)).unwrap();}
  for (i,s) in shells.iter().enumerate() {
   let expected:Vec<u32>=r["output"]["teams"][i]["members"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();
   assert_eq!(s.team_members.as_ref().unwrap().iter().map(|id|id.raw()).collect::<Vec<_>>(),expected,"{r}");
   assert_eq!(s.words[&0x68],r["output"]["teams"][i]["counter"].as_u64().unwrap()as u32,"{r}");
  }
 }
}
#[test]
fn native_color_counts_and_rng_pool_order_match_actual_leaves() {
 let f=fixture();assert_eq!(f["colors"].as_array().unwrap().len(),96);assert_eq!(f["pools"].as_array().unwrap().len(),54);
 for r in f["colors"].as_array().unwrap() {
  let mut owner=KingdomRegistryState::from_saved_counts(0,0).unwrap();owner.bind_color_counts(vec![0,u32::MAX,10]).unwrap();
  let index=|v:&serde_json::Value|{let n=v.as_u64().unwrap()as u32;if n==0{None}else{Some(n-1)}};
  owner.transition_color(index(&r["input"][0]),index(&r["input"][1])).unwrap();
  let expected:Vec<u32>=r["output"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();assert_eq!(owner.color_counts().unwrap(),expected,"{r}");
 }
 for r in f["pools"].as_array().unwrap() {
  let mut rng=NativeRng::new(r["input"]["seed"].as_u64().unwrap()as u32);
  let mut pool:Vec<u32>=r["input"]["remaining"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();
  let chosen=select_color(&mut pool,4,||Ok(rng.next_u32())).unwrap();
  assert_eq!(chosen,r["output"]["chosen"].as_u64().unwrap()as u32,"{r}");
  assert_eq!(rng.state,r["output"]["seed_after"].as_u64().unwrap()as u32);
  let expected:Vec<u32>=r["output"]["remaining"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();assert_eq!(pool,expected);
 }
 let mut pool=Vec::new();let mut calls=0;assert!(select_color(&mut pool,0,||{calls+=1;Ok(7)}).is_err());assert_eq!(calls,1);
}
#[test]
fn native_start_property_unique_append_preserves_duplicates_and_null_identity() {
 let f=fixture();assert_eq!(f["properties"].as_array().unwrap().len(),72);
 for r in f["properties"].as_array().unwrap() {
  let mut shell=SpecialShell::kingdom(ActorId::from_raw(7),0).unwrap();
  let input=&r["input"];let initial:Vec<u32>=input["initial"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();
  shell.vectors.get_mut(&0x24c).unwrap().replace_values(&initial).unwrap();
  append_start_property(&mut shell,input["value"].as_u64().unwrap()as u32).unwrap();
  let expected:Vec<u32>=r["output"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();
  assert_eq!(shell.vectors[&0x24c].values,expected,"{r}");
 }
}
