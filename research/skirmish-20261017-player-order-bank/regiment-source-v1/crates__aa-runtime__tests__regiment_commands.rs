use aa_runtime::regiment_command::{priority,can_activate_sleep_51d650};
use aa_runtime::regiment_command::{can_activate_move_51d730,MovePermissionFields};
struct Permission<'a>{row:&'a serde_json::Value,reads:Vec<&'static str>}
impl MovePermissionFields for Permission<'_>{
    fn current_is_retreat_command(&mut self)->aa_mind::Result<bool>{
        if self.row["command"].is_null(){return Ok(false)}self.reads.push("command");
        if self.row["command"][0].as_u64().unwrap()&64==0{return Ok(false)}self.reads.push("definition");
        Ok(self.row["command"][1].as_u64()==Some(7))
    }
    fn engage_status_is_two(&mut self)->aa_mind::Result<bool>{Ok(self.row["engage_status"].as_u64()==Some(2))}
    fn organization_override(&mut self)->aa_mind::Result<u8>{self.reads.push("organization");Ok(self.row["organization_override"].as_u64().unwrap()as u8)}
    fn current_priority(&mut self)->aa_mind::Result<i32>{self.reads.push("stack");self.row["current"].as_u64().map(|v|priority(v as u32)).unwrap_or(Ok(0))}
    fn engage_is_current(&mut self)->aa_mind::Result<bool>{self.reads.push("stack");Ok(self.row["current"].as_u64()==Some(0x1cc))}
}
#[test]
fn both_pe_move_permission_preserves_lazy_owner_reads(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-player-order-bank/native-company-permission.json")).unwrap();
    let mut count=0;
    for machine in fixture["machines"].as_array().unwrap(){for row in machine["rows"].as_array().unwrap(){
        let mut fields=Permission{row,reads:vec![]};
        assert_eq!(can_activate_move_51d730(priority(0x68).unwrap(),&mut fields).unwrap(),row["output"]["allowed"].as_bool().unwrap());
        assert_eq!(fields.reads,row["output"]["reads"].as_array().unwrap().iter().map(|v|v.as_str().unwrap()).collect::<Vec<_>>());count+=1;
    }}assert_eq!(count,3960);
}
#[test]
fn ground_command_arguments_use_native_defaults_and_x87_copies(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-player-order-bank/native-company-arguments-v2.json")).unwrap();
    let mut count=0;
    for machine in fixture["machines"].as_array().unwrap(){
        assert_eq!(machine["fresh_path_log"],0);assert_eq!(machine["default_speed"],0);
        assert_eq!(machine["default_auxiliary"],serde_json::json!([0,0]));
        for row in machine["rows"].as_array().unwrap(){
            let flags=row["flags"].as_u64().unwrap()as u32;
            let w:Vec<u32>=row["words"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();
            let(aux,speed)=aa_runtime::regiment_command::move_arguments(flags,[w[2],w[3]],w[4]);
            assert_eq!(row["args"],serde_json::json!([2,w[0],w[1],speed,aux[0],aux[1]]));count+=1;
        }
    }assert_eq!(count,108);
}
#[test]
fn real_move_actor_state_return_preserves_positive_zoc_boundary(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-player-order-bank/native-company-actor-state-v2.json")).unwrap();
    let mut count=0;let mut required=0;
    for machine in fixture["machines"].as_array().unwrap(){for row in machine["rows"].as_array().unwrap(){
        let positive=aa_runtime::regiment_command::move_zoc_requires_predicates(row["length"].as_u64().unwrap()as usize,||!row["zone_y"].is_null());
        if positive{assert_eq!(row["output"]["required"],"52FD00");required+=1;}
        else{assert_eq!(aa_runtime::regiment_command::move_actor_state_after_zoc(row["command_type"].as_u64().map(|v|v as u32)),row["output"]["index"].as_u64().unwrap()as u8);}
        count+=1;
    }}assert_eq!((count,required),(216,108));
}
#[test]
fn signalling_nan_optional_fields_keep_hardware_quiet_copy(){
    //Hardware-correct shared FLD/FSTP rule, separately qualified from the
    //finite Unicorn oracle. Never change this to match its known raw-SNaN bug.
    assert_eq!(aa_runtime::regiment_command::move_arguments(24,[0x7f812345,0xff812346],0x7f812347),([0x7fc12345,0xffc12346],0x7fc12347));
    assert_eq!(aa_runtime::regiment_command::move_arguments(0,[0x7f812345,0xff812346],0x7f812347),([0,0],0));
}
#[test]
fn native_ground_target_completes_getter_with_actual_zoc_present(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-player-order-bank/native-company-target.json")).unwrap();
    let mut count=0;let mut positive=0;
    for machine in fixture["machines"].as_array().unwrap(){for row in machine["rows"].as_array().unwrap(){
        let mode=row["mode"].as_str().unwrap();let flags=row["flags"].as_u64().unwrap()as u32;
        let current=matches!(mode,"self"|"foreign").then_some(flags);
        let required=aa_runtime::regiment_command::move_has_live_target(current,||mode=="self");
        if required{assert_eq!(row["output"]["required"],"4CEDE0");positive+=1;}
        else{assert_eq!(aa_runtime::regiment_command::move_actor_state_after_zoc(row["command_type"].as_u64().map(|v|v as u32)),row["output"]["index"].as_u64().unwrap()as u8);}
        count+=1;
    }}assert_eq!((count,positive),(720,48));
}
#[test]
fn constructor_collection_transfer_has_no_second_path_or_arrival_owner(){
    use aa_mind::construct::{self,Kind};
    struct World;
    impl construct::World for World{
        fn random_word(&mut self)->aa_mind::Result<u32>{panic!("Regiment ctor does not draw RNG")}
        fn performer_ability_count(&self,_:u32)->aa_mind::Result<usize>{panic!("Regiment ctor does not inspect abilities")}
        fn set_ability_jitter(&mut self,_:u32,_:usize,_:u32)->aa_mind::Result<()>{panic!("Regiment ctor does not write ability jitter")}
    }
    let constructor=construct::construct(Kind::Regiment,7,&mut World,aa_numeric::Precision::Double53).unwrap();
    let ids=constructor.states.iter().enumerate().map(|(i,s)|(s.offset,aa_mind::StateId(i as u32+10))).collect();
    let mut arena=aa_mind::Arena::default();
    let mut installed=constructor.install(aa_mind::MindId(4),&ids,&mut arena).unwrap();
    let mut damaged=installed.clone();damaged.words.insert(0xa0,1);
    assert!(aa_mind::regiment_load::take_constructed_owned(&mut damaged).is_err());assert_eq!(damaged.words.get(&0x2bc),Some(&0));
    let owned=aa_mind::regiment_load::take_constructed_owned(&mut installed).unwrap();
    assert!(owned.tracked_words.is_empty()&&owned.path.nodes.is_empty());assert_eq!(owned.path.word18,u32::MAX);
    assert!(owned.path.dirty);assert!(owned.desired_formation.is_none());
    for offset in [0xa0,0xa4,0xa8,0xac,0xb4,0xb8,0x148,0x1dc,0x1e0,0x268,0x26c,0x2bc,0x2c0]{assert!(!installed.words.contains_key(&offset));}
    assert!(aa_mind::regiment_load::take_constructed_owned(&mut installed).is_err());
    assert_eq!(arena.state(*installed.state_bindings.get(&0x68).unwrap()).unwrap().status,0);
}
#[test]
fn both_pe_crt_priorities_and_real_stop_permission_match(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-player-order-bank/native-regiment-stop.json")).unwrap();
    let mut metadata=0;let mut cases=0;
    for machine in fixture["machines"].as_array().unwrap(){
        for row in machine["metadata"].as_array().unwrap(){
            assert_eq!(priority(row["offset"].as_u64().unwrap()as u32).unwrap(),row["word4"].as_u64().unwrap()as u32 as i32);
            metadata+=1;
        }
        for row in machine["rows"].as_array().unwrap(){
            let current=row["current"].as_u64().map(|v|v as u32);
            assert_eq!(can_activate_sleep_51d650(current).unwrap(),row["output"]["allowed"].as_bool().unwrap());
            if row["output"]["stop_result"]=="returnedFalse"{assert!(!can_activate_sleep_51d650(current).unwrap())}
            cases+=1;
        }
    }
    assert_eq!((metadata,cases),(120,132));
    assert!(priority(0xdeadbeef).is_err());
    assert!(can_activate_sleep_51d650(Some(0xdeadbeef)).is_err());
}
