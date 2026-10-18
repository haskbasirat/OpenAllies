use super::*;
use serde_json::{Value,json};
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/skirmish-20261017-object-animation/native-actor-animation.json")).unwrap()}
struct Input<'a>{input:&'a Value,trace:Vec<String>}
impl Input<'_>{
    fn read(&mut self,key:&str)->Result<()>{let call=format!("read:{key}");self.trace.push(call.clone());if self.input["stop"]==call{Err(format!("required:{call}"))}else{Ok(())}}
    fn children(&mut self,key:&str)->Result<()>{self.trace.push(format!("children:{key}"));if self.input["stop"]==key{Err(format!("required:{key}"))}else{Ok(())}}
}
impl Source<usize,Vec<()>> for Input<'_>{
    fn string(&mut self,key:&str)->Result<String>{self.read(key)?;Ok(self.input[key].as_str().unwrap().into())}
    fn resource(&mut self,path:&str)->Result<Option<usize>>{
        let path=if path.starts_with('/') {path.trim_start_matches('/').into()}else{format!("{}/{path}",self.input["folder"].as_str().unwrap())};
        self.trace.push(format!("relative:{path}"));self.trace.push(format!("sequence:{path}"));
        if self.input["stop"]=="sequence"{Err("required:sequence".into())}else{Ok(self.input["resource"].as_bool().unwrap().then_some(7))}
    }
    fn fixed(&mut self,key:&str)->Result<u32>{self.read(key)?;Ok(self.input[key].as_u64().unwrap()as u32)}
    fn boolean(&mut self,key:&str)->Result<u8>{self.read(key)?;Ok(self.input[key].as_u64().unwrap()as u8)}
    fn integer(&mut self,key:&str)->Result<i32>{self.read(key)?;Ok(self.input[key].as_u64().unwrap()as u32 as i32)}
    fn resolve_kind(&mut self,text:&str)->Result<i32>{Ok(registered_kind(text))}
    fn validate_damage_filter(&mut self,_:i32)->Result<()>{Err("required actual DamageFilter registry".into())}
    fn begin_fx(&mut self)->Result<Vec<()>>{Ok(vec![])}
    fn append_fx(&mut self,context:&mut Vec<()>,list:FxList)->Result<usize>{
        let FxList::Standard(index)=list else{self.trace.push("timedFX".into());return Err("required:timedFX".into())};
        self.children(["WhenStart","WhenDo","WhenEnd"][index])?;let key=context.len();context.push(());Ok(key)
    }
    fn permanent(&mut self,_:&Vec<()>,_:usize)->Result<bool>{Ok(false)}
    fn timed_count(&mut self)->Result<usize>{self.children("WhenDoAtTime")?;Ok(self.input["times"].as_array().map_or(0,Vec::len))}
    fn timed_time(&mut self,index:usize)->Result<u32>{self.read("time")?;Ok(self.input["times"][index].as_u64().unwrap()as u32)}
    fn finish_fx(&mut self,_:&mut Vec<()>)->Result<()>{Ok(())}
}
fn fields(a:&Animation<usize,Vec<()>>)->Value{json!({"0x4":a.ordinal,"0x8":a.kind,"0x10":a.length,"0x14":a.key_ratio,"0x1c":a.sister as u32,"0x20":a.weight,"0x24":a.damage_threshold,"0x28":a.damage_filter as u32,"0x2c":a.walk_distance,"0x30":a.move_threshold,"ids":a.ids,"blend":a.blend,"same_resource":a.resource==Some(7),"resource_present":a.resource.is_some(),"fx_counts":[0,0,0],"timed_present":!a.timed.is_empty(),"timed":a.timed.iter().map(|cue|json!({"time":cue.time,"fx_present":cue.list.is_some()})).collect::<Vec<_>>()})}
#[test]
fn actual_actor_source_appends_before_fields_and_preserves_errors_all_pc(){
    for row in fixture()["rows"].as_array().unwrap(){
        let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,_=>Precision::Extended64};
        for alias in row["output"]["aliases"].as_array().unwrap(){assert_eq!(registered_kind(alias["name"].as_str().unwrap()),alias["kind"].as_i64().unwrap()as i32);}
        for case in row["output"]["loads"].as_array().unwrap(){
            let mut set=Set::constructed();let input=&case["input"];let mut source=Input{input,trace:vec!["children:Animation".into()]};
            if input["seed"].is_object(){let seed=&input["seed"];set.load(seed["prefix"].as_str().unwrap(),false,&mut Input{input:seed,trace:vec![]},pc).unwrap();}
            let result=set.load(input["prefix"].as_str().unwrap(),input["inherit"].as_bool().unwrap_or(false),&mut source,pc);
            let status=match result{Ok(())=>"ok".into(),Err(e)if e.starts_with("410C40")=>{source.trace.push("diagnostic".into());"required:diagnostic".into()},Err(e)=>e};
            assert_eq!(status,case["status"],"{case}");assert_eq!(json!(source.trace),case["trace"],"{case}");
            assert_eq!(set.rows.len(),case["count"].as_u64().unwrap()as usize);
            assert_eq!(set.rows.last().map(fields).unwrap_or(Value::Null),case["fields"],"{case}");
            assert_eq!(json!(set.by_type.iter().map(Vec::len).collect::<Vec<_>>()),case["typed_counts"]);
            assert_eq!(json!(set.weights),case["weights"]);
        }
    }
}
#[test]
fn actual_set_finalizer_required_kinds_authored_moves_and_death_gate_all_pc(){
    for row in fixture()["rows"].as_array().unwrap(){
        let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,_=>Precision::Extended64};
        for case in row["output"]["finalizers"].as_array().unwrap(){
            let mut set=Set::<usize,Vec<()>>::constructed();
            for (i,kind)in case["kinds"].as_array().unwrap().iter().enumerate(){
                let kind=kind.as_u64().unwrap()as usize;set.by_type[kind].push(i);
                //Explicit constructor/typed-field transport, exactly the
                //native finalizer fixture's input, not a source-ready claim.
                set.rows.push(Animation{ordinal:i,ids:format!("animation{i}"),kind,resource:None,length:0,key_ratio:0x3f000000,walk_distance:case["walks"][i].as_u64().unwrap()as u32,blend:0,sister:-1,weight:0x3f800000,damage_threshold:if kind==3{case["death_threshold"].as_u64().unwrap()as u32}else{0},damage_filter:-1,move_threshold:0,fx:None,lists:[None;3],timed:vec![],complete:false});
            }
            let result=set.finalize(pc);
            assert_eq!(if result.is_ok(){"ok"}else{"required:diagnostic"},case["status"]);
            assert_eq!(json!(set.rows.iter().map(|a|a.move_threshold).collect::<Vec<_>>()),case["move_thresholds"],"{case}");
        }
    }
}
#[test]
fn actual_animation_request_literal_zero_priority_and_linked_cues_all_pc(){
    for row in fixture()["rows"].as_array().unwrap(){for case in row["output"]["requests"].as_array().unwrap(){
        let count=case["timed_count"].as_u64().unwrap()as usize;
        let a=Animation{ordinal:0,ids:String::new(),kind:0,resource:case["present"].as_bool().unwrap().then_some(7usize),length:0,key_ratio:0,walk_distance:0,blend:0,sister:-1,weight:0,damage_threshold:0,damage_filter:-1,move_threshold:0,fx:Some(Vec::<()>::new()),lists:[Some(0),Some(1),Some(2)],timed:(0..count).map(|i|Cue{time:Some(1),list:Some(i+3)}).collect(),complete:true};
        let mut calls=vec![];
        a.request(case["delta"].as_u64().unwrap()as u32,|call,delta,priority|{
            calls.push(match call{Request::Resource(_)=>json!({"kind":"sequence","delta":delta,"priority":priority}),Request::Fx(_,key)=>json!({"kind":"fx","list":key,"delta":delta,"priority":priority})});Ok(())
        }).unwrap();
        assert_eq!(json!(calls),case["calls"]);
    }}
}
