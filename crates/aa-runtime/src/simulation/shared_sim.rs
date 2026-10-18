//! A synchronous loan of the SAME actual Sim for driver and order callbacks.
//! Each field loan ends before the next callback. The cell contains only a
//! temporary mutable reference, never the whole Host or a copied Timeline.
use super::{
    FloatBits, Result,
    borrowed::{SimFields, TimeWord},
};
use crate::live_sim::Sim;
use std::cell::RefCell;

pub struct SharedSim<'frame, 'owner> {
    owner: &'frame RefCell<&'owner mut Sim>,
}
impl<'frame, 'owner> SharedSim<'frame, 'owner> {
    pub fn new(owner: &'frame RefCell<&'owner mut Sim>) -> Self {
        Self { owner }
    }
    fn read<R>(&self, call: impl FnOnce(&Sim) -> R) -> R {
        let owner = self.owner.borrow();
        call(&owner)
    }
    fn write<R>(&self, call: impl FnOnce(&mut Sim) -> R) -> R {
        let mut owner = self.owner.borrow_mut();
        call(&mut owner)
    }
}
// A reentrant caller retaining a field loan across the synchronous callback
// boundary is outside this sequential native domain. It must not be disguised
// as a null World, false flag, absent rate, or a fresh replacement Sim.
impl SimFields for SharedSim<'_, '_> {
    fn frozen(&self) -> bool {
        self.read(SimFields::frozen)
    }
    fn frozen_for_log(&self) -> Result<bool> {
        self.read(SimFields::frozen_for_log)
    }
    fn paused(&self) -> bool {
        self.read(SimFields::paused)
    }
    fn time(&self, field: TimeWord) -> Result<FloatBits> {
        self.read(|owner| SimFields::time(owner, field))
    }
    fn store_time(&mut self, field: TimeWord, value: FloatBits) -> Result<()> {
        self.write(|owner| SimFields::store_time(owner, field, value))
    }
    fn game_per_admin_tick(&self) -> Result<FloatBits> {
        self.read(SimFields::game_per_admin_tick)
    }
    fn avi_frames(&self) -> i32 {
        self.read(SimFields::avi_frames)
    }
    fn store_avi_frames(&mut self, value: i32) {
        self.write(|owner| SimFields::store_avi_frames(owner, value))
    }
    fn store_frozen(&mut self, value: bool) {
        self.write(|owner| SimFields::store_frozen(owner, value))
    }
    fn store_maximum(&mut self, value: FloatBits) {
        self.write(|owner| SimFields::store_maximum(owner, value))
    }
}
