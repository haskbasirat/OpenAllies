use super::*;
use serde_json::{json,Value};
struct Input{value:Value,list:Vec<ActorId>,reads:Vec<&'static str>}
impl World for Input{
 fn required_city_158(&mut self)->Result<ActorId>{self.reads.push("required");Ok(ActorId::from_raw(match self.value["reference"].as_str().unwrap(){"null"=>0,"self"=>0x10001,"stale"=>0x30002,_=>0x20002}))}
 fn resolve_current(&mut self,id:ActorId)->Result<Option<ActorId>>{
  self.reads.push("resolve");Ok((id.raw()==0x10001||(id.raw()==0x20002&&self.value["reference"]!="absent")).then_some(id))
 }
 fn definition_24(&mut self,_:ActorId)->Result<bool>{self.reads.push("descriptor");Ok(!self.value["thing"].as_bool().unwrap()&&self.value["predicate"].as_u64().unwrap()!=0)}
 fn current_actor(&self)->ActorId{ActorId::from_raw(0x10001)}
 fn has_city_d4(&mut self)->Result<bool>{self.reads.push("city");Ok(self.value["city"].as_bool().unwrap())}
 fn list_30c(&mut self)->Result<&mut Vec<ActorId>>{self.reads.push("list");Ok(&mut self.list)}
}
#[test]
fn whole_native_required_city_selection_dedup_removal_and_count(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/development10/native-kingdom-ownership.json")).unwrap();
 let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),1200);
 for row in rows{
  let value=row["input"].clone();let list=value["list"].as_array().unwrap().iter().map(|v|ActorId::from_raw(v.as_u64().unwrap()as u32)).collect::<Vec<_>>();
  let before=list.len();let mut world=Input{value,list,reads:Vec::new()};
  update(&mut world,row["input"]["add"].as_bool().unwrap()).unwrap();
  let values=world.list.iter().map(|id|id.raw()).collect::<Vec<_>>();
  assert_eq!(json!({"values":values,"count":count(&world.list),"freed":before.saturating_sub(world.list.len()),
   "head_null":world.list.is_empty(),"tail_null":world.list.is_empty()}),row["output"],"{row}");
  if world.reads.contains(&"descriptor")&&!row["input"]["thing"].as_bool().unwrap()&&row["input"]["predicate"].as_u64().unwrap()!=0{
   assert_eq!(world.reads.iter().filter(|&&s|s=="required").count(),2);
  }
 }
}
