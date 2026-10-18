//! Same77A950/77A750 procedure over direct retained owner access. The wire
//! Session and live LocalBootstrap can each implement this boundary without
//! cloning mutable Session state into a laboratory projection and copying back.
use super::*;

pub trait World {
    fn launch_state(&self) -> Result<&LaunchState>;
    fn state(&self) -> Result<u32>;
    fn launch_game_type(&self) -> Result<u32>;
    fn mark_save_last_settings(&mut self) -> Result<()>;
    fn increment_launch_and_clear_countdown(&mut self) -> Result<()>;
    fn launch_step(&mut self, step: LaunchStep) -> Result<()>;
    fn load(&mut self) -> Result<bool>;
    fn clear_flag_16b(&mut self) -> Result<()>;
    fn dump_launch_settings(&mut self, label: &'static str) -> Result<()>;
    fn timeline_freeze(&mut self, value: bool) -> Result<()>;
    fn reset_order_version(&mut self) -> Result<()>;
    fn store_flag_16a(&mut self, value: bool) -> Result<()>;
    fn leave_state(&mut self) -> Result<()>;
    fn clear_launch_game_type(&mut self) -> Result<()>;
    ///77A750 adjacent scalar stores:100=0,C8/C9=1,16C=0,E4=state.
    fn commit_state_prefix(&mut self, state: u32) -> Result<()>;
    fn store_phase(&mut self, phase: u32) -> Result<()>;
    fn launch_text(&self) -> Result<&[u16]>;
    fn state_changed(&mut self, event: StateEvent) -> Result<()>;
    fn clear_launch_text(&mut self) -> Result<()>;
    fn restore_last_launch_settings(&mut self) -> Result<()>;
}
pub fn launch(w: &mut impl World) -> Result<()> {
    let st = w.launch_state()?;
    if st.kind == 0xc && st.field_10 == 1 && !st.flag_14 {
        w.mark_save_last_settings()?;
    }
    w.increment_launch_and_clear_countdown()?;
    w.launch_step(LaunchStep::Step775DB0)?;
    w.launch_step(LaunchStep::Step770500)?;
    w.launch_step(LaunchStep::Step776B20)?;
    let loaded = w.load()?;
    w.clear_flag_16b()?;
    w.dump_launch_settings(LAUNCH_SETTINGS_DUMP_LABEL)?;
    let next_state = w.launch_game_type()?;
    change_to_state(w, next_state, loaded)?;
    w.timeline_freeze(!(in_game(w.state()?) && !loaded))?;
    w.reset_order_version()?;
    // Actual native branch readsE4 only when the earlier loaded byte is true.
    let active = loaded && in_game(w.state()?);
    w.store_flag_16a(active)
}
pub fn change_to_state(w: &mut impl World, state: u32, flag: bool) -> Result<()> {
    let old = w.state()?;
    let restore = old != 1 && state == 1;
    if !restore {
        if (old == 1 && state != 1) || (old == 2 && state == 2) || (old == 5 && state == 2) {
            w.leave_state()?;
        }
        w.clear_launch_game_type()?;
    }
    w.commit_state_prefix(state)?;
    let ls = w.launch_state()?;
    w.store_phase(if ls.kind == 6 { ls.field_18 } else { ls.kind })?;
    let event = StateEvent {
        text: w.launch_text()?.to_vec(),
        state,
        launch_state: w.launch_state()?.clone(),
        flag,
    };
    w.state_changed(event)?;
    w.clear_launch_text()?;
    if restore {
        w.restore_last_launch_settings()?;
    }
    Ok(())
}

pub(super) struct Laboratory<'a> {
    pub session: &'a mut Session,
    pub node: &'a StagingNode,
    pub host: &'a mut dyn StagingHost,
}
impl World for Laboratory<'_> {
    fn launch_state(&self) -> Result<&LaunchState> {
        Ok(&self.session.launch.state)
    }
    fn state(&self) -> Result<u32> {
        Ok(self.session.state)
    }
    fn launch_game_type(&self) -> Result<u32> {
        Ok(self.session.launch.game_type)
    }
    fn mark_save_last_settings(&mut self) -> Result<()> {
        self.session.flag_169 = true;
        Ok(())
    }
    fn increment_launch_and_clear_countdown(&mut self) -> Result<()> {
        self.session.field_164 = self.session.field_164.wrapping_add(1);
        self.session.launch_countdown = 0;
        Ok(())
    }
    fn launch_step(&mut self, step: LaunchStep) -> Result<()> {
        self.host.launch_step(self.session, step)
    }
    fn load(&mut self) -> Result<bool> {
        self.host.load(self.session)
    }
    fn clear_flag_16b(&mut self) -> Result<()> {
        self.session.flag_16b = false;
        Ok(())
    }
    fn dump_launch_settings(&mut self, label: &'static str) -> Result<()> {
        self.host.dump_launch_settings(self.session, label)
    }
    fn timeline_freeze(&mut self, value: bool) -> Result<()> {
        self.host.timeline_freeze(value)
    }
    fn reset_order_version(&mut self) -> Result<()> {
        self.host.reset_order_version()
    }
    fn store_flag_16a(&mut self, value: bool) -> Result<()> {
        self.session.flag_16a = value;
        Ok(())
    }
    fn leave_state(&mut self) -> Result<()> {
        super::leave_state(self.session, self.node, self.host)
    }
    fn clear_launch_game_type(&mut self) -> Result<()> {
        self.session.launch.game_type = 0;
        Ok(())
    }
    fn commit_state_prefix(&mut self, state: u32) -> Result<()> {
        self.session.field_100 = 0;
        self.session.changed = true;
        self.session.changed_c9 = true;
        self.session.flag_16c = false;
        self.session.state = state;
        Ok(())
    }
    fn store_phase(&mut self, phase: u32) -> Result<()> {
        self.session.field_f8 = phase;
        Ok(())
    }
    fn launch_text(&self) -> Result<&[u16]> {
        Ok(&self.session.launch.text_30)
    }
    fn state_changed(&mut self, event: StateEvent) -> Result<()> {
        self.host.state_changed(self.session, event)
    }
    fn clear_launch_text(&mut self) -> Result<()> {
        self.session.launch.text_30.clear();
        Ok(())
    }
    fn restore_last_launch_settings(&mut self) -> Result<()> {
        self.host
            .copy_launch_settings(self.session, SettingsSlot::Launch, SettingsSlot::Launch88)
    }
}
