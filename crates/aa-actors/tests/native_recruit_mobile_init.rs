use aa_actors::{actor_creation::{Error,Result},component_virtuals::{self as native,RecruitInitWorld,MobileInitWorld}};
use aa_numeric::Precision;
use serde_json::{Value,json};
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/skirmish-20261017-stored-recruit-mobile/native-recruit-mobile.json")).unwrap()}
fn pc(s:&str)->Precision{match s{"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!("{s}")}}
struct Recruit{targets:Vec<u32>,bytes:[u8;2],writes:Vec<u32>}
impl RecruitInitWorld for Recruit{
    type Actor=();type Target=u32;
    fn recruit_init_targets(&mut self,_:())->Result<Vec<u32>>{Ok(self.targets.clone())}
    fn recruit_init_location(&mut self,t:&u32)->Result<bool>{Ok(*t!=0)}
    fn recruit_init_store(&mut self,_:(),o:u32,v:u8)->Result<()>{self.bytes[(o-0x10)as usize]=v;self.writes.push(o);Ok(())}
}
#[test]
fn whole_native_recruit_byte_stores_keep_order_and_prior_values(){
    for row in fixture()["rows"].as_array().unwrap(){for c in row["output"]["recruit"].as_array().unwrap(){
        let mut w=Recruit{targets:c["targets"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect(),bytes:[c["prior"][0].as_u64().unwrap()as u8,c["prior"][1].as_u64().unwrap()as u8],writes:vec![]};
        assert!(native::init_recruit((),&mut w).unwrap());assert_eq!(json!(w.bytes),c["bytes"]);assert_eq!(json!(w.writes),c["writes"]);assert_eq!(c["result"],1);
    }}
}
struct Mobile{low:u32,high:u32,delay:u32,begin:u32,word14:u32,mode:String,calls:Vec<Value>}
impl MobileInitWorld for Mobile{
    type Actor=();
    fn mobile_init_active(&mut self,_:())->Result<bool>{Ok(self.low&(1<<26)!=0||self.high&16!=0)}
    fn mobile_init_delay(&mut self,_:())->Result<u32>{Ok(self.delay)}
    fn mobile_init_schedule(&mut self,_:(),delay:u32)->Result<()>{self.calls.push(json!({"delay":delay,"data":0}));if self.mode=="required"{return Err(Error::World("queue".into()))}if self.mode=="mutate"{self.begin=0x40600000;self.delay=0x40000000;}Ok(())}
    fn mobile_init_begin(&mut self)->Result<u32>{Ok(self.begin)}
    fn mobile_init_store(&mut self,_:(),v:u32)->Result<()>{self.word14=v;Ok(())}
}
#[test]
fn whole_native_mobile_scheduling_prefix_and_source_capture_match_all_pc(){
    for row in fixture()["rows"].as_array().unwrap(){assert_eq!(row["output"]["source554_constructor"],0);assert_eq!(row["output"]["unpacking_source_name"],"unpacking_length");let p=pc(row["precision"].as_str().unwrap());for c in row["output"]["mobile"].as_array().unwrap(){
        let mut w=Mobile{low:c["low"].as_u64().unwrap()as u32,high:c["high"].as_u64().unwrap()as u32,delay:c["delay"].as_u64().unwrap()as u32,begin:c["initial_begin"].as_u64().unwrap()as u32,word14:0xbf800000,mode:c["mode"].as_str().unwrap().into(),calls:vec![]};
        let result=native::init_mobile_deployment((),&mut w,p);assert_eq!(result.is_err(),c["status"]!="ok");assert_eq!(json!(w.calls),c["calls"]);assert_eq!(json!(w.word14),c["word14"]);assert_eq!(json!(w.begin),c["begin"]);assert_eq!(json!(w.delay),c["definition_delay"]);
    }}
}
