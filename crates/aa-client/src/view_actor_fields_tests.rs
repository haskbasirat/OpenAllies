use super::*;
use serde_json::{json,Value};
struct Source<'a>{input:&'a Value,events:Vec<Value>,globals:usize,turrets:usize,secondary:usize}
impl Source<'_>{
 fn event(&mut self,value:Value)->Result<()>{
  self.events.push(value);ensure!(self.input["fail"].as_u64()!=Some(self.events.len()as u64),"{}",self.events.last().unwrap()[0].as_str().unwrap());Ok(())
 }
 fn count(&mut self,secondary:bool)->Result<i32>{
  let(name,seq,index)=if secondary{("secondary-count",self.input["secondary_counts"].as_array().unwrap(),&mut self.secondary)}else{("turret-count",self.input["turret_counts"].as_array().unwrap(),&mut self.turrets)};
  let value=seq[(*index).min(seq.len()-1)].as_i64().unwrap()as i32;*index+=1;self.event(json!([name,value]))?;Ok(value)
 }
}
impl Sources for Source<'_>{
 fn global_word(&mut self,address:u32)->Result<u32>{let value=self.input["globals"][self.globals].as_u64().unwrap()as u32;self.globals+=1;self.event(json!(["global",address,value]))?;Ok(value)}
 fn actor_data_38(&mut self)->Result<bool>{let value=self.input["actor"].as_bool().unwrap();self.event(json!(["actor",value]))?;Ok(value)}
 fn turret_count_1fc(&mut self)->Result<i32>{self.count(false)}
 fn turret_pitch_57dbe0(&mut self,index:usize)->Result<u32>{let value=self.input["pitches"][index].as_u64().unwrap()as u32;self.event(json!(["pitch",index,value]))?;Ok(value)}
 fn secondary_count_244(&mut self)->Result<i32>{self.count(true)}
 fn source_word_23c(&mut self)->Result<u32>{let value=self.input["source_word"].as_u64().unwrap()as u32;self.event(json!(["source-word",value]))?;Ok(value)}
}
#[test]
fn actual_453f10_tail_known_fields_growable_lists_and_required_source_prefixes(){
 let doc:Value=serde_json::from_str(include_str!("../../../research/development10/view-actor-fields/native-actor-fields.json")).unwrap();let mut rows=0;
 for row in doc["rows"].as_array().unwrap(){
  let mut source=Source{input:&row["input"],events:vec![],globals:0,turrets:0,secondary:0};
  let(fields,error)=match Fields::construct(&mut source){Ok(f)=>(f,None),Err(f)=>(f.fields,Some(f.error.to_string()))};
  assert_eq!(error.as_deref().unwrap_or("ok"),row["output"]["outcome"].as_str().unwrap(),"row{rows}");
  assert_eq!(json!(source.events),row["output"]["events"],"source order row{rows}");
  for(offset,value)in row["output"]["words"].as_object().unwrap(){let offset:usize=offset.parse().unwrap();assert_eq!(json!(fields.word(offset).unwrap()),*value,"row{rows} offset{offset:X}")}
  let mut arrays=vec![];
  let first=fields.turrets.as_ref().map(|array|json!({"allocated":array.allocated,"records":array.items.iter().map(|v|json!({"words":[v.words[0],v.words[1],v.enabled,v.pitch],"known":[u32::MAX,u32::MAX,255,u32::MAX]})).collect::<Vec<_>>()}));
  let second=fields.secondary.as_ref().map(|array|json!({"allocated":array.allocated,"records":array.items.iter().map(|v|json!({"words":[v.words[0],v.words[1],v.enabled],"known":[u32::MAX,u32::MAX,255]})).collect::<Vec<_>>()}));
  arrays.push(first.unwrap_or_else(||json!({"allocated":null,"records":[]})));arrays.push(second.unwrap_or_else(||json!({"allocated":null,"records":[]})));
  assert_eq!(json!(arrays),row["output"]["arrays"],"same lists row{rows}");
  assert_eq!(fields.require_constructed().is_ok(),error.is_none());rows+=1;
 }
 assert_eq!(rows,99);
}
#[test]
fn native_unwritten_fields_and_record_padding_are_not_zero_defaults(){
 let input=json!({"actor":true,"globals":[0,0,0,0,0,0,0,0,0],"turret_counts":[1],"secondary_counts":[1],"pitches":[0x80000000u32],"source_word":7,"fail":null});
 let mut source=Source{input:&input,events:vec![],globals:0,turrets:0,secondary:0};let fields=Fields::construct(&mut source).unwrap();
 for offset in (0xd4..0xf0).step_by(4){assert_eq!(fields.word(offset).unwrap(),None)}
 assert_eq!(fields.word(0x128).unwrap(),None);assert_eq!(fields.word(0x12c).unwrap(),Some(1));
 let turret=&fields.turrets().unwrap()[0];assert_eq!(turret.word(8).unwrap(),None);assert_eq!(turret.enabled(),0);assert_eq!(turret.word(12).unwrap(),Some(0x80000000));
 let secondary=&fields.secondary().unwrap()[0];assert_eq!(secondary.word(8).unwrap(),None);assert_eq!(secondary.enabled(),0);
}
