//!Loading682EE0 -> Session76ACA0 -> Admin74FF50 over the actual retained
//!Session and sole Node90/304 request stream. This never unfreezes the Sim;
//!the subsequent actual304/315/775BF0 protocol remains authoritative.
use super::*;
use aa_runtime::{application_loading,view_init_prefix,world_revision::ConstructedWorldRevision};

#[derive(Debug,PartialEq,Eq)]
pub enum Diagnostic { SessionOnVWorldComplete(bool),AdminSetVWorldSuccess(bool) }

///Positive native children not yet owned by the current UI/application path.
///Single-player loading does not read the multiplayer status child. A real
///kind9 transition may not be replaced with a frontend screen switch.
pub trait Effects {
    fn waiting_on_other_players_773ce0(&mut self,owner:&mut LocalBootstrap)->Result<()>;
    fn set_application_state_40ce10(&mut self,state:u32)->Result<()>;
}
pub struct RequiredEffects;
impl Effects for RequiredEffects {
    fn waiting_on_other_players_773ce0(&mut self,_:&mut LocalBootstrap)->Result<()>{Err(Error::Unsupported("773CE0 actual Session148/14C status owner"))}
    fn set_application_state_40ce10(&mut self,_:u32)->Result<()>{Err(Error::Unsupported("40CE10 actual Application10C/110 state dispatch"))}
}
trait CompletionWorld {
    fn session_diagnostic(&mut self,success:bool)->Result<()>;
    fn store_status(&mut self,status:u32)->Result<()>;
    fn admin_diagnostic(&mut self,success:bool)->Result<()>;
    fn emit_status(&mut self)->Result<()>;
}
fn complete(world:&mut impl CompletionWorld,success:bool)->Result<()> {
    world.session_diagnostic(success)?;
    world.store_status(if success {3}else{4})?;
    world.admin_diagnostic(success)?;
    world.emit_status()
}
struct Actual<'a>{owner:&'a mut LocalBootstrap,diagnostics:&'a mut Vec<Diagnostic>}
impl CompletionWorld for Actual<'_> {
    fn session_diagnostic(&mut self,success:bool)->Result<()>{self.diagnostics.push(Diagnostic::SessionOnVWorldComplete(success));Ok(())}
    fn store_status(&mut self,status:u32)->Result<()>{
        self.owner.host_node.as_mut().ok_or_else(||missing("74FF50 actual Node absent"))?.task_status_90=Some(status);Ok(())
    }
    fn admin_diagnostic(&mut self,success:bool)->Result<()>{self.diagnostics.push(Diagnostic::AdminSetVWorldSuccess(success));Ok(())}
    fn emit_status(&mut self)->Result<()>{self.owner.emit_task_status_74e970()}
}
struct Loading<'a,G,E>{owner:&'a mut LocalBootstrap,view:&'a view_init_prefix::State<G>,world:&'a ConstructedWorldRevision,diagnostics:&'a mut Vec<Diagnostic>,effects:&'a mut E}
impl<G,E:Effects> application_loading::TickServices for Loading<'_,G,E> {
    fn view_initialized(&mut self)->std::result::Result<bool,String>{self.view.initialized_for(self.world)}
    fn multiplayer(&mut self)->std::result::Result<bool,String>{Ok(self.owner.session.multiplayer)}
    fn waiting_on_other_players_773ce0(&mut self)->std::result::Result<(),String>{self.effects.waiting_on_other_players_773ce0(self.owner).map_err(|e|e.to_string())}
    fn on_vworld_complete_76aca0(&mut self,success:bool)->std::result::Result<(),String>{complete(&mut Actual{owner:self.owner,diagnostics:self.diagnostics},success).map_err(|e|e.to_string())}
    fn set_application_state_40ce10(&mut self,state:u32)->std::result::Result<(),String>{self.effects.set_application_state_40ce10(state).map_err(|e|e.to_string())}
}
impl world_ready::ActualActivatedSession {
    ///Only a genuine7757/7738 session can reach this public adapter. Every
    ///tick borrows the SAME retained ApplicationE4, SessionF4, World identity
    ///and nativeAD field; it creates no snapshot readiness bool or secondNode.
    pub fn tick_current_loading<G>(&mut self,view:&view_init_prefix::State<G>,
        diagnostics:&mut Vec<Diagnostic>,effects:&mut impl Effects)->Result<()> {
        let (host,world,application)=self.parts();
        let presentation=world.presentation_parts().map_err(missing)?;
        application.loading_mut().tick_682ee0(&mut Loading{owner:host,view,world:presentation.revision,diagnostics,effects}).map_err(missing)
    }
    ///Only the concrete resource queue completion dispatch may call this.
    ///The Task object came from actual UI5C3000, and its World/view identity
    ///is rechecked before any store. This callback alone never writes AD.
    pub fn view_resource_callback_5b3550<G>(&mut self,task:&mut aa_runtime::view_resource_task::Task,
        view:&mut view_init_prefix::State<G>)->Result<()> {
        let (_,world,application)=self.parts();
        let presentation=world.presentation_parts().map_err(missing)?;
        task.complete_5b3550(presentation.revision,view,application.loading_mut()).map_err(missing)
    }
}

///784220's kind6 direct682E10 call, with actual retained notification values.
///This is only that state binding, not a substitute for the surrounding
///Application state transition/OnEnter services or notification consumption.
pub fn bind_loading_notification(owner:&LocalBootstrap,application:&mut aa_runtime::world_ready_services::Application,
    notification:&Notification)->Result<()> {
    if notification.kind!=0||notification.launch_state.kind!=6{return Err(Error::Unsupported("784220 current notification is not kind0/Loading6"));}
    application.loading_mut().bind_682e10(notification.launch_state.field_18,notification.launch_state.field_1c,||Ok(owner.session.multiplayer)).map_err(missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value,json};
    struct Replay<'a>{input:&'a Value,status:u32,trace:Vec<Value>}
    impl Replay<'_>{fn stop(&self,label:&str)->Result<()>{if self.input["fail"].as_str()==Some(label){Err(missing("required child cutoff"))}else{Ok(())}}}
    impl CompletionWorld for Replay<'_>{
        fn session_diagnostic(&mut self,_:bool)->Result<()>{self.trace.push(json!("session_diagnostic"));self.stop("session_diagnostic")}
        fn store_status(&mut self,status:u32)->Result<()>{self.status=status;Ok(())}
        fn admin_diagnostic(&mut self,_:bool)->Result<()>{self.trace.push(json!("admin_diagnostic"));self.stop("admin_diagnostic")}
        fn emit_status(&mut self)->Result<()>{self.trace.push(json!(["emit",self.status]));self.stop("emit")}
    }
    #[test]
    fn native_vworld_complete_store_precedes_admin_diagnostic_and_request() {
        let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-ui-world-completion/native-loading.json")).unwrap();let mut checked=0;
        for row in fixture["rows"].as_array().unwrap(){let input=&row["input"];if input["op"]!="vworld_status"{continue;}
            let mut world=Replay{input,status:input["old"].as_u64().unwrap()as u32,trace:Vec::new()};let result=complete(&mut world,input["success"].as_u64().unwrap()!=0);
            assert_eq!(json!({"status":world.status,"trace":world.trace,"error":if result.is_err(){Some("required child cutoff")}else{None}}),row["output"],"{input}");checked+=1;
        }
        assert_eq!(checked,216);
    }
}
