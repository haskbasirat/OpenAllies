//!Read-only reads of the process input globals the in-battle world input
//!(aa_runtime::player_order_input) tests. Each value is read from its
//!retained owner at the call; nothing here is a snapshot or a writer.
use super::*;

impl Notifications {
    ///DAT_00B396C0, the application kind this queue's 7851C0 already tests:
    ///KKC_Input::AddMouseEvent 0x59A6E0 drops every mouse event while it
    ///equals 3 (`cmp dword ptr [0xb396c0], 3; je`).
    pub fn application_kind_b396c0(&self) -> u32 {
        self.application_kind.load(Ordering::SeqCst)
    }
}
