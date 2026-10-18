//! The retained Loading AppState fields from 682DE0, distinct from the
//! GameAppState, SessionE4/F8 and the rendered loading-screen widgets.
//! No flag here certifies that VWorld43F0A0 or its resource queue finished.
use std::sync::{Arc,Weak,atomic::{AtomicU8,Ordering}};
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
///Only the native prefix before40CE10. Separating its argument capture lets
///a concrete caller release the SAME Loading loan and immediately reenter
///the application's40CE10/OnLeave/OnEnter on that allocation.
pub trait CompletionServices {
    fn view_initialized(&mut self)->Result<bool>;
    fn multiplayer(&mut self)->Result<bool>;
    fn waiting_on_other_players_773ce0(&mut self)->Result<()>;
    fn on_vworld_complete_76aca0(&mut self,initialized:bool)->Result<()>;
}
impl<T:TickServices+?Sized> CompletionServices for T {
    fn view_initialized(&mut self)->Result<bool>{TickServices::view_initialized(self)}
    fn multiplayer(&mut self)->Result<bool>{TickServices::multiplayer(self)}
    fn waiting_on_other_players_773ce0(&mut self)->Result<()>{TickServices::waiting_on_other_players_773ce0(self)}
    fn on_vworld_complete_76aca0(&mut self,v:bool)->Result<()>{TickServices::on_vworld_complete_76aca0(self,v)}
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
    flags_14_16: [u8; 3],
    flag_17:Arc<AtomicU8>,
}
impl State {
    /// 682DE0 -> 40CA60(0), then C/10/14/15/16/17 zero stores. The base UI
    /// pointer+4 is constructor-null and is not treated as a rendered widget.
    pub fn from_native_constructor() -> Self {
        Self { changed_8: 0, return_state_c: 0, kind_10: 0, flags_14_16: [0; 3],flag_17:Arc::new(AtomicU8::new(0)) }
    }
    /// 682E10. SessionF4 is read only when kind is nonzero, after all six
    /// field stores. A missing F4 retains that exact cleared prefix.
    pub fn bind_682e10(&mut self, return_state: u32, kind: u32,
        mut multiplayer: impl FnMut() -> Result<bool>) -> Result<()> {
        self.return_state_c = return_state;
        self.kind_10 = kind;
        self.flags_14_16 = [0; 3];self.flag_17.store(0,Ordering::Release);
        if kind == 0 || multiplayer()? || matches!(return_state, 7 | 5 | 14) {
            self.flags_14_16[2] = 1;
        }
        self.changed_8 = 1;
        Ok(())
    }
    /// 682DB0's actual byte argument. Nonzero does not clear an earlier15.
    pub fn menu_finished_682db0(&mut self, argument: u8) {
        crate::application_menu_callbacks::loading_finished_682db0(&mut self.flags_14_16[1],argument);
    }
    /// 682FD0 derived notification prefix, then the real base414620 handler.
    /// Kind1 status failure precedes the15 store; kind9 stores14 first.
    pub fn notification_682fd0(&mut self, kind: u32, services: &mut impl EventServices) -> Result<()> {
        self.notification_prefix_682fd0(kind,||services.loading_world_status_773ce0())?;
        services.base_notification_414620(kind)
    }
    ///The derived prefix is split only to release this same mutable Loading
    ///loan before the base handler invokes whole-World UI5C3000.
    pub fn notification_prefix_682fd0(&mut self,kind:u32,mut status:impl FnMut()->Result<()>)->Result<()> {
        if kind == 1 {
            status()?;
            self.flags_14_16[1] = 1;
        } else if kind == 9 {
            self.flags_14_16[0] = 1;
        }
        Ok(())
    }
    /// The17 store at5B356D. Only the resource-task callback in this crate
    /// writes it; a GPU upload, worker success or painted page cannot do so.
    #[cfg(test)]
    pub(crate) fn resource_callback_5b356d(&mut self) { self.flag_17.store(1,Ordering::Release); }
    /// 682EE0, including the reread of AD and clearing17 only after the
    /// Session/Admin callback succeeds. This returns no ready capability.
    pub fn tick_682ee0(&mut self, services: &mut impl TickServices) -> Result<()> {
        if let Some(state)=self.tick_before_state_transition_682ee0(services)? {
            services.set_application_state_40ce10(state)?;
        }
        Ok(())
    }
    ///Return only EDX's captured+C argument at682FAD. The concrete caller
    ///must invoke40CE10 immediately after releasing this loan, with no queue,
    ///observer or other callback in between. This grants no ready capability.
    pub fn tick_before_state_transition_682ee0(&mut self,services:&mut impl CompletionServices)->Result<Option<u32>> {
        if self.flags_14_16[1] != 0 && self.flag_17.load(Ordering::Acquire) != 0 && self.flags_14_16[2] != 0 {
            if services.view_initialized()? && services.multiplayer()? {
                services.waiting_on_other_players_773ce0()?;
            }
            let initialized = services.view_initialized()?;
            services.on_vworld_complete_76aca0(initialized)?;
            self.flag_17.store(0,Ordering::Release);
        }
        if self.flags_14_16[0] != 0 {
            return Ok(Some(self.return_state_c));
        }
        Ok(None)
    }
    pub fn changed_8(&self) -> u8 { self.changed_8 }
    pub fn resource_complete_17(&self)->bool{self.flag_17.load(Ordering::Acquire)!=0}
    pub fn user_ready_16(&self)->bool{self.flags_14_16[2]!=0}
    ///Actual UserReady command6E80F0→40D970→6E80FA, distinct from AD/17.
    ///The frontend supplies this only from its real source-bound button action.
    pub fn user_ready_command_6e80f0(&mut self){self.flags_14_16[2]=1;}
    ///Loading's actual vf18=40CAA0 only clears the change byte. It does not
    ///hide, replace or complete an interface.
    pub fn leave_40caa0(&mut self) { crate::application_menu_callbacks::leave_loading_40caa0(&mut self.changed_8); }
    pub fn return_state(&self) -> u32 { self.return_state_c }
    pub fn kind(&self) -> u32 { self.kind_10 }
    pub fn flags_14_17(&self) -> [u8; 4] { [self.flags_14_16[0],self.flags_14_16[1],self.flags_14_16[2],self.flag_17.load(Ordering::Acquire)] }
    pub(crate) fn resource_callback_writer(&self)->ResourceCallback{ResourceCallback{field:Arc::downgrade(&self.flag_17)}}
}
#[cfg(test)]
pub(crate) fn laboratory_queue_fields(flag17:u8)->State{
    //The native queue fixture explicitly seeds this byte before dispatch.
    let state=State::from_native_constructor();state.flag_17.store(flag17,Ordering::Release);state
}

#[cfg(test)]
#[path = "application_loading_tests.rs"]
mod tests;

///Nonowning loan of the SAME Loading17 cell, never a copied ready flag.
pub(crate) struct ResourceCallback{field:Weak<AtomicU8>}
impl ResourceCallback{pub(crate) fn set_5b356d(&self)->Result<()>{self.field.upgrade().ok_or("Loading17 allocation is retired")?.store(1,Ordering::Release);Ok(())}}
