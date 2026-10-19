//!Actual FIFO notification -> AppState -> UI5C3000 on the retained simulation
//!thread. Mutable menus stay on the UI thread behind the installed bridge.
//!Errors retain the popped value, native prefixes and a stop latch.
use crate::{
    battle_menus::{Handle, Selectors},
    battle_ui_bridge, battle_view_startup,
    live_scene::world_owner::ApplicationViews,
    windows_reset_os::PerformanceTimer,
};
use aa_netcode::{
    local_bootstrap::{self, Notification, Notifications, world_ready::ActualActivatedSession},
    local_host::{MillisecondTimer, NodeTime},
};
use aa_numeric::Precision;
use aa_runtime::{
    application_loading, application_menu_callbacks as callbacks, application_state,
    catalog::Catalog, simulation_execution::Execution,
};
type Result<T> = std::result::Result<T, String>;
///The live frame's own Catalog (aa-netcode `run_native_frame_bound`): the
///SAME source, Resource catalog, settings and load mode as the notification
///Catalog, plus the application's retained AVar25 cell, borrowed per frame.
pub struct FrameCatalog<'s> {
    pub source: &'s aa_rules::actors::ActorCatalog,
    pub resources: std::sync::Arc<aa_rules::resources::ResourceCatalog>,
    pub settings: &'s aa_runtime::settings::Settings,
    pub load_mode: Option<u32>,
}
impl local_bootstrap::live_frame::FrameCatalog for FrameCatalog<'_> {
    fn build<'o>(&'o self,orientation:&'o aa_runtime::view_renderer_variables::State)->Result<Catalog<'o>> {
        let mut catalog=Catalog::new(self.source);
        catalog.bind_resource_catalog(std::sync::Arc::clone(&self.resources))?;
        catalog.settings=Some(self.settings.clone());
        catalog.native_load_mode=self.load_mode;
        catalog.bind_renderer_orientation(orientation)?;
        Ok(catalog)
    }
}
pub struct State {
    application: application_state::State,
    selectors: Option<Selectors>,
    main_4: Option<Handle>,
    game_4:Option<Handle>,
    loading_4: Option<Handle>,
    pending: Option<Notification>,
    stopped: Option<String>,
    diagnostics: Vec<String>,
    status_time: PerformanceTimer,
    frame_diagnostics:local_bootstrap::live_frame::Diagnostics,
}
impl State {
    ///6E80F0: App(B39690)->40D970 returns Application+E4 (the Loading node,
    ///whatever the current AppState is) and stores its+16=1. No AppState test.
    pub fn user_ready_command(&mut self,session:&mut ActualActivatedSession)->Result<()>{
        session.parts().2.loading_mut().user_ready_command_6e80f0();Ok(())
    }
    ///Application+10C, read on its owning simulation thread.
    pub fn current_state(&self)->u32{self.application.current()}
    ///One real native frame after this job's FIFO/Loading dispatch. The
    ///Admin304/315/775BF0 protocol owns unfreeze; an AppState/menu never does.
    ///Native 410E70 order: the main thread holds the game-state lock
    ///([B36524]+8 vt+4) across 40FBE0 notifications, the node tick and the UI
    ///update 5C92D0 ([`Self::drain`], which runs Game 612010 in AppState 4),
    ///releases it (vt+8) and renders; the NetNode Update 765410 -> AdminNode
    ///75CA70 frame runs between those locked sections. This job therefore
    ///follows the drain on the SAME simulation thread. `frame_catalog`
    ///builds the frame's Catalog with the SAME application's AVar25
    ///RendererDefaultOrientation owner bound (552DD0, Disturbable scans).
    #[allow(clippy::too_many_arguments)]
    pub fn run_frame<T:MillisecondTimer>(
        &mut self,session:&mut ActualActivatedSession,catalog:&Catalog<'_>,
        frame_catalog:&FrameCatalog<'_>,
        notifications:&Notifications,time:&mut NodeTime<T>,
        focus:&aa_runtime::application_window_state::Reader,
        bridge:&battle_ui_bridge::Client,execution:&mut Execution<'_>,
    )->Result<()> {
        if let Some(error)=&self.stopped{return Err(error.clone())}
        let identity=execution.identity();
        let os=std::cell::RefCell::new(crate::windows_reset_os::WindowsThreadIds);
        let mut audio=CurrentAudio{bridge,execution};
        match session.run_native_frame_bound(catalog,frame_catalog,time,focus,notifications,&identity,&os,&mut audio,&mut self.frame_diagnostics,Precision::Double53){
            Ok(_)=>Ok(()),
            Err(error)=>{let error=error.to_string();self.stopped=Some(error.clone());self.diagnostics.push(error.clone());Err(error)},
        }
    }
    pub fn selected_map_confirmation(
        &self,
    ) -> Result<local_bootstrap::selected_map_confirm::OrdinaryEntry> {
        local_bootstrap::selected_map_confirm::OrdinaryEntry::capture_current_application(
            &self.application,
        )
        .map_err(|e| e.to_string())
    }
    pub fn from_application_constructor() -> Result<Self> {
        Ok(Self {
            application: application_state::State::from_constructor(),
            selectors: None,
            main_4: None,
            game_4:None,
            loading_4: None,
            pending: None,
            stopped: None,
            diagnostics: Vec::new(),
            //Separate actual QPC anchor for global B396E0's modern timer backend;
            //Node+50 remains a distinct timer and is never substituted here.
            status_time: PerformanceTimer::from_native_constructor().map_err(|e| e.to_string())?,
            frame_diagnostics:local_bootstrap::live_frame::Diagnostics::default(),
        })
    }
    fn set_state(
        &mut self,
        session: &mut ActualActivatedSession,
        state: u32,
        arg8: Option<Handle>,
        bridge: &battle_ui_bridge::Client,
        execution: &mut Execution<'_>,
    ) -> Result<()> {
        let selectors = self
            .selectors
            .as_ref()
            .ok_or("actual App menu table is unbound")?;
        //The LoadingScreen OnConnected 6E8730 inside this transition reads
        //the Session/GameAppState; the same values are read before the
        //Loading allocation is lent to the transition.
        let loading_frame = (state == 6).then(|| crate::battle_menus::game_hud::LoadingFrame::read(session));
        let mut services = Callbacks {
            main_4: &self.main_4,
            game_4:&self.game_4,
            loading_4: &mut self.loading_4,
            selectors,
            loading: session.parts().2.loading_mut(),
            loading_frame,
            bridge,
            execution,
            diagnostics: &mut self.diagnostics,
        };
        self.application.set_state(state, &arg8, &(), &mut services)
    }
    fn bind_menus(
        &mut self,
        session: &mut ActualActivatedSession,
        catalog: &Catalog<'_>,
        bridge: &battle_ui_bridge::Client,
        execution: &mut Execution<'_>,
    ) -> Result<()> {
        if self.selectors.is_none() {
            //The Game interface Init6182B0 builds its EconomyBar from the
            //actual completed native Resource registry ([B36530]+0x34).
            let resource_ids = catalog
                .actors
                .native_resource_registry(Precision::Double53)
                .map_err(|e| format!("Game interface Init6182B0 Resource registry: {e}"))?
                .ids;
            let selectors = bridge.call(execution, move |context| context.menus.bind(context.ui, &resource_ids))?;
            //The same registry's 48B800 entries feed the Game children.
            let source = session.world().evidence().resource_source.as_ref()
                .ok_or("Game interface Resource source owner is absent")?;
            let entries = crate::battle_menus::game_hud::resource_entries(catalog, source)?;
            bridge.call(execution, move |context| context.menus.bind_game_resources(entries))?;
            self.main_4 = Some(selectors.splash.clone());
            //608C60→40CA10 copies actual globalB3974C, distinct from the
            //main/splash/loading interface allocations. Node4 owns this loan.
            self.game_4=Some(selectors.game.clone());
            let main = selectors.main.clone();
            self.selectors = Some(selectors);
            //Modern front-end enters real MainMenu0 explicitly. The native
            //Splash/Intro prelude is a separately qualified presentation gap.
            self.set_state(session, 2, Some(main), bridge, execution)?;
        }
        Ok(())
    }
    ///Drain the SAME real queue in order. No pop, repaint, worker receipt or
    ///GPU upload changes Loading15/17 or Sim29.
    pub fn drain<T: MillisecondTimer>(
        &mut self,
        session: &mut ActualActivatedSession,
        catalog: &Catalog<'_>,
        notifications: &Notifications,
        time: &mut NodeTime<T>,
        views: &mut ApplicationViews,
        view_startup: &mut Option<battle_view_startup::State>,
        bridge: &battle_ui_bridge::Client,
        execution: &mut Execution<'_>,
        map_id: &str,
    ) -> Result<()> {
        if let Some(error) = &self.stopped {
            return Err(error.clone());
        }
        let result = (|| {
            self.bind_menus(session, catalog, bridge, execution)?;
            loop {
                if self.pending.is_none() {
                    self.pending = notifications.pop().map_err(|e| e.to_string())?;
                }
                let Some(mut event) = self.pending.take() else {
                    break;
                };
                if let Err(error) = self.event(
                    &mut event,
                    session,
                    catalog,
                    time,
                    views,
                    view_startup,
                    bridge,
                    execution,
                    map_id,
                ) {
                    self.pending = Some(event);
                    return Err(error);
                }
            }
            self.tick_loading(session, catalog, view_startup, bridge, execution)?;
            //5C92D0 UI update: the current Game interface +0C (612010).
            if self.application.current() == 4 {
                let menu = self.game_4.clone().ok_or("current Game+4 is null")?;
                let view = view_startup
                    .as_ref()
                    .ok_or("Game interface update requires the retained VWorld")?;
                let frame = crate::battle_menus::game_hud::LiveFrame::read(
                    session,
                    view.native_view().native().prefix(),
                    self.application.current(),
                    || bridge.call(execution, |c| c.ui.uvars.dev_mode_current()),
                );
                bridge.call(execution, move |c| c.menus.game_update(c.ui, &menu, &frame))?;
            }
            if self.application.current() == 6 {
                let menu = self.loading_4.clone().ok_or("current Loading+4 is null")?;
                callbacks::ready_prompt_6e8c00(&mut ReadyPrompt{session,menu:&menu,bridge,execution,time:&mut self.status_time,diagnostics:&mut self.diagnostics})?;
                let text = session.parts().0.status_text_148().to_vec();
                bridge.call(execution, move |c| {
                    c.menus.refresh_status(c.ui, &menu, &text)
                })?;
            }
            Ok(())
        })();
        if let Err(error) = &result {
            self.diagnostics.push(error.clone());
            self.stopped = Some(error.clone());
        }
        result
    }
    #[allow(clippy::too_many_arguments)]
    fn event<T: MillisecondTimer>(
        &mut self,
        event: &mut Notification,
        session: &mut ActualActivatedSession,
        catalog: &Catalog<'_>,
        time: &mut NodeTime<T>,
        views: &mut ApplicationViews,
        view_startup: &mut Option<battle_view_startup::State>,
        bridge: &battle_ui_bridge::Client,
        execution: &mut Execution<'_>,
        map_id: &str,
    ) -> Result<()> {
        //40FBE0 dispatches each popped event to the current node's +1C:
        //Loading682FD0 runs its derived prefix before the base414620 switch;
        //Game uses 414620 itself and MainMenu 414D60, a jmp to 414620.
        let node = match self.application.current() {
            2 => Node::Main,
            4 => Node::Game,
            6 => Node::Loading,
            _ => return Err("414620 actual current AppState handler is not implemented".into()),
        };
        let derived_loading = match node.slots().event_1c {
            0x682fd0 => true,
            0x414620 | 0x414d60 => false,
            other => return Err(format!("AppState event handler {other:#x} is not ported")),
        };
        if derived_loading {
            let (host, world, app) = session.parts();
            let mut sink = StatusSink {
                time: &mut self.status_time,
                diagnostics: &mut self.diagnostics,
            };
            app.loading_mut()
                .notification_prefix_682fd0(event.kind, || {
                    host.update_status_773ce0(
                        &world.evidence().live_text,
                        &"app_status_loading_resources"
                            .encode_utf16()
                            .collect::<Vec<_>>(),
                        0,
                        1,
                        &mut sink,
                        Precision::Double53,
                    )
                    .map_err(|e| e.to_string())
                })?;
        }
        match event.kind {
            0 => {
                //784220's flag04 mode branch remains required on the actual
                //positive predicate; it cannot be replaced with a null manager.
                if event.launch_state.flag_04 {
                    return Err(
                        "784220 positive GameApp651840 mode/manager reset binding is required"
                            .into(),
                    );
                }
                if event.launch_state.kind != 6 {
                    return Err(format!(
                        "784220 actual launch state{} is not bound",
                        event.launch_state.kind
                    ));
                }
                {
                    let (host, _, app) = session.parts();
                    local_bootstrap::world_view::bind_loading_notification(host, app, event)
                        .map_err(|e| e.to_string())?;
                }
                let arg8 = match event.launch_state.field_0c {
                    u32::MAX => None,
                    0 => Some(
                        self.selectors
                            .as_ref()
                            .ok_or("actual menus absent")?
                            .main
                            .clone(),
                    ),
                    2 => Some(
                        self.selectors
                            .as_ref()
                            .ok_or("actual menus absent")?
                            .loading
                            .clone(),
                    ),
                    other => {
                        return Err(format!("40D0F0 actual menu slot{other} is not constructed"));
                    }
                };
                self.set_state(session, event.launch_state.kind, arg8, bridge, execution)?;
                let node = match self.application.current() {
                    2 => Node::Main,
                    4 => Node::Game,
                    6 => Node::Loading,
                    other => return Err(format!("784220 current state{other} vf28 is unbound")),
                };
                match node.slots().finished_28 {
                    0x682db0 => session
                        .parts()
                        .2
                        .loading_mut()
                        .menu_finished_682db0(u8::from(event.flag_34)),
                    //943800 is ret 4 (MainMenu and Game).
                    0x943800 => {}
                    other => return Err(format!("784220 vf28 {other:#x} is not ported")),
                }
                event.launch_state = local_bootstrap::constructor_launch_state();
                session
                    .parts()
                    .0
                    .request_version_ack_766d40()
                    .map_err(|e| e.to_string())?;
                //B3E1E0 belongs to the native event/log transport; this modern
                //queue has no native numeric pointer or fabricated timer word.
                Ok(())
            }
            1 => {
                //ActualActivatedSession supplies the genuine current World.
                //40CF00 invokes5C3000 first, then rereads current node+4/E8.
                if view_startup.is_some() {
                    return Err(
                        "40CF00 would repeat a retained VWorld initialization attempt".into(),
                    );
                }
                *view_startup = Some(battle_view_startup::State::construct(
                    views,
                    session.world(),
                )?);
                view_startup
                    .as_mut()
                    .expect("retained actual constructor")
                    .initialize(
                        session,
                        catalog,
                        time,
                        bridge,
                        execution,
                        views,
                        map_id,
                        event.word_38,
                        Precision::Double53,
                    )?;
                let menu = match self.application.current() {
                    2 => self.main_4.clone(),
                    4=>self.game_4.clone(),
                    6 => self.loading_4.clone(),
                    _ => None,
                }
                .ok_or("40CF00 actual current state menu+4 is null/unbound")?;
                bridge.call(execution, move |c| c.menus.initialized_callback_e8(&menu))
            }
            2 | 3 => {
                if !views.current_initialized_for_application()? {
                    self.diagnostics.push(
                        "414620 EndSession before VWorld.AD; actual native diagnostic branch"
                            .into(),
                    );
                    Ok(())
                } else {
                    Err(
                        "414620 positive40CF30 same VWorld destruction/clear path is required"
                            .into(),
                    )
                }
            }
            9 => Ok(()),
            other => Err(format!(
                "414620 notification kind{other} requires its actual child"
            )),
        }
    }
    fn tick_loading(
        &mut self,
        session: &mut ActualActivatedSession,
        _catalog: &Catalog<'_>,
        view: &Option<battle_view_startup::State>,
        bridge: &battle_ui_bridge::Client,
        execution: &mut Execution<'_>,
    ) -> Result<()> {
        if self.application.current() != 6 {
            return Ok(());
        }
        //Before real5C3000 there is no world-bound prefix loan. No false AD
        //is inferred here; the actual preWorld owner services EndSession.
        let Some(view) = view else { return Ok(()) };
        let mut diagnostics = Vec::new();
        let mut effects = LoadingEffects {
            time: &mut self.status_time,
            diagnostics: &mut self.diagnostics,
        };
        let transition=session
            .tick_current_loading_before_state_transition(
                view.native_view().native().prefix(),
                &mut diagnostics,
                &mut effects,
            )
            .map_err(|e| e.to_string())?;
        //Native682FAD captures+C, then calls40CE10 immediately. Releasing the
        //Loading loan permits its actual40CAA0 OnLeave on the SAME owner.
        if let Some(state)=transition {
            self.set_state(session,state,None,bridge,execution)?;
        }
        self.diagnostics
            .extend(diagnostics.into_iter().map(|d| format!("{d:?}")));
        Ok(())
    }
}
struct StatusSink<'a> {
    time: &'a mut PerformanceTimer,
    diagnostics: &'a mut Vec<String>,
}
struct ReadyPrompt<'a,'e>{session:&'a mut ActualActivatedSession,menu:&'a Handle,bridge:&'a battle_ui_bridge::Client,execution:&'a mut Execution<'e>,time:&'a mut PerformanceTimer,diagnostics:&'a mut Vec<String>}
impl callbacks::ReadyServices for ReadyPrompt<'_, '_>{
 fn button_visible(&mut self)->Result<bool>{let menu=self.menu.clone();self.bridge.call(self.execution,move|c|c.menus.ready_button_visible(c.ui,&menu))}
 fn resource_complete_17(&mut self)->Result<bool>{Ok(self.session.services().loading().resource_complete_17())}
 fn user_ready_16(&mut self)->Result<bool>{Ok(self.session.services().loading().user_ready_16())}
 fn finished_status_773ce0(&mut self)->Result<()>{let(host,world,_)=self.session.parts();host.update_status_773ce0(&world.evidence().live_text,&"app_status_finished".encode_utf16().collect::<Vec<_>>(),0,1,&mut StatusSink{time:self.time,diagnostics:self.diagnostics},Precision::Double53).map_err(|e|e.to_string())}
 fn show_ready_button(&mut self)->Result<()>{let menu=self.menu.clone();self.bridge.call(self.execution,move|c|c.menus.show_ready_button(c.ui,&menu))}
}
struct CurrentAudio<'a,'e>{bridge:&'a battle_ui_bridge::Client,execution:&'a mut Execution<'e>}
impl local_bootstrap::live_frame::Frontend for CurrentAudio<'_,'_>{
    fn dev_mode_current(&mut self)->Result<u8>{self.bridge.call(self.execution,|c|c.ui.uvars.dev_mode_current())}
    fn swap_right_click_with_alt_right_click_current(&mut self)->Result<bool>{
        self.bridge.call(self.execution,|c|c.ui.uvars.swap_right_click_with_alt_right_click_current())
    }
    fn hotkey_alt_right_click_command_current(&mut self)->Result<Vec<u16>>{
        self.bridge.call(self.execution,|c|c.ui.uvars.hotkey_alt_right_click_command_current())
    }
}
impl aa_runtime::live_first_tick::AudioOutput for CurrentAudio<'_, '_>{
    fn play_747630(&mut self,request:&aa_runtime::live_first_tick::MusicRequest)->Result<aa_runtime::live_first_tick::Playback>{
        let ids=request.audio.id().to_owned();
        self.bridge.call(self.execution,move|c|Ok(if c.audio.play_music(&ids){aa_runtime::live_first_tick::Playback::Accepted}else{aa_runtime::live_first_tick::Playback::Declined}))
    }
}
impl local_bootstrap::session_status::Services for StatusSink<'_> {
    fn milliseconds_59aba0(&mut self) -> aa_netcode::Result<u32> {
        PerformanceTimer::milliseconds(self.time)
    }
    fn progress_diagnostic(&mut self, seconds: u32, text: &[u16]) -> aa_netcode::Result<()> {
        let line = format!(
            "Session Progress at {}s: {}",
            f32::from_bits(seconds),
            String::from_utf16_lossy(text)
        );
        log::info!("{line}");
        self.diagnostics.push(line);
        Ok(())
    }
}
struct LoadingEffects<'a> {
    time: &'a mut PerformanceTimer,
    diagnostics: &'a mut Vec<String>,
}
impl local_bootstrap::world_view::StatusEffects for LoadingEffects<'_> {
    fn waiting_on_other_players_in_world(
        &mut self,
        owner: &mut local_bootstrap::LocalBootstrap,
        text: &aa_runtime::localization::Localization,
    ) -> aa_netcode::Result<()> {
        owner.update_status_773ce0(
            text,
            &"app_status_waiting_on_other_players"
                .encode_utf16()
                .collect::<Vec<_>>(),
            0,
            1,
            &mut StatusSink {
                time: self.time,
                diagnostics: self.diagnostics,
            },
            Precision::Double53,
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Node {
    Main,
    Loading,
    Game,
}
///Virtual slots of the constructed AppState nodes in Application+CC (40CB40):
///+14 OnEnter and +18 OnLeave (40CE10), +28 menu finished (784220). Proven by
///research/development10/game-state-entry/native-game-state.json.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct NodeSlots {
    enter_14: u32,
    leave_18: u32,
    event_1c: u32,
    finished_28: u32,
}
impl Node {
    fn slots(self) -> NodeSlots {
        match self {
            //6E7FF0, vtable A7ED98.
            Node::Main => NodeSlots { enter_14: 0x40ca80, leave_18: 0x4b2830, event_1c: 0x414d60, finished_28: 0x943800 },
            //682DE0, vtable A6E6A4.
            Node::Loading => NodeSlots { enter_14: 0x682e70, leave_18: 0x40caa0, event_1c: 0x682fd0, finished_28: 0x682db0 },
            //608C60, vtable A56C00: 608C80 is a jmp to 40CA80, 5E8900 a jmp to 4B2830.
            Node::Game => NodeSlots { enter_14: 0x608c80, leave_18: 0x5e8900, event_1c: 0x414620, finished_28: 0x943800 },
        }
    }
}
#[derive(Debug, PartialEq, Eq)]
enum LeaveEffect {
    ///40CAA0: mov byte [ecx+8],0.
    ClearChanged8,
    ///4B2830 (or a jmp to it): bare RET.
    Ret,
}
fn leave_effect(slots: NodeSlots) -> Result<LeaveEffect> {
    match slots.leave_18 {
        0x40caa0 => Ok(LeaveEffect::ClearChanged8),
        0x4b2830 | 0x5e8900 => Ok(LeaveEffect::Ret),
        other => Err(format!("AppState OnLeave {other:#x} is not ported")),
    }
}
#[derive(Debug, PartialEq, Eq)]
enum EnterEffect {
    ///40CA80: explicit Arg8 else this+4, then 5C0440(1).
    Base40ca80,
    ///682E70: Loading kind selection, then 40CA80.
    Loading682e70,
}
fn enter_effect(slots: NodeSlots) -> Result<EnterEffect> {
    match slots.enter_14 {
        0x40ca80 | 0x608c80 => Ok(EnterEffect::Base40ca80),
        0x682e70 => Ok(EnterEffect::Loading682e70),
        other => Err(format!("AppState OnEnter {other:#x} is not ported")),
    }
}
struct Callbacks<'a, 'e> {
    main_4: &'a Option<Handle>,
    game_4:&'a Option<Handle>,
    loading_4: &'a mut Option<Handle>,
    selectors: &'a Selectors,
    loading: &'a mut application_loading::State,
    loading_frame: Option<crate::battle_menus::game_hud::LoadingFrame>,
    bridge: &'a battle_ui_bridge::Client,
    execution: &'a mut Execution<'e>,
    diagnostics: &'a mut Vec<String>,
}
impl application_state::Services for Callbacks<'_, '_> {
    type Node = Node;
    type Arg8 = Option<Handle>;
    type ArgC = ();
    fn diagnostic(&mut self, _: &mut application_state::State, from: u32, to: u32) -> Result<()> {
        let line = format!("Application SetState {from}->{to}");
        log::info!("{line}");
        self.diagnostics.push(line);
        Ok(())
    }
    fn state_node(&mut self, index: u32) -> Result<Option<Node>> {
        match index {
            2 => Ok(Some(Node::Main)),
            4=>Ok(Some(Node::Game)),
            6 => Ok(Some(Node::Loading)),
            other => Err(format!("actual AppState node{other} not constructed")),
        }
    }
    fn leave(&mut self, _: &mut application_state::State, node: Node) -> Result<()> {
        match leave_effect(node.slots())? {
            LeaveEffect::ClearChanged8 => self.loading.leave_40caa0(),
            LeaveEffect::Ret => {}
        }
        Ok(())
    }
    fn enter(
        &mut self,
        _: &mut application_state::State,
        node: Node,
        arg8: &Option<Handle>,
        _: &(),
    ) -> Result<()> {
        let mut menu = MenuEnter {
            current: match node {
                Node::Main => self.main_4,
                Node::Game=>self.game_4,
                Node::Loading => self.loading_4,
            },
            bridge: self.bridge,
            execution: self.execution,
        };
        match enter_effect(node.slots())? {
            EnterEffect::Base40ca80 => callbacks::enter(&mut menu, arg8.clone()),
            EnterEffect::Loading682e70 => {
                drop(menu);
                let mut loading = LoadingEnter {
                    current: self.loading_4,
                    kind: self.loading,
                    selectors: self.selectors,
                    frame: self.loading_frame.clone(),
                    bridge: self.bridge,
                    execution: self.execution,
                };
                callbacks::enter_loading(&mut loading)
            }
        }
    }
}
struct MenuEnter<'a, 'e> {
    current: &'a Option<Handle>,
    bridge: &'a battle_ui_bridge::Client,
    execution: &'a mut Execution<'e>,
}
impl callbacks::Services for MenuEnter<'_, '_> {
    type Menu = Handle;
    fn current_menu(&mut self) -> Result<Option<Handle>> {
        Ok(self.current.clone())
    }
    fn show_5c0440(&mut self, menu: Handle, _: bool) -> Result<()> {
        self.bridge
            .call(self.execution, move |c| c.menus.show_connected(c.ui, &menu, None))
    }
}
struct LoadingEnter<'a, 'e> {
    current: &'a mut Option<Handle>,
    kind: &'a application_loading::State,
    selectors: &'a Selectors,
    frame: Option<crate::battle_menus::game_hud::LoadingFrame>,
    bridge: &'a battle_ui_bridge::Client,
    execution: &'a mut Execution<'e>,
}
impl callbacks::Services for LoadingEnter<'_, '_> {
    type Menu = Handle;
    fn current_menu(&mut self) -> Result<Option<Handle>> {
        Ok(self.current.clone())
    }
    fn show_5c0440(&mut self, menu: Handle, _: bool) -> Result<()> {
        let frame = self.frame.clone();
        self.bridge
            .call(self.execution, move |c| c.menus.show_connected(c.ui, &menu, frame.as_ref()))
    }
}
impl callbacks::LoadingServices for LoadingEnter<'_, '_> {
    fn kind_10(&mut self) -> Result<u32> {
        Ok(self.kind.kind())
    }
    fn splash_b39748(&mut self) -> Result<Option<Handle>> {
        Ok(Some(self.selectors.splash.clone()))
    }
    fn menu_table_2_40d0f0(&mut self) -> Result<Option<Handle>> {
        Ok(Some(self.selectors.loading.clone()))
    }
    fn store_menu_4(&mut self, menu: Option<Handle>) -> Result<()> {
        *self.current = menu;
        Ok(())
    }
}

#[cfg(test)]
mod game_state_tests {
    use super::*;
    use serde_json::{Value, json};
    fn fixture() -> Value {
        serde_json::from_str(include_str!(
            "../../../research/development10/game-state-entry/native-game-state.json"
        ))
        .unwrap()
    }
    fn hex(v: &Value) -> u32 {
        u32::from_str_radix(v.as_str().unwrap().trim_start_matches("0x"), 16).unwrap()
    }
    fn head(f: &Value, table: &str, slot: &str) -> String {
        f["tables"][table][slot]["head"].as_str().unwrap().to_owned()
    }
    #[test]
    fn node_slots_are_the_native_vtables() {
        let f = fixture();
        for (name, node) in [("main_node", Node::Main), ("loading_node", Node::Loading), ("game_node", Node::Game)] {
            let t = &f["tables"][name];
            let slots = node.slots();
            assert_eq!(slots.enter_14, hex(&t["enter_14"]["target"]), "{name}");
            assert_eq!(slots.leave_18, hex(&t["leave_18"]["target"]), "{name}");
            assert_eq!(slots.finished_28, hex(&t["finished_28"]["target"]), "{name}");
            assert_eq!(slots.event_1c, hex(&t["event_1c"]["target"]), "{name}");
        }
        //414D60: e9 bbf8ffff = jmp 414D65-0x745 = 414620.
        assert_eq!(head(&f, "main_node", "event_1c"), "e9bbf8ffff");
        assert_eq!(0x414d65u32.wrapping_add(0xfffff8bb), 0x414620);
        //The jmp/ret heads behind the RET/40CA80 classifications.
        assert_eq!(head(&f, "game_node", "enter_14"), "e9fb3de0ff"); //jmp 40CA80
        assert_eq!(head(&f, "game_node", "leave_18"), "e92b9fecff"); //jmp 4B2830
        assert!(head(&f, "main_node", "leave_18").starts_with("c3"));
        assert!(head(&f, "game_node", "finished_28").starts_with("c20400"));
        assert!(head(&f, "loading_node", "leave_18").starts_with("c6410800c3"));
        //Game interface +E8 (initialized callback after 5C3000) is a bare RET.
        assert_eq!(hex(&f["tables"]["game_interface"]["initialized_e8"]["target"]), 0x4b2830);
        assert!(head(&f, "game_interface", "initialized_e8").starts_with("c3"));
        //KKC_Application::Init runs [App+BC]->vt+C8, the Game interface Init 6182B0.
        assert_eq!(f["tables"]["static_pins"]["0x41aa7a"], "8b8ebc0000008b01ff90c8000000");
        assert_eq!(hex(&f["tables"]["game_interface"]["init_c8"]["target"]), 0x6182b0);
        assert_eq!(leave_effect(Node::Loading.slots()).unwrap(), LeaveEffect::ClearChanged8);
        assert_eq!(leave_effect(Node::Game.slots()).unwrap(), LeaveEffect::Ret);
        assert_eq!(leave_effect(Node::Main.slots()).unwrap(), LeaveEffect::Ret);
        assert_eq!(enter_effect(Node::Game.slots()).unwrap(), EnterEffect::Base40ca80);
        assert_eq!(enter_effect(Node::Main.slots()).unwrap(), EnterEffect::Base40ca80);
        assert_eq!(enter_effect(Node::Loading.slots()).unwrap(), EnterEffect::Loading682e70);
    }
    ///Production node table and 40CA80 selection over the real 40CE10 port,
    ///replaying every native 682EE0 -> 40CE10 -> OnLeave/OnEnter row.
    struct Replay {
        changed_8: u8,
        game_interface: Option<&'static str>,
        trace: Vec<Value>,
    }
    struct Menu<'a>(&'a mut Vec<Value>, Option<&'static str>);
    impl callbacks::Services for Menu<'_> {
        type Menu = &'static str;
        fn current_menu(&mut self) -> Result<Option<&'static str>> {
            Ok(self.1)
        }
        fn show_5c0440(&mut self, menu: &'static str, animate: bool) -> Result<()> {
            self.0.push(json!(["show_5c0440", menu, u8::from(animate)]));
            Ok(())
        }
    }
    impl application_state::Services for Replay {
        type Node = Node;
        type Arg8 = Option<&'static str>;
        type ArgC = ();
        fn diagnostic(&mut self, _: &mut application_state::State, from: u32, to: u32) -> Result<()> {
            self.trace.push(json!(["set_state_diagnostic", from, to]));
            Ok(())
        }
        fn state_node(&mut self, index: u32) -> Result<Option<Node>> {
            Ok(match index {
                2 => Some(Node::Main),
                4 => Some(Node::Game),
                6 => Some(Node::Loading),
                _ => None,
            })
        }
        fn leave(&mut self, _: &mut application_state::State, node: Node) -> Result<()> {
            if leave_effect(node.slots())? == LeaveEffect::ClearChanged8 {
                self.changed_8 = 0;
            }
            Ok(())
        }
        fn enter(&mut self, _: &mut application_state::State, node: Node, arg8: &Option<&'static str>, _: &()) -> Result<()> {
            assert_eq!(enter_effect(node.slots())?, EnterEffect::Base40ca80);
            let current = self.game_interface;
            callbacks::enter(&mut Menu(&mut self.trace, current), *arg8)
        }
    }
    struct Prime;
    impl application_state::Services for Prime {
        type Node = Node;
        type Arg8 = Option<&'static str>;
        type ArgC = ();
        fn diagnostic(&mut self, _: &mut application_state::State, _: u32, _: u32) -> Result<()> {
            Ok(())
        }
        fn state_node(&mut self, index: u32) -> Result<Option<Node>> {
            Ok(match index {
                2 => Some(Node::Main),
                6 => Some(Node::Loading),
                _ => None,
            })
        }
        fn leave(&mut self, _: &mut application_state::State, _: Node) -> Result<()> {
            Ok(())
        }
        fn enter(&mut self, _: &mut application_state::State, _: Node, _: &Option<&'static str>, _: &()) -> Result<()> {
            Ok(())
        }
    }
    #[test]
    fn native_loading_to_game_set_state_rows() {
        let f = fixture();
        let mut checked = 0;
        for row in f["rows"].as_array().unwrap() {
            let input = &row["input"];
            let out = &row["output"];
            if input["op"] == "user_ready" {
                //6E80F0 stores App+E4+16 in every current AppState.
                let mut loading = application_loading::State::from_native_constructor();
                loading.user_ready_command_6e80f0();
                assert_eq!(json!(loading.flags_14_17().to_vec()), out["loading"]["flags_14_17"], "{input}");
                assert_eq!(out["current_10c"], input["current"]);
                checked += 1;
                continue;
            }
            let flag14 = input["flags"][0].as_u64().unwrap() != 0;
            //682EE0 returns +C only when +14 is set (proven by the skirmish
            //native-loading rows); the trace prefix is its own children.
            let prefix: Vec<Value> = out["trace"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| !matches!(v.get(0).and_then(Value::as_str), Some("set_state_diagnostic" | "show_5c0440")))
                .cloned()
                .collect();
            let mut replay = Replay {
                changed_8: 1,
                game_interface: (input["game_interface"] == 1).then_some("game_interface"),
                trace: prefix,
            };
            //AppState 0 -> 2 -> 6 as in the real session start.
            let mut state = application_state::State::from_constructor();
            state.set_state(2, &None, &(), &mut Prime).unwrap();
            state.set_state(6, &None, &(), &mut Prime).unwrap();
            if flag14 {
                let requested = input["return_state"].as_u64().unwrap() as u32;
                state.set_state(requested, &None, &(), &mut replay).unwrap();
            }
            assert_eq!(json!(replay.trace), out["trace"], "{input}");
            assert_eq!(json!(state.current()), out["current_10c"], "{input}");
            assert_eq!(json!(state.previous()), out["previous_110"], "{input}");
            assert_eq!(json!(replay.changed_8), out["loading"]["changed_8"], "{input}");
            checked += 1;
        }
        assert_eq!(checked, 264 * 3);
    }
}
