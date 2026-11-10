//! Native City component tick `5647A0` (queue 1, once per second while the
//! City `+18` candidate list is not empty) and the City block hand-over
//! `5643A0` (the actor `+D4` step of the owner change `4D8450`).
//!
//! `5647A0` weighs the candidates of City `+18` against the owner of the
//! City actor (City `+4`, virtual `+108`). With an owner, each candidate the
//! owner's ally (virtual `+138`) adds its combat value (`4D0180`) to the
//! allied sum and, when it is the owner's own (virtual `+134`), to the
//! defence; any other candidate the owner's enemy (virtual `+140`) adds to
//! the attack. An attack above zero with no defence above zero, or no owner
//! at all, lets the zone decide; otherwise City `+20` (under siege) is set
//! unless the attack is below the allied sum. The decision sums per kingdom
//! (kingdom `+18C` index into two arrays of World `+158` entries) each
//! candidate's value for its owner and, for every other kingdom of World
//! `+154` that regards that owner as an enemy (kingdom virtual `+140`), as a
//! threat to that kingdom. The first kingdom in World `+154` order without a
//! threat above zero, with a value above zero, and with no ally (kingdom
//! virtual `+138`) holding more value or under a threat above zero, takes the
//! city: when it differs from the owner read again at the end, the City
//! actor's capture attempt `4D5230` runs with the record event kind 1,
//! reason 10.
//!
//! All arithmetic is the native x87 order: each `4D0180` value is added
//! unstored to the binary32 accumulator, and the sum is stored to binary32.
//! The comparisons are `fcomp` against binary32 operands (`[9F32DC]` is 0.0).
//! The two arrays are allocated and freed inside the tick (`6F7810`,
//! `45F180` zero fill, `6F7430`); they are local here.
use crate::{Error, Result};
use aa_body::{F32, damage::DamageRecord};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

pub trait World {
    /// City `+4` actor virtual `+108`: the city's owner. Read at entry and
    /// again before the hand-over.
    fn city_owner(&mut self) -> Result<Option<u32>>;
    /// Store City `+20` (byte, under siege).
    fn set_under_siege(&mut self, value: bool) -> Result<()>;
    /// City `+18` head node (each node: `+0` GTID, `+4` next).
    fn first_candidate(&mut self) -> Result<Option<u32>>;
    /// Node `+4`.
    fn next_node(&mut self, node: u32) -> Result<Option<u32>>;
    /// Node `+0` GTID through `[B364C0]`: the object when the `+4` table
    /// generation word matches and the `+20004` table entry is not null.
    fn resolve_node(&mut self, node: u32) -> Result<Option<u32>>;
    /// Object virtual `+108`.
    fn owner(&mut self, object: u32) -> Result<Option<u32>>;
    /// Object virtual `+134(other)`.
    fn virtual_134(&mut self, object: u32, other: Option<u32>) -> Result<bool>;
    /// Object virtual `+138(other)`.
    fn virtual_138(&mut self, object: u32, other: Option<u32>) -> Result<bool>;
    /// Object virtual `+140(other)`.
    fn virtual_140(&mut self, object: u32, other: Option<u32>) -> Result<bool>;
    /// `4D0180` on the object (ST0, unstored); bind
    /// [`crate::candidate_value::value`].
    fn combat_value(&mut self, object: u32) -> Result<Binary>;
    /// World `+158` (signed count of the `+154` kingdom table).
    fn kingdom_count(&mut self) -> Result<i32>;
    /// World `+154` table entry `index` (may be null).
    fn kingdom_slot(&mut self, index: i32) -> Result<Option<u32>>;
    /// Kingdom `+18C`.
    fn kingdom_index(&mut self, kingdom: u32) -> Result<i32>;
    /// `4D5230(winner, record)` on the City `+4` actor; bind
    /// [`crate::capture::attempt`]. Its result is not read.
    fn capture(&mut self, winner: Option<u32>, record: &DamageRecord) -> Result<()>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// `fadd dword [acc]` then `fstp dword [acc]`.
fn accumulate(value: &Binary, acc: F32, p: Precision) -> Result<F32> {
    Ok(F32(value.add(&n(acc)?, p)?.store_f32()?))
}

fn cmp_zero(value: F32) -> Result<Ordering> {
    Ok(n(value)?.compare(&Binary::from_u32(0)))
}

/// `4F0680` then `4F06B0` over World `+154`: the iterator index lands on
/// non-null slots only; the count is read again at every step.
struct Kingdoms {
    index: i32,
}
impl Kingdoms {
    fn first(world: &mut impl World) -> Result<Self> {
        let mut index = 0;
        if 0 < world.kingdom_count()? {
            loop {
                if world.kingdom_slot(index)?.is_some() {
                    break;
                }
                index += 1;
                if index >= world.kingdom_count()? {
                    break;
                }
            }
        }
        Ok(Self { index })
    }
    fn advance(&mut self, world: &mut impl World) -> Result<()> {
        if self.index < world.kingdom_count()? {
            loop {
                self.index += 1;
                if self.index < world.kingdom_count()? && world.kingdom_slot(self.index)?.is_some() {
                    return Ok(());
                }
                if self.index >= world.kingdom_count()? {
                    return Ok(());
                }
            }
        }
        Ok(())
    }
    /// The loop test `index < [+158]` and the slot read.
    fn current(&self, world: &mut impl World) -> Result<Option<u32>> {
        if self.index < world.kingdom_count()? {
            world
                .kingdom_slot(self.index)?
                .map(Some)
                .ok_or(Error::Invalid("5647A0 iterator on a null kingdom slot"))
        } else {
            Ok(None)
        }
    }
}

fn slot(index: i32, len: usize) -> Result<usize> {
    usize::try_from(index)
        .ok()
        .filter(|&i| i < len)
        .ok_or(Error::Invalid("kingdom +18C outside the World +158 arrays"))
}

/// The record `5647A0` passes to `4D5230`: event kind 1, reason 10, rest zero.
pub fn capture_record() -> DamageRecord {
    DamageRecord {
        source_id: 0,
        allegiance_id: 0,
        damage_type: 0,
        amount: F32::ZERO,
        time: F32::ZERO,
        event_kind: 1,
        reason: 10,
    }
}

/// The zone decision (`5648D8`): the winner, or `None` when no kingdom
/// qualifies.
fn decide(world: &mut impl World, p: Precision) -> Result<Option<u32>> {
    let count = world.kingdom_count()?;
    let len = usize::try_from(count).map_err(|_| Error::Invalid("negative World +158"))?;
    let mut value = vec![F32::ZERO; len];
    let mut threat = vec![F32::ZERO; len];
    let mut node = world.first_candidate()?;
    while let Some(at) = node {
        if let Some(object) = world.resolve_node(at)? {
            let owner = world
                .owner(object)?
                .ok_or(Error::Invalid("5647A0 candidate without an owner (+18C of null)"))?;
            let own = slot(world.kingdom_index(owner)?, len)?;
            let v = world.combat_value(object)?;
            value[own] = accumulate(&v, value[own], p)?;
            let mut walk = Kingdoms::first(world)?;
            while let Some(kingdom) = walk.current(world)? {
                if kingdom != owner && world.virtual_140(kingdom, Some(owner))? {
                    let k = slot(world.kingdom_index(kingdom)?, len)?;
                    let v = world.combat_value(object)?;
                    threat[k] = accumulate(&v, threat[k], p)?;
                }
                walk.advance(world)?;
            }
        }
        node = world.next_node(at)?;
    }
    let mut outer = Kingdoms::first(world)?;
    'outer: while let Some(kingdom) = outer.current(world)? {
        let k = slot(world.kingdom_index(kingdom)?, len)?;
        // 564A7E: a threat above zero disqualifies; 564A92: so does a value
        // not above zero.
        if cmp_zero(threat[k])? != Ordering::Greater && cmp_zero(value[k])? == Ordering::Greater {
            let mut inner = Kingdoms::first(world)?;
            while let Some(other) = inner.current(world)? {
                if kingdom != other && world.virtual_138(kingdom, Some(other))? {
                    let own = slot(world.kingdom_index(kingdom)?, len)?;
                    let o = slot(world.kingdom_index(other)?, len)?;
                    // 564AFD: an ally with more value; 564B0E: an ally
                    // under a threat above zero.
                    if n(value[own])?.compare(&n(value[o])?) == Ordering::Less
                        || cmp_zero(threat[o])? == Ordering::Greater
                    {
                        outer.advance(world)?;
                        continue 'outer;
                    }
                }
                inner.advance(world)?;
            }
            return Ok(Some(kingdom));
        }
        outer.advance(world)?;
    }
    Ok(None)
}

/// Native `5647A0` on a City component.
pub fn tick(world: &mut impl World, p: Precision) -> Result<()> {
    let owner = world.city_owner()?;
    world.set_under_siege(false)?;
    let mut result = owner;
    let decide_zone = match owner {
        None => true,
        Some(_) => 'held: {
            let mut node = world.first_candidate()?;
            if node.is_none() {
                // 564802 -> 5648CF
                world.set_under_siege(true)?;
                break 'held false;
            }
            let (mut defend, mut ally, mut attack) = (F32::ZERO, F32::ZERO, F32::ZERO);
            while let Some(at) = node {
                if let Some(object) = world.resolve_node(at)? {
                    if world.virtual_138(object, owner)? {
                        let v = world.combat_value(object)?;
                        ally = accumulate(&v, ally, p)?;
                        if world.virtual_134(object, owner)? {
                            let v = world.combat_value(object)?;
                            defend = accumulate(&v, defend, p)?;
                        }
                    } else if world.virtual_140(object, owner)? {
                        let v = world.combat_value(object)?;
                        attack = accumulate(&v, attack, p)?;
                    }
                }
                node = world.next_node(at)?;
            }
            // 56489E: attack above zero and defence not above zero decide.
            if cmp_zero(attack)? == Ordering::Greater && cmp_zero(defend)? != Ordering::Greater {
                break 'held true;
            }
            // 5648C0: under siege unless the attack is below the allied sum.
            if n(attack)?.compare(&n(ally)?) != Ordering::Less {
                world.set_under_siege(true)?;
            }
            false
        }
    };
    if decide_zone && let Some(winner) = decide(world, p)? {
        result = Some(winner);
    }
    if result != world.city_owner()? {
        world.capture(result, &capture_record())?;
    }
    Ok(())
}

/// The services of `5643A0`.
pub trait BlocksWorld {
    /// City `+10` head node (each node: `+0` GTID, `+4` next).
    fn first_block(&mut self) -> Result<Option<u32>>;
    /// Node `+4`.
    fn next_node(&mut self, node: u32) -> Result<Option<u32>>;
    /// Node `+0` GTID through `[B364C0]` (generation word and live entry).
    fn resolve_node(&mut self, node: u32) -> Result<Option<u32>>;
    /// The block's virtual `+160(kingdom)` (owner change, `4D8450` for an
    /// actor); bind [`crate::ownership::change_kingdom`].
    fn change_kingdom(&mut self, block: u32, kingdom: Option<u32>) -> Result<()>;
}

/// Native `5643A0(kingdom)` on a City component: every live block of City
/// `+10`, in list order, changes owner.
pub fn blocks_change_kingdom(world: &mut impl BlocksWorld, kingdom: Option<u32>) -> Result<()> {
    let mut node = world.first_block()?;
    while let Some(at) = node {
        if let Some(block) = world.resolve_node(at)? {
            world.change_kingdom(block, kingdom)?;
        }
        node = world.next_node(at)?;
    }
    Ok(())
}
