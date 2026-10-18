//!One retained client VWorld allocation around the SAME native coordinator.
//!No independent prefix/scene/grid or application-ready capability is made.
use super::view_directory::{ViewDirectory,ViewHandle};
use aa_runtime::{live_world_frame::World,view_init_world};
use anyhow::{Context,Result,ensure};
use std::sync::{Arc,Weak};

///Retain once with the actual client application on SimulationThread, across
///missions. Portable view IDs replace pointers and have no32-bit pool ceiling.
pub struct ApplicationViews{origin:Arc<()>,next:usize,visual_map:aa_runtime::view_visual_map::Global,
    request_policy:super::view_request_policy::State,camera:crate::battle_camera::Process,
    actor_globals:super::view_actor_globals::State}
impl ApplicationViews{
    pub fn from_application_constructor()->Result<Self>{Ok(Self{origin:Arc::new(()),next:1,
        visual_map:aa_runtime::view_visual_map::Global::from_fresh_process_image(),
        request_policy:super::view_request_policy::State::from_application_initialize()?,
        camera:crate::battle_camera::Process::from_application_constructor().map_err(anyhow::Error::msg)?,
        actor_globals:super::view_actor_globals::State::from_process_image()})}
    pub fn request_policy(&self)->&super::view_request_policy::State{&self.request_policy}
    pub fn actor_globals(&self)->&super::view_actor_globals::State{&self.actor_globals}
    pub fn camera_and_visual_map(&mut self)->(&mut crate::battle_camera::Process,&aa_runtime::view_visual_map::Global){
        (&mut self.camera,&self.visual_map)
    }
    ///One actual logical B36518 owner across missions, separate from the
    ///simulation Map and inspector assets. Native camera reads it lazily.
    pub fn visual_map(&self)->&aa_runtime::view_visual_map::Global{&self.visual_map}
    pub fn visual_map_mut(&mut self)->&mut aa_runtime::view_visual_map::Global{&mut self.visual_map}
    pub fn construct<A>(&mut self,world:&World<'_>,actual_assets:A)->std::result::Result<VWorldOwner<A>,(A,String)>{
        //Preflight before moving the actual asset owner into its constructor.
        let directory=match ViewDirectory::for_world_allocation(world){Ok(v)=>v,Err(e)=>return Err((actual_assets,e.to_string()))};
        let next=match self.next.checked_add(1){Some(v)=>v,None=>return Err((actual_assets,"portable VWorld allocation identity exhausted".into()))};
        let allocation=Arc::new(Allocation{origin:Arc::downgrade(&self.origin),handle:self.next});
        let native=view_init_world::State::from_constructor_fields(actual_assets);
        self.next=next;
        Ok(VWorldOwner{allocation,native,directory,factory:aa_runtime::view_factory::State::not_attempted()})
    }
}
struct Allocation{origin:Weak<()>,handle:usize}
///Nonowning identity of this actual VWorld allocation, distinct from GTID,
///World revision, native serialized pointers and renderer/device addresses.
#[derive(Clone)]
pub struct ViewToken(Weak<Allocation>);
impl ViewToken{
    pub fn opaque_handle(&self)->Result<usize>{let allocation=self.0.upgrade().context("VWorld allocation was dropped")?;
        ensure!(allocation.origin.upgrade().is_some(),"VWorld application allocation owner was dropped");Ok(allocation.handle)}
    pub fn is_for<A>(&self,owner:&VWorldOwner<A>)->bool{self.opaque_handle().is_ok()&&self.0.ptr_eq(&Arc::downgrade(&owner.allocation))}
}
pub struct VWorldOwner<A>{allocation:Arc<Allocation>,native:view_init_world::State<ViewHandle,A>,directory:ViewDirectory,
    factory:aa_runtime::view_factory::State<aa_actors::ActorId>}
///Actual process/Node timer and diagnostic transport supplied by the locked
///UI caller. No elapsed value or second timer is invented by this binding.
pub trait FactoryDiagnostics{
    fn begin(&mut self)->std::result::Result<(),String>;
    fn end(&mut self,existing:i32,attempted_missing:i32)->std::result::Result<(),String>;
}
///Actual application/UI loans for a reached derived Init. Constructing this
///does not initialize a view or borrow its native Grid before50CAB0.
pub struct FactoryInit<'owners,'run>{
    application:&'owners mut aa_runtime::resource_depot::Application,
    client:&'owners crate::battle_ui_bridge::Client,
    execution:&'owners mut aa_runtime::simulation_execution::Execution<'run>,
    request_policy:&'owners super::view_request_policy::State,
    actor_globals:Option<&'owners super::view_actor_globals::State>,
    session:Option<&'owners aa_netcode::local_bootstrap::LocalBootstrap>,
    precision:aa_numeric::Precision,
}
impl<'owners,'run> FactoryInit<'owners,'run>{
    pub fn new(application:&'owners mut aa_runtime::resource_depot::Application,
        client:&'owners crate::battle_ui_bridge::Client,execution:&'owners mut aa_runtime::simulation_execution::Execution<'run>,
        request_policy:&'owners super::view_request_policy::State,precision:aa_numeric::Precision)->Self{
        Self{application,client,execution,request_policy,actor_globals:None,session:None,precision}
    }
    pub fn with_actor_globals(mut self,globals:&'owners super::view_actor_globals::State)->Self{self.actor_globals=Some(globals);self}
    pub fn with_session(mut self,session:&'owners aa_netcode::local_bootstrap::LocalBootstrap)->Self{self.session=Some(session);self}
}
pub struct Parts<'a,A>{pub native:&'a mut view_init_world::State<ViewHandle,A>,pub directory:&'a mut ViewDirectory,pub token:ViewToken}
impl<A> VWorldOwner<A>{
    pub fn token(&self)->ViewToken{ViewToken(Arc::downgrade(&self.allocation))}
    pub fn native(&self)->&view_init_world::State<ViewHandle,A>{&self.native}
    pub fn directory(&self)->&ViewDirectory{&self.directory}
    ///Prefix metadata only; completion is not derived/model/VWorld readiness.
    pub fn factory_progress(&self)->&aa_runtime::view_factory::State<aa_actors::ActorId>{&self.factory}
    pub fn parts(&mut self,world:&World<'_>)->Result<Parts<'_,A>>{
        self.directory.require_world(world)?;self.token().opaque_handle()?;
        Ok(Parts{native:&mut self.native,directory:&mut self.directory,token:ViewToken(Arc::downgrade(&self.allocation))})
    }
    ///4F0750's first child50CAB0. Each original pending payload transfers to
    ///this SAME VWorld's43B870 list; failed children remain manager.detached.
    ///This does not implement the following Thing/view factory traversal.
    pub fn attach_pending_commands(&mut self,world:&mut World<'_>)->Result<()>{
        self.directory.require_world(world)?;
        let token=self.token();let raw=u64::try_from(token.opaque_handle()?).context("portable VWorld handle exceeds FX transport width")?;
        let native=&mut self.native;
        world.with_view_binding(|parts|{
            let mut effects=Attachment{native,token:&token,world:parts.revision};
            parts.presentation.attach_view(Some(raw),&mut effects)
        }).map_err(anyhow::Error::msg)
    }
    ///Real4F0750 over this allocation and the SAME locked mutable World. The
    ///Directory retains any allocated constructor/Init prefix on failure, and
    ///only its private completed-Init receipt can publish positive Thing+C.
    pub fn publish_world_views(&mut self,world:&mut World<'_>,catalog:&aa_runtime::catalog::Catalog<'_>,
        inputs:&aa_runtime::view_startup_inputs::Inputs,diagnostics:&mut impl FactoryDiagnostics)
        ->Result<aa_runtime::view_factory::Report>{
        self.run_factory(world,catalog,inputs,diagnostics,None)
    }
    ///Concrete same-owner successor. Derived Init context is built lazily at
    ///its native callsite, after attachment/allocation/constructor prefix.
    pub fn publish_world_views_with(&mut self,world:&mut World<'_>,catalog:&aa_runtime::catalog::Catalog<'_>,
        inputs:&aa_runtime::view_startup_inputs::Inputs,diagnostics:&mut impl FactoryDiagnostics,
        initialization:FactoryInit<'_,'_>)->Result<aa_runtime::view_factory::Report>{
        self.run_factory(world,catalog,inputs,diagnostics,Some(initialization))
    }
    fn run_factory(&mut self,world:&mut World<'_>,catalog:&aa_runtime::catalog::Catalog<'_>,
        inputs:&aa_runtime::view_startup_inputs::Inputs,diagnostics:&mut impl FactoryDiagnostics,
        initialization:Option<FactoryInit<'_,'_>>)->Result<aa_runtime::view_factory::Report>{
        self.directory.require_world(world)?;
        let token=self.token();token.opaque_handle()?;
        let mut services=Factory{native:&mut self.native,directory:&mut self.directory,token:&token,catalog,inputs,diagnostics,initialization};
        let state=&mut self.factory;
        world.with_view_binding(|parts|aa_runtime::view_factory::run(state,parts,&mut services)).map_err(anyhow::Error::msg)
    }
}
struct Factory<'a,'assets,'run,A,D>{native:&'a mut view_init_world::State<ViewHandle,A>,directory:&'a mut ViewDirectory,
    token:&'a ViewToken,catalog:&'a aa_runtime::catalog::Catalog<'assets>,
    inputs:&'a aa_runtime::view_startup_inputs::Inputs,diagnostics:&'a mut D,initialization:Option<FactoryInit<'a,'run>>}
impl<A,D:FactoryDiagnostics> aa_runtime::view_factory::Services for Factory<'_,'_,'_,A,D>{
    type View=ViewHandle;
    fn begin_diagnostic(&mut self)->std::result::Result<(),String>{self.diagnostics.begin()}
    fn attach_view(&mut self,p:&mut aa_runtime::live_world_frame::ViewBindingParts<'_>)->std::result::Result<(),String>{
        let raw=u64::try_from(self.token.opaque_handle().map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        let mut effects=Attachment{native:self.native,token:self.token,world:p.revision};
        p.presentation.attach_view(Some(raw),&mut effects)
    }
    fn source_view_type(&mut self,p:&aa_runtime::live_world_frame::ViewBindingParts<'_>,id:aa_actors::ActorId)->std::result::Result<u32,String>{
        self.directory.source_view_type_bound(p,self.catalog,id).map_err(|e|e.to_string())
    }
    fn construct_view(&mut self,p:&mut aa_runtime::live_world_frame::ViewBindingParts<'_>,id:aa_actors::ActorId)->std::result::Result<Option<ViewHandle>,String>{
        if let Some(init)=self.initialization.as_mut(){
            if let Some(globals)=init.actor_globals{
                return self.directory.construct_bound_factory_with_sources(p,self.catalog,id,self.inputs,globals,&mut *init.application).map_err(|e|e.to_string());
            }
        }
        self.directory.construct_bound_factory(p,self.catalog,id,self.inputs).map_err(|e|e.to_string())
    }
    fn initialize_view(&mut self,p:&mut aa_runtime::live_world_frame::ViewBindingParts<'_>,view:ViewHandle)->std::result::Result<bool,String>{
        if let Some(init)=self.initialization.as_mut(){
            let object=self.native.object_init_parts_for(p.revision)?;
            let mut context=super::view_directory::InitContext::new(object,&mut *init.application,init.client,
                &mut *init.execution,init.request_policy,init.precision).with_replacement_color(self.inputs.replacement_color());
            if let Some(session)=init.session{context=context.with_session(session);}
            self.directory.initialize_bound_with(p,self.catalog,view,&mut context).map_err(|e|e.to_string())
        }else{self.directory.initialize_bound(p,self.catalog,view).map_err(|e|e.to_string())}
    }
    fn destroy_view(&mut self,p:&mut aa_runtime::live_world_frame::ViewBindingParts<'_>,view:ViewHandle,free:u32)->std::result::Result<(),String>{
        self.directory.destroy_bound(p,self.catalog,view,free).map_err(|e|e.to_string())
    }
    fn view_flags(&mut self,p:&aa_runtime::live_world_frame::ViewBindingParts<'_>,view:ViewHandle)->std::result::Result<u32,String>{
        self.directory.flags_bound(p,self.catalog,view).map_err(|e|e.to_string())
    }
    fn prepend_update_view(&mut self,p:&mut aa_runtime::live_world_frame::ViewBindingParts<'_>,view:ViewHandle)->std::result::Result<(),String>{
        self.directory.prepend_update_bound(p,self.catalog,view).map_err(|e|e.to_string())
    }
    fn publish_view(&mut self,p:&mut aa_runtime::live_world_frame::ViewBindingParts<'_>,id:aa_actors::ActorId,view:Option<ViewHandle>)->std::result::Result<(),String>{
        self.directory.publish_bound(p,self.catalog,id,view).map_err(|e|e.to_string())
    }
    fn end_diagnostic(&mut self,existing:i32,attempted_missing:i32)->std::result::Result<(),String>{self.diagnostics.end(existing,attempted_missing)}
}
struct Attachment<'a,A>{native:&'a mut view_init_world::State<ViewHandle,A>,token:&'a ViewToken,
    world:&'a aa_runtime::world_revision::ConstructedWorldRevision}
impl<A> aa_runtime::fx_manager::AttachmentEffects<aa_runtime::fx_graph::Command> for Attachment<'_,A>{
    fn deliver(&mut self,view:u64,message:&mut Option<aa_runtime::fx_graph::Command>)->std::result::Result<(),String>{
        let expected=u64::try_from(self.token.opaque_handle().map_err(|e|e.to_string())?).map_err(|e|e.to_string())?;
        if view!=expected{return Err("43B870 portable VWorld token belongs to another allocation".into())}
        self.native.fx_commands_mut_for(self.world)?.accept_43b870(message)?;Ok(())
    }
    fn destroy(&mut self,_:&mut aa_runtime::fx_graph::Command)->std::result::Result<(),String>{
        Err("50CAB0 muted pending command requires actual428350 destruction".into())
    }
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn view_token_cannot_keep_view_or_application_allocations_alive(){
        let application=ApplicationViews::from_application_constructor().unwrap();
        let allocation=Arc::new(Allocation{origin:Arc::downgrade(&application.origin),handle:7});
        let token=ViewToken(Arc::downgrade(&allocation));assert_eq!(token.opaque_handle().unwrap(),7);
        let foreign=Arc::new(Allocation{origin:Arc::downgrade(&application.origin),handle:7});
        assert!(!token.0.ptr_eq(&Arc::downgrade(&foreign)));drop(foreign);
        drop(application);assert!(token.opaque_handle().is_err());drop(allocation);assert!(token.0.upgrade().is_none());
        //The camera directory now owns actual Rc scene nodes and remains on
        //SimulationThread. Only the nonowning token may cross to the UI.
        fn send<T:Send>(){}send::<ViewToken>();
    }
}
