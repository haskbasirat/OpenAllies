//!Concrete UI5C3000 over the retained activated Session/World and ONE client
//!VWorld allocation. Backend jobs borrow the installed UiBridge; no inspector
//!scene, second World/variable manager or GPU acknowledgement is readiness.
use crate::{battle_ui_bridge,live_scene::world_owner::{ApplicationViews,VWorldOwner}};
use aa_actors::ActorId;
use aa_netcode::{local_bootstrap::{LocalBootstrap,world_ready::ActualActivatedSession},local_host::{MillisecondTimer,NodeTime}};
use aa_numeric::Precision;
use aa_runtime::{catalog::Catalog,live_world_frame::World,server_clock::TimeSource,
    simulation_execution::Execution,ui_world_init,view_assets,view_grid_state,view_init_prefix,world_ready_services::Application};

type Result<T>=std::result::Result<T,String>;
#[path="vworld_frame.rs"]
mod vworld_frame;
#[derive(Debug)]
pub enum Diagnostic{
    ///Actual59ABA0 input retained by the concrete modern diagnostic sink.
    ///No rewritten printf formatting is used as a gameplay input.
    InitVWorld{milliseconds:u32},
    ResettingEditorVariables,
    BeginWorldViews{milliseconds:u32},
    EndWorldViews{milliseconds:u32,existing:i32,attempted_missing:i32},
}
///Failed prefixes remain here beside the same activated Session. No Clone,
///raw ready constructor or replacement view is supplied on retry.
pub struct State{
    ui:ui_world_init::State<ActorId>,
    view:VWorldOwner<view_assets::State>,
    diagnostics:Vec<Diagnostic>,
    failed_grid:Option<view_grid_state::Failure<crate::live_scene::view_directory::ViewHandle>>,
    scene:crate::battle_scene_startup::State,
}
impl State{
    pub fn construct(views:&mut ApplicationViews,world:&World<'_>)->Result<Self>{
        let view=views.construct(world).map_err(|e|e.to_string())?;
        Ok(Self{ui:ui_world_init::State::unbound_ui_fields(),view,diagnostics:Vec::new(),failed_grid:None,
            scene:crate::battle_scene_startup::State::not_attempted()})
    }
    pub fn native_view(&self)->&VWorldOwner<view_assets::State>{&self.view}
    ///battle_orders_input::Live::views for one orders frame, without the
    ///Session/UI words (those leaves are explicit errors); see order_views_with.
    pub fn order_views<'s>(&'s mut self,world:&'s World<'_>)->Result<crate::live_scene::view_directory::DirectoryViews<'s>>{
        self.view.order_views(world,None,None).map_err(|e|format!("{e:#}"))
    }
    ///battle_orders_input::Live::views with the frame's [`Self::order_context`];
    ///actor_virtual_150 then errs (use order_views_with_catalog).
    pub fn order_views_with<'s>(&'s mut self,world:&'s World<'_>,context:&'s crate::live_scene::view_directory::OrderContext)
        ->Result<crate::live_scene::view_directory::DirectoryViews<'s>>{
        self.view.order_views(world,Some(context),None).map_err(|e|format!("{e:#}"))
    }
    ///As order_views_with, plus the frame's Catalog for 4CF430 (actor_virtual_150).
    pub fn order_views_with_catalog<'s>(&'s mut self,world:&'s World<'_>,context:&'s crate::live_scene::view_directory::OrderContext,
        catalog:&'s Catalog<'_>)->Result<crate::live_scene::view_directory::DirectoryViews<'s>>{
        self.view.order_views(world,Some(context),Some(catalog)).map_err(|e|format!("{e:#}"))
    }
    ///The orders frame's Session (E4, Node+8, 76AC70, F8, 40EAA0), UI+1E0 and
    ///UVar PerformanceDisableDoodads words, read from their actual owners
    ///before the AdminNode loan (call it before session.parts()).
    pub fn order_context(&self,session:&ActualActivatedSession,bridge:&battle_ui_bridge::Client,
        execution:&mut Execution<'_>)->crate::live_scene::view_directory::OrderContext{
        let kingdom=session.local_player_kingdom_76ac70().map_err(|e|e.to_string());
        let can_process=session.host().can_process_40eaa0(||bridge.call(&mut *execution,|c|c.ui.uvars.dev_mode_current()));
        let doodads=bridge.call(&mut *execution,|c|Ok(c.ui.uvars.view_bool_raw("PerformanceDisableDoodads")));
        let swap=bridge.call(&mut *execution,|c|c.ui.uvars.swap_right_click_with_alt_right_click_current());
        let hotkey=bridge.call(&mut *execution,|c|c.ui.uvars.hotkey_alt_right_click_command_current());
        crate::live_scene::view_directory::OrderContext::new(session.session_state_e4(),session.local_player_present_76ac60(),
            kingdom,session.session_phase_f8(),can_process,self.ui.kingdom_1e0().map(|k|k.copied()),doodads,swap,hotkey)
    }
    ///One native UI frame of the VWorld while VWorld+AD is set: 5C92F0's
    ///43AB30, then the widget tree's 43FF60 -> 43EC60 (vworld_frame.rs). The
    ///43EC60 VWorld+208 segment releases through Data vf18(1) -> Resource vf14
    ///5AA550 with the SAME ResourceManager and resource-task backend. This
    ///entry lends neither the B396C4 reader nor ApplicationViews; children
    ///that need them refuse explicitly (see update_vworld_frame).
    pub fn update_view_requests_43ec60(&mut self,source:&std::sync::Arc<aa_rules::actors::ActorCatalog>,
        session:&mut ActualActivatedSession,bridge:&battle_ui_bridge::Client,execution:&mut Execution<'_>,pc:Precision)->Result<()>{
        self.update_vworld_frame(source,session,bridge,execution,None,None,pc)
    }
    ///update_view_requests_43ec60 with the frame's application window reader
    ///(76B120's B396C4) and ApplicationViews (current camera, B36518).
    #[allow(clippy::too_many_arguments)]
    pub fn update_vworld_frame(&mut self,source:&std::sync::Arc<aa_rules::actors::ActorCatalog>,
        session:&mut ActualActivatedSession,bridge:&battle_ui_bridge::Client,execution:&mut Execution<'_>,
        focus:Option<&aa_runtime::application_window_state::Reader>,views:Option<&mut ApplicationViews>,pc:Precision)->Result<()>{
        let mut catalog=Catalog::new(source);
        let resources=session.world().evidence().resource_source.as_ref()
            .ok_or("VWorld request queue actual Resource source owner absent")?;
        catalog.bind_resource_catalog(std::sync::Arc::clone(resources))?;
        catalog.settings=Some(session.world().presentation_parts()?.settings.clone());
        catalog.native_load_mode=session.world().evidence().native_load_mode;
        let session_f4=session.session_multiplayer_f4();
        let(host,world,application)=session.parts();
        vworld_frame::run(vworld_frame::Loans{host,world,application,catalog:&catalog,view:&mut self.view,bridge,execution,
            focus,views,session_f4,pc})
    }
    ///One VWorld update's 4312F0 (43EC60 at 43ED0A): `pass` runs the terrain
    ///visibility owner's 42EED0/430E50/42FF50 with these objects as its
    ///BlockObjects (42FAD0 per active block before 720080, then 42F670), over
    ///the SAME World, directory, VGrid, heightfield and Data/Resource driver.
    #[allow(clippy::too_many_arguments)]
    pub fn stream_views_4312f0(&mut self,source:&std::sync::Arc<aa_rules::actors::ActorCatalog>,
        session:&mut ActualActivatedSession,bridge:&battle_ui_bridge::Client,execution:&mut Execution<'_>,
        model:&mut dyn crate::live_scene::view_directory::ModelLifecycle,pc:Precision,
        pass:impl FnOnce(&World<'_>,&mut crate::live_scene::view_directory::StreamingObjects<'_>)->Result<()>)->Result<()>{
        let mut catalog=Catalog::new(source);
        let resources=session.world().evidence().resource_source.as_ref()
            .ok_or("view streaming actual Resource source owner absent")?;
        catalog.bind_resource_catalog(std::sync::Arc::clone(resources))?;
        catalog.settings=Some(session.world().presentation_parts()?.settings.clone());
        catalog.native_load_mode=session.world().evidence().native_load_mode;
        let(host,world,application)=session.parts();
        let parts=world.presentation_parts()?;
        let guards=application.depot_mut().resource_task_guards();
        let driver=crate::battle_resource_tasks::Current::new(bridge,execution,guards,pc).with_session(host);
        let mut data=crate::live_scene::view_directory::CurrentData{catalog:&catalog,bank:parts.definitions,biome:parts.view_biome,
            application:application.depot_mut(),driver:std::cell::RefCell::new(driver)};
        let mut objects=self.view.streaming_objects(world,model,&mut data,pc).map_err(|e|format!("{e:#}"))?;
        pass(world,&mut objects)
    }
    ///The view-cell half of one 4312F0 (ViewCellHooks for the terrain
    ///visibility pass): `planes` are the current camera's +19C..+1F8.
    pub fn view_cells<'s>(&'s mut self,world:&'s World<'_>,planes:[[u32;4];6],
        model:&'s mut dyn crate::live_scene::view_directory::ModelLifecycle,
        data:&'s mut dyn crate::live_scene::view_directory::DataLifecycle,pc:Precision)
        ->Result<crate::live_scene::view_directory::ViewCells<'s>>{
        let objects=self.view.streaming_objects(world,model,data,pc).map_err(|e|format!("{e:#}"))?;
        Ok(crate::live_scene::view_directory::ViewCells{objects,planes})
    }
    pub fn ui_fields(&self)->&ui_world_init::State<ActorId>{&self.ui}
    pub fn diagnostics(&self)->&[Diagnostic]{&self.diagnostics}
    pub fn initialize<T:MillisecondTimer>(&mut self,session:&mut ActualActivatedSession,catalog:&Catalog<'_>,
        time:&mut NodeTime<T>,bridge:&battle_ui_bridge::Client,execution:&mut Execution<'_>,views:&mut ApplicationViews,map_id:&str,init_word_b8:u32,pc:Precision)->Result<()>{
        let(host,world,application)=session.parts();
        //Constructor/current World correlation is checked before lending its
        //same native coordinator; no numeric-revision match can replace it.
        self.view.parts(world).map_err(|e|e.to_string())?;
        let revision=world.presentation_parts()?.revision.witness();
        let mut services=Current{host,world,application,catalog,time,bridge,execution,views,
            view:&mut self.view,diagnostics:&mut self.diagnostics,failed_grid:&mut self.failed_grid,
            scene:&mut self.scene,map_id,phase:None,revision:revision.clone(),pc};
        let initialized=self.ui.initialize_5c3000_for_notification(&revision,init_word_b8,&mut services);
        drop(services); //The actual world/Session lock ends BEFORE task dispatch.
        initialized?;
        self.dispatch_view_resource_barrier(session,catalog,bridge,execution,pc)
    }
    fn dispatch_view_resource_barrier(&mut self,session:&mut ActualActivatedSession,catalog:&Catalog<'_>,
        bridge:&battle_ui_bridge::Client,execution:&mut Execution<'_>,pc:Precision)->Result<()>{
        let task=self.ui.task().ok_or("actual queued View task witness is absent")?;
        let(host,world,application)=session.parts();
        let parts=world.presentation_parts()?;
        let app=application.view_startup_parts(catalog,parts.definitions)?;
        let guards=app.depot.resource_task_guards();
        let mut services=crate::battle_resource_tasks::Current::new(bridge,execution,guards,pc).with_session(host);
        loop{
            let report=app.depot.dispatch_queued_task_5addd0(&mut services)?
                .ok_or("resource queue emptied before the actual published View task was dispatched")?;
            if report.is_for_view(task){
                if report.state!=2{return Err("actual View resource task was cancelled before its completion callback".into())}
                return Ok(())
            }
        }
    }
}
struct Current<'a,'source,'execution,T:MillisecondTimer>{
    host:&'a LocalBootstrap,world:&'a mut World<'static>,application:&'a mut Application,catalog:&'a Catalog<'source>,
    time:&'a mut NodeTime<T>,bridge:&'a battle_ui_bridge::Client,execution:&'a mut Execution<'execution>,
    views:&'a mut ApplicationViews,view:&'a mut VWorldOwner<view_assets::State>,diagnostics:&'a mut Vec<Diagnostic>,pc:Precision,
    failed_grid:&'a mut Option<view_grid_state::Failure<crate::live_scene::view_directory::ViewHandle>>,
    scene:&'a mut crate::battle_scene_startup::State,map_id:&'a str,
    phase:Option<aa_netcode::local_bootstrap::view_startup::SessionPhaseLease>,
    revision:aa_runtime::world_revision::Witness,
}
impl<T:MillisecondTimer> Current<'_,'_,'_,T>{
    ///Called only after the same actual43F1D4 asset owner completed. The
    ///scene coordinator independently repeats that guard before its first child.
    fn initialize_scene_after_assets(&mut self)->Result<()>{
        let world=self.world.presentation_parts()?;
        let parts=self.application.view_startup_parts(self.catalog,world.definitions)?;
        let native=self.view.parts(self.world).map_err(|e|e.to_string())?.native;
        self.scene.initialize(native,self.world,self.catalog,parts.inputs,parts.depot,self.views,
            self.host,self.bridge,self.execution,self.map_id,&mut ||TimeSource::milliseconds(self.time),self.pc)?;
        //The specialized guard checks every actual asset/scene/terrain/tail
        //owner before43FF36. This leaves AC for the real resource callback.
        native.complete_init_after_children(self.world,self.catalog)
    }
}
impl<T:MillisecondTimer> ui_world_init::Services for Current<'_,'_,'_,T>{
    type Kingdom=ActorId;
    fn init_diagnostic(&mut self)->Result<()>{
        let milliseconds=TimeSource::milliseconds(self.time)?;
        log::info!("native InitVWorld start, actual Node timer={milliseconds}ms");
        self.diagnostics.push(Diagnostic::InitVWorld{milliseconds});Ok(())
    }
    fn with_world_lock<R>(&mut self,call:impl FnOnce(&mut Self)->Result<R>)->Result<R>{
        let host=self.host;host.with_view_phase_lease_5c3000(|phase|{
            self.phase=Some(phase);
            let result=call(self);
            self.phase=None;
            result
        })
    }
    fn selected_kingdom_76c2b0(&mut self)->Result<Option<ActorId>>{
        self.host.ui_selected_kingdom_76c2b0(self.world).map_err(|e|e.to_string())
    }
    fn reset_camera_700e30(&mut self)->Result<()>{
        let world=self.world.presentation_parts()?;
        let parts=self.application.view_startup_parts(self.catalog,world.definitions)?;
        let(camera,visual)=self.views.camera_and_visual_map();
        let op=crate::battle_camera::Operation::new(self.catalog,self.world,&*parts.inputs,visual,
            self.view.native().heightfield(),self.host,self.bridge,self.execution,self.pc)?;
        camera.reset_current_700e30(&op).map(|_|())
    }
    fn initialize_vworld_43f0a0(&mut self,init_word_b8:u32)->Result<()>{
        {
            let world=self.world.presentation_parts()?;
            if !self.revision.is_for(world.revision){return Err("UI/VWorld initialization names another current World".into())}
            let parts=self.application.view_startup_parts(self.catalog,world.definitions)?;
            let input=&*parts.inputs;
            input.validate(self.catalog,world.definitions)?;
            let mut prefix=Prefix{catalog:self.catalog,input,bank:world.definitions,biome:world.view_biome,depot:parts.depot,host:self.host,
                bridge:self.bridge,execution:self.execution,diagnostics:self.diagnostics,failed_grid:self.failed_grid,
                phase:&mut self.phase};
            self.view.parts(self.world).map_err(|e|e.to_string())?.native.initialize_prefix_with_argument(world.revision,init_word_b8,&mut prefix)?;
        }
        {
            let world=self.world.presentation_parts()?;
            let parts=self.application.view_startup_parts(self.catalog,world.definitions)?;
            //Deliberate modern710433 mapping from the actual successful
            //renderer/device/pipeline capability, before Source factories.
            crate::battle_texture_services::Current::for_controlled_fresh_start(self.bridge,self.execution)
                .apply_initialized_renderer(parts.depot)?;
            let guards=parts.depot.resource_task_guards();
            let mut services=crate::battle_resource_tasks::Current::new(self.bridge,self.execution,guards,self.pc).with_session(self.host);
            let mut globals=parts.inputs.seeds().bind(self.catalog)?;
            self.view.parts(self.world).map_err(|e|e.to_string())?.native.assets_mut().initialize(
                self.catalog,world.definitions,world.revision,parts.inputs.aliases(),parts.depot,&mut services,&mut globals,self.pc)?;
        }
        self.initialize_scene_after_assets()
    }
    fn camera_zero_700500(&mut self,zero:[u32;2])->Result<()>{
        let world=self.world.presentation_parts()?;
        let parts=self.application.view_startup_parts(self.catalog,world.definitions)?;
        let(camera,visual)=self.views.camera_and_visual_map();
        let op=crate::battle_camera::Operation::new(self.catalog,self.world,&*parts.inputs,visual,
            self.view.native().heightfield(),self.host,self.bridge,self.execution,self.pc)?;
        camera.zero_pan_current_700500(zero,&op)
    }
    fn world_on_vworld_created_4f0750(&mut self)->Result<()>{
        let world=self.world.presentation_parts()?;
        if !self.revision.is_for(world.revision){return Err("4F0750 publication names another current World allocation".into())}
        let parts=self.application.view_startup_parts(self.catalog,world.definitions)?;
        let initialization=crate::live_scene::world_owner::FactoryInit::new(parts.depot,self.bridge,
            self.execution,self.views.request_policy(),self.pc).with_actor_globals(self.views.actor_globals()).with_session(self.host);
        let mut diagnostics=FactoryDiagnostics{time:self.time,events:self.diagnostics};
        self.view.publish_world_views_with(self.world,self.catalog,&*parts.inputs,&mut diagnostics,initialization).map(|_|()).map_err(|e|e.to_string())
    }
    fn enqueue_view_task_5aed10(&mut self,slot:&mut Option<aa_runtime::view_resource_task::Task>)->Result<()>{
        let world=self.world.presentation_parts()?;
        self.application.enqueue_view_resource_task(self.catalog,world.definitions,world.revision,
            self.view.native().prefix(),slot)
    }
}
struct FactoryDiagnostics<'a,T:MillisecondTimer>{time:&'a mut NodeTime<T>,events:&'a mut Vec<Diagnostic>}
impl<T:MillisecondTimer> crate::live_scene::world_owner::FactoryDiagnostics for FactoryDiagnostics<'_,T>{
    fn begin(&mut self)->Result<()>{
        let milliseconds=TimeSource::milliseconds(self.time)?;
        log::info!("native World view creation begin, actual timer={milliseconds}ms");
        self.events.push(Diagnostic::BeginWorldViews{milliseconds});Ok(())
    }
    fn end(&mut self,existing:i32,attempted_missing:i32)->Result<()>{
        let milliseconds=TimeSource::milliseconds(self.time)?;
        log::info!("native World view creation end: existing={existing}, attempted missing={attempted_missing}, actual timer={milliseconds}ms");
        self.events.push(Diagnostic::EndWorldViews{milliseconds,existing,attempted_missing});Ok(())
    }
}
struct Prefix<'a,'source,'execution>{catalog:&'a Catalog<'source>,input:&'a aa_runtime::view_startup_inputs::Inputs,
    host:&'a LocalBootstrap,
    bank:&'a aa_runtime::definitions::Bank,biome:Option<&'a aa_runtime::vworld_resource_context::Biome>,
    depot:&'a mut aa_runtime::resource_depot::Application,
    bridge:&'a battle_ui_bridge::Client,execution:&'a mut Execution<'execution>,
    diagnostics:&'a mut Vec<Diagnostic>,
    failed_grid:&'a mut Option<view_grid_state::Failure<crate::live_scene::view_directory::ViewHandle>>,
    phase:&'a mut Option<aa_netcode::local_bootstrap::view_startup::SessionPhaseLease>}
impl view_init_prefix::Services for Prefix<'_,'_,'_>{
    type Grid=view_grid_state::State<crate::live_scene::view_directory::ViewHandle>;
    fn begin_screen_7040e0(&mut self)->Result<()>{
        self.bridge.call(self.execution,|context|{context.renderer.screen_manager_mut().begin_7040e0();Ok(())})
    }
    fn reset_user_746e30(&mut self)->Result<()>{
        let enabled=self.host.can_process_40eaa0(||self.bridge.call(self.execution,|context|context.ui.uvars.dev_mode_current()))?;
        if enabled {
            //The actual4B2830 callback is RET, but746790 must still traverse
            //the SAME Audio manager's three lists. No invented empty list.
            Err("746E30 positive gate requires actual AudioManager+20/+28/+30 list owners for746790 traversal".into())
        }else{Ok(())}
    }
    fn resource_context_5ae0a0(&mut self)->Result<()>{
        let biome=self.biome.ok_or("43F0F1 requires same current GMap+24 Biome owner")?;
        self.depot.set_vworld_resource_context(self.catalog,self.bank,biome).map(|_|())
    }
    fn construct_grid_430290(&mut self)->Result<Option<Self::Grid>>{
        let mut seeds=self.input.seeds().bind(self.catalog)?;
        match view_grid_state::State::from_constructor(&mut seeds){
            Ok(grid)=>Ok(Some(grid)),Err(failure)=>{
                let error=failure.error.clone();*self.failed_grid=Some(failure);Err(error)
            }
        }
    }
    fn resetting_editor_variables_diagnostic(&mut self)->Result<()>{
        log::info!("native VWorld resetting editor variables");self.diagnostics.push(Diagnostic::ResettingEditorVariables);Ok(())
    }
    fn reset_view_variables(&mut self)->Result<()>{
        //The immutable Host and actual mutex remain loaned through this RPC.
        //76AC90 is a pure F8 getter; the UI performs each logical read at the
        //native index44 branch. The Weak lease expires when that scope ends.
        let phase=self.phase.take().ok_or("view reset is outside actual locked Session phase scope")?;
        self.bridge.call(self.execution,move|context|context.ui.uvars.reset_view_for_loaded_map(||
            phase.read_76ac90()).map(|_|()).map_err(|failure|failure.reason))
    }
}
