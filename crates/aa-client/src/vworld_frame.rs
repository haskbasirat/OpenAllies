//! Client binding of the VWorld per-frame update (aa_runtime::vworld_update):
//! 5C92F0's 43AB30 then the widget-tree 43FF60 -> 43EC60, once per UI frame
//! while VWorld+AD is set, over the SAME activated Session, World, VWorld
//! allocation and view directory. Each child is bound to its actual owner or
//! is an explicit refusal naming the missing owner; no child is a silent
//! success. Children owned by other modules are called, never reimplemented.
use crate::live_scene::{view_directory::ViewHandle, world_owner::{ApplicationViews, VWorldOwner}};
use aa_netcode::local_bootstrap::LocalBootstrap;
use aa_numeric::{Binary, Precision};
use aa_runtime::{
    catalog::Catalog, live_world_frame::World,
    vworld_update::{self, Fields, Refusals, Services, SimTime, WorldValid},
    world_ready_services::Application,
};

type Result<T> = std::result::Result<T, String>;

/// The loans of one UI frame. `focus` (B396C4) and `views` (the current
/// camera and the B36518 TerrainMap owner) are lent by the frame owner; a
/// child that needs an owner that was not lent refuses explicitly.
pub(crate) struct Loans<'a, 'source, 'execution> {
    pub host: &'a LocalBootstrap,
    pub world: &'a mut World<'static>,
    pub application: &'a mut Application,
    pub catalog: &'a Catalog<'source>,
    pub view: &'a mut VWorldOwner<aa_runtime::view_assets::State>,
    pub bridge: &'a crate::battle_ui_bridge::Client,
    pub execution: &'a mut aa_runtime::simulation_execution::Execution<'execution>,
    pub focus: Option<&'a aa_runtime::application_window_state::Reader>,
    pub views: Option<&'a mut ApplicationViews>,
    /// Session+F4, read from the same Session before its parts are lent.
    pub session_f4: bool,
    pub pc: Precision,
}

/// AA_STARTUP_CENSUS + AA_STARTUP_CENSUS_CONTINUE: diagnostic look-ahead.
fn look_ahead() -> bool {
    std::env::var_os("AA_STARTUP_CENSUS").is_some() && std::env::var_os("AA_STARTUP_CENSUS_CONTINUE").is_some()
}

/// One UI frame: 43AB30 (5C92F0, AD already checked by the caller), then
/// 43FF60. The product returns the first refusal; the look-ahead returns
/// every refusal of the frame joined in update order.
pub(crate) fn run(loans: Loans<'_, '_, '_>) -> Result<()> {
    let pc = loans.pc;
    let mut refusals = if look_ahead() { Refusals::look_ahead() } else { Refusals::strict() };
    let mut frame = Frame { loans, look_ahead: refusals.is_look_ahead() };
    let time = frame.view_time_43ab30();
    refusals.step("5C92F0 43AB30 VWorld view time", time)?;
    vworld_update::update_43ff60(&mut frame, &mut refusals, pc)?;
    if refusals.list().is_empty() {
        return Ok(());
    }
    let joined: Vec<String> = refusals.list().iter().map(|(step, error)| format!("{step}: {error}")).collect();
    Err(format!("look-ahead (diagnostic, not native state): {}", joined.join(" | ")))
}

struct Frame<'a, 'source, 'execution> {
    loans: Loans<'a, 'source, 'execution>,
    look_ahead: bool,
}
impl Frame<'_, '_, '_> {
    fn view_time_43ab30(&mut self) -> Result<()> {
        let pc = self.loans.pc;
        let world: &World<'static> = self.loans.world;
        let revision = world.presentation_parts()?.revision;
        let before = self.loans.view.native().prefix().view_time_words_for(revision)?;
        let mut sim = Sim { host: self.loans.host, world, application: &*self.loans.application,
            focus: self.loans.focus, session_f4: self.loans.session_f4 };
        self.loans.view.parts(world).map_err(|e| format!("{e:#}"))?.native.update_view_time_43ab30(revision, &mut sim, pc)?;
        let after = self.loans.view.native().prefix().view_time_words_for(revision)?;
        if after[2] != before[2] {
            log::info!("native 43AB30 VWorld+8C {:#010x} -> {:#010x} (+84 {:#010x} +88 {:#010x} +90 {:#010x})",
                before[2], after[2], after[0], after[1], after[3]);
        }
        Ok(())
    }
    fn revision_value(&self) -> Result<u32> {
        Ok(self.loans.world.presentation_parts()?.revision.value())
    }
}

/// 76B120's reads on the SAME Session, Sim, World and frame UVar cells.
struct Sim<'a> {
    host: &'a LocalBootstrap,
    world: &'a World<'static>,
    application: &'a Application,
    focus: Option<&'a aa_runtime::application_window_state::Reader>,
    session_f4: bool,
}
impl SimTime for Sim<'_> {
    fn session_f4(&mut self) -> Result<bool> {
        Ok(self.session_f4)
    }
    fn flags_b396c4(&mut self) -> Result<u32> {
        self.focus.ok_or("76B120 B396C4 application window word: the frame owner did not lend its reader to the VWorld update")?
            .flags_b396c4()
    }
    fn single_player_auto_pause(&mut self) -> Result<bool> {
        Ok(self.application.frame_variables().auto_pause_byte() != 0)
    }
    fn paused_28(&mut self) -> Result<u8> {
        Ok(self.host.sim().paused_byte())
    }
    fn frozen_29(&mut self) -> Result<u8> {
        Ok(self.host.sim().frozen_byte())
    }
    fn render_dump_avi(&mut self) -> Result<bool> {
        Ok(self.application.frame_variables().render_dump_avi_byte() != 0)
    }
    fn avi_time_76b080(&mut self) -> Result<Binary> {
        Err("76B080 RenderDumpAVI frame time is not ported".into())
    }
    fn time_words(&mut self) -> Result<[u32; 4]> {
        let t = self.host.sim().time_words();
        let absent = "76B120 Sim+10..+1C: the 76DCE0 producer has not run";
        Ok([t.begin_game.ok_or(absent)?.0, t.begin_admin.ok_or(absent)?.0, t.end_game.ok_or(absent)?.0, t.end_admin.ok_or(absent)?.0])
    }
    fn admin_clock_74b180(&mut self, _: u32) -> Result<Binary> {
        Err("76B120 74B180 AdminNode real-clock estimate is not lent to the UI-frame VWorld update".into())
    }
    fn current_34(&mut self) -> Result<u32> {
        Ok(self.host.sim().constructor_field_34())
    }
    fn store_current_34(&mut self, _: u32) -> Result<()> {
        Err("76B120 Sim+34 store: live_sim::Sim has no render-time writer".into())
    }
    fn world_e0(&mut self) -> Result<u32> {
        Ok(self.world.clock().begin.0)
    }
    fn world_e4(&mut self) -> Result<u32> {
        Ok(self.world.clock().end.0)
    }
}

/// The 50C600 children for each World+130 entry. 427950 has no script
/// processor; kinds 0/2/3/4 have no live Rust producer yet and their children
/// no client owner. The product refuses each one; the look-ahead records it
/// and continues as if the child had no effect (diagnostic, not native).
struct Messages {
    look_ahead: bool,
    texts: Vec<String>,
}
impl Messages {
    fn refuse<T>(&mut self, child: String, neutral: T) -> Result<T> {
        if !self.look_ahead {
            return Err(child);
        }
        log::warn!("native 50C750 look-ahead: {child} (recorded, not native state)");
        Ok(neutral)
    }
}
impl aa_runtime::fx_manager::BroadcastDelivery for Messages {
    type Thing = ();
    type Vfx = ();
    fn thing_432e40(&mut self, handle: u32) -> Result<Option<()>> {
        self.refuse(format!("432E40 [B364C0] view lookup of handle {handle:#010x} has no client owner"), None)
    }
    fn thing_record_43b270(&mut self, view: u64, _: Option<()>, _: &[u32; 7]) -> Result<()> {
        self.refuse(format!("43B270 damage number on attached view {view} has no client owner"), ())
    }
    fn script_eval_427950(&mut self, text: &[u16], flag: u8) -> Result<()> {
        let text = String::from_utf16_lossy(text);
        let child = format!("427950 Eval(`{text}`, {flag}) on [B36500]: TGC_ScriptProcessor 426C20 has no Rust owner");
        self.refuse(child, ())?;
        self.texts.push(text);
        Ok(())
    }
    fn mission_result_440190(&mut self) -> Result<()> {
        self.refuse("440190 VWorld mission result messages (HUD) have no client owner".into(), ())
    }
    fn team_victory_4408a0(&mut self) -> Result<()> {
        self.refuse("4408A0 VWorld team victory message (HUD) has no client owner".into(), ())
    }
    fn attached_vfx_43cd00(&mut self, definition: u64, _: ()) -> Result<Option<()>> {
        self.refuse(format!("43CD00 attached VFX (definition {definition:#x}) is the FX package's, not bound"), None)
    }
    fn detached_vfx_43cc50(&mut self, definition: u64, _: [u32; 3], _: [u32; 3]) -> Result<Option<()>> {
        self.refuse(format!("43CC50 detached VFX (definition {definition:#x}) is the FX package's, not bound"), None)
    }
    fn vfx_time_50(&mut self, _: (), _: u32) -> Result<()> {
        self.refuse("VFX view +50 store has no client owner".into(), ())
    }
}

impl Services for Frame<'_, '_, '_> {
    type View = ViewHandle;
    fn fields(&mut self) -> Result<&mut Fields<ViewHandle>> {
        let world: &World<'static> = self.loans.world;
        let revision = world.presentation_parts()?.revision;
        self.loans.view.parts(world).map_err(|e| format!("{e:#}"))?.native.update_fields_mut_for(revision)
    }
    fn widget_base_update_5eec70(&mut self) -> Result<()> {
        Err("5EEC70 on the VWorld widget (UI+D0: +64/+68 fade, DummyVE 5E25C0 state bits, +28 children) has no aa-ui entry point".into())
    }
    fn vworld_ad(&mut self) -> Result<bool> {
        let world: &World<'static> = self.loans.world;
        self.loans.view.native().prefix().initialized_for(world.presentation_parts()?.revision)
    }
    fn construct_minimap_4341a0(&mut self) -> Result<()> {
        Err("4341A0 +140 KKC minimap owner (712D60 KKC_RMinimap, four 7130E0 layers) is not ported".into())
    }
    fn look_at_timer_construct_59aa70(&mut self) -> Result<()> {
        Err("43FF60 B39F68 RandomLookAt timer (59AA70) has no owner".into())
    }
    fn look_at_timer_restart_59ab60(&mut self) -> Result<()> {
        Err("43FF60 B39F68 RandomLookAt timer (59AB60) has no owner".into())
    }
    fn look_at_timer_exit_96cab0(&mut self) -> Result<()> {
        Err("43FF60 B39F68 RandomLookAt timer exit registration (96CAB0) has no owner".into())
    }
    fn look_at_timer_milliseconds_59aba0(&mut self) -> Result<u32> {
        Err("43FF60 B39F68 RandomLookAt timer (59ABA0) has no owner".into())
    }
    fn random_b39714_85d840(&mut self) -> Result<u32> {
        Err("43FF60 B39714 RandomLookAt generator (85D840) has no owner".into())
    }
    fn map_extent_30(&mut self) -> Result<[u32; 2]> {
        Err("43FF60 [GWorld+30]+C/+10 RandomLookAt extent is not bound".into())
    }
    fn camera_look_at_6ff130(&mut self, _: u32, _: u32) -> Result<()> {
        Err("43FF60 RandomLookAt 6FF130 on the current camera is not bound".into())
    }
    fn camera_update_701590(&mut self) -> Result<()> {
        Err("701590 current-camera dynamics has no native_camera entry point".into())
    }
    fn visual_map_b36518(&mut self) -> Result<bool> {
        let world: &World<'static> = self.loans.world;
        let revision = world.presentation_parts()?.revision;
        let views = self.loans.views.as_deref().ok_or("B36518 TerrainMap owner (ApplicationViews) is not lent to the VWorld update")?;
        Ok(views.visual_map().dimensions_for_world(revision, self.loans.view.native().heightfield())?.is_some())
    }
    fn uninitializing_ae(&mut self) -> Result<bool> {
        Ok(self.loans.view.native().prefix().uninitializing())
    }
    fn deferred_flags_54(&mut self, _: &ViewHandle) -> Result<u32> {
        Err("43DA00 deferred view +54: 43D190 has no Rust producer".into())
    }
    fn deferred_presenting_bc(&mut self, _: &ViewHandle) -> Result<bool> {
        Err("43DA00 deferred view vt+BC: 43D190 has no Rust producer".into())
    }
    fn deferred_count_19c(&mut self, _: &ViewHandle) -> Result<i32> {
        Err("43DA00 deferred view +19C: 43D190 has no Rust producer".into())
    }
    fn destroy_view_43cf00(&mut self, _: ViewHandle) -> Result<()> {
        Err("43CF00 VWorld view removal is not ported".into())
    }
    fn cursor_world_point_43c4b0(&mut self, _: &mut [u32; 3]) -> Result<[u32; 3]> {
        Err("43C4B0 cursor world point needs the current camera and the 71D840 TerrainMap B36518 query, which the client does not bind".into())
    }
    fn ui_world_state_43a1c0(&mut self) -> Result<()> {
        Err("43A1C0 UIWorldState per-frame update ([VWorld+13C]) has no entry point".into())
    }
    fn terrain_map_71fac0(&mut self) -> Result<()> {
        Err("71FAC0 TerrainMap B36518 update is not ported (the client binds no TerrainMap)".into())
    }
    fn minimap_4361b0(&mut self) -> Result<()> {
        Err("4361B0 +140 KKC minimap update is not ported".into())
    }
    fn world_valid(&mut self) -> Result<WorldValid> {
        let revision = self.revision_value()?;
        let world: &World<'static> = self.loans.world;
        let prefix = self.loans.view.native().prefix();
        let initialized_ad = prefix.initialized_for(world.presentation_parts()?.revision)?;
        //B3CC94 is the process World revision counter; the one live World
        //is its latest construction.
        let revision_matches_b8 = prefix.init_word_b8() == revision;
        let world_d8 = self.loans.world.with_view_binding(|p| Ok(*p.world_phase))?;
        Ok(WorldValid { revision_matches_b8, initialized_ad, world_d8 })
    }
    fn assert_855d30(&mut self, expression: &str, file: &str, line: u32) -> Result<u8> {
        Err(format!("855D30 assertion `{expression}` ({file}:{line}) reached; its log/dialog result is not ported"))
    }
    fn world_messages_50c750(&mut self) -> Result<()> {
        let mut script = Messages { look_ahead: self.look_ahead, texts: Vec::new() };
        let delivered = self.loans.world.with_view_binding(|p| p.presentation.deliver_broadcasts_50c750(&mut script))?;
        if delivered > 0 {
            log::info!("native 50C750: World+130 count {delivered} -> 0; 427950 received {:?}", script.texts);
        }
        Ok(())
    }
    fn scene_light_43a400(&mut self) -> Result<()> {
        Err("43A400 VWorld+108 scene light (World+54..+B8 sun transition, 43A390, 7178E0, 73A430, 73C240) is not ported".into())
    }
    fn heightfield_rects_4318e0(&mut self) -> Result<()> {
        let pc = self.loans.pc;
        let world: &World<'static> = self.loans.world;
        let resampled = self.loans.view.parts(world).map_err(|e| format!("{e:#}"))?.native.update_heightfield_rects_4318e0(world, pc)?;
        if resampled > 0 {
            log::info!("native 4318E0 resampled {resampled} VWorld+BC rects");
        }
        Ok(())
    }
    fn terrain_visibility_4312f0(&mut self) -> Result<()> {
        Err("4312F0 needs the VWorld+94 TerrainVisibility and the B36518 TerrainMap, which the client does not bind (terrain_visibility::update_4312f0 with the view-cell hooks)".into())
    }
    fn terrain_map_725340(&mut self) -> Result<()> {
        Err("725340 needs the B36518 TerrainMap, which the client does not bind (TerrainMap::update_725340 and the fog part)".into())
    }
    fn view_requests_208(&mut self) -> Result<()> {
        let pc = self.loans.pc;
        let guards = self.loans.application.depot_mut().resource_task_guards();
        let mut backend = crate::battle_resource_tasks::Current::new(self.loans.bridge, &mut *self.loans.execution, guards, pc)
            .with_session(self.loans.host);
        self.loans.view.drain_ready_requests_43ec60(self.loans.world, self.loans.catalog, self.loans.application.depot_mut(),
            &mut aa_runtime::resource_depot::release::ManagerRelease(&mut backend)).map_err(|e| format!("{e:#}"))
    }
    fn fx_command_count_e8(&mut self) -> Result<i32> {
        Ok(self.loans.view.native().fx_commands().native_count() as i32)
    }
    fn fx_commands_e0(&mut self, count: i32) -> Result<()> {
        Err(format!("43ED5D FX command loop over {count} VWorld+E0 commands (4289A0, 429050, 45C7A0) has no entry point"))
    }
}
