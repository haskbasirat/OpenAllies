//!77A950/77A750 shared native kernel over the actual retained Host owners.
//!Tasks enqueue before state changes; execution may begin after this method
//!returns, a valid native delayed-worker schedule. This is not battle ready.
use super::*;
use crate::staging::{launch_owner, LaunchStep, StateEvent};

pub trait Effects: finalize::Effects + load_task::Effects {
    ///The real76D6C0 traversal. It must borrow every actual constructor-owned
    ///launch subobject; disabled synchronization alone cannot invent NULLs.
    fn dump_launch_settings(&mut self, owner: &LocalBootstrap, label: &'static str) -> Result<()>;
    fn sync_log_gate(&mut self) -> Result<bool>;
    fn reset_order_version_diagnostic(&mut self) -> Result<()>;
}
struct World<'a, 'source, E: ?Sized> {
    owner: &'a mut LocalBootstrap,
    catalog: &'a aa_runtime::catalog::Catalog<'source>,
    bank: &'a aa_runtime::definitions::Bank,
    depot: &'a mut aa_runtime::resource_depot::Application,
    app: &'a aa_runtime::application_mode::GameAppState,
    manager: &'a aa_runtime::game_spy_manager_gate::State,
    text: Option<&'a Localization>,
    profiles: &'a mut dyn aa_runtime::load_sai_players::ProfileService,
    notifications: &'a Notifications,
    effects: &'a mut E,
}
impl<E: Effects> launch_owner::World for World<'_, '_, E> {
    fn launch_state(&self) -> Result<&LaunchState> {
        Ok(&self.owner.session.launch_state)
    }
    fn state(&self) -> Result<u32> {
        Ok(self.owner.session.state)
    }
    fn launch_game_type(&self) -> Result<u32> {
        Ok(self.owner.session.launch_game_type)
    }
    fn mark_save_last_settings(&mut self) -> Result<()> {
        self.owner.session.flags_168_16e[1] = Some(true);
        Ok(())
    }
    fn increment_launch_and_clear_countdown(&mut self) -> Result<()> {
        self.owner.session.field_164 = self.owner.session.field_164.wrapping_add(1);
        self.owner.session.launch_countdown = 0;
        Ok(())
    }
    fn launch_step(&mut self, step: LaunchStep) -> Result<()> {
        match step {
            LaunchStep::Step775DB0 => self.owner.check_finished_task_775db0(true),
            LaunchStep::Step770500 => self.owner.launch_step_770500(self.notifications),
            LaunchStep::Step776B20 => self.owner.finalize_host_launch_with_text_776b20(
                self.catalog,
                self.bank,
                self.depot,
                self.app,
                self.manager,
                self.text,
                self.profiles,
                self.effects,
            ),
        }
    }
    fn load(&mut self) -> Result<bool> {
        self.owner.create_host_load_task_775ee0(
            self.catalog,
            self.bank,
            self.depot,
            self.notifications,
            self.effects,
        )
    }
    fn clear_flag_16b(&mut self) -> Result<()> {
        self.owner.session.flags_168_16e[3] = Some(false);
        Ok(())
    }
    fn dump_launch_settings(&mut self, label: &'static str) -> Result<()> {
        self.effects.dump_launch_settings(self.owner, label)
    }
    fn timeline_freeze(&mut self, value: bool) -> Result<()> {
        if !value {
            return Err(Error::Unsupported(
                "77A950 reached actual unfreeze76DD90/time producers",
            ));
        }
        let sync = self.owner.sync_a.get_mut();
        self.owner.sim.store_frozen_after_sync(1, || -> Result<()> {
            let gate = if sync.enabled && sync.suspend < 1 {
                self.effects.sync_log_gate()?
            } else {
                false
            };
            sync.log_bool(gate, "_frozen", true);
            Ok(())
        })
    }
    fn reset_order_version(&mut self) -> Result<()> {
        self.effects.reset_order_version_diagnostic()?;
        for peer in &self.owner.peer_list {
            match peer {
                PeerLink::Node => {
                    self.owner
                        .host_node
                        .as_mut()
                        .ok_or_else(|| missing("74DBC0 actual Node absent"))?
                        .order_version = 0
                }
                PeerLink::Null => return Err(missing("74DBC0 actual peer is NULL")),
                PeerLink::Conn(_) => {
                    return Err(Error::Unsupported("74DBC0 actual remote peer6C owner"));
                }
            }
        }
        Ok(())
    }
    fn store_flag_16a(&mut self, value: bool) -> Result<()> {
        self.owner.session.flags_168_16e[2] = Some(value);
        Ok(())
    }
    fn leave_state(&mut self) -> Result<()> {
        Err(Error::Unsupported(
            "77A6E0 previous launch-settings value copies",
        ))
    }
    fn clear_launch_game_type(&mut self) -> Result<()> {
        self.owner.session.launch_game_type = 0;
        Ok(())
    }
    fn commit_state_prefix(&mut self, state: u32) -> Result<()> {
        self.owner.session.seed_override_100 = Some(0);
        self.owner.session.changed[0] = true;
        self.owner.session.changed[1] = true;
        self.owner.session.flags_168_16e[4] = Some(false);
        self.owner.session.state = state;
        Ok(())
    }
    fn store_phase(&mut self, phase: u32) -> Result<()> {
        self.owner.session.phase = phase;
        Ok(())
    }
    fn launch_text(&self) -> Result<&[u16]> {
        Ok(&self.owner.session.launch_text_84)
    }
    fn state_changed(&mut self, event: StateEvent) -> Result<()> {
        self.notifications.push(Notification {
            kind: 0,
            text: event.text,
            state: event.state,
            launch_state: event.launch_state,
            flag_34: event.flag,
            word_38: 0,
        })
    }
    fn clear_launch_text(&mut self) -> Result<()> {
        self.owner.session.launch_text_84.clear();
        Ok(())
    }
    fn restore_last_launch_settings(&mut self) -> Result<()> {
        Err(Error::Unsupported(
            "77A750 restore actual Session88 launch settings",
        ))
    }
}
impl LocalBootstrap {
    ///Concrete native76D6C0 traversal over this SAME launch owner and SyncA.
    ///Every source read remains active when logging is disabled; the external
    ///thread/application logger gate itself stays lazy at each actual logger.
    pub fn dump_actual_launch_settings(
        &self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        label: &str,
        gate: &mut dyn FnMut() -> std::result::Result<bool, String>,
    ) -> Result<()> {
        let mut binding = aa_runtime::live_clocked::SyncBinding::Shared {
            owner: &self.sync_a,
            gate,
        };
        let mut sink = aa_runtime::launch_settings_dump::SyncSink {
            binding: &mut binding,
        };
        aa_runtime::launch_settings_dump::dump(catalog, self, &mut sink, label).map_err(missing)
    }
    pub fn launch_ordinary_host_77a950(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        depot: &mut aa_runtime::resource_depot::Application,
        app: &aa_runtime::application_mode::GameAppState,
        manager: &aa_runtime::game_spy_manager_gate::State,
        text: &Localization,
        profiles: &mut dyn aa_runtime::load_sai_players::ProfileService,
        notifications: &Notifications,
        effects: &mut impl Effects,
    ) -> Result<()> {
        self.launch_ordinary_host_with_text_77a950(
            catalog,
            bank,
            depot,
            app,
            manager,
            Some(text),
            profiles,
            notifications,
            effects,
        )
    }
    pub(crate) fn launch_ordinary_host_with_text_77a950(
        &mut self,
        catalog: &aa_runtime::catalog::Catalog<'_>,
        bank: &aa_runtime::definitions::Bank,
        depot: &mut aa_runtime::resource_depot::Application,
        app: &aa_runtime::application_mode::GameAppState,
        manager: &aa_runtime::game_spy_manager_gate::State,
        text: Option<&Localization>,
        profiles: &mut dyn aa_runtime::load_sai_players::ProfileService,
        notifications: &Notifications,
        effects: &mut impl Effects,
    ) -> Result<()> {
        if self.host_input_stage()? != launch::InputStage::PlayersRefreshed
            || self.session.state != 0
            || self.session.launch_mode != 2
            || self.session.launch_game_type != 2
        {
            return Err(missing(
                "77A950 concrete fresh Host requires actual7791F0/map mode2/game type2/state0",
            ));
        }
        launch_owner::launch(&mut World {
            owner: self,
            catalog,
            bank,
            depot,
            app,
            manager,
            text,
            profiles,
            notifications,
            effects,
        })?;
        self.host_input.as_mut().unwrap().stage = launch::InputStage::MainLaunchComplete;
        Ok(())
    }
    pub fn launch_kind_60(&self) -> u32 {
        self.session.launch_kind_60
    }
    pub fn launch_text_84(&self) -> &[u16] {
        &self.session.launch_text_84
    }
}

impl aa_runtime::launch_settings_dump::AppRead for LaunchState {
    fn word(&self, offset: u32) -> std::result::Result<u32, String> {
        match offset {
            0 => Ok(self.kind),
            0xc => Ok(self.field_0c),
            0x10 => Ok(self.field_10),
            0x18 => Ok(self.field_18),
            0x1c => Ok(self.field_1c),
            0x24 => Ok(self.field_24),
            _ => Err("AppLaunchState word offset is not an owned native field".into()),
        }
    }
    fn byte(&self, offset: u32) -> std::result::Result<bool, String> {
        match offset {
            4 => Ok(self.flag_04),
            0x14 => Ok(self.flag_14),
            0x15 => Ok(self.flag_15),
            0x16 => Ok(self.flag_16),
            0x20 => Ok(self.flag_20),
            _ => Err("AppLaunchState byte offset is not an owned native field".into()),
        }
    }
    fn failure_text_8(&self) -> std::result::Result<&[u16], String> {
        Ok(&self.text)
    }
}
impl aa_runtime::launch_settings_dump::LaunchRead for LocalBootstrap {
    fn session_state_0(&self) -> std::result::Result<u32, String> {
        Ok(self.session.launch_game_type)
    }
    fn map_type_4(&self) -> std::result::Result<u32, String> {
        Ok(self.session.launch_mode)
    }
    fn filename_8(&self) -> std::result::Result<&[u16], String> {
        Ok(&self.session.launch_file_5c)
    }
    fn launch_type_0c(&self) -> std::result::Result<u32, String> {
        Ok(self.session.launch_kind_60)
    }
    fn header_14(&self) -> std::result::Result<&aa_runtime::game_settings_value::Owner, String> {
        self.current_game_settings_68().map_err(|e| e.to_string())
    }
    fn load_1c(&self) -> std::result::Result<&aa_runtime::resource_creator::Creator, String> {
        self.selected_load_creator().map_err(|e| e.to_string())
    }
    fn others(
        &self,
    ) -> std::result::Result<&aa_runtime::resource_creator::launch_creators::Others, String> {
        self.other_launch_creators().map_err(|e| e.to_string())
    }
    fn app_2c(
        &self,
    ) -> std::result::Result<&dyn aa_runtime::launch_settings_dump::AppRead, String> {
        Ok(&self.session.launch_state)
    }
}
