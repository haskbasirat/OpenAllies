use super::*;
struct Source {events:Vec<&'static str>,stop:bool}
impl Constructor for Source {
    type Timer=u64;
    fn timer_59aa70_59ab60(&mut self)->Result<u64>{self.events.push("timer");Ok(0x1234567812345678)}
    fn wildcard_display_4b9f40(&mut self,key:&str)->Result<Vec<u16>>{
        assert_eq!(key,"static_group_wildcard_name");self.events.push("display");
        if self.stop{Err("actual display lookup unavailable".into())}else{Ok("All Units".encode_utf16().collect())}
    }
}
#[test]fn actual_constructor_prefix_survives_later_display_refusal(){
    let mut state=State::<usize,u64>::before_children_438e10();let mut source=Source{events:vec![],stop:true};
    assert!(state.construct_640640(&mut source).is_err());
    assert_eq!(source.events,vec!["timer","display"]);assert_eq!(*state.brush().unwrap().timer(),0x1234567812345678);
    assert!(state.wildcard().is_none());assert!(state.require_constructed().is_err());
    assert!(state.construct_640640(&mut source).is_err());assert_eq!(source.events.len(),2);
}
#[test]fn native_640640_constructor_field_projection_all_pcs(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-ui-world-pointer/native-ui-pointer.json")).unwrap();
    for row in fixture["constructors"].as_array().unwrap(){
        struct NativeSource<'a>(&'a serde_json::Value,Vec<&'static str>);
        impl Constructor for NativeSource<'_>{
            type Timer=u64;
            fn timer_59aa70_59ab60(&mut self)->Result<u64>{self.1.extend(["frequency","anchor"]);Ok(self.0["anchor"].as_u64().unwrap())}
            fn wildcard_display_4b9f40(&mut self,key:&str)->Result<Vec<u16>>{assert_eq!(key,"static_group_wildcard_name");self.1.push("display");Ok(self.0["caption"].as_str().unwrap().encode_utf16().collect())}
        }
        let mut source=NativeSource(&row["input"],vec![]);let mut s=State::<usize,u64>::before_children_438e10();s.construct_640640(&mut source).unwrap();
        s.require_constructed().unwrap();let b=s.brush().unwrap();let g=s.wildcard().unwrap();
        assert!(s.selected_8.is_empty()&&s.hovered_10.is_none()&&s.fields_14_18.iter().all(Option::is_none)&&s.fields_20_2c.iter().all(Option::is_none)&&s.groups_34.is_empty()&&s.field_40.is_none()&&s.fields_48_4c.iter().all(Option::is_none));
        assert_eq!(s.bytes_1c_1d,[0;2]);assert_eq!(s.derived_50_54,Some([0;2]));
        assert_eq!(serde_json::json!({"name":g.name(),"display":g.display(),"text":g.text_8,"count":g.members().len(),"capacity":1,"flag18":g.byte_18,"word1c":g.word_1c,"flag20":g.byte_20}),row["native"]["group"]);
        assert_eq!(serde_json::json!({"parent":true,"enabled":b.enabled_8,"anchor":b.timer(),"words":[b.words_18_1c[0],b.words_18_1c[1],u32::from(b.attached_24.is_some())],"attached_parent":true}),row["native"]["brush"]);
        assert_eq!(serde_json::json!(source.1),row["native"]["trace"]);
        assert!(row["native"]["zero_words"].as_array().unwrap().iter().all(|v|v.as_u64()==Some(0)));
        assert_eq!(row["native"]["unwritten4"],0xa5a5a5a5u32);assert_eq!(row["native"]["unwritten1e"],0xa5a5u32);
    }
}
