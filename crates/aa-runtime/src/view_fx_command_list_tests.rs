use super::*;
use serde_json::{json,Value};
struct Payload { next:Option<Key>, poisoned:bool }
impl Message for Payload {
    fn set_next(&mut self,next:Option<Key>){self.next=next;self.poisoned=false;}
    fn next(&self)->Option<&Key>{self.next.as_ref()}
}
#[test]
fn original_v14_43b870_links_null_and_retention_prefixes_match_all_precisions(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-view-fx-commands/native-view-fx-commands.json")).unwrap();
 assert_eq!(fixture["rows"].as_array().unwrap().len(),90);
 for row in fixture["rows"].as_array().unwrap(){
  let c=&row["input"];let out=&row["output"];let mut list=List::from_native_constructor();let mut prior=vec![];
  for _ in 0..c["prior"].as_u64().unwrap(){
   let mut p=Some(Payload{next:None,poisoned:false});prior.push(list.append(&mut p,|_|Ok(())).unwrap());
  }
  //Explicit paired fixture pre-state, including native u32 counter wrap.
  list.count=c["count"].as_u64().unwrap()as u32;
  let mut incoming=(!c["null_command"].as_bool().unwrap()).then_some(Payload{next:None,poisoned:true});
  let mut retained=9u32;
  let result=list.append(&mut incoming,|_|match c["definition"].as_str().unwrap(){
   "required"=>Err("retention45A940".into()),"true"=>{retained=retained.wrapping_add(1);Ok(())},_=>Ok(())});
  let new=result.as_ref().ok();
  let name=|key:Option<&Key>|->Value{match key{None=>Value::Null,Some(k)if Some(k)==new=>json!("new"),Some(k)=>json!(prior.iter().position(|p|p==k).unwrap())}};
  assert_eq!(name(list.head()),out["head"],"{c}");assert_eq!(name(list.tail()),out["tail"],"{c}");
  assert_eq!(json!(list.native_count()),out["count"],"{c}");assert_eq!(json!(retained),out["retain_count"],"{c}");
  let links:Vec<_>=prior.iter().map(|key|name(list.get(key).unwrap().next.as_ref())).collect();assert_eq!(json!(links),out["prior_next"],"{c}");
  assert_eq!(result.is_err(),!out["stopped"].is_null());
  if let Ok(key)=result{assert!(incoming.is_none());assert!(!list.get(&key).unwrap().poisoned);assert!(list.get(&key).unwrap().next.is_none());}
  else if !c["null_command"].as_bool().unwrap(){assert!(incoming.as_ref().unwrap().poisoned);}
 }
 for row in fixture["constructor_rows"].as_array().unwrap(){let list=List::<Payload>::from_native_constructor();
  assert!(list.head().is_none()&&list.tail().is_none());assert_eq!(list.native_count(),0);
  for offset in ["0xe0","0xe4","0xe8"]{assert_eq!(row["output"][offset],0);}
 }
}
#[test]
fn intrusive_keys_reject_another_or_replaced_list_owner(){
 let mut first=List::from_native_constructor();let mut p=Some(Payload{next:None,poisoned:false});let key=first.append(&mut p,|_|Ok(())).unwrap();
 let mut other=List::from_native_constructor();let mut p=Some(Payload{next:None,poisoned:false});other.append(&mut p,|_|Ok(())).unwrap();
 assert!(first.get(&key).is_some());assert!(other.get(&key).is_none());drop(first);
 let replacement=List::<Payload>::from_native_constructor();assert!(replacement.get(&key).is_none());
}
