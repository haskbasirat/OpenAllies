//! Actual GameAppState returned by40D960(Application+FC).
//!
//! Application40CB40 installs651810's allocation in that slot. This is not
//! SessionE4/F8 or MapResource+A8.651840 consumes the actual launch-state mode
//! when784220 processes its notification. The same owner must survive that
//! transition; constructing a second mode object is not a launch substitute.
//! Evidence: research/skirmish-20261016-local-bootstrap/application.

/// The two optional manager predicates and actual posted reset effects in
///651840, read lazily in native order. No implicit manager absence exists.
pub trait ModeServices {
    /// B40D00 nonnull && manager+4 nonnull. A false result certifies the actual
    /// pointer/field predicate, rather than an unbound callback default.
    fn manager_a_active(&mut self) -> Result<bool, String>;
    ///794B70: allocate/post the actual reset command through787380.
    fn post_manager_a_reset(&mut self) -> Result<(), String>;
    /// B40D28 nonnull && manager+14 nonnull.
    fn manager_b_active(&mut self) -> Result<bool, String>;
    ///79A340: allocate/post the actual reset command through787380.
    fn post_manager_b_reset(&mut self) -> Result<(), String>;
}

/// Non-clone owning projection of all GameAppState constructor fields read or
/// written by651840. Base+4 is constructor0; native vtable addresses become
/// type identity. No unknown/padding word is given an invented value.
#[derive(Debug)]
pub struct GameAppState {
    base_4: u32,
    changed_8: bool,
    mode_0c: u32,
    flag_10: bool,
    flag_11: bool,
}
impl GameAppState {
    ///651810 ->40CA60(0): base+4=0,+8=false,+C=1,+10/+11=false.
    pub fn from_native_constructor() -> Self {
        Self {
            base_4: 0,
            changed_8: false,
            mode_0c: 1,
            flag_10: false,
            flag_11: false,
        }
    }
    ///40D960 returns this allocation; callers then read its actual+C.
    pub fn mode(&self) -> u32 {
        self.mode_0c
    }
    pub fn changed(&self) -> bool {
        self.changed_8
    }
    pub fn constructor_flags(&self) -> (u32, bool, bool) {
        (self.base_4, self.flag_10, self.flag_11)
    }
    ///651840. The mode store precedes both conditional effects and survives
    /// a reached service error. +8 is written only after both complete.
    pub fn set_mode(
        &mut self,
        actual_launch_mode: u32,
        services: &mut impl ModeServices,
    ) -> Result<(), String> {
        self.mode_0c = actual_launch_mode;
        if services.manager_a_active()? {
            services.post_manager_a_reset()?;
        }
        if services.manager_b_active()? {
            services.post_manager_b_reset()?;
        }
        self.changed_8 = true;
        Ok(())
    }
}

#[cfg(test)]
#[path = "application_mode_tests.rs"]
mod tests;
