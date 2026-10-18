use super::*;
#[test]
fn control_entry_uses_actual_leaf_owner_before_later_components(){
    struct Fields{seen:Vec<ActorId>}
    impl crate::load_zone_entry::EnteredFields for Fields{
        fn control_entered(&mut self,owner:ActorId)->Result<()>{self.seen.push(owner);Ok(())}
        fn city_entered(&mut self,owner:ActorId)->Result<()>{self.seen.push(owner);Ok(())}
        fn detection_present(&mut self)->Result<bool>{Ok(false)}
    }
    let owner=ActorId::from_raw(0x12345678);let mut fields=Fields{seen:Vec::new()};
    crate::load_zone_entry::entered_fields(aa_zones::set::Entry{actor:owner.raw(),flags:0x110},&mut fields).unwrap();
    assert_eq!(fields.seen,[owner,owner]);
    fields.seen.clear();
    let result=crate::load_zone_entry::entered_fields(aa_zones::set::Entry{actor:owner.raw(),flags:0x118},&mut fields);
    assert!(result.unwrap_err().to_string().contains("slot3"));assert_eq!(fields.seen,[owner]);
    fields.seen.clear();
    let result=crate::load_zone_entry::entered_fields(aa_zones::set::Entry{actor:owner.raw(),flags:0x130},&mut fields);
    assert!(result.unwrap_err().to_string().contains("slot5"));assert!(fields.seen.is_empty());
}
#[test]
fn original_v14_zone_prefix_reaches_only_the_first_required_owner(){
    struct Fields{detection:bool,calls:Vec<&'static str>}
    impl crate::load_zone_entry::EnteredFields for Fields{
        fn city_entered(&mut self,_owner:ActorId)->Result<()>{self.calls.push("city");Err(Error::Unsupported("explicit actual City owner boundary"))}
        fn detection_present(&mut self)->Result<bool>{self.calls.push("detection");Ok(self.detection)}
    }
    let proof:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-stored-actor-registration/native-entry-prefix.json")).unwrap();
    let rows=proof["rows"].as_array().unwrap();assert_eq!(rows.len(),108);
    for row in rows{
        let flags=row["flags"].as_u64().unwrap()as u16;
        let mut fields=Fields{detection:row["detection"].as_bool().unwrap(),calls:Vec::new()};
        let result=crate::load_zone_entry::entered_fields(aa_zones::set::Entry{actor:37,flags},&mut fields);
        let actual=match &result{
            Ok(())=>"complete",
            Err(e)if e.to_string().contains("slot5")=>"slot5",
            Err(e)if e.to_string().contains("slot4")=>"slot4",
            Err(e)if e.to_string().contains("slot3")=>"slot3",
            Err(e)if e.to_string().contains("City")=>"city",
            Err(e)if e.to_string().contains("marker/discovery")=>"discovery",
            Err(e)=>panic!("unexpected prefix result:{e}"),
        };
        assert_eq!(actual,row["outcome"].as_str().unwrap(),"{row}");
        match actual{
            "slot5"|"slot4"|"slot3"=>assert!(fields.calls.is_empty(),"{row}"),
            "city"=>assert_eq!(fields.calls,vec!["city"],"{row}"),
            "discovery"=>assert_eq!(fields.calls,vec!["detection"],"{row}"),
            "complete"=>assert_eq!(fields.calls,if flags&4!=0{vec!["detection"]}else{vec![]},"{row}"),
            _=>unreachable!(),
        }
    }
}
