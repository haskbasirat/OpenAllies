use super::*;
use serde_json::{Value,json};

struct Explicit<'a>(&'a Value);
fn word(v:&Value)->u32{v.as_u64().unwrap().try_into().unwrap()}
impl Read for Explicit<'_>{
    fn word(&self,offset:u32)->Result<u32>{Ok(match offset{
        0x170=>self.0["teams"].as_array().unwrap().len()as u32,
        0x14c=>self.0["kingdoms"].as_array().unwrap().len()as u32,
        0x290=>if self.0["weather"].is_null(){0}else{word(&self.0["weather"])},
        _=>word(&self.0["words"][format!("{offset:x}")]),
    })}
    fn rule_header(&self,field:RuleHeader)->Result<u32>{Ok(word(&self.0["rules"][match field{RuleHeader::Mode=>"kind",RuleHeader::GameType=>"condition"}]))}
    fn rule_count(&self)->usize{self.0["rules"]["rules"].as_array().unwrap().len()}
    fn rule(&self,index:usize,field:RuleField)->Result<u32>{
        let row=&self.0["rules"]["rules"][index];
        Ok(match field{RuleField::Definition=>word(&row["rule"]),RuleField::Locked=>u32::from(row["enabled"].as_bool().unwrap()),
            RuleField::Kind=>word(&row["kind"]),RuleField::Value=>word(&row["value"])})
    }
    fn team(&self,index:usize)->Result<Option<ActorId>>{Ok(self.0["teams"][index].as_u64().map(|v|ActorId::from_raw(v as u32)))}
    fn kingdom(&self,index:usize)->Result<Option<ActorId>>{Ok(self.0["kingdoms"][index].as_u64().map(|v|ActorId::from_raw(v as u32)))}
    fn thing(&self,id:ActorId,offset:u32)->Result<u32>{
        let row=self.0["things"].as_array().unwrap().iter().find(|r|word(&r["id"])==id.raw()).unwrap();
        Ok(match offset{0x14=>word(&row["id"]),0x1c=>word(&row["created"]),0x20|0x24|0x28=>word(&row["position"][((offset-0x20)/4)as usize]),_=>panic!("unbound Thing offset")})
    }
    fn things(&self)->impl Iterator<Item=ActorId> + '_{
        let mut ids=self.0["things"].as_array().unwrap().iter().map(|r|ActorId::from_raw(word(&r["id"]))).collect::<Vec<_>>();
        ids.sort_by_key(|id|id.slot());ids.into_iter()
    }
}
#[derive(Default)]
struct Log{events:Vec<Value>,stop:Option<usize>}
impl Log{
    fn add(&mut self,kind:&str,label:&str,value:Value)->Result<()>{
        if self.stop==Some(self.events.len()){return Err("explicit native logger cutoff".into())}
        self.events.push(json!({"kind":kind,"label":label,"value":value}));Ok(())
    }
}
impl Sink for Log{
    fn marker(&mut self,l:&str)->Result<()>{self.add("marker",l,json!(0))}
    fn int(&mut self,l:&str,v:u32)->Result<()>{self.add("int",l,json!(v))}
    fn float(&mut self,l:&str,v:u32)->Result<()>{self.add("float",l,json!(v))}
    fn boolean(&mut self,l:&str,v:bool)->Result<()>{self.add("bool",l,json!(v))}
    fn text(&mut self,l:&str,v:&[u16])->Result<()>{self.add("text",l,json!(v))}
}
fn fixture()->Value{serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/../../research/development10/native-initial-world-sync.json")).unwrap()).unwrap()}
fn precision(row:&Value)->Precision{match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!("unknown native precision")}}
#[test]
fn full_native_sync_traversal_rng_slot_order_and_error_prefixes(){
    let fixture=fixture();assert_eq!(fixture["rows"].as_array().unwrap().len(),84);
    for(row_index,row)in fixture["rows"].as_array().unwrap().iter().enumerate(){
        let mut rng=NativeRng::new(word(&row["input"]["rng"]));
        let mut log=Log{stop:row["stop"].as_u64().map(|v|v as usize),..Log::default()};
        let result=traverse(&Explicit(&row["input"]),&mut rng,precision(row),"g_gworld",&mut log);
        assert_eq!(result.is_err(),row["output"]["stopped"].as_bool().unwrap(),"row{row_index}");
        assert_eq!(json!(log.events),row["output"]["events"],"row{row_index}");
        assert_eq!(rng.state,word(&row["output"]["rng"]),"row{row_index}");
    }
}
#[test]
fn disabled_actual_sync_owner_still_advances_authoritative_rng_once(){
    use crate::live_clocked::SyncLog;
    let fixture=fixture();
    for row in fixture["rows"].as_array().unwrap().iter().filter(|r|r["stop"].is_null()){
        let mut actual=aa_orders::net::sync::Synchronizer::new(64);
        assert!(!actual.enabled);let mut gate_calls=0;
        let mut gate=||{gate_calls+=1;Err("disabled native logger must not reach gate".into())};
        let mut rng=NativeRng::new(word(&row["input"]["rng"]));
        {let mut binding=SyncBinding::Direct(SyncLog{owner:&mut actual,gate:&mut gate});
         traverse(&Explicit(&row["input"]),&mut rng,precision(row),"g_gworld",&mut SyncSink{binding:&mut binding}).unwrap();}
        assert_eq!(gate_calls,0);assert_eq!(rng.state,word(&row["output"]["rng"]));
    }
}
#[test]
fn constructor_counters_and_versioned_native_load_stores(){
    let native:Value=serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/../../research/development10/native-world-counters.json")).unwrap()).unwrap();
    assert_eq!(native["rows"].as_array().unwrap().len(),18);
    for row in native["rows"].as_array().unwrap(){
        for offset in[0x140,0x184,0x194,0x198]{assert_eq!(word(&row["output"]["counters"][format!("{offset:x}")]),0);}
    }
    for version in[1,3,4,25,26,27,28]{
        let input=aa_mapworld::WorldState{
            source:aa_mapworld::SourceIdentity::of(&[]),version,rng_state:0,word_dc:0,begin_time_bits:0,word_e4:0,delta_bits:0,
            word_140:0x7fffffff,word_184:(version>=4).then_some(3),word_194:(version>=4).then_some(5),word_198:(version>=27).then_some(7),
            word_44:None,word_48:None,trailing:vec![],
        };
        let retained=Counters::from_loaded_world(&input);
        assert_eq!(retained.read(0x140).unwrap(),0x7fffffff);
        assert_eq!(retained.read(0x184).unwrap(),if version>=4{3}else{0});
        assert_eq!(retained.read(0x194).unwrap(),if version>=4{5}else{0});
        assert_eq!(retained.read(0x198).unwrap(),if version>=27{7}else{0});
    }
}
