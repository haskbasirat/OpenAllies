//! Original `KKC_UAIStateBombard::Activate(mode, target)` `538120` (element
//! bombard state, Mind+10C; UAI.cpp): start bombarding an object.
//!
//! Order of the original:
//! 1. A null target answers false.
//! 2. The state already current (`52A000`, [`crate::is_current`]) on the
//!    target's GTID (state +10 == target +14) answers true.
//! 3. `4D4FF0(target, 3, no_locomotion)` on the Mind actor, where
//!    `no_locomotion` is actor +6C == null: 5 (target in fog) goes on, any
//!    other nonzero code answers false. The diagnostics (`404F60`, UAI.cpp
//!    lines 0xED4 / 0xEDA) only read state.
//! 4. `537AF0(target)` stores the target (state +10 GTID and the +14 name
//!    reference), the bombard point state +18/+1C takes target +20/+24, and
//!    the result is `52A810(mode)` ([`crate::activate`]).
use crate::Result;

/// Services of `538120`, in native order.
pub trait BombardActivateWorld {
    /// `52A000` on the bombard state.
    fn is_current(&mut self) -> Result<bool>;
    /// Bombard state +10 (target GTID).
    fn target_gtid(&mut self) -> Result<u32>;
    /// Object +14 (its GTID).
    fn object_gtid(&mut self, object: u32) -> Result<u32>;
    /// Mind actor (+4) +6C locomotion present.
    fn has_locomotion(&mut self) -> Result<bool>;
    /// `4D4FF0(object, a, b)` on the Mind actor.
    fn relation(&mut self, object: u32, a: u32, b: u32) -> Result<i32>;
    /// `537AF0(object)` on the bombard state: +10 = object +14 and the +14
    /// name reference from object virtual +24.
    fn set_target(&mut self, object: u32) -> Result<()>;
    /// Object +20/+24 (binary32).
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    /// Bombard state +18/+1C write.
    fn set_point(&mut self, point: [u32; 2]) -> Result<()>;
    /// `52A810(mode)` on the bombard state ([`crate::activate`]).
    fn activate(&mut self, mode: u32) -> Result<bool>;
}

/// The relation code that still bombards (the target is in fog).
pub const IN_FOG: i32 = 5;

/// Complete `538120(mode, target)`; returns the original's bool.
pub fn activate(mode: u32, target: Option<u32>, world: &mut impl BombardActivateWorld) -> Result<bool> {
    let Some(target) = target else {
        return Ok(false);
    };
    if world.is_current()? && world.target_gtid()? == world.object_gtid(target)? {
        return Ok(true);
    }
    let no_locomotion = !world.has_locomotion()?;
    let code = world.relation(target, 3, u32::from(no_locomotion))?;
    if code != IN_FOG && code != 0 {
        return Ok(false);
    }
    world.set_target(target)?;
    let point = world.object_position(target)?;
    world.set_point(point)?;
    world.activate(mode)
}
