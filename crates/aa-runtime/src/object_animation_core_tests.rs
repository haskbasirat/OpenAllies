use super::*;
use serde_json::{Value,json};
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/skirmish-20261017-object-animation/native-object-animation.json")).unwrap()}
struct Input<'a>{input:&'a Value,trace:Vec<String>}
impl Input<'_>{
    fn read(&mut self,key:&str)->Result<()>{
        let call=format!("read:{key}");self.trace.push(call.clone());
        if self.input["stop"]==call{Err(format!("required:{call}"))}else{Ok(())}
    }
    fn child(&mut self,key:&str)->Result<()>{
        self.trace.push(format!("children:{key}"));
        if self.input["stop"]==key{Err(format!("required:{key}"))}else{Ok(())}
    }
}
impl Source<usize> for Input<'_>{
    fn string(&mut self,key:&str)->Result<String>{self.read(key)?;Ok(self.input[key].as_str().unwrap().into())}
    fn resource(&mut self,path:&str)->Result<Option<usize>>{
        //Explicit source-folder child transport, independently proven by
        //native-relative-source-path.json; not part of the generic kernel.
        let path=if path.starts_with('/') {path.trim_start_matches('/').into()}else{format!("{}/{path}",self.input["folder"].as_str().unwrap())};
        self.trace.push(format!("relative:{path}"));self.trace.push(format!("sequence:{path}"));
        if self.input["stop"]=="sequence"{return Err("required:sequence".into())}
        Ok(self.input["resource"].as_bool().unwrap().then_some(7))
    }
    fn float(&mut self,key:&str)->Result<u32>{self.read(key)?;Ok(self.input[key].as_u64().unwrap()as u32)}
    fn boolean(&mut self,key:&str)->Result<u8>{self.read(key)?;Ok(self.input[key].as_u64().unwrap()as u8)}
    fn create_vfx(&mut self)->Result<()>{self.child("CreateVFX")}
    fn play_sfx(&mut self)->Result<()>{self.child("PlaySFX")}
}
fn fields(a:&Animation<usize>)->Value{json!({"ordinal":a.ordinal,"ids":a.ids,"kind":a.kind,"length":a.length,"exclusive":a.exclusive,"start_min":a.start_min,"start_max":a.start_max,"same_resource":a.resource==Some(7)})}
#[test]
fn original_source_load_prefix_append_order_and_enum_all_pc(){
    for row in fixture()["rows"].as_array().unwrap(){
        let mut core=Core::<usize>::constructed();
        assert!(core.animations.is_empty()&&core.pending.is_none()&&core.by_type.iter().all(Vec::is_empty));
        let ctor=&row["output"]["constructor"];assert_eq!(ctor["overall_count"],0);
        assert!(ctor["typed_count"].as_array().unwrap().iter().all(|v|v==0));
        for e in row["output"]["enum"].as_array().unwrap(){assert_eq!(object_kind(e["name"].as_str().unwrap()),e["kind"].as_i64().unwrap()as i32)}
        for case in row["output"]["loads"].as_array().unwrap(){
            //Native generated rows share only successful topology. Each
            //failed allocation is an observed prefix; production Set latches
            //the failure and cannot issue another source append on that Set.
            core.pending=None;
            let input=&case["input"];let mut source=Input{input,trace:vec![]};
            assert_eq!(core.animations.len(),case["before"].as_u64().unwrap()as usize);
            let result=core.load(input["prefix"].as_str().unwrap(),&mut source);
            let status=match result{
                Ok(())=>"ok".into(),Err(e)if e.contains("duplicate IDS")=>{source.trace.push("duplicate".into());"required:duplicate".into()},
                Err(e)if e.contains("m_type>=0")=>{source.trace.push("assert".into());"required:assert".into()},Err(e)=>e,
            };
            assert_eq!(status,case["status"],"{case}");
            assert_eq!(json!(source.trace),case["trace"],"{case}");
            assert_eq!(core.animations.len(),case["count"].as_u64().unwrap()as usize);
            assert_eq!(json!(core.by_type.iter().map(Vec::len).collect::<Vec<_>>()),case["typed_counts"]);
            let actual=if case["pending"].is_null(){Value::Null}else if status=="ok"{fields(core.animations.last().unwrap())}else{fields(core.pending.as_ref().unwrap())};
            assert_eq!(actual,case["pending"],"{case}");
            for (kind,indices)in core.by_type.iter().enumerate(){for ordinal in indices{assert_eq!(core.animations[*ordinal].kind,kind as i32);assert_eq!(core.animations[*ordinal].ordinal,*ordinal)}}
        }
    }
}
#[test]
fn original_request_nulls_current_priority_and_error_prefix_all_pc(){
    for row in fixture()["rows"].as_array().unwrap(){for case in row["output"]["requests"].as_array().unwrap(){
        let mut core=Core::constructed();
        for i in 0..4{core.animations.push(Animation{ordinal:i,ids:String::new(),kind:0,length:0,start_min:0,start_max:0,exclusive:0,resource:(i!=1).then_some(i)});}
        let mut priority=1u32;let mut calls=vec![];
        let result=core.request(case["delta"].as_u64().unwrap()as u32,|index,delta|{
            calls.push(json!({"index":index,"delta":delta,"priority":priority}));
            if case["fail"].as_u64()==Some((calls.len()-1)as u64){return Err("required:resource".into())}
            priority=priority.wrapping_add(3);Ok(())
        });
        assert_eq!(json!(calls),case["calls"]);assert_eq!(priority,case["priority"]);
        assert_eq!(result.map(|_|"ok".to_owned()).unwrap_or_else(|e|e),case["status"]);
    }}
}
