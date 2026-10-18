//! The retained Loading AppState fields from 682DE0, distinct from the
//! GameAppState, SessionE4/F8 and the rendered loading-screen widgets.
//! No flag here certifies that VWorld43F0A0 or its resource queue finished.
type Result<T> = std::result::Result<T, String>;

/// Required lazy children of the original Loading682EE0 tick. In particular
/// the second AD read occurs after the optional status update; it is not a
/// captured readiness boolean. Positive children have no success defaults.
pub trait TickServices {
    fn view_initialized(&mut self) -> Result<bool>;
    fn multiplayer(&mut self) -> Result<bool>;
    fn waiting_on_other_players_773ce0(&mut self) -> Result<()>;
    fn on_vworld_complete_76aca0(&mut self, initialized: bool) -> Result<()>;
    fn set_application_state_40ce10(&mut self, state: u32) -> Result<()>;
}
pub trait EventServices {
    fn loading_world_status_773ce0(&mut self) -> Result<()>;
    /// The actual base414620 notification dispatch follows the derived writes.
    fn base_notification_414620(&mut self, kind: u32) -> Result<()>;
}
/// One actual Application+E4 Loading allocation. No Clone/Default or arbitrary
/// flag setter is exposed. Allocation failure and arbitrary callback mutation
/// of this allocation are outside this single UI-dispatch domain.
pub struct State {
    changed_8: u8,
    return_state_c: u32,
    kind_10: u32,
    flags_14_17: [u8; 4],
}
impl State {
    /// 682DE0 -> 40CA60(0), then C/10/14/15/16/17 zero stores. The base UI
    /// pointer+4 is constructor-null and is not treated as a rendered widget.
    pub fn from_native_constructor() -> Self {
        Self { changed_8: 0, return_state_c: 0, kind_10: 0, flags_14_17: [0; 4] }
    }
    /// 682E10. SessionF4 is read only when kind is nonzero, after all six
    /// field stores. A missing F4 retains that exact cleared prefix.
    pub fn bind_682e10(&mut self, return_state: u32, kind: u32,
        mut multiplayer: impl FnMut() -> Result<bool>) -> Result<()> {
        self.return_state_c = return_state;
        self.kind_10 = kind;
        self.flags_14_17 = [0; 4];
        if kind == 0 || multiplayer()? || matches!(return_state, 7 | 5 | 14) {
            self.flags_14_17[2] = 1;
        }
        self.changed_8 = 1;
        Ok(())
    }
    /// 682DB0's actual byte argument. Nonzero does not clear an earlier15.
    pub fn menu_finished_682db0(&mut self, argument: u8) {
        if argument == 0 { self.flags_14_17[1] = 1; }
    }
    /// 682FD0 derived notification prefix, then the real base414620 handler.
    /// Kind1 status failure precedes the15 store; kind9 stores14 first.
    pub fn notification_682fd0(&mut self, kind: u32, services: &mut impl EventServices) -> Result<()> {
        if kind == 1 {
            services.loading_world_status_773ce0()?;
            self.flags_14_17[1] = 1;
        } else if kind == 9 {
            self.flags_14_17[0] = 1;
        }
        services.base_notification_414620(kind)
    }
    /// The17 store at5B356D. Only the resource-task callback in this crate
    /// writes it; a GPU upload, worker success or painted page cannot do so.
    pub(crate) fn resource_callback_5b356d(&mut self) { self.flags_14_17[3] = 1; }
    /// 682EE0, including the reread of AD and clearing17 only after the
    /// Session/Admin callback succeeds. This returns no ready capability.
    pub fn tick_682ee0(&mut self, services: &mut impl TickServices) -> Result<()> {
        if self.flags_14_17[1] != 0 && self.flags_14_17[3] != 0 && self.flags_14_17[2] != 0 {
            if services.view_initialized()? && services.multiplayer()? {
                services.waiting_on_other_players_773ce0()?;
            }
            let initialized = services.view_initialized()?;
            services.on_vworld_complete_76aca0(initialized)?;
            self.flags_14_17[3] = 0;
        }
        if self.flags_14_17[0] != 0 {
            services.set_application_state_40ce10(self.return_state_c)?;
        }
        Ok(())
    }
    pub fn changed_8(&self) -> u8 { self.changed_8 }
    pub fn return_state(&self) -> u32 { self.return_state_c }
    pub fn kind(&self) -> u32 { self.kind_10 }
    pub fn flags_14_17(&self) -> [u8; 4] { self.flags_14_17 }
}

#[cfg(test)]
#[path = "application_loading_tests.rs"]
mod tests;
