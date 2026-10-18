//! Borrowed, progressively mutating `0x4BBF90` mean-value producer.
//!
//! The source owner must supply the manager's native-kind-2 linked list after
//! its preceding completion passes, and getters on those same shared objects.
//! No source-load precision, profile, completed definition, or mean is invented
//! here. This finite, round-to-nearest port excludes native allocator/SEH
//! failure behavior and floating-point exception flags. Storage grows with the
//! input; the native initial allocation count is not a gameplay capacity.
//! Both trimmed ranges must be nonempty, the terminal offense divisor nonzero,
//! and every reached numeric operation/store inside `aa_numeric`'s finite
//! domain. Unsupported native Inf/NaN, overflow and unsafe-index paths stop
//! with their already completed shared writes retained.
use crate::{Error, Result};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// Shared manager field offsets, read again at each native memory operand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Field {
    Maximum = 0x74,
    Attack = 0x78,
    AttackRate = 0x7c,
    Defense = 0x80,
    Movement = 0x84,
    Health = 0x88,
    Lifetime = 0x8c,
    Offense = 0x90,
}

/// Required operations on one shared source graph, not independent snapshots.
/// Handles identify native list links, definitions, and local abilities in the
/// implementing owner. A failed required operation returns immediately without
/// rolling back earlier manager stores or callback effects.
pub trait World {
    /// `[[manager+68]+8]`; called separately for counting and processing.
    fn first_link(&mut self) -> Result<Option<u32>>;
    /// `[link+8]`, read after processing the current definition.
    fn next_link(&mut self, link: u32) -> Result<Option<u32>>;
    /// `[link]`.
    fn link_definition(&mut self, link: u32) -> Result<u32>;
    fn flags_174(&mut self, definition: u32) -> Result<u32>;
    /// `0x45E6D0`, only reached for Settlement bit 14. The second call on a
    /// new maximum is independent and may observe mutations from the first.
    fn extent(&mut self, definition: u32) -> Result<Binary>;
    /// Signed `[definition+1F0]`, captured once for that definition.
    fn ability_count(&mut self, definition: u32) -> Result<i32>;
    /// Reread `[definition+1EC]`, then its indexed local-ability pointer.
    fn ability_at(&mut self, definition: u32, index: i32) -> Result<u32>;
    /// `*local`, captured before the power gate and used by `power`.
    fn ability_definition(&mut self, local: u32) -> Result<u32>;
    fn base_power_64(&mut self, ability_definition: u32) -> Result<F32>;
    /// `0x4BE3F0(definition, modifier=0)`.
    fn power(&mut self, ability_definition: u32) -> Result<Binary>;
    /// Captured before `first_animation_length` is called.
    fn reload_04(&mut self, local: u32) -> Result<F32>;
    /// `0x4BE680(local)`.
    fn first_animation_length(&mut self, local: u32) -> Result<Binary>;
    /// `0x4BE4B0(local, modifier=0)`; called twice when its first result is
    /// at least the stored animation-plus-reload floor.
    fn recharge(&mut self, local: u32) -> Result<Binary>;
    fn defense(&mut self, definition: u32) -> Result<Binary>;
    fn movement(&mut self, definition: u32) -> Result<Binary>;
    fn health(&mut self, definition: u32) -> Result<Binary>;
    /// Actual typed SVar 87, `[SettingsTable+15C]`; read ONCE and retained on
    /// the native x87 stack across both trimmed-count calculations.
    fn discard_fraction(&mut self) -> Result<F32>;
    fn read_mean(&mut self, field: Field) -> Result<F32>;
    fn write_mean(&mut self, field: Field, value: F32) -> Result<()>;
    /// `[0xB3B4F8]`, read before the last three scaling stores.
    fn assertion_guard(&mut self) -> Result<u8>;
    /// `0x855D30`, expression `m_mean_attack_value > m_mean_defense_value`,
    /// `.\Source\Data\ThingData.cpp`, line 0x24B. Return native AL.
    fn assertion(&mut self) -> Result<u8>;
    fn write_assertion_guard(&mut self, value: u8) -> Result<()>;
    /// `0x404F60` with the currently loaded field widened exactly to binary64.
    /// Later log calls reread their fields after this callback returns.
    fn log_mean(&mut self, field: Field, line: u32, value_binary64: u64) -> Result<()>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}
fn stored(value: &Binary) -> Result<F32> {
    Ok(F32(value.store_f32()?))
}
fn insert(column: &mut Vec<F32>, value: F32) -> Result<()> {
    let number = n(value)?;
    let (mut low, mut high) = (0, column.len());
    while low < high {
        let middle = (low + high) / 2;
        if number.compare(&n(column[middle])?) == Ordering::Greater {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    column.insert(low, value);
    Ok(())
}
fn trim(count: usize, discard: &Binary, precision: Precision) -> Result<(usize, usize)> {
    let count =
        i32::try_from(count).map_err(|_| Error::Invalid("mean column count exceeds int32"))?;
    let cut = Binary::from_i32(count)
        .mul(discard, precision)?
        .truncate_i32()?;
    if cut < 0 {
        return Err(Error::Invalid(
            "negative mean discard would read outside the native column",
        ));
    }
    let end = count
        .checked_sub(cut)
        .ok_or(Error::Invalid("mean trim count overflow"))?;
    // Native skips its sum loop for end <= cut, then divides by a nonpositive
    // count. This bounded finite port refuses before that unsupported scaling.
    if end <= cut {
        return Err(Error::Invalid("mean trimmed range is empty or negative"));
    }
    Ok((cut as usize, end as usize))
}
fn sum(world: &mut impl World, field: Field, value: F32, precision: Precision) -> Result<()> {
    let old = world.read_mean(field)?;
    world.write_mean(field, stored(&n(value)?.add(&n(old)?, precision)?)?)
}
fn scale(
    world: &mut impl World,
    field: Field,
    inverse: &Binary,
    precision: Precision,
) -> Result<F32> {
    let old = world.read_mean(field)?;
    let value = stored(&inverse.mul(&n(old)?, precision)?)?;
    world.write_mean(field, value)?;
    Ok(value)
}

/// Execute native `0x4BBF90` on the supplied shared owner and explicit x87 PC.
/// Successful return certifies this bounded producer only. The required prior
/// source completion passes and later `0x4BB590` ActorData+1E4 computation are
/// separate operations; this function supplies neither receipt implicitly.
pub fn compute(world: &mut impl World, precision: Precision) -> Result<()> {
    let mut link = world.first_link()?;
    let mut count = 0_i32;
    while let Some(current) = link {
        link = world.next_link(current)?;
        count = count
            .checked_add(1)
            .ok_or(Error::Invalid("native kind-2 list count overflow"))?;
    }
    // Native allocates five arrays here. Rust ownership/allocation policy is
    // deliberately outside the native allocator/SEH proof; no fixed cap is used.
    let (mut attack, mut rate, mut defense, mut movement, mut health) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for field in [
        Field::Maximum,
        Field::Attack,
        Field::AttackRate,
        Field::Defense,
        Field::Movement,
        Field::Health,
    ] {
        world.write_mean(field, F32::ZERO)?;
    }
    link = world.first_link()?;
    while let Some(current) = link {
        let definition = world.link_definition(current)?;
        if world.flags_174(definition)? & (1 << 14) != 0 {
            let extent = world.extent(definition)?;
            if extent.compare(&n(world.read_mean(Field::Maximum)?)?) == Ordering::Greater {
                let extent = world.extent(definition)?;
                world.write_mean(Field::Maximum, stored(&extent)?)?;
            }
        }
        let flags = world.flags_174(definition)?;
        if flags & 0x29 == 0x29 && (flags & (1 << 17) == 0 || flags & (1 << 8) == 0) {
            let ability_count = world.ability_count(definition)?;
            for index in 0..ability_count {
                let local = world.ability_at(definition, index)?;
                let ability = world.ability_definition(local)?;
                if n(world.base_power_64(ability)?)?.compare(&Binary::from_i32(0))
                    == Ordering::Greater
                {
                    insert(&mut attack, stored(&world.power(ability)?)?)?;
                    let reload = world.reload_04(local)?;
                    let mut cycle = stored(
                        &world
                            .first_animation_length(local)?
                            .add(&n(reload)?, precision)?,
                    )?;
                    if world.recharge(local)?.compare(&n(cycle)?) != Ordering::Less {
                        cycle = stored(&world.recharge(local)?)?;
                    }
                    insert(&mut rate, cycle)?;
                }
            }
            insert(&mut defense, stored(&world.defense(definition)?)?)?;
            insert(&mut movement, stored(&world.movement(definition)?)?)?;
            insert(&mut health, stored(&world.health(definition)?)?)?;
        }
        link = world.next_link(current)?;
    }
    let discard = n(world.discard_fraction()?)?;
    let (start, end) = trim(attack.len(), &discard, precision)?;
    for index in start..end {
        sum(world, Field::Attack, attack[index], precision)?;
        sum(world, Field::AttackRate, rate[index], precision)?;
    }
    let inverse = Binary::from_i32(1).div(&Binary::from_i32((end - start) as i32), precision)?;
    scale(world, Field::Attack, &inverse, precision)?;
    scale(world, Field::AttackRate, &inverse, precision)?;
    let (start, end) = trim(defense.len(), &discard, precision)?;
    for index in start..end {
        sum(world, Field::Defense, defense[index], precision)?;
        sum(world, Field::Movement, movement[index], precision)?;
        sum(world, Field::Health, health[index], precision)?;
    }
    let guard = world.assertion_guard()?;
    let inverse = Binary::from_i32(1).div(&Binary::from_i32((end - start) as i32), precision)?;
    let defense = scale(world, Field::Defense, &inverse, precision)?;
    scale(world, Field::Movement, &inverse, precision)?;
    scale(world, Field::Health, &inverse, precision)?;
    if guard == 0 && n(world.read_mean(Field::Attack)?)?.compare(&n(defense)?) != Ordering::Greater
    {
        let result = world.assertion()?;
        world.write_assertion_guard(result)?;
    }
    let attack = n(world.read_mean(Field::Attack)?)?;
    let offense = attack.sub(&n(world.read_mean(Field::Defense)?)?, precision)?;
    let lifetime = n(world.read_mean(Field::Health)?)?.div(&offense, precision)?;
    world.write_mean(Field::Lifetime, stored(&lifetime)?)?;
    world.write_mean(Field::Offense, stored(&offense)?)?;
    for (field, line) in [
        (Field::Attack, 0x253),
        (Field::AttackRate, 0x254),
        (Field::Defense, 0x255),
        (Field::Movement, 0x256),
        (Field::Health, 0x257),
        (Field::Offense, 0x258),
        (Field::Lifetime, 0x259),
    ] {
        let value = n(world.read_mean(field)?)?;
        world.log_mean(field, line, aa_numeric::crt_pow::store_binary64(&value)?)?;
    }
    Ok(())
}
