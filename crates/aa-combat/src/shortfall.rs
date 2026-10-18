//! Native resource-shortfall attrition: OrganizationComponent `578660`
//! (TickShortfallDamage) and Body `562D10`. The organization tick (`57C8E0`)
//! calls this while its kingdom is in shortfall; that gate is the caller's.
//! Each living member Body receives one authoritative RNG draw (`4F02B0` +
//! `85D840`) and a DealDamage (`5625C0`) request with event kind 4.
use crate::Result;
use aa_body::{
    F32,
    damage::{DamageRecord, DamageRequest},
};
use aa_numeric::{Binary, Precision};

/// Member Body fields read by `562D10`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemberBody {
    /// Body `+4` owner actor GTID (`owner+14`). The native routine then calls
    /// the owner's allegiance virtual unconditionally, so an owner is required.
    pub owner_id: u32,
    /// Body `+14` maximum health.
    pub maximum: F32,
}

pub trait World {
    /// Organization `+2C`, reread before every iteration.
    fn member_count(&self) -> Result<i32>;
    /// Organization `+28[index]`, reread each iteration.
    fn member(&self, index: i32) -> Result<Option<u32>>;
    /// Member actor `+60` Body.
    fn body(&self, member: u32) -> Result<Option<MemberBody>>;
    /// `4F02B0` world RNG plus one `85D840` draw (authoritative sequence).
    fn draw_random(&mut self) -> Result<u32>;
    /// Owner virtual `+108` allegiance, then its `+14` GTID (0 when absent).
    fn allegiance_id(&mut self, member: u32) -> Result<u32>;
    /// World `+E0`.
    fn now(&self) -> Result<F32>;
    /// `5625C0` on the member's Body (aa_body::damage::deal_damage).
    fn deal_damage(&mut self, member: u32, request: DamageRequest, p: Precision) -> Result<()>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Native `562D10(delta, rate)` on `member`'s Body. The draw is converted as
/// unsigned (`+2^32` when negative), scaled by binary64 `1/(2^32-1)`, shifted
/// by `-0.25`, and applied as `f*r + f` with `f = f32(delta*maximum*rate)`.
pub fn damage(
    member: u32,
    body: MemberBody,
    delta: F32,
    rate: F32,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    let base = F32(n(delta)?
        .mul(&n(body.maximum)?, p)?
        .mul(&n(rate)?, p)?
        .store_f32()?);
    let draw = world.draw_random()? as i32;
    let mut unit = Binary::from_i32(draw);
    if draw < 0 {
        unit = unit.add(&Binary::from_f64_bits(0x41f0_0000_0000_0000)?, p)?;
    }
    let unit = unit
        .mul(&Binary::from_f64_bits(0x3df0_0000_0010_0000)?, p)?
        .sub(&n(F32(0x3e80_0000))?, p)?;
    let amount = F32(unit.mul(&n(base)?, p)?.add(&n(base)?, p)?.store_f32()?);
    let allegiance_id = world.allegiance_id(member)?;
    let record = DamageRecord {
        source_id: body.owner_id,
        allegiance_id,
        damage_type: 0,
        amount,
        time: world.now()?,
        event_kind: 4,
        reason: 10,
    };
    world.deal_damage(
        member,
        DamageRequest {
            record,
            apply_type_modifier: false,
            weapon_id: None,
            weapon_definition: None,
            propagate_to_neighbors: true,
        },
        p,
    )
}

/// Native `578660(delta)`. `rate` is the owner definition `+2F8`
/// (`unit_shortfall_damage_rate`), read once before the member loop.
pub fn tick(delta: F32, rate: F32, world: &mut impl World, p: Precision) -> Result<()> {
    let mut index = 0;
    while index < world.member_count()? {
        if let Some(member) = world.member(index)?
            && let Some(body) = world.body(member)?
        {
            damage(member, body, delta, rate, world, p)?;
        }
        index += 1;
    }
    Ok(())
}
