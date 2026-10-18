use super::*;
use aa_triggers::decode::{Parameter,SavedManager,Span,Trigger};
use serde_json::{Value as Json,json};
use std::collections::BTreeMap;
fn span()->Span{Span{offset:0,length:0}}
fn manager(graph:&Json)->Manager{
    Manager::from_saved(SavedManager{
        next_id:1,variables:vec![],strings:vec![],
        triggers:graph.as_array().unwrap().iter().enumerate().map(|(i,actions)|Trigger{
            source:span(),id:i as u32,mode:0,enabled:1,repeat:0,activation_bits:0,
            name:vec![],description:vec![],word_2c:0,events:vec![],conditions:vec![],
            actions:actions.as_array().unwrap().iter().map(|a|{
                let opcode=a["opcode"].as_u64().unwrap() as u32;
                Element{source:span(),name:format!("lab{opcode}"),opcode,word_10:0,
                    //Native nonqualifying rows have NULL parameters. Keep
                    //the Rust owner empty too, so eager reads fail the test.
                    parameters:if (82..=84).contains(&opcode){vec![Parameter{
                        source:span(),argument:String::new(),item:String::new(),item_index:0,
                        storage:0,flag:0,value:Value::Text(a["ids"].as_str().unwrap().encode_utf16().collect()),
                    }]}else{vec![]},
                }
            }).collect(),
        }).collect(),
    })
}
#[test]
fn original_v14_collector_duplicate_head_order_clear_and_partial_lookup_prefix(){
    let fixture:Json=serde_json::from_str(include_str!("../../../research/development10/view-init-world/native-trigger-vfx.json")).unwrap();
    let mut rows=0;
    for row in fixture["rows"].as_array().unwrap(){
        let input=&row["input"];
        let manager=manager(&input["triggers"]);
        let mut list=VecDeque::from(vec![String::from("old");input["old_nodes"].as_u64().unwrap() as usize]);
        let mut events=Vec::new();let mut lookup=0usize;
        let result=collect(&mut list,&manager,|ids|{
            let stop=input["stop_lookup"].as_u64().map(|i|i as usize);
            if stop==Some(lookup){return Err("lookup".into())}
            lookup+=1;
            let ids=String::from_utf16(ids).unwrap();events.push(format!("lookup:{ids}"));
            Ok(match ids.as_str(){"one"|"two"|"model"=>Some(ids),_=>None})
        });
        let outcome=if result.is_ok(){"complete"}else{"error:lookup"};
        let mut counts:BTreeMap<String,u32>=["one","two","not_vfx","model"].into_iter().map(|s|(s.into(),0xffff_fffe)).collect();
        let mut requested=0;
        if result.is_ok(){
            //This private lab request applies the fixture's declared NULL
            //resource40 child. Production keeps that current source owner
            //required; these rows do not establish actual source-nullness.
            request(&list,&mut requested,|effect|{
                events.push(format!("request:{effect}"));
                let count=counts.get_mut(effect).unwrap();*count=count.wrapping_add(1);Ok(())
            }).unwrap();
            assert_eq!(requested,list.len());
        }
        assert_eq!(json!({"outcome":outcome,"events":events,"list":list,"counts":counts,"ad":0}),row["output"],"{row}");
        rows+=1;
    }
    assert_eq!(rows,45);
}
#[test]
fn request_error_keeps_committed_child_prefix_and_counts_only_completed_occurrences(){
    let entries=VecDeque::from(["one","two","one"]);let mut completed=0;let mut calls=Vec::new();
    let result=request(&entries,&mut completed,|id|{calls.push(*id);if *id=="two"{Err("required child AFTER committed base".into())}else{Ok(())}});
    assert!(result.is_err());assert_eq!(calls,["one","two"]);assert_eq!(completed,1);
}
