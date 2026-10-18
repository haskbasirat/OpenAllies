use super::*;
use serde_json::{Value, json};

struct Effects<'a> { input: &'a Value, trace: Vec<Value>, view: bool }
impl Effects<'_> {
    fn stop(&self, name: &str) -> Result<()> {
        if self.input["fail"].as_str() == Some(name) { Err("required child cutoff".into()) } else { Ok(()) }
    }
}
impl TickServices for Effects<'_> {
    fn view_initialized(&mut self) -> Result<bool> { self.trace.push(json!(["view",self.view])); Ok(self.view) }
    fn multiplayer(&mut self) -> Result<bool> { let v=self.input["multi"].as_u64().unwrap()!=0;self.trace.push(json!(["multiplayer",v]));Ok(v) }
    fn waiting_on_other_players_773ce0(&mut self) -> Result<()> {
        self.trace.push(json!("waiting"));self.stop("waiting")?;
        if let Some(v)=self.input["after_wait"].as_u64(){self.view=v!=0;} Ok(())
    }
    fn on_vworld_complete_76aca0(&mut self, initialized: bool) -> Result<()> { self.trace.push(json!(["complete",initialized])); self.stop("complete") }
    fn set_application_state_40ce10(&mut self, state: u32) -> Result<()> { self.trace.push(json!(["set_state",state])); self.stop("state") }
}
impl EventServices for Effects<'_> {
    fn loading_world_status_773ce0(&mut self) -> Result<()> { self.trace.push(json!("loading_status"));self.stop("status") }
    fn base_notification_414620(&mut self, kind: u32) -> Result<()> { self.trace.push(json!(["base",kind]));self.stop("base") }
}
#[test]
fn native_loading_fields_and_ordered_lazy_callbacks() {
    let data:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-ui-world-completion/native-loading.json")).unwrap();
    let mut checked=0;
    for row in data["rows"].as_array().unwrap() {
        let input=&row["input"];let op=input["op"].as_str().unwrap();if matches!(op,"vworld_status"|"ui_init"){continue;}
        let mut state=State::from_native_constructor();
        let mut effects=Effects {input,trace:Vec::new(),view:input["view"].as_u64().is_some_and(|v|v!=0)};
        let result=match op {
            "constructor"=>Ok(()),
            "bind"=>state.bind_682e10(input["return_state"].as_u64().unwrap()as u32,input["kind"].as_u64().unwrap()as u32,|| {
                if input["fail"].as_bool()==Some(true){Err("required child cutoff".into())}else{Ok(input["multi"].as_u64().unwrap()!=0)}
            }),
            "menu"=>{state.flags_14_16[1]=input["old"].as_u64().unwrap()as u8;state.menu_finished_682db0(input["argument"].as_u64().unwrap()as u8);Ok(())},
            "event"|"tick"=>{
                for (index,value)in input["flags"].as_array().unwrap().iter().enumerate(){if index==3{state.flag_17.store(value.as_u64().unwrap()as u8,Ordering::Release)}else{state.flags_14_16[index]=value.as_u64().unwrap()as u8;}}
                if op=="event"{state.notification_682fd0(input["kind"].as_u64().unwrap()as u32,&mut effects)}
                else {state.return_state_c=input["return_state"].as_u64().unwrap()as u32;state.tick_682ee0(&mut effects)}
            },
            _=>panic!("unknown native case {input}"),
        };
        let expected=&row["output"];
        assert_eq!(result.is_err(),!expected["error"].is_null(),"{input}");
        assert_eq!(json!({"changed":state.changed_8(),"return_state":state.return_state(),"kind":state.kind(),"flags":state.flags_14_17()}),expected["fields"],"{input}");
        let expected_trace:Vec<_>=expected["trace"].as_array().unwrap().iter().filter(|v|v.as_str()!=Some("loading_status_string")).cloned().collect();
        assert_eq!(effects.trace,expected_trace,"{input}");checked+=1;
    }
    assert_eq!(checked,1320);
}
