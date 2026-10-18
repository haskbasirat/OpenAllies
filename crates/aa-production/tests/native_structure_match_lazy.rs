//! Exact5890C0 read order, including gates before either linked source list.
use aa_production::{Result,construction::{self,StructureMatchWorld,StructureMatchFacts}};
use serde_json::Value;
struct Read<'a>{input:&'a Value,events:Vec<&'static str>}
impl Read<'_>{fn flag(&self,name:&str)->bool{self.input[name].as_u64()==Some(1)}}
impl StructureMatchWorld for Read<'_>{
 fn state_25(&mut self)->Result<bool>{self.events.push("state25");Ok(self.flag("state25"))}
 fn transient(&mut self)->Result<bool>{self.events.push("transient");Ok(self.flag("state26")||self.flag("state36"))}
 fn definition(&mut self)->Result<u32>{self.events.push("definition");Ok(if self.input["match"]=="direct"{7}else{8})}
 fn contains(&mut self,offset:u32,_:u32)->Result<bool>{
  self.events.push(if offset==0x4c0{"contains4c0"}else{"contains4d0"});
  Ok(self.input["match"]==if offset==0x4c0{"upgrade"}else{"predecessor"})
 }
 fn constructing(&mut self)->Result<bool>{self.events.push("constructing");Ok(self.flag("constructing"))}
}
#[test]
fn actual_5890c0_lazy_read_order_matches_both_images_all_pc(){
 let doc:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-stored-requirements/native-structure-match.json")).unwrap();
 assert_eq!(doc["rows"].as_array().unwrap().len(),768);
 for row in doc["rows"].as_array().unwrap(){
  let input=&row["input"];let mut read=Read{input,events:vec![]};
  let complete=input["complete"].as_bool().unwrap();let upgrades=input["upgrades"].as_bool().unwrap();
  let output=construction::structure_matches_with(&mut read,7,complete,upgrades).unwrap();
  assert_eq!(output,row["output"]["result"].as_bool().unwrap(),"{row}");
  assert_eq!(serde_json::to_value(&read.events).unwrap(),row["output"]["events"],"{row}");
  let facts=StructureMatchFacts{state_25:read.flag("state25"),transient:read.flag("state26")||read.flag("state36"),
   definition:if input["match"]=="direct"{7}else{8},constructing:read.flag("constructing"),
   upgrades:if input["match"]=="upgrade"{vec![7]}else{vec![]},list_4d0:if input["match"]=="predecessor"{vec![7]}else{vec![]},structure_component:true};
  assert_eq!(construction::structure_matches(&facts,7,complete,upgrades),output);
 }
}
