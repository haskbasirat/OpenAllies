//! Original aircraft attack-state release test `51B290` (thiscall on the
//! `aai_attack` state, `(out*)`, `ret 4`), run by the attack Think `51B8C0`
//! ([`crate::uai_thinks::aai_attack`]) as `51B290(&distance)`.
//!
//! `*out` is the facing term, at the caller's x87 precision:
//! - dog-fighter (Mind `+1D0` set): `d` = the target's (`+18` GTID slot,
//!   generation checked) `+20`/`+24`/`+28` minus the owner's, each stored to
//!   binary32 and normalized in place (`4453A0`, aa-movement
//!   `falling::normalize`); `*out = binary32((hz*dz + hy*dy) + dx*hx)` with
//!   the owner's `+2C`/`+30`/`+34` heading;
//! - otherwise: `d` = the state's aim point `+24`/`+28` minus the owner's
//!   `+20`/`+24`, stored and normalized (`409230`, aa-movement
//!   `transport_drop::normalize2`); `*out = binary32(dy*hy + dx*hx)` with the
//!   owner's `+2C`/`+30`.
//!
//! Then `51A260(slot, +24, +28, +2C, &ability, 0)` on the Mind
//! (aa-combat `ability_pick::pick`, the ability starting null). Class 3
//! answers AL 0. Class 0 with `*out` not below binary32 0.9 (`A326BC`)
//! arms the owner's flight (`+BC`): a state `+20` byte set stores the `+18`
//! GTID in flight `+94`, else the aim point in flight `+98`/`+9C`/`+A0`; both
//! store the ability in flight `+A4`. Any other class answers AL 1.
use crate::Result;
use crate::x87::{n, num, store};
use aa_numeric::Precision;
use std::cmp::Ordering;

/// binary32 0.9 at `A326BC`.
pub const RELEASE_FACING_BITS: u32 = 0x3f66_6666;

/// The reads and calls of `51B290`, in native order.
pub trait AttackReleaseWorld {
    type Object: Copy;
    type Ability: Copy;
    /// Fact: Mind `+1D0` byte.
    fn dogfighter(&mut self) -> Result<u8>;
    /// Fact: the `+18` GTID's table slot on a generation match, else null.
    fn target_slot(&mut self) -> Result<Option<Self::Object>>;
    /// Fact: object `+20`/`+24`/`+28` (the dog-fight branch dereferences the
    /// slot unchecked: a world returns an error for a null one).
    fn object_position(&mut self, object: Option<Self::Object>) -> Result<[u32; 3]>;
    /// Fact: the owner's `+20`/`+24`/`+28` and `+2C`/`+30`/`+34`.
    fn owner_position(&mut self) -> Result<[u32; 3]>;
    fn owner_heading(&mut self) -> Result<[u32; 3]>;
    /// Fact: state `+24`/`+28`/`+2C`.
    fn aim_point(&mut self) -> Result<[u32; 3]>;
    /// `4453A0` / `409230` in place on the stored difference.
    fn normalize3(&mut self, v: [u32; 3]) -> Result<[u32; 3]>;
    fn normalize2(&mut self, v: [u32; 2]) -> Result<[u32; 2]>;
    /// `51A260(slot, aim, &ability, 0)` on the Mind: the class and the
    /// ability written (null stays `None`).
    fn pick_ability(
        &mut self,
        slot: Option<Self::Object>,
        aim: [u32; 3],
    ) -> Result<(u32, Option<Self::Ability>)>;
    /// Fact: state `+20` byte and `+18` GTID.
    fn attack_object_byte(&mut self) -> Result<u8>;
    fn target_gtid(&mut self) -> Result<u32>;
    /// The owner's flight `+94` (GTID) or `+98..+A0` (point) store, then
    /// `+A4` (ability).
    fn arm_object(&mut self, gtid: u32) -> Result<()>;
    fn arm_point(&mut self, point: [u32; 3]) -> Result<()>;
    fn arm_ability(&mut self, ability: Option<Self::Ability>) -> Result<()>;
}

/// `51B290`: AL and the binary32 written to `*out`.
pub fn release<W: AttackReleaseWorld + ?Sized>(
    world: &mut W,
    precision: Precision,
) -> Result<(bool, u32)> {
    let p = precision;
    let out = if world.dogfighter()? != 0 {
        let slot = world.target_slot()?;
        let t = world.object_position(slot)?;
        let o = world.owner_position()?;
        let h = world.owner_heading()?;
        let mut d = [0u32; 3];
        for i in 0..3 {
            d[i] = store(num(n(t[i])?.sub(&n(o[i])?, p))?)?;
        }
        let d = world.normalize3(d)?;
        let zz = num(n(h[2])?.mul(&n(d[2])?, p))?;
        let yy = num(n(h[1])?.mul(&n(d[1])?, p))?;
        let xx = num(n(d[0])?.mul(&n(h[0])?, p))?;
        store(num(num(zz.add(&yy, p))?.add(&xx, p))?)?
    } else {
        let o = world.owner_position()?;
        let h = world.owner_heading()?;
        let a = world.aim_point()?;
        let dx = store(num(n(a[0])?.sub(&n(o[0])?, p))?)?;
        let dy = store(num(n(a[1])?.sub(&n(o[1])?, p))?)?;
        let [dx, dy] = world.normalize2([dx, dy])?;
        let yy = num(n(dy)?.mul(&n(h[1])?, p))?;
        let xx = num(n(dx)?.mul(&n(h[0])?, p))?;
        store(num(yy.add(&xx, p))?)?
    };
    let slot = world.target_slot()?;
    let aim = world.aim_point()?;
    let (class, ability) = world.pick_ability(slot, aim)?;
    if class == 0 {
        if n(out)?.compare(&n(RELEASE_FACING_BITS)?) != Ordering::Less {
            if world.attack_object_byte()? != 0 {
                let gtid = world.target_gtid()?;
                world.arm_object(gtid)?;
            } else {
                // Re-read after 51A260, as the original does.
                let point = world.aim_point()?;
                world.arm_point(point)?;
            }
            world.arm_ability(ability)?;
        }
        return Ok((true, out));
    }
    Ok((class != 3, out))
}
