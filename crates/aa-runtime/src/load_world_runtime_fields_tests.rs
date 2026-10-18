use super::*;
use serde_json::Value;
fn fixture()->Value {
    serde_json::from_slice(include_bytes!("../../../research/development10/live-battle/native-runtime-fields.json")).unwrap()
}
#[test]
fn actual_constructor_and_fresh_init_native_words_both_images_all_pcs() {
    let f=fixture();let constructors=f["constructor_rows"].as_array().unwrap();assert_eq!(constructors.len(),18);
    for row in constructors {
        let state=State::from_world_constructor();
        assert_eq!(*state.fog_cheat_word(),row["native"]["fog_275"].as_u64().unwrap() as u8);
        assert_eq!(state.auxiliary.is_none(),row["native"]["auxiliary_4"]==0);
        assert!(state.require_initialized().is_err());
    }
    let rows=f["rows"].as_array().unwrap();assert_eq!(rows.len(),48);
    let mut fresh=0;
    for row in rows {
        if row["input"]["old"]==true {continue;}
        fresh+=1;let n=&row["native"];let mut state=State::from_world_constructor();
        state.fog_cheat_275=128; //Declared lab mutation before the actual reset.
        let error=row["input"]["error"]=="allocate";
        let null=row["input"]["null"]==true;
        let result=state.initialize_with(|| {
            if error {Err("AllocationServiceError".into())}
            else if null {Ok(None)} else {Ok(Some(Box::new([0;13])))}
        });
        assert_eq!(*state.fog_cheat_word(),n["fog_275"].as_u64().unwrap() as u8);
        if error {
            assert_eq!(result.unwrap_err(),n["outcome"].as_str().unwrap());
            assert!(state.require_initialized().is_err());
            assert!(state.counter_loan().is_err());
        } else {
            result.unwrap();state.require_initialized().unwrap();
            match state.auxiliary_words().unwrap() {
                None=>assert_eq!(n["pointer"],0),
                Some(words)=>assert_eq!(serde_json::json!(words),n["new_words"]),
            }
        }
    }
    assert_eq!(fresh,24);
}
#[test]
fn actual_box_counter_and_raw_byte_are_borrowed_without_second_owner() {
    let mut state=State::from_world_constructor();state.initialize_fresh_auxiliary_4f27e0().unwrap();
    let address=std::ptr::from_ref(state.auxiliary_words().unwrap().unwrap()).addr();
    let mut moved=state;
    assert_eq!(std::ptr::from_ref(moved.auxiliary_words().unwrap().unwrap()).addr(),address);
    moved.auxiliary.as_deref_mut().unwrap()[0]=u32::MAX;
    let raw=128u8;
    let fields=moved.frame_fields(&raw).unwrap();
    assert!(fields.svar_fog_cheat.enabled());assert!(!fields.world_fog_cheat.enabled());
    match fields.auxiliary {crate::live_clocked::Counter::Present(word)=>*word=word.wrapping_add(1),_=>panic!("actual allocated counter absent")}
    assert_eq!(moved.auxiliary_words().unwrap().unwrap()[0],0);
    assert!(moved.initialize_fresh_auxiliary_4f27e0().is_err());
    for byte in [0u8,1,2,128,255] {
        assert_eq!(crate::live_clocked::BoolSource::enabled(&byte),byte!=0);
    }
    for value in [false,true] {assert_eq!(crate::live_clocked::BoolSource::enabled(&value),value);}
}
