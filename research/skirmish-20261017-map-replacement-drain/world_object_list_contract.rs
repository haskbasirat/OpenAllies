#[path="../../crates/aa-runtime/src/world_object_list.rs"]
mod production;
use production::{WorldObjectList,NodeId,PreviousLink};
use serde_json::{Value,json};
#[test]
fn same_production_list_matches_native_intrusive_unlink_and_stable_ids(){
    let doc:Value=serde_json::from_str(include_str!("native-removal.json")).unwrap();
    let mut rows=0;
    for row in doc["rows"].as_array().unwrap().iter().filter(|r|r["kind"]=="intrusive") {
        let input=&row["input"];let mut list=WorldObjectList::constructed();let mut ids=vec![];
        for value in input["values"].as_array().unwrap().iter().rev(){ids.push(list.insert(if value=="old"{7u32}else{8u32},1<<8).unwrap().unwrap());}
        ids.reverse();let removed=list.remove_first(7,input["flags"].as_u64().unwrap()as u32).unwrap();
        let mut values=vec![];let mut identities=vec![];let mut current=list.head();let mut previous=None::<NodeId>;
        while let Some(id)=current {
            let node=list.node(id).unwrap();assert_eq!(node.previous_link,previous.map(PreviousLink::Next).unwrap_or(PreviousLink::Head));
            values.push(if node.object==7{"old"}else{"other"});identities.push(ids.iter().position(|old|*old==id).unwrap()+1);previous=Some(id);current=node.next;
        }
        assert_eq!(json!(values),row["output"]["values"],"{row}");assert_eq!(json!(identities),row["output"]["identities"],"{row}");
        assert_eq!(removed,!row["output"]["trace"].as_array().unwrap().is_empty());
        if removed{let i=input["values"].as_array().unwrap().iter().position(|v|v=="old").unwrap();assert!(list.node(ids[i]).is_none());}
        assert_eq!(list.nodes().len(),ids.len());rows+=1;
    }
    assert_eq!(rows,72);
    //Later insertion follows the current head; freed identities never revive.
    let mut list=WorldObjectList::constructed();let first=list.insert(7,1<<8).unwrap().unwrap();list.remove_first(7,1<<8).unwrap();
    let next=list.insert(9,1<<8).unwrap().unwrap();assert_ne!(first,next);assert!(list.node(first).is_none());assert_eq!(next.ordinal(),2);
}
