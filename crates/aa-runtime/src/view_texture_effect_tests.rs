use super::*;
use serde_json::{json,Value};
struct Source{seeds:[u32;3],reads:Vec<[u32;2]>,fail:Option<usize>}
impl Globals for Source{
 fn word(&mut self,address:u32)->Result<u32>{
  let i=[0xb43cf8,0xb43cfc,0xb43d00].iter().position(|a|*a==address).unwrap();let value=self.seeds[i];self.reads.push([address,value]);
  if self.fail==Some(self.reads.len()){Err("required global".into())}else{Ok(value)}
 }
}
struct Memory{words:Vec<u32>,reads:Vec<u32>,writes:Vec<[u32;2]>,fail:Option<usize>}
impl Fields for Memory{
 fn word(&mut self,offset:u32)->Result<u32>{self.reads.push(offset);if self.fail==Some(self.reads.len()){Err("required field".into())}else{Ok(self.words[offset as usize/4])}}
 fn put(&mut self,offset:u32,value:u32){self.words[offset as usize/4]=value;self.writes.push([offset,value])}
}
impl UpdateFields for Memory{
 fn dispatch(&mut self,_slot:u32)->Result<()>{self.word(0)?;Ok(())}
}
#[test]
fn actual_fresh_effect_constructor_update_and_all_reached_read_prefixes(){
 let doc:Value=serde_json::from_str(include_str!("../../../research/development10/view-environment-projection/native-effect.json")).unwrap();let mut count=0;
 for row in doc["rows"].as_array().unwrap(){
  let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};
  let c=&row["input"];let mut source=Source{seeds:serde_json::from_value(c["seeds"].clone()).unwrap(),reads:vec![],fail:c["seed_fail"].as_u64().map(|n|n as usize)};
  let(core,constructor_ok)=match Core::from_native_constructor(&mut source){Ok(core)=>(core,true),Err(f)=>(f.core,false)};let mut core=core;
  assert_eq!(json!(source.reads),row["output"]["global_reads"],"globals row{count}");
  let mut outcome=if constructor_ok{"ok"}else{"required global"};
  let mut memory=None;
  if constructor_ok&&c["flow"]=="update"{
   for(i,value)in c["local"].as_array().unwrap().iter().enumerate(){core.put(0x38+4*i as u32,value.as_u64().unwrap()as u32)}
   core.configure_environment_43ea50().unwrap();core.project_mode2_86ba10(pc).unwrap();core.enable_after_projection_43ea50().unwrap();
   let mut words=vec![0xa5a5a5a5;100];for offset in doc["supported_offsets"].as_array().unwrap(){let offset=offset.as_u64().unwrap()as u32;words[offset as usize/4]=Core::word(&core,offset).unwrap()}
   let mut m=Memory{words,reads:vec![],writes:vec![],fail:c["read_fail"].as_u64().map(|n|n as usize)};
   let result=update_fresh_86e8d0(&mut m,c["time"].as_u64().unwrap()as u32,c["controllers"].as_u64().unwrap()as u8,pc);
   if result.is_err(){outcome="required field"}
   assert_eq!(json!(m.reads),row["output"]["reads"],"reads row{count}");assert_eq!(json!(m.writes),row["output"]["writes"],"writes row{count}");
   if c["read_fail"].is_null(){
    core.update_fresh_86e8d0(c["time"].as_u64().unwrap()as u32,c["controllers"].as_u64().unwrap()as u8,pc).unwrap();
    for offset in doc["supported_offsets"].as_array().unwrap(){let offset=offset.as_u64().unwrap()as u32;assert_eq!(Core::word(&core,offset).unwrap(),m.words[offset as usize/4],"same core row{count} offset{offset:X}")}
   }
   memory=Some(m);
  }
  assert_eq!(outcome,row["output"]["outcome"].as_str().unwrap(),"outcome row{count}");
  for(offset,expected)in row["output"]["words"].as_object().unwrap(){
   let offset:u32=offset.parse().unwrap();let current=if let Some(m)=&memory{Some(m.words[offset as usize/4])}else{Core::word(&core,offset).ok()};
   assert_eq!(json!(current),*expected,"word row{count} offset{offset:X}");
  }
  assert_eq!(json!(core.enabled_c4().ok()),row["output"]["enabled_c4"],"C4 row{count}");
  assert_eq!(core.require_constructor().is_ok(),constructor_ok);
  assert!(Core::word(&core,0xc8).is_err(),"native ABI identity is not a portable scalar");count+=1;
 }
 assert_eq!(count,234);
}
#[test]
fn positive_update_links_remain_required_before_their_real_child(){
 for(offset,expected_reads)in [(0xa4,vec![0,0xa4]),(0x10,vec![0,0xa4,0x10]),(0x24,vec![0,0xa4,0x10,0,0x24])]{
  let mut memory=Memory{words:vec![0;100],reads:vec![],writes:vec![],fail:None};memory.words[offset/4]=0x1234;
  assert!(update_fresh_86e8d0(&mut memory,0,1,Precision::Double53).is_err());assert_eq!(memory.reads,expected_reads);assert!(memory.writes.is_empty());
 }
}
