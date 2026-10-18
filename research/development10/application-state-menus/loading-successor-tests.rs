#[path="../../../crates/aa-runtime/src/application_loading.rs"]mod application_loading;
#[path="../../../crates/aa-runtime/src/application_menu_callbacks.rs"]mod application_menu_callbacks;
#[path="../../../crates/aa-runtime/src/application_state.rs"]mod application_state;
use serde_json::{Value,json};
struct Prompt<'a>{input:&'a Value,trace:Vec<Value>,visible:u32}
impl application_menu_callbacks::ReadyServices for Prompt<'_>{
 fn button_visible(&mut self)->Result<bool,String>{Ok(self.visible&1!=0)}
 fn resource_complete_17(&mut self)->Result<bool,String>{Ok(self.input["complete"]==1)}
 fn user_ready_16(&mut self)->Result<bool,String>{Ok(self.input["ready"]==1)}
 fn finished_status_773ce0(&mut self)->Result<(),String>{self.trace.push(json!("string"));self.trace.push(json!("finished_status"));if self.input["fail"]=="status"{Err("required status child".into())}else{Ok(())}}
 fn show_ready_button(&mut self)->Result<(),String>{self.trace.push(json!(["show_ready",1]));self.visible|=1;Ok(())}
}
#[test]fn actual_ready_prompt_native_order_and_failure_prefix(){
 let f:Value=serde_json::from_str(include_str!("native-loading-return.json")).unwrap();let mut n=0;
 for row in f["rows"].as_array().unwrap(){let i=&row["input"];if i["op"]!="prompt"{continue;}let mut p=Prompt{input:i,trace:vec![json!("base_update")],visible:i["visible"].as_u64().unwrap()as u32};let _=application_menu_callbacks::ready_prompt_6e8c00(&mut p);assert_eq!(json!({"visible":p.visible,"trace":p.trace}),row["output"]);n+=1;}assert_eq!(n,48);
}
struct Complete;
impl application_loading::CompletionServices for Complete{
 fn view_initialized(&mut self)->Result<bool,String>{Ok(true)}fn multiplayer(&mut self)->Result<bool,String>{Ok(false)}
 fn waiting_on_other_players_773ce0(&mut self)->Result<(),String>{panic!("singleplayer must not read status")}
 fn on_vworld_complete_76aca0(&mut self,_:bool)->Result<(),String>{Ok(())}
}
struct StateCallbacks<'a>{loading:&'a mut application_loading::State,events:Vec<&'static str>}
impl application_state::Services for StateCallbacks<'_>{type Node=u32;type Arg8=();type ArgC=();
 fn diagnostic(&mut self,_:&mut application_state::State,from:u32,to:u32)->Result<(),String>{assert_eq!((from,to),(6,4));self.events.push("diagnostic");Ok(())}
 fn state_node(&mut self,i:u32)->Result<Option<u32>,String>{Ok(Some(i))}
 fn leave(&mut self,_:&mut application_state::State,i:u32)->Result<(),String>{assert_eq!(i,6);self.loading.leave_40caa0();self.events.push("leave_loading");Ok(())}
 fn enter(&mut self,s:&mut application_state::State,i:u32,_:&(),_:&())->Result<(),String>{assert_eq!(i,4);assert_eq!(s.previous(),6);assert_eq!(self.loading.changed_8(),0);self.events.push("enter_game");Ok(())}
}
#[test]fn captured_transition_immediately_reenters_same_loading_on_leave(){
 //SetState6 uses the actual constructor pair/algorithm; its preliminary
 //callbacks are explicit setup, not a manufactured application raw setter.
 struct Initial;impl application_state::Services for Initial{type Node=();type Arg8=();type ArgC=();fn diagnostic(&mut self,_:&mut application_state::State,_:u32,_:u32)->Result<(),String>{Ok(())}fn state_node(&mut self,_:u32)->Result<Option<()>,String>{Ok(Some(()))}fn leave(&mut self,_:&mut application_state::State,_:())->Result<(),String>{unreachable!()}fn enter(&mut self,_:&mut application_state::State,_:(),_:&(),_:&())->Result<(),String>{Ok(())}}
 let mut app=application_state::State::from_constructor();app.set_state(6,&(),&(),&mut Initial).unwrap();
 let mut loading=application_loading::State::from_native_constructor();loading.bind_682e10(4,1,||Ok(false)).unwrap();assert!(!loading.user_ready_16());
 loading.user_ready_command_6e80f0();assert!(loading.user_ready_16());
 loading.notification_prefix_682fd0(9,||panic!("kind9 does not read status")).unwrap();
 let captured=loading.tick_before_state_transition_682ee0(&mut Complete).unwrap().unwrap();
 let mut callbacks=StateCallbacks{loading:&mut loading,events:vec![]};app.set_state(captured,&(),&(),&mut callbacks).unwrap();
 assert_eq!(callbacks.events,vec!["diagnostic","leave_loading","enter_game"]);assert_eq!(app.current(),4);
}
