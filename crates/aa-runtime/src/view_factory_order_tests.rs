use super::*;
use serde_json::{Value,json};
use std::collections::BTreeMap;
struct Replay<'a>{c:&'a Value,table:&'a Value,events:Vec<Value>,links:BTreeMap<u32,Value>,
    views:Vec<Value>,allocations:Vec<u32>,updates:Vec<Value>,tail:Value,attached:bool,
    kingdom_count:i32,final_counts:Value,registry_started:bool}
impl Replay<'_>{
 fn event(&mut self,e:Value)->Result<()>{let fail=self.c["fail"].as_str()==e[0].as_str();self.events.push(e.clone());if fail{Err(e[0].as_str().unwrap().into())}else{Ok(())}}
 fn row(&self,id:u32)->&Value{self.c["things"].as_array().unwrap().iter().find(|r|((r["generation"].as_u64().unwrap()as u32)<<16)|r["slot"].as_u64().unwrap()as u32==id).unwrap()}
 fn name(view:usize)->Value{json!(format!("v{}",view+1))}
 fn projection(&self,stopped:Option<String>)->Value{json!({"events":self.events,"links":self.links,"attached":self.attached,
  "update_order":self.updates,"tail":self.tail,"allocations":self.allocations,"views":self.views,
  "stopped":stopped,"final":self.final_counts,"registry_started":self.registry_started})}
}
impl Fields for Replay<'_>{
 type Thing=u32;type View=usize;
 fn begin_diagnostic(&mut self)->Result<()>{self.event(json!(["begin"]))}
 fn attach_view(&mut self)->Result<()>{self.event(json!(["attach"]))?;self.attached=self.c["view"].as_bool().unwrap();Ok(())}
 fn kingdom_count(&mut self)->Result<i32>{self.event(json!(["kingdom_count",self.kingdom_count]))?;Ok(self.kingdom_count)}
 fn kingdom(&mut self,i:i32)->Result<Option<u32>>{let x=self.c["kingdoms"][i as usize].as_u64().map(|x|(2<<16)|x as u32);self.event(json!(["kingdom",i,x]))?;Ok(x)}
 fn world_phase(&mut self)->Result<u32>{let phase=self.c["phase"].as_u64().unwrap()as u32;self.event(json!(["phase",phase]))?;Ok(phase)}
 fn attached_view_present(&mut self)->Result<bool>{self.event(json!(["attached",self.attached]))?;Ok(self.attached)}
 fn source_view_type(&mut self,id:u32)->Result<u32>{let r=self.row(id);let kind=if r["mask"].as_u64().unwrap()&4!=0{4}else{r["kind"].as_u64().unwrap()as u32};self.event(json!(["source",id,kind]))?;Ok(kind)}
 fn construct_view(&mut self,id:u32)->Result<Option<usize>>{
  self.event(json!(["factory",id]))?;let kind=self.source_view_type(id)?;
  let Some(size)=self.table[kind.to_string()]["size"].as_u64().map(|x|x as u32)else{return Ok(None)};
  self.allocations.push(size);if self.c["allocation_null"].as_bool()==Some(true){return Ok(None)}
  let key=self.views.len();self.views.push(json!({"label":Self::name(key),"actor":id,"size":size,"flags":0xcdcdcdcdu32}));
  self.event(json!(["construct",Self::name(key),kind,size]))?;
  self.views[key]["flags"]=self.row(id)["flags"].clone();Ok(Some(key))
 }
 fn initialize_view(&mut self,key:usize)->Result<bool>{self.event(json!(["init",Self::name(key)]))?;
  if let Some(count)=self.c["after_init_count"].as_i64(){self.kingdom_count=count as i32;}
  Ok(self.row(self.views[key]["actor"].as_u64().unwrap()as u32)["init"].as_bool().unwrap())}
 fn destroy_view(&mut self,key:usize,free:u32)->Result<()>{self.event(json!(["destroy",Self::name(key),free]))}
 fn view_flags(&mut self,key:usize)->Result<u32>{Ok(self.views[key]["flags"].as_u64().unwrap()as u32)}
 fn prepend_update_view(&mut self,key:usize)->Result<()>{self.event(json!(["update",Self::name(key)]))?;if self.tail.is_null(){self.tail=Self::name(key)}self.updates.insert(0,Self::name(key));Ok(())}
 fn publish_view(&mut self,id:u32,key:Option<usize>)->Result<()>{let value=key.map(Self::name).unwrap_or(Value::Null);self.event(json!(["publish",id,value]))?;self.links.insert(id,value);Ok(())}
 fn next_thing(&mut self,cursor:&mut u32)->Result<Option<u32>>{self.registry_started=true;self.event(json!(["next",*cursor]))?;
  let first=if *cursor==0{1}else{(*cursor&0xffff)+1};let next=self.links.keys().filter(|&&id|id&0xffff>=first).min_by_key(|&&id|id&0xffff).copied();*cursor=next.unwrap_or(0);Ok(next)}
 fn thing_has_view(&mut self,id:u32)->Result<bool>{let has=!self.links[&id].is_null();self.event(json!(["has",id,has]))?;Ok(has)}
 fn end_diagnostic(&mut self,existing:i32,attempted:i32)->Result<()>{self.final_counts=json!([existing,attempted]);self.event(json!(["final",existing,attempted]))}
}
#[test]
fn actual_original_v14_factory_order_source_rereads_publication_and_required_prefixes(){
 let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-view-factory/native-view-factory.json")).unwrap();
 assert_eq!(fixture["rows"].as_array().unwrap().len(),195);
 for row in fixture["rows"].as_array().unwrap(){let c=&row["input"];let old=c["old_update"].as_bool().unwrap();
  let links=c["things"].as_array().unwrap().iter().map(|r|{let id=((r["generation"].as_u64().unwrap()as u32)<<16)|r["slot"].as_u64().unwrap()as u32;(id,if r["link"]==0{Value::Null}else{r["link"].clone()})}).collect();
  let mut fields=Replay{c,table:&fixture["factory_table"],events:vec![],links,views:vec![],allocations:vec![],updates:if old{vec![json!("prior")]}else{vec![]},
   tail:if old{json!("prior")}else{Value::Null},attached:false,kingdom_count:c["count"].as_i64().unwrap_or(c["kingdoms"].as_array().unwrap().len()as i64)as i32,
   final_counts:Value::Null,registry_started:false};
  let mut state=State::not_attempted();let result=state.run(&mut fields);
  let error=result.as_ref().err().map(|e|if e.contains("43C426"){"null43C426".into()}else{e.clone()});
  assert_eq!(fields.projection(error),row["output"],"{} {}",row["precision"],c["name"]);
  assert_eq!(state.traversal_completed(),result.is_ok());
  if let Ok(report)=result{assert_eq!(json!([report.existing,report.attempted_missing]),row["output"]["final"]);assert_eq!(state.stage(),Stage::Complete);}
  let before=fields.projection(None);assert!(state.run(&mut fields).unwrap_err().contains("already attempted"));assert_eq!(before,fields.projection(None));
 }
}
