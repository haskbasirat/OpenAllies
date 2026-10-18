use super::*;
use serde_json::{Value,json};
struct Source{seeds:[u32;3],trace:Vec<[u32;2]>,fail:Option<usize>}
impl Globals for Source{
 fn word(&mut self,address:u32)->Result<u32>{
  let index=[0xb43cf8,0xb43cfc,0xb43d00].iter().position(|a|*a==address).unwrap();let value=self.seeds[index];
  self.trace.push([address,value]);if self.fail==Some(self.trace.len()){Err("required global".into())}else{Ok(value)}
 }
}
#[test]
fn native_874b40_fields_known_masks_seed_error_prefixes_and_owned_arrays(){
 let doc:Value=serde_json::from_str(include_str!("../../../research/development10/native-camera-base/native-base.json")).unwrap();let mut rows=0;
 for row in doc["rows"].as_array().unwrap(){
  let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};
  let mut state=State::unbound_allocation();let mut source=Source{seeds:serde_json::from_value(row["input"]["seeds"].clone()).unwrap(),trace:vec![],fail:row["input"]["fail"].as_u64().map(|v|v as usize)};
  let result=initialize_874b40(&mut state,&mut source,pc);
  assert_eq!(json!(source.trace),row["output"]["trace"],"row{rows}");
  assert_eq!(if result.is_ok(){"ok"}else{"required global"},row["output"]["outcome"].as_str().unwrap(),"row{rows} {result:?}");
  for(index,expected)in row["output"]["known"].as_array().unwrap().iter().enumerate(){
   let offset=4*index as u32;if[0,0xa0,0x168,0x16c,0x180,0x184].contains(&offset){continue}
   let expected=expected.as_u64().unwrap()as u32;
   let (known,bits)=match state.base.word(offset){Some(Ok(value))=>(u32::MAX,value),Some(Err(e))=>panic!("{offset:X} {e}"),None=>(state.words[index].known,state.words[index].bits)};
   assert_eq!(known,expected,"known mask row{rows} offset{offset:X}");
   let native=row["output"]["words"][index].as_u64().unwrap()as u32;
   assert_eq!(bits&known,native&known,"word row{rows} offset{offset:X}");
  }
  let arrays:Vec<_>=[(0x168,&state.base.first),(0x180,&state.base.second)].into_iter().filter_map(|(offset,v)|v.as_ref().map(|a|json!({"offset":offset,"capacity":a.slots.len(),"used":a.used,"effective":a.effective,"grow":a.grow_by,"null_slots":a.slots.iter().map(Option::is_none).collect::<Vec<_>>()}))).collect();
  assert_eq!(json!(arrays),row["output"]["arrays"],"row{rows}");
  assert_eq!(state.require_ni_base_constructor().is_ok(),result.is_ok());
  assert!(state.word(0x108).is_err(),"native108 remains unwritten");
  if result.is_ok(){assert!(state.word(0x16c).is_err());assert!(state.word(0x184).is_err());assert_eq!(state.word(0x174).unwrap(),0)}
  let before=source.trace.len();assert!(initialize_874b40(&mut state,&mut source,pc).is_err());assert_eq!(before,source.trace.len());
  rows+=1;
 }
 assert_eq!(rows,99);
}
#[test]
fn array_metadata_has_one_owner_and_no_numeric_allocation_pointer(){
 let mut state=State::unbound_allocation();let mut source=Source{seeds:[0;3],trace:vec![],fail:None};initialize_874b40(&mut state,&mut source,Precision::Double53).unwrap();
 assert_eq!(source.trace.len(),27);assert_eq!(state.word(0x170).unwrap(),5);
 assert_eq!(state.words[0x174/4].known,0);assert_eq!(state.word(0x174).unwrap(),0);
 state.put(0x174,0);assert_eq!(state.words[0x174/4].known,0);assert_eq!(state.word(0x174).unwrap(),0);
}
