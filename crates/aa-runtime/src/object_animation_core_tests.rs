use super::*;
use serde_json::{Value,json};
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/skirmish-20261017-object-animation/native-object-animation.json")).unwrap()}
fn children_fixture()->Value{serde_json::from_str(include_str!("../../../research/development10/object-animation-children/native-object-animation-children.json")).unwrap()}
///Explicit oracle seams: typed getters, child count/iterator, the three
///DataManager Finds and the manager+20 name are fixture inputs, recorded in
///the same order and spelling as the native producer.
struct Input<'a>{input:&'a Value,trace:Vec<String>}
impl Input<'_>{
    fn stop(&self,entry:&str)->Result<()>{if self.input["stop"]==entry{Err(format!("required:{entry}"))}else{Ok(())}}
    fn read(&mut self,key:&str)->Result<()>{
        let call=format!("read:{key}");self.trace.push(call.clone());self.stop(&call)
    }
    fn child_read(&mut self,child:&(String,usize),key:&str)->Result<&Value>{
        let call=format!("read:{}[{}]:{key}",child.0,child.1);self.trace.push(call.clone());self.stop(&call)?;
        Ok(&self.input["child_values"][&child.0][child.1][key])
    }
}
impl Source<usize> for Input<'_>{
    type Audio=String;type Vfx=String;type Child=(String,usize);
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
    fn count(&mut self,name:&str)->Result<usize>{
        self.trace.push(format!("children:{name}"));self.stop(name)?;
        Ok(self.input["children"][name].as_u64().unwrap_or(0)as usize)
    }
    fn child(&mut self,name:&str,index:usize)->Result<(String,usize)>{
        self.trace.push(format!("child:{name}:{index}"));Ok((name.into(),index))
    }
    fn child_string(&mut self,child:&(String,usize),key:&str)->Result<String>{Ok(self.child_read(child,key)?.as_str().unwrap().into())}
    fn child_boolean(&mut self,child:&(String,usize),key:&str)->Result<u8>{Ok(self.child_read(child,key)?.as_u64().unwrap()as u8)}
    fn vfx(&mut self,ids:&str)->Result<Option<String>>{
        self.trace.push(format!("vfx:{ids}"));
        Ok(self.input["vfx"][ids].as_bool().unwrap_or(false).then(||ids.into()))
    }
    fn bone(&mut self,ids:&str)->Result<Option<i32>>{
        self.trace.push(format!("bone:{ids}"));
        Ok(self.input["bone"][ids].as_i64().map(|v|v as i32))
    }
    fn audio(&mut self,ids:&str)->Result<Option<String>>{
        let call=format!("audio:{ids}");self.trace.push(call.clone());self.stop(&call)?;
        Ok(self.input["audio"][ids].as_bool().unwrap_or(false).then(||ids.into()))
    }
    fn audio_manager(&mut self)->Result<String>{Ok(self.input["audio_manager"].as_str().unwrap().into())}
}
type Kernel=Core<usize,String,String>;
fn fields(a:&Animation<usize,String,String>)->Value{json!({"ordinal":a.ordinal,"ids":a.ids,"kind":a.kind,"length":a.length,"exclusive":a.exclusive,"start_min":a.start_min,"start_max":a.start_max,"same_resource":a.resource==Some(7)})}
fn with_children(a:&Animation<usize,String,String>)->Value{
    let mut out=fields(a);
    out["vfx"]=json!(a.vfx.iter().map(|v|json!({"vfx":v.vfx,"location":v.location})).collect::<Vec<_>>());
    out["sfx"]=json!(a.sfx.iter().map(|s|json!({"audio":s.audio,"looping":s.looping})).collect::<Vec<_>>());
    out
}
///Replays one native load case; returns the native-spelled status.
fn replay(core:&mut Kernel,case:&Value)->(String,Vec<String>){
    let input=&case["input"];let mut source=Input{input,trace:vec![]};
    let result=core.load(input["prefix"].as_str().unwrap(),&mut source);
    let status=match result{
        Ok(())=>"ok".into(),Err(e)if e.contains("duplicate IDS")=>{source.trace.push("duplicate".into());"required:duplicate".into()},
        Err(e)if e.contains("m_type>=0")=>{source.trace.push("assert".into());"required:assert".into()},
        Err(e)if e.starts_with("410C40 ")=>{source.trace.push("throw".into());format!("required:{}",&e["410C40 ".len()..])},
        Err(e)=>e,
    };
    (status,source.trace)
}
fn check_topology(core:&Kernel,case:&Value){
    assert_eq!(core.animations.len(),case["count"].as_u64().unwrap()as usize);
    assert_eq!(json!(core.by_type.iter().map(Vec::len).collect::<Vec<_>>()),case["typed_counts"]);
    for (kind,indices)in core.by_type.iter().enumerate(){for ordinal in indices{assert_eq!(core.animations[*ordinal].kind,kind as i32);assert_eq!(core.animations[*ordinal].ordinal,*ordinal)}}
}
#[test]
fn original_source_load_prefix_append_order_and_enum_all_pc(){
    for row in fixture()["rows"].as_array().unwrap(){
        let mut core=Kernel::constructed();
        assert!(core.animations.is_empty()&&core.pending.is_none()&&core.by_type.iter().all(Vec::is_empty));
        let ctor=&row["output"]["constructor"];assert_eq!(ctor["overall_count"],0);
        assert!(ctor["typed_count"].as_array().unwrap().iter().all(|v|v==0));
        for e in row["output"]["enum"].as_array().unwrap(){assert_eq!(object_kind(e["name"].as_str().unwrap()),e["kind"].as_i64().unwrap()as i32)}
        for case in row["output"]["loads"].as_array().unwrap(){
            //Native generated rows share only successful topology. Each
            //failed allocation is an observed prefix; production Set latches
            //the failure and cannot issue another source append on that Set.
            core.pending=None;
            assert_eq!(core.animations.len(),case["before"].as_u64().unwrap()as usize);
            let (status,trace)=replay(&mut core,case);
            assert_eq!(status,case["status"],"{case}");
            assert_eq!(json!(trace),case["trace"],"{case}");
            check_topology(&core,case);
            let actual=if case["pending"].is_null(){Value::Null}else if status=="ok"{fields(core.animations.last().unwrap())}else{fields(core.pending.as_ref().unwrap())};
            assert_eq!(actual,case["pending"],"{case}");
        }
    }
}
///49D0A0/49CF80 positive children: native append-before-read, empty and
///unknown IDS diagnostics, partial pair stores and later typed appends.
#[test]
fn original_positive_create_vfx_play_sfx_children_all_pc(){
    let doc=children_fixture();
    assert_eq!(doc["rows"].as_array().unwrap().len(),3);
    for row in doc["rows"].as_array().unwrap(){
        let mut core=Kernel::constructed();
        for e in row["output"]["enum"].as_array().unwrap(){assert_eq!(object_kind(e["name"].as_str().unwrap()),e["kind"].as_i64().unwrap()as i32)}
        let loads=row["output"]["loads"].as_array().unwrap();
        assert!(loads.iter().any(|c|c["status"]=="ok"&&c["pending"]["sfx"].as_array().is_some_and(|s|s.len()==5)));
        for case in loads{
            core.pending=None;
            assert_eq!(core.animations.len(),case["before"].as_u64().unwrap()as usize);
            let (status,trace)=replay(&mut core,case);
            assert_eq!(status,case["status"],"{}",case["input"]["ids"]);
            assert_eq!(json!(trace),case["trace"],"{}",case["input"]["ids"]);
            check_topology(&core,case);
            let actual=if case["pending"].is_null(){Value::Null}else if status=="ok"{with_children(core.animations.last().unwrap())}else{with_children(core.pending.as_ref().unwrap())};
            assert_eq!(actual,case["pending"],"{}",case["input"]["ids"]);
        }
    }
}
#[test]
fn original_request_nulls_current_priority_and_error_prefix_all_pc(){
    for row in fixture()["rows"].as_array().unwrap(){for case in row["output"]["requests"].as_array().unwrap(){
        let mut core=Kernel::constructed();
        for i in 0..4{core.animations.push(Animation{ordinal:i,ids:String::new(),kind:0,length:0,start_min:0,start_max:0,exclusive:0,resource:(i!=1).then_some(i),vfx:vec![],sfx:vec![]});}
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
