use super::*;
use serde_json::Value;
fn fixture()->Value{serde_json::from_str(include_str!(
    "../../../research/skirmish-20261017-frame-leaves/native-presentation-timing.json"
)).unwrap()}
struct Input {time:u64,factor:u32,failure:Option<String>,calls:Vec<&'static str>}
impl Services for Input {
    fn time_seconds(&mut self)->Result<Binary>{
        self.calls.push("clock");
        if self.failure.as_deref()==Some("time"){return Err("required time".into())}
        Binary::from_f64_bits(self.time).map_err(|e|e.to_string())
    }
    fn smoothing_factor_bits(&mut self)->Result<u32>{
        self.calls.push("factor");
        if self.failure.as_deref()==Some("factor"){Err("required factor".into())}else{Ok(self.factor)}
    }
}
#[test]
fn actual_native_constructor_and_finite_smoothing_all_pc(){
    let fixture=fixture();
    assert_eq!(fixture["rows"].as_array().unwrap().len(),3048);
    assert_eq!(fixture["constructors"].as_array().unwrap().len(),6);
    for c in fixture["constructors"].as_array().unwrap(){
        let state=State::from_native_constructor();
        assert_eq!(serde_json::json!({"0x4":state.enabled_4,"0x154":state.smoothed_154,
            "0x158":state.frequency_158,"0x15c":state.previous_15c}),c["fields"]);
        assert_eq!(c["counter"],0);
    }
    for row in fixture["rows"].as_array().unwrap(){
        let c=&row["input"];
        let precision=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};
        let mut state=State{enabled_4:c["enabled"].as_u64().unwrap()as u8,
            smoothed_154:c["mean"].as_u64().unwrap()as u32,frequency_158:c["rate"].as_u64().unwrap()as u32,
            previous_15c:c["previous"].as_u64().unwrap()as u32};
        let mut input=Input{time:c["time"].as_u64().unwrap(),factor:c["factor"].as_u64().unwrap()as u32,
            failure:c["failure"].as_str().map(str::to_owned),calls:Vec::new()};
        let result=state.update(&mut input,precision);
        assert_eq!(result.is_err(),!row["output"]["stop"].is_null(),"{row}");
        assert_eq!(serde_json::json!({"0x4":state.enabled_4,"0x154":state.smoothed_154,
            "0x158":state.frequency_158,"0x15c":state.previous_15c}),row["output"]["fields"],"{row}");
        let expected=if state.enabled_4==0{vec![]}else if input.failure.as_deref()==Some("time"){vec!["clock"]}else{vec!["clock","factor"]};
        assert_eq!(input.calls,expected);
    }
}
#[test]
fn retained_disabled_constructor_reads_no_substitute_clock_or_avar(){
    let mut state=State::from_native_constructor();
    for p in [Precision::Single24,Precision::Double53,Precision::Extended64]{
        state.update_with_unbound_services(p).unwrap();
    }
    state.enabled_4=1; //Explicit synthetic activation; no real enabling-writer claim.
    assert!(state.update_with_unbound_services(Precision::Double53).unwrap_err().contains("866060"));
    assert_eq!(state.previous_bits(),0);
}
