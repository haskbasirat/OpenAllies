use super::*;
use serde_json::{Value,json};
struct Replay<'a>{input:&'a Value,trace:Vec<Value>,kingdom_index:usize,queue:Vec<view_resource_task::Task>}
impl Replay<'_>{fn stop(&self,label:&str)->Result<()>{if self.input["fail"].as_str()==Some(label){Err("required child cutoff".into())}else{Ok(())}}}
#[test]
fn nonowning_ui_entry_refuses_expired_world_before_any_native_prefix(){
    let mut counter=crate::world_revision::Counter::fresh_application();let world=counter.construct();
    let witness=world.witness();drop(world);
    let input=json!({"kingdoms":[0,0],"fail":null});
    let mut replay=Replay{input:&input,trace:Vec::new(),kingdom_index:0,queue:Vec::new()};
    let mut ui=State::<u32>::unbound_ui_fields();
    assert!(ui.initialize_5c3000_for_witness(&witness,&mut replay).unwrap_err().contains("destroyed"));
    assert!(replay.trace.is_empty());assert!(!ui.attempted);assert!(ui.task().is_none());
    assert!(view_resource_task::Task::construct_for_ui_witness(&witness).is_err());
}
impl Services for Replay<'_>{
    type Kingdom=u32;
    fn init_diagnostic(&mut self)->Result<()>{self.trace.push(json!("diagnostic"));self.stop("diagnostic")}
    fn with_world_lock<R>(&mut self,call:impl FnOnce(&mut Self)->Result<R>)->Result<R>{
        self.trace.push(json!("lock"));
        //A dependency cutoff is a paused native prefix, not a thrown C++
        //exception. The replay does not claim native SEH unwind effects.
        let result=call(self)?;self.trace.push(json!("unlock"));self.stop("unlock")?;Ok(result)
    }
    fn selected_kingdom_76c2b0(&mut self)->Result<Option<u32>>{
        let value=self.input["kingdoms"][self.kingdom_index].as_u64().unwrap()as u32;self.kingdom_index+=1;
        self.trace.push(json!(["kingdom",value]));self.stop(&format!("kingdom{}",self.kingdom_index))?;Ok((value!=0).then_some(value))
    }
    fn reset_camera_700e30(&mut self)->Result<()>{self.trace.push(json!("camera_reset"));self.stop("camera_reset")}
    fn initialize_vworld_43f0a0(&mut self,revision:u32)->Result<()>{self.trace.push(json!(["view_init",revision]));self.stop("view_init")}
    fn camera_zero_700500(&mut self,pair:[u32;2])->Result<()>{assert_eq!(pair,[0,0]);self.trace.push(json!("camera_zero"));self.stop("camera_zero")}
    fn world_on_vworld_created_4f0750(&mut self)->Result<()>{self.trace.push(json!("world_onview"));self.stop("world_onview")}
    fn enqueue_view_task_5aed10(&mut self,slot:&mut Option<view_resource_task::Task>)->Result<()>{
        self.trace.push(json!("enqueue"));self.stop("enqueue")?;self.queue.push(slot.take().ok_or("actual task absent")?);Ok(())
    }
}
#[test]
fn native_ui_initialization_child_order_rereads_and_owned_task_prefix(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-ui-world-completion/native-loading.json")).unwrap();let mut checked=0;
    for row in fixture["rows"].as_array().unwrap(){let input=&row["input"];if input["op"]!="ui_init"{continue;}
        let mut counter=crate::world_revision::Counter::fresh_application();let world=counter.construct();
        let mut ui=State::unbound_ui_fields();assert!(ui.has_kingdom_1e4().is_err());assert!(ui.kingdom_1e0().is_err());
        //Explicit lab prefields, not UI-constructor values.
        ui.has_kingdom_1e4=Some(0xa5);ui.kingdom_1e0=Some(Some(input["old"].as_u64().unwrap()as u32));
        let mut replay=Replay{input,trace:Vec::new(),kingdom_index:0,queue:Vec::new()};let result=ui.initialize_5c3000(&world,&mut replay);
        let expected=&row["output"];
        assert_eq!(result.is_err(),!expected["error"].is_null(),"{input}");
        assert_eq!(json!(ui.has_kingdom_1e4().unwrap()),expected["has_kingdom"],"{input}");
        assert_eq!(json!(ui.kingdom_1e0().unwrap().copied().unwrap_or(0)),expected["kingdom"],"{input}");
        assert_eq!(json!(replay.trace),expected["trace"],"{input}");
        if !expected["task_fields"].is_null(){
            let task=ui.pending_task.as_ref().or_else(||replay.queue.first()).unwrap();
            assert_eq!(json!(task.state_14().unwrap()),expected["task_fields"][4]);
            assert_eq!(json!(task.thread_18().unwrap()),expected["task_fields"][5]);
            assert_eq!(ui.task().unwrap().state_14().unwrap(),0);
        }else{assert!(ui.task().is_none());}
        assert!(ui.initialize_5c3000(&world,&mut replay).is_err(),"fresh-domain retry must not replace an actual prefix/task");checked+=1;
    }
    assert_eq!(checked,180);
}

struct Prefix;
impl crate::view_init_prefix::Services for Prefix{
    type Grid=();
    fn begin_screen_7040e0(&mut self)->Result<()>{Ok(())}
    fn reset_user_746e30(&mut self)->Result<()>{Ok(())}
    fn resource_context_5ae0a0(&mut self)->Result<()>{Ok(())}
    fn construct_grid_430290(&mut self)->Result<Option<()>>{Ok(None)}
    fn resetting_editor_variables_diagnostic(&mut self)->Result<()>{Ok(())}
    fn reset_view_variables(&mut self)->Result<()>{Ok(())}
}
#[test]
fn actual_resource_callback_fields_do_not_promote_view_ad_or_other_flags(){
    let native:Value=serde_json::from_str(include_str!("../../../research/development10/native-view-resource-completion.json")).unwrap();assert_eq!(native["rows"].as_array().unwrap().len(),108);
    let mut first=crate::world_revision::Counter::fresh_application();let world=first.construct();
    let mut other=crate::world_revision::Counter::fresh_application();let foreign_same_revision=other.construct();
    let mut view=crate::view_init_prefix::State::from_constructor_fields();view.initialize_prefix(&world,&mut Prefix).unwrap();
    let mut loading=crate::application_loading::State::from_native_constructor();loading.bind_682e10(12,0,||panic!("must remain lazy")).unwrap();loading.menu_finished_682db0(0);
    let mut task=view_resource_task::Task::construct_for_ui(&world);let witness=task.witness();
    assert!(task.complete_5b3550(&foreign_same_revision,&mut view,&mut loading).is_err());
    assert!(view.initializing());assert!(!view.initialized_for(&world).unwrap());assert_eq!(loading.flags_14_17(),[0,1,1,0]);assert_eq!(task.state_14().unwrap(),0);
    task.complete_5b3550(&world,&mut view,&mut loading).unwrap();
    assert!(!view.initializing());assert!(!view.initialized_for(&world).unwrap());assert_eq!(loading.flags_14_17(),[0,1,1,1]);assert_eq!(task.state_14().unwrap(),2);assert_eq!(task.thread_18().unwrap(),0);assert_eq!(witness.state_14().unwrap(),2);
    drop(task);assert!(witness.state_14().is_err());
}
