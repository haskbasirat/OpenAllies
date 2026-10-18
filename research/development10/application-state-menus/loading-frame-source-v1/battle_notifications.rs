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
    pub fn user_ready_command(&mut self,session:&mut ActualActivatedSession)->Result<()>{
        if self.application.current()!=6{return Err("UserReady requires actual current Loading AppState6".into())}
        session.parts().2.loading_mut().user_ready_command_6e80f0();Ok(())
    }
    ///One real native frame after this job's FIFO/Loading dispatch. The
    ///Admin304/315/775BF0 protocol owns unfreeze; an AppState/menu never does.
    pub fn run_frame<T:MillisecondTimer>(
        &mut self,session:&mut ActualActivatedSession,catalog:&Catalog<'_>,
        notifications:&Notifications,time:&mut NodeTime<T>,
        focus:&aa_runtime::application_window_state::Reader,
        bridge:&battle_ui_bridge::Client,execution:&mut Execution<'_>,
    )->Result<()> {
        if let Some(error)=&self.stopped{return Err(error.clone())}
        let identity=execution.identity();
        let os=std::cell::RefCell::new(crate::windows_reset_os::WindowsThreadIds);
        let mut audio=CurrentAudio{bridge,execution};
        match session.run_native_frame(catalog,time,focus,notifications,&identity,&os,&mut audio,&mut self.frame_diagnostics,Precision::Double53){
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
        let mut services = Callbacks {
            main_4: &self.main_4,
            game_4:&self.game_4,
            loading_4: &mut self.loading_4,
            selectors,
            loading: session.parts().2.loading_mut(),
            bridge,
            execution,
            diagnostics: &mut self.diagnostics,
        };
        self.application.set_state(state, &arg8, &(), &mut services)
    }
    fn bind_menus(
        &mut self,
        session: &mut ActualActivatedSession,
        bridge: &battle_ui_bridge::Client,
        execution: &mut Execution<'_>,
    ) -> Result<()> {
        if self.selectors.is_none() {
            let selectors = bridge.call(execution, |context| context.menus.bind(context.ui))?;
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
            self.bind_menus(session, bridge, execution)?;
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
        //682FD0 derived prefix precedes the base414620 switch.
        if self.application.current() == 6 {
            let (host, world, app) = session.parts();
            let mut sink = StatusSink {
                time: &mut self.status_time,
                diagnostics: &mut self.diagnostics,
            };
            app.loading_mut()
                .notification_prefix_682fd0(event.kind, || {
                    host.update_status_773ce0(
                        &world.evidence().live_text,
                        &"app_status_loading_world"
                            .encode_utf16()
                            .collect::<Vec<_>>(),
                        0,
                        1,
                        &mut sink,
                        Precision::Double53,
                    )
                    .map_err(|e| e.to_string())
                })?;
        } else if !matches!(self.application.current(),2|4) {
            return Err("414620 actual current AppState handler is not implemented".into());
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
                match self.application.current() {
                    6 => session
                        .parts()
                        .2
                        .loading_mut()
                        .menu_finished_682db0(u8::from(event.flag_34)),
                    2 => {}
                    other => return Err(format!("784220 current state{other} vf28 is unbound")),
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
#[derive(Clone, Copy)]
enum Node {
    Main,
    Loading,
    Game,
}
struct Callbacks<'a, 'e> {
    main_4: &'a Option<Handle>,
    game_4:&'a Option<Handle>,
    loading_4: &'a mut Option<Handle>,
    selectors: &'a Selectors,
    loading: &'a mut application_loading::State,
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
        if matches!(node, Node::Loading) {
            self.loading.leave_40caa0()
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
        match node {
            Node::Main|Node::Game => callbacks::enter(&mut menu, arg8.clone()),
            Node::Loading => {
                drop(menu);
                let mut loading = LoadingEnter {
                    current: self.loading_4,
                    kind: self.loading,
                    selectors: self.selectors,
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
            .call(self.execution, move |c| c.menus.show(c.ui, &menu))
    }
}
struct LoadingEnter<'a, 'e> {
    current: &'a mut Option<Handle>,
    kind: &'a application_loading::State,
    selectors: &'a Selectors,
    bridge: &'a battle_ui_bridge::Client,
    execution: &'a mut Execution<'e>,
}
impl callbacks::Services for LoadingEnter<'_, '_> {
    type Menu = Handle;
    fn current_menu(&mut self) -> Result<Option<Handle>> {
        Ok(self.current.clone())
    }
    fn show_5c0440(&mut self, menu: Handle, _: bool) -> Result<()> {
        self.bridge
            .call(self.execution, move |c| c.menus.show(c.ui, &menu))
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
