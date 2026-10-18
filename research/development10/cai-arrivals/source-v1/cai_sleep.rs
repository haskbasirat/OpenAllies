//! Original `KKC_CAIStateSleep::Think` (`524D70`, regiment state `cai_sleep`
//! at Mind+34, vtable A329F0 slot +C): the idle behaviour of an organization.
//!
//! Order (all reads and calls exactly as native; the diagnostic log lines are
//! omitted):
//! 1. `KKC_TAIState::Think` base (`529E60`), then schedule the next update in
//!    2.0 s (Mind Updatable +8 virtual +4).
//! 2. While current (`52A000`): an organization whose status (`578140`) is 0
//!    tries the exhausted reaction (`51F9F0`); success ends the Think.
//! 3. While current, with a follow target (Mind+2C GTID): an unresolved
//!    target calls Mind virtual +190; else the organization's +164(target)
//!    either hands it to Mind virtual +188 or the follow state (Mind+C0) is
//!    activated in mode 2 for the re-resolved target (`51FFE0`, `51D920`) and
//!    thinks (+C).
//! 4. While current, with a home (Mind+28 GTID): an unresolved or removed home
//!    calls `4D7960` on the organization; a home without a denizen component
//!    (+88) ends the Think (native warning); otherwise Mind+30 selects
//!    `56AEC0(Mind+30, Mind+24)` on that component or Mind virtual +F4(org).
//! 5. Drain the arrival list (Mind+2BC): each live object whose membership
//!    (+80 -> +10) is this organization joins (`522890(object, 0)`).
//! 6. A sleep point (Mind+2B4/+2B8) different from the null point
//!    (`[B279C0]/[B279C4]`, unordered counts as different) is sent to the
//!    organization component (`57BF30`) and reset to the null point.
//! 7. While current and the organization is idle (`520120` false): a resumed
//!    pending command (`52BF30`) ends the Think; an organization flag
//!    (`[org+4]+310`) activates the Mind+54 state in mode 2 (`52A810`).
//! 8. When state+1C differs from the component word (+7C -> +74) it is copied
//!    and `522F60` runs; otherwise state+10 (byte) selects `523000`.
use crate::{Arena, MindId, Result, StateId, World, activate, is_current, think_base};

/// binary32 2.0: sleep re-think delay.
pub const SLEEP_DELAY_BITS: u32 = 0x4000_0000;
/// Embedded regiment states used here: +C0 is the `kill` state (metadata
/// name, Think `520730`; labelled follow in these identifiers).
pub const FOLLOW_OFFSET: u32 = 0xc0;
pub const FLAG310_STATE_OFFSET: u32 = 0x54;

/// Services in native order (`World` drives the Mind primitives).
pub trait SleepThinkWorld: World {
    /// `578140` on the organization component (+7C).
    fn organization_status(&mut self, mind: MindId, arena: &Arena) -> Result<u32>;
    /// `51F9F0` (`cai_reactions::exhausted`).
    fn exhausted_reaction(&mut self, mind: MindId, arena: &mut Arena) -> Result<bool>;
    /// GTID registry (`[B364C0]`): the live object for a GTID whose
    /// generation matches, `None` for a mismatch or a null entry.
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// Object+8 bit 0 (removed).
    fn object_removed(&mut self, object: u32) -> Result<bool>;
    /// Organization virtual +164(object).
    fn organization_follows(
        &mut self,
        mind: MindId,
        object: u32,
        arena: &mut Arena,
    ) -> Result<bool>;
    /// Mind virtual +188(object).
    fn mind_virtual_188(&mut self, mind: MindId, object: u32, arena: &mut Arena) -> Result<()>;
    /// Mind virtual +190.
    fn mind_virtual_190(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// `51D920(2, target)` on the follow state (Mind+C0); result unused.
    fn follow_activate(&mut self, mind: MindId, target: u32, arena: &mut Arena) -> Result<()>;
    /// Virtual +C (Think) of the embedded state at Mind+`offset`.
    fn embedded_think(&mut self, mind: MindId, offset: u32, arena: &mut Arena) -> Result<()>;
    /// `4D7960` on the organization.
    fn organization_4d7960(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Home object+88 (denizen component), `None` when null.
    fn home_component(&mut self, object: u32) -> Result<Option<u32>>;
    /// `56AEC0(word30, word24)` on a denizen component.
    fn home_settle(&mut self, component: u32, word30: u32, word24: u32) -> Result<()>;
    /// Mind virtual +F4(organization).
    fn mind_virtual_f4(&mut self, mind: MindId, arena: &mut Arena) -> Result<()>;
    /// Pops the arrival list head (Mind+2BC/+2C0), returning its GTID.
    fn take_arrival(&mut self, mind: MindId) -> Result<Option<u32>>;
    ///Legacy laboratory adapters may keep their historical early pop. Actual
    ///owners override BOTH methods to resolve before native unlink/free.
    fn peek_arrival(&mut self, mind: MindId, _arena: &Arena) -> Result<Option<u32>> {
        self.take_arrival(mind)
    }
    fn remove_arrival(&mut self, _mind: MindId, _arena: &Arena) -> Result<()> {
        Ok(())
    }
    /// Object+80 -> +10 (its membership's organization).
    fn member_organization(&mut self, object: u32) -> Result<u32>;
    /// `529C30`: the Mind's organization.
    fn organization(&mut self, mind: MindId, arena: &Arena) -> Result<u32>;
    /// `522890(object, 0)` on the Mind.
    fn member_arrived(&mut self, mind: MindId, object: u32, arena: &mut Arena) -> Result<()>;
    /// Mind+2B4/+2B8 (binary32).
    fn sleep_point(&mut self, mind: MindId) -> Result<[u32; 2]>;
    fn set_sleep_point(&mut self, mind: MindId, point: [u32; 2]) -> Result<()>;
    /// `[B279C0]/[B279C4]`.
    fn null_point(&mut self) -> Result<[u32; 2]>;
    /// `57BF30(x, y)` on the organization component.
    fn organization_move(&mut self, mind: MindId, point: [u32; 2], arena: &mut Arena)
    -> Result<()>;
    /// `520120` on the sleep state.
    fn organization_busy(&mut self, state: StateId, arena: &mut Arena) -> Result<bool>;
    /// `52BF30` on the Mind (`command::resume_pending`).
    fn resume_pending(&mut self, mind: MindId, arena: &mut Arena) -> Result<bool>;
    /// `[org+4]+310` byte.
    fn organization_flag310(&mut self, mind: MindId, arena: &Arena) -> Result<bool>;
    /// Identity of the embedded state at Mind+`offset`.
    fn embedded_state(&mut self, mind: MindId, offset: u32) -> Result<StateId>;
    /// Organization component (+7C) word +74.
    fn organization_word74(&mut self, mind: MindId, arena: &Arena) -> Result<u32>;
    /// Sleep state+1C.
    fn sleep_word1c(&mut self, state: StateId) -> Result<u32>;
    fn set_sleep_word1c(&mut self, state: StateId, value: u32) -> Result<()>;
    /// Sleep state+10 byte.
    fn sleep_byte10(&mut self, state: StateId) -> Result<bool>;
    /// `522F60` on the sleep state.
    fn sleep_522f60(&mut self, state: StateId, arena: &mut Arena) -> Result<()>;
    /// `523000` on the sleep state.
    fn sleep_523000(&mut self, state: StateId, arena: &mut Arena) -> Result<()>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
/// `fld dword`/`fstp dword` of a binary32: a signalling NaN is quieted.
fn x87_copy(bits: u32) -> u32 {
    let nan = bits & 0x7f80_0000 == 0x7f80_0000 && bits & 0x007f_ffff != 0;
    if nan { bits | 0x0040_0000 } else { bits }
}

/// Complete `524D70`. Returns the base Think's wrong-activity flag.
pub fn think(state: StateId, arena: &mut Arena, world: &mut impl SleepThinkWorld) -> Result<bool> {
    let wrong = think_base(state, arena)?;
    let mind = arena.state(state)?.parent;
    world.schedule(mind, SLEEP_DELAY_BITS, 0, arena)?;
    if is_current(state, arena)?
        && world.organization_status(mind, arena)? == 0
        && world.exhausted_reaction(mind, arena)?
    {
        return Ok(wrong);
    }
    if is_current(state, arena)? {
        let target = arena.mind(mind)?.words_24_30[2];
        if target != 0 {
            match world.resolve(target)? {
                None => world.mind_virtual_190(mind, arena)?,
                Some(object) => {
                    if world.organization_follows(mind, object, arena)? {
                        world.mind_virtual_188(mind, object, arena)?;
                    } else {
                        let gtid = arena.mind(mind)?.words_24_30[2];
                        let target = world.resolve(gtid)?.unwrap_or(0);
                        world.follow_activate(mind, target, arena)?;
                        world.embedded_think(mind, FOLLOW_OFFSET, arena)?;
                    }
                }
            }
            return Ok(wrong);
        }
    }
    if is_current(state, arena)? {
        let home = arena.mind(mind)?.words_24_30[1];
        if home != 0 {
            let object = match world.resolve(home)? {
                Some(object) if !world.object_removed(object)? => object,
                _ => {
                    world.organization_4d7960(mind, arena)?;
                    return Ok(wrong);
                }
            };
            let Some(component) = world.home_component(object)? else {
                return Ok(wrong);
            };
            let words = arena.mind(mind)?.words_24_30;
            if words[3] != 0 {
                world.home_settle(component, words[3], words[0])?;
            } else {
                world.mind_virtual_f4(mind, arena)?;
            }
            return Ok(wrong);
        }
    }
    crate::cai_arrivals::drain(mind, arena, world)?;
    let point = world.sleep_point(mind)?;
    let null = world.null_point()?;
    if host(point[0]) != host(null[0]) || host(point[1]) != host(null[1]) {
        world.organization_move(mind, point, arena)?;
        let null = world.null_point()?;
        world.set_sleep_point(mind, [null[0], x87_copy(null[1])])?;
    }
    if is_current(state, arena)? && !world.organization_busy(state, arena)? {
        if world.resume_pending(mind, arena)? {
            return Ok(wrong);
        }
        if world.organization_flag310(mind, arena)? {
            let target = world.embedded_state(mind, FLAG310_STATE_OFFSET)?;
            activate(target, 2, arena, world)?;
            return Ok(wrong);
        }
    }
    if world.sleep_word1c(state)? != world.organization_word74(mind, arena)? {
        let value = world.organization_word74(mind, arena)?;
        world.set_sleep_word1c(state, value)?;
        world.sleep_522f60(state, arena)?;
        return Ok(wrong);
    }
    if world.sleep_byte10(state)? {
        world.sleep_523000(state, arena)?;
    }
    Ok(wrong)
}

impl<W: SleepThinkWorld> crate::cai_arrivals::DrainWorld for W {
    fn front(&mut self, mind: MindId, arena: &Arena) -> Result<Option<u32>> {
        self.peek_arrival(mind, arena)
    }
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>> {
        SleepThinkWorld::resolve(self, gtid)
    }
    fn pop_front(&mut self, mind: MindId, arena: &Arena) -> Result<()> {
        self.remove_arrival(mind, arena)
    }
    fn member_organization(&mut self, object: u32) -> Result<u32> {
        SleepThinkWorld::member_organization(self, object)
    }
    fn organization(&mut self, mind: MindId, arena: &Arena) -> Result<u32> {
        SleepThinkWorld::organization(self, mind, arena)
    }
    fn member_arrived(&mut self, mind: MindId, object: u32, arena: &mut Arena) -> Result<()> {
        SleepThinkWorld::member_arrived(self, mind, object, arena)
    }
}
#[path="cai_sleep_fields.rs"]
pub mod fields;
