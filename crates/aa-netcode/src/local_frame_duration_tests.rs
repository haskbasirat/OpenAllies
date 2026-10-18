use super::*;
use serde_json::Value;
#[test]
fn native_duration_finite_rounding_same_ring_retirement_and_ratio_stores(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-frame-leaves/native-frame-duration.json")).unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(),792);
    for row in fixture["rows"].as_array().unwrap(){
        let c=&row["input"];let p=match row["precision"].as_str().unwrap(){"Single"=>Precision::Single24,"Double"=>Precision::Double53,"Extended"=>Precision::Extended64,_=>panic!()};
        let values:Vec<u32>=c["values"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap() as u32).collect();
        let mut owner=LocalBootstrap::from_native_constructors();
        let mut node=crate::local_host::Node::before_session_reset().unwrap();
        node.task_history_238=super::super::world_ready::tasks::TaskHistory::from_explicit_duration_fixture(c["begin"].as_i64().unwrap() as i32,&values,c["capacity"].as_u64().unwrap() as usize);
        node.frame_ratio_60=Some(c["ratio"].as_u64().unwrap() as u32);owner.host_node=Some(node);
        let node_addr=owner.host_node.as_ref().unwrap() as *const _;
        let history_addr=&owner.host_node.as_ref().unwrap().task_history_238 as *const _;
        for (i,out) in row["steps"].as_array().unwrap().iter().enumerate(){
            let times=&c["times"][i];
            //A failed first timer/World/second timer never invokes the real
            //history suffix. Positive callbacks are typed oracle inputs here,
            //not fabricated actual platform/World implementations.
            let failure=c["failure"].as_str();
            if failure!=Some("time0"){
                let first=begin(times[0].as_u64().unwrap() as u32,p).unwrap();
                assert_eq!(Some(first),out["start"].as_u64().map(|v|v as u32),"{c}");
                if failure.is_none(){
                    let mut b=owner.runtime_borrow().unwrap();finish(&mut b.local_client,first,times[1].as_u64().unwrap() as u32,p).unwrap();
                }
            }
            let node=owner.host_node.as_ref().unwrap();let h=&node.task_history_238;
            assert_eq!(h.begin_word(),out["begin"].as_i64().unwrap() as i32,"{c}");
            assert_eq!(h.end_word(),out["end"].as_i64().unwrap() as i32,"{c}");
            let actual:Vec<u32>=(h.begin_word()..h.end_word()).map(|i|h.duration_word(i).unwrap()).collect();
            let expected:Vec<u32>=out["values"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap() as u32).collect();
            assert_eq!(actual,expected,"{c} {p:?}");assert_eq!(node.frame_ratio_60,Some(out["ratio"].as_u64().unwrap() as u32),"{c} {p:?}");
            assert_eq!(node as *const _,node_addr);assert_eq!(&node.task_history_238 as *const _,history_addr);
        }
    }
}
#[test]
fn task_constructor_history_is_not_fabricated_into_completed_duration_window(){
    let mut owner=LocalBootstrap::from_native_constructors();owner.host_node=Some(crate::local_host::Node::before_session_reset().unwrap());
    let mut b=owner.runtime_borrow().unwrap();
    assert!(finish(&mut b.local_client,0,16,Precision::Double53).unwrap_err().to_string().contains("actual retired"));
    assert_eq!(b.local_client.task_history_238.begin_word(),i32::MIN);
    assert_eq!(*b.local_client.heartbeat.frame_ratio,None);
}
