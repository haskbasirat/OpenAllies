//! Original `57E360(&context)`: a Performer task (+4 actor, +28 target GTID,
//! +2C/+30 point) fills the `4C8300` targeting context its callers
//! (`57E8C0`, `57ED20`, `57F040`) hand to `4C8F20`.
//!
//! Written words: +4 = 0, +8 = the task's actor, +1C = the target GTID's
//! registry entry (`[B364C0]`; 0 on a generation mismatch), +20/+24 = the
//! target's +20/+24 when the GTID resolves to a non-null entry, else the
//! task's +2C/+30 point, and +28 = 0. The point goes through `fld`/`fstp`
//! binary32, which on the x87 quiets a signalling NaN (the Unicorn oracle
//! does not model that case); other words are copied as they are. Every
//! other context word is left as it was.
use crate::Result;

/// The context words `57E360` writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TargetingWords {
    /// +4.
    pub word4: u32,
    /// +8.
    pub actor: u32,
    /// +1C (0 = null).
    pub target: u32,
    /// +20/+24.
    pub point: [u32; 2],
    /// +28.
    pub word28: u32,
}

/// Services of `57E360`, in native order.
pub trait PerformerContextWorld {
    /// Task +4 (its actor).
    fn actor(&mut self) -> Result<u32>;
    /// Task +28 (target GTID).
    fn target_gtid(&mut self) -> Result<u32>;
    /// `[B364C0]` lookup: the entry for a matching generation (`None` for a
    /// mismatch or a null entry).
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// Object +20/+24.
    fn object_position(&mut self, object: u32) -> Result<[u32; 2]>;
    /// Task +2C/+30.
    fn task_point(&mut self) -> Result<[u32; 2]>;
}

/// `fld dword` / `fstp dword`: a signalling NaN comes back quiet.
fn through_x87(bits: u32) -> u32 {
    if bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0 {
        bits | 0x0040_0000
    } else {
        bits
    }
}

/// Complete `57E360`.
pub fn fill(world: &mut impl PerformerContextWorld) -> Result<TargetingWords> {
    let actor = world.actor()?;
    let gtid = world.target_gtid()?;
    let target = world.resolve(gtid)?;
    let point = match target {
        Some(object) => world.object_position(object)?,
        None => world.task_point()?,
    };
    Ok(TargetingWords {
        word4: 0,
        actor,
        target: target.unwrap_or(0),
        point: [through_x87(point[0]), through_x87(point[1])],
        word28: 0,
    })
}
