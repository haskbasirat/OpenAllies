use super::*;
use serde_json::{Value,json};
fn fixture()->Value{let value:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-actor-request/native-actor-request.json")).unwrap();assert_eq!(value["rows"].as_array().unwrap().len(),3);value}
#[test]
fn native_registered_request_list_names(){
    for row in fixture()["rows"].as_array().unwrap(){
        for offset in PAIR_OFFSETS{for word in [offset,offset+4]{assert_eq!(row["output"]["constructor"][format!("0x{word:x}")],0);}}
        for (key,group)in [("mover",Group::Mover),("body",Group::Body),("portal",Group::Portal),("ability",Group::Ability)]{
            assert_eq!(json!(group.names()),row["output"]["names"][key]);
            assert!(group.base()>0);let _=group.component();
        }
    }
}
#[test]
fn linked_ability_five_lists_and_error_prefix_match_native(){
    for row in fixture()["rows"].as_array().unwrap(){for case in row["output"]["requests"].as_array().unwrap(){
        let input=&case["input"];let delta=input["delta"].as_u64().unwrap()as u32;let mode=input["mode"].as_u64().unwrap()as u32;
        let stop=input["stop"].as_i64().unwrap();let mut calls=vec![];
        let result=request_ability(delta,mode,|offset,d,m|{let index=(offset-0x70)/8;calls.push(json!({"list":index,"delta":d,"mode":m}));if index as i64==stop{Err("required".into())}else{Ok(())}});
        assert_eq!(json!(calls),case["calls"]);assert_eq!(result.is_err(),case["status"]!="ok");
    }}
}
#[test]
fn component_context_order_self_health_synthesis_and_prefix_match_native(){
    for row in fixture()["rows"].as_array().unwrap(){for case in row["output"]["pairs"].as_array().unwrap(){
        let group=match case["group"].as_str().unwrap(){"mover"=>Group::Mover,"body"=>Group::Body,"portal"=>Group::Portal,"ability"=>Group::Ability,"root"=>continue,_=>panic!()};
        let stop=case["stop"].as_str();let mut calls=vec![];
        let result=compile_group(group,|op|match op{
            Compile::Load(name)=>{calls.push(json!(format!("children:{name}")));if stop==Some(name){calls.push(json!("positiveFX"));Err("required".into())}else{Ok(())}},
            Compile::Synthesize{source,destination}=>{calls.push(json!({"synthesize":[group.base()+source as u32*8,group.base()+destination as u32*8],"mode":0}));Ok(())},
            Compile::ValidateHealth=>Ok(()),
        });
        assert_eq!(json!(calls),case["trace"]);assert_eq!(result.is_err(),case["status"]!="ok");assert_eq!(case["same_context"],true);
    }}
}
#[test]
fn when_health_signed_trigger_validation_matches_native(){
    for row in fixture()["rows"].as_array().unwrap(){for case in row["output"]["health"].as_array().unwrap(){
        assert_eq!(validate_health(case["trigger"].as_u64().unwrap()as u32).is_err(),case["status"]!="ok");
    }}
}
