//! 0x61F210's host reads and 404F90 sink for the accepted86 decree.
//! The ordering player is the SAME Admin+284 Session player whose selection
//! the decree delivers; its Player+8 name and Player+28 kingdom are read from
//! that one allocation. The B396E0 application timer and the [B39B1C] view
//! time are the frontend's current owners (no default exists).
use super::*;
use aa_runtime::player_order_decree::describe::{Description, Sources};

pub(crate) struct Describe<'a, 'backend, F: ?Sized> {
    pub(crate) name_8: &'a [u16],
    /// Player+28 captured from the SAME ordering allocation (765E00 with a
    /// created World: [B36534] is present during an accepted gameplay decree).
    pub(crate) kingdom_28: Option<aa_runtime::load_tail_world::KingdomId>,
    pub(crate) frontend: &'a RefCell<&'backend mut F>,
    pub(crate) sink: &'a mut Vec<String>,
}
fn frontend<'f, 'b, F: ?Sized>(cell: &'f RefCell<&'b mut F>) -> std::result::Result<std::cell::RefMut<'f, &'b mut F>, String> {
    cell.try_borrow_mut().map_err(|e| format!("61F210 frontend loan: {e}"))
}
impl<F: live_frame::Frontend + ?Sized> Sources for Describe<'_, '_, F> {
    fn ordering_player_name_8(&mut self) -> std::result::Result<Vec<u16>, String> {
        Ok(self.name_8.to_vec())
    }
    fn ordering_player_kingdom_765e00(&mut self) -> std::result::Result<Option<aa_runtime::load_tail_world::KingdomId>, String> {
        Ok(self.kingdom_28)
    }
    fn application_milliseconds_59aba0(&mut self) -> std::result::Result<u32, String> {
        frontend(self.frontend)?.application_milliseconds_b396e0_current()
    }
    fn view_time_b39b1c(&mut self) -> std::result::Result<u32, String> {
        frontend(self.frontend)?.view_time_b39b1c_current()
    }
    fn diagnostic_404f90(&mut self, d: Description) -> std::result::Result<(), String> {
        // The exact native varargs (doubles as binary64 bits); rendering
        // %.1f is not claimed here.
        self.sink.push(format!("Native 404F90 B38B70 {} line {} {:?}", aa_runtime::player_order_decree::describe::FUNCTION, d.line(), d));
        Ok(())
    }
}
