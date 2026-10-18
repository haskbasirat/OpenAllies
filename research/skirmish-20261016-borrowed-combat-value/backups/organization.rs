//! Native OrganizationComponent maintenance tick `57C8E0` and its helpers:
//! `4D7DA0` (state 40 `targeted`), `5788B0`/`57DC40` (current and maximum
//! combat value), `579110` (regeneration gate), `4D7CE0` (state 19
//! `out_of_supply` sync), `57C800` (follow the leader, with `578E90` and the
//! `4515F0` XY equality), `5147C0` (marker) and `58B150` (watched target
//! finished). The proven sub-steps (morale tick `57A0A0`, engagement
//! experience `579600`, attack efficiency `5785D0`, shortfall `578660`) are
//! world callbacks that the runtime binds to `morale::tick`,
//! `morale::engagement_experience`, `morale::attack_efficiency` and
//! `shortfall::tick`; this module fixes their order and gates.
use crate::{
    Error, Result,
    morale::{Actor, Mind, Organization, StateDescriptor},
};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A member's combat component (`member+74`): its `+60` value and the owner
/// Body's `+10/+14` health and maximum when the owner has a Body.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemberCombat {
    pub value: F32,
    pub health: Option<[F32; 2]>,
}

/// A vacant slot's definition (`org+1C[definition+28[i]]`): `+174` flags and
/// `+1E4` combat value (counted in the maximum when flag bit5 is set).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotDefinition {
    pub flags174: u32,
    pub value1e4: F32,
}

/// Owner `+148/+14C`: the object count and the `+10` entry with its `+8` word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnerObjects {
    pub count: i32,
    pub object10: Option<(u32, F32)>,
}

pub trait World {
    fn session_mode(&self) -> Result<u32>;
    fn actor(&self, actor: u32) -> Result<Actor>;
    fn mind(&self, actor: u32) -> Result<Option<Mind>>;
    fn position(&self, actor: u32) -> Result<[F32; 2]>;
    fn now(&self) -> Result<F32>;
    /// `4D64F0(owner, state, enabled)`.
    fn set_state(&mut self, o: &mut Organization, state: u8, enabled: bool) -> Result<()>;
    /// `4D49B0(owner, group)`.
    fn state_group(&mut self, o: &mut Organization, group: u32) -> Result<Option<StateDescriptor>>;
    /// `4D1350` actor notification.
    fn notify(&mut self, o: &mut Organization, actor: u32) -> Result<()>;
    /// Proven sub-steps, in native order.
    fn morale_tick(&mut self, o: &mut Organization, delta: F32) -> Result<()>;
    fn engagement_experience(&mut self, o: &mut Organization, delta: F32) -> Result<()>;
    fn attack_efficiency(&mut self, o: &mut Organization, delta: F32) -> Result<()>;
    fn shortfall(&mut self, o: &mut Organization, delta: F32) -> Result<()>;
    /// `57B5A0(delta)` regeneration: bind [`crate::regeneration::regenerate`].
    fn regenerate(&mut self, o: &mut Organization, delta: F32) -> Result<()>;
    /// `57AEB0` regroup attempt; true when it succeeded: bind
    /// [`crate::regeneration::regroup`].
    fn regroup_check(&mut self, o: &mut Organization) -> Result<bool>;
    /// Organization.cpp line 0x586 "Company unable to regroup" diagnostic.
    fn unable_to_regroup(&mut self, o: &mut Organization) -> Result<()>;
    /// Mind virtuals `+64`, `+7C`, `+150`.
    fn mind_busy64(&mut self, o: &mut Organization) -> Result<bool>;
    fn mind_busy7c(&mut self, o: &mut Organization) -> Result<bool>;
    fn mind_regroup(&mut self, o: &mut Organization) -> Result<()>;
    /// Organization members `+28/+2C` (count reread where native rereads).
    fn member_count(&self, o: &Organization) -> Result<i32>;
    fn member(&self, o: &Organization, index: i32) -> Result<Option<u32>>;
    fn member_combat(&self, member: u32) -> Result<Option<MemberCombat>>;
    fn slot_definition(&self, o: &Organization, index: i32) -> Result<Option<SlotDefinition>>;
    /// Owner definition `+2F4` byte (supply/shortfall rules enabled).
    fn definition_flag2f4(&self, owner: u32) -> Result<bool>;
    /// `4D0A80` in-supply predicate.
    fn in_supply(&mut self, o: &mut Organization) -> Result<bool>;
    /// Owner virtual `+108` allegiance `+1F0` shortage ratio; None is the
    /// native null allegiance (dereferenced, so an error at the caller).
    fn allegiance_shortage(&mut self, o: &mut Organization) -> Result<Option<F32>>;
    fn owner_objects(&self, owner: u32) -> Result<OwnerObjects>;
    /// `515F40(target, position, word)`: `KKC_Zone::Move` of the owner's zone
    /// object by world position (`552640` clamp, cell shift, `515970`), owned
    /// by the zone/detection solver (Sim/Zones.cpp), not combat.
    fn flare(
        &mut self,
        o: &mut Organization,
        target: Option<u32>,
        position: [F32; 2],
        word: u32,
    ) -> Result<()>;
    /// `540690(object)` on world `+244`.
    fn marker(&mut self, o: &mut Organization, object: u32) -> Result<()>;
    /// `57C680(position)`: move the organization toward its leader.
    fn move_to_leader(&mut self, o: &mut Organization, position: [F32; 2]) -> Result<()>;
    /// `45D520(owner definition, 4CF110 modifier)` entrenchment delay; bind
    /// [`entrench_delay`].
    fn entrench_time(&mut self, o: &mut Organization) -> Result<Binary>;
    /// Registry lookup of `+D4`; `(destructed, finished)` of the object where
    /// finished is `58B150` on its `+C0` component (`+40 == +3C`).
    fn watched_target(&self, gtid: u32) -> Result<Option<(bool, bool)>>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Native `4D7DA0`: state 40 is set exactly when the owner has an
/// OrganizationComponent, `+100` bit4 is clear and `+118` is nonnull.
/// Native `0x45D520(definition; modifier)`: definition `+2EC`, divided
/// (extended, `0x5014D0`) by the modifier's `+40` scalar when a modifier
/// context exists.
pub fn entrench_delay(
    base_2ec: F32,
    modifier: Option<&aa_body::properties::ModifierState>,
    p: Precision,
) -> Result<Binary> {
    match modifier {
        None => n(base_2ec),
        Some(m) => m
            .divide(0x40, base_2ec, p)
            .map_err(|e| crate::Error::World(e.to_string())),
    }
}
pub fn targeted_state(o: &mut Organization, world: &mut impl World) -> Result<()> {
    let a = world.actor(o.owner)?;
    let targeted = a.has_organization && a.flags100 & 0x10 == 0 && a.engaged;
    world.set_state(o, 40, targeted)
}

/// Native `5788B0`: `+68` = sum of health-ratio-weighted member values,
/// `+6C` = sum of member values plus flagged vacant-slot values; each add is
/// stored to binary32 in member order.
pub fn combat_value(o: &mut Organization, world: &impl World, p: Precision) -> Result<()> {
    o.current_combat_value = F32::ZERO;
    o.maximum_combat_value = F32::ZERO;
    let mut index = 0;
    while index < world.member_count(o)? {
        if let Some(member) = world.member(o, index)? {
            if let Some(combat) = world.member_combat(member)? {
                let ratio = match combat.health {
                    Some([health, maximum]) => n(health)?.div(&n(maximum)?, p)?,
                    None => Binary::from_u32(1),
                };
                o.current_combat_value = F32(ratio
                    .mul(&n(combat.value)?, p)?
                    .add(&n(o.current_combat_value)?, p)?
                    .store_f32()?);
                let value = world
                    .member_combat(member)?
                    .ok_or(Error::Invalid("member combat component vanished"))?
                    .value;
                o.maximum_combat_value =
                    F32(n(value)?.add(&n(o.maximum_combat_value)?, p)?.store_f32()?);
            }
        } else if let Some(slot) = world.slot_definition(o, index)?
            && (slot.flags174 >> 5) & 1 != 0
        {
            o.maximum_combat_value = F32(n(slot.value1e4)?
                .add(&n(o.maximum_combat_value)?, p)?
                .store_f32()?);
        }
        index += 1;
    }
    Ok(())
}

/// Native `579110`.
pub fn regeneration_gate(o: &mut Organization, world: &mut impl World) -> Result<bool> {
    let Some(descriptor) = world.state_group(o, 4)? else {
        return Ok(false);
    };
    if world.actor(o.owner)?.engaged {
        return Ok(false);
    }
    Ok(world.definition_flag2f4(o.owner)?
        && world.actor(o.owner)?.flags100 & 0x80000 == 0
        && descriptor.byte1c)
}

/// Native `4D7CE0`: when the in-supply predicate equals the owner's `+100`
/// bit19, set state 19 to its negation and notify. Returns whether it did.
pub fn supply_state(o: &mut Organization, world: &mut impl World) -> Result<bool> {
    let in_supply = world.in_supply(o)?;
    let flag = world.actor(o.owner)?.flags100 & 0x80000 != 0;
    if in_supply != flag {
        return Ok(false);
    }
    world.set_state(o, 19, !in_supply)?;
    let owner = o.owner;
    world.notify(o, owner)?;
    Ok(true)
}

fn mind_free(o: &mut Organization, world: &mut impl World) -> Result<bool> {
    if world.mind(o.owner)?.is_none() {
        return Ok(true);
    }
    if world.mind_busy64(o)? {
        return Ok(false);
    }
    Ok(!world.mind_busy7c(o)?)
}

/// Native `57C800(leader)`.
pub fn follow_leader(o: &mut Organization, leader: u32, world: &mut impl World) -> Result<()> {
    if world.session_mode()? == 6 || Some(leader) != o.leader {
        return Ok(());
    }
    let both_bit30 = world.actor(o.owner)?.flags8 & 0x4000_0000 != 0
        && world.actor(leader)?.flags8 & 0x4000_0000 != 0;
    if !both_bit30 {
        if mind_free(o, world)? {
            return Ok(());
        }
        let owner = world.position(o.owner)?;
        let target = world.position(leader)?;
        if n(owner[0])?.compare(&n(target[0])?) == Ordering::Equal
            && n(owner[1])?.compare(&n(target[1])?) == Ordering::Equal
        {
            return Ok(());
        }
    }
    let position = world.position(leader)?;
    world.move_to_leader(o, position)
}

/// Native `57C8E0(delta)`. `regroup_time` is settings-cache `+BC`.
pub fn tick_maintenance(
    o: &mut Organization,
    delta: F32,
    regroup_time: F32,
    world: &mut impl World,
    p: Precision,
) -> Result<()> {
    targeted_state(o, world)?;
    if o.leader.is_none() {
        return Ok(());
    }
    world.morale_tick(o, delta)?;
    world.engagement_experience(o, delta)?;
    world.attack_efficiency(o, delta)?;
    combat_value(o, world, p)?;
    let owner = o.owner;
    if o.flare_pending {
        let objects = world.owner_objects(owner)?;
        let target = if objects.count > 4 {
            objects.object10.map(|(object, _)| object)
        } else {
            None
        };
        let position = world.position(owner)?;
        let word = o.flare_word;
        world.flare(o, target, position, word)?;
        o.flare_pending = false;
    }
    if world.mind(owner)?.is_some() && (world.mind_busy64(o)? || world.mind_busy7c(o)?) {
        let leader = o.leader.ok_or(Error::Invalid("leader vanished"))?;
        follow_leader(o, leader, world)?;
    }
    if world.definition_flag2f4(owner)? {
        if supply_state(o, world)? {
            let count = world.member_count(o)?;
            for index in 0..count.max(0) {
                if let Some(member) = world.member(o, index)? {
                    world.notify(o, member)?;
                }
            }
        }
        if world.actor(owner)?.flags100 & 0x80000 != 0 {
            let ratio = world.allegiance_shortage(o)?.ok_or(Error::Invalid(
                "shortfall check reached a null allegiance (native null dereference)",
            ))?;
            if n(ratio)?.compare(&Binary::from_u32(0)) == Ordering::Greater {
                world.shortfall(o, delta)?;
            }
        }
    }
    if regeneration_gate(o, world)? {
        world.regenerate(o, delta)?;
    }
    if world.actor(owner)?.flags100 & 0x400 != 0 {
        if o.primary.is_some() && world.mind(owner)?.is_some() {
            world.mind_regroup(o)?;
        } else if n(world.now()?)?
            .sub(&n(o.regroup_started)?, p)?
            .compare(&n(regroup_time)?)
            != Ordering::Less
            && !world.regroup_check(o)?
        {
            world.unable_to_regroup(o)?;
        }
    }
    if o.marker_pending {
        o.marker_pending = false;
        let objects = world.owner_objects(owner)?;
        if objects.count > 4
            && let Some((object, word)) = objects.object10
            && word.0 != 0xbf80_0000
        {
            world.marker(o, object)?;
        }
    }
    if world.actor(owner)?.flags100 & 0x40_0000 == 0 {
        let descriptor = world.state_group(o, 4)?.ok_or(Error::Invalid(
            "entrenchment check reached a null state descriptor (native null dereference)",
        ))?;
        let primary_ok = match o.primary {
            None => true,
            Some(primary) => world.actor(primary)?.flags8 & 0x2000_0000 == 0,
        };
        if descriptor.byte1e && primary_ok {
            let ready = world.entrench_time(o)?.add(&n(o.entrench_base)?, p)?;
            if ready.compare(&n(world.now()?)?) != Ordering::Greater {
                world.set_state(o, 22, true)?;
            }
        }
    }
    if o.watched_target != 0 {
        match world.watched_target(o.watched_target)? {
            None | Some((true, _)) | Some((false, true)) => o.watched_target = 0,
            Some((false, false)) => {}
        }
    }
    world.notify(o, owner)
}

/// One owner property entry (`owner+78` list `+10/+14`): `+8` kind, `+C`
/// property definition, `+10` disabled byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyEntry {
    pub kind: u32,
    pub property: u32,
    pub disabled: bool,
}

/// Member-removal services beyond the morale ones (`morale::World`).
pub trait RemovalWorld: crate::morale::World {
    /// `org+34[slot]` record whose `+C` GTID resolves; then `4E1270(record)`.
    fn slot_record_live(&self, o: &Organization, slot: i32) -> Result<bool>;
    fn recruit_release(&mut self, o: &mut Organization, slot: i32) -> Result<()>;
    /// Member definition `+2D8` property list values (`45EC60`), in node order.
    fn member_definition_properties(&self, member: u32) -> Result<Vec<u32>>;
    /// `4CEF70(actor; property, source)`, i.e. `581810` on `actor+78`.
    fn remove_property(
        &mut self,
        o: &mut Organization,
        actor: u32,
        property: u32,
        source: u32,
    ) -> Result<()>;
    /// Owner property entries; the count is reread every iteration.
    fn owner_property_count(&self, owner: u32) -> Result<i32>;
    fn owner_property(&self, owner: u32, index: i32) -> Result<PropertyEntry>;
    /// Member `+78` present, and member `+A4` nonnull.
    fn member_has_properties(&self, member: u32) -> Result<bool>;
    fn member_a4(&self, member: u32) -> Result<bool>;
    /// `570670(owner, slot)` on the member membership component; bind
    /// [`detach_membership`].
    fn detach(&mut self, o: &mut Organization, member: u32, slot: i32) -> Result<()>;
    /// `org+28[slot] = 0` and `org+50[slot] = 0`.
    fn clear_slot(&mut self, o: &mut Organization, slot: i32) -> Result<()>;
    /// Member definition `+2C0` (`45D580(0)`).
    fn morale_value(&self, member: u32) -> Result<F32>;
    /// Organization definition slot entry byte `+E` (`def+1C[def+28[slot]]`).
    fn living_slot(&self, o: &Organization, slot: i32) -> Result<bool>;
    /// Member `+6C` component `+28`.
    fn member_value6c(&self, member: u32) -> Result<Option<F32>>;
    /// `57A640`: bind [`recompute_speed`].
    fn recompute_6c(&mut self, o: &mut Organization) -> Result<()>;
    /// Member `+100` and `+104` (for `4D1050` and bit25).
    fn member_flags(&self, member: u32) -> Result<[u32; 2]>;
    fn member_flags8(&self, member: u32) -> Result<u32>;
    /// Mind virtuals `+110` (primary lost) and `+114` (leader changed).
    fn mind_primary_lost(&mut self, o: &mut Organization) -> Result<()>;
    fn mind_leader_changed(&mut self, o: &mut Organization) -> Result<()>;
    /// Members `+28/+2C` and the membership attached byte, for `4364C0`.
    fn member_count(&self, o: &Organization) -> Result<i32>;
    fn member(&self, o: &Organization, index: i32) -> Result<Option<u32>>;
    fn member_attached(&self, member: u32) -> Result<bool>;
    /// Membership `+18` slot of a member.
    fn member_slot(&self, member: u32) -> Result<i32>;
    /// `5797E0(slot)`.
    fn slot_refresh(&mut self, o: &mut Organization, slot: i32) -> Result<()>;
    /// [`follow_leader`] (`57C800`), proven by the organization-tick fixtures.
    fn follow_leader(&mut self, o: &mut Organization, leader: u32) -> Result<()>;
    /// `4D4CF0(owner)`, `4CF0F0(owner)` and `4D1350(owner)`.
    fn update_4cf0(&mut self, o: &mut Organization) -> Result<()>;
    fn update_0f0(&mut self, o: &mut Organization) -> Result<()>;
    fn notify_owner(&mut self, o: &mut Organization) -> Result<()>;
    /// Organization definition `+18` (signed).
    fn definition18(&self, o: &Organization) -> Result<i32>;
    /// Owner removal leaves: `4F5490` destroy, `4D7960` BeginDeath and
    /// `4F6760(owner, 0)` (`aa_world::World::request_removal`). BeginDeath
    /// binds [`crate::begin_death::begin_death`].
    fn destroy_owner(&mut self, o: &mut Organization) -> Result<()>;
    fn begin_owner_death(&mut self, o: &mut Organization) -> Result<()>;
    fn request_owner_removal(&mut self, o: &mut Organization) -> Result<()>;
}

fn removal_blocked(world: &impl RemovalWorld, member: u32) -> Result<bool> {
    let [flags100, flags104] = world.member_flags(member)?;
    // 4D1050 (+100 bit26 or +104 bit4), then +100 bit25.
    Ok(flags100 & 0x400_0000 != 0 || flags104 & 0x10 != 0 || flags100 & 0x200_0000 != 0)
}

/// Native `57CD70(slot, member)`: detach properties and membership, apply
/// the MoraleDeathFactor loss, reselect primary/leader (first attached, else
/// first detached member), refresh state 37 (`missing_primary`) and remove
/// the organization owner when no leader remains. `death_factor` is
/// settings-cache `+D4`.
pub fn remove_member<W: RemovalWorld>(
    o: &mut Organization,
    slot: i32,
    member: Option<u32>,
    death_factor: F32,
    world: &mut W,
    p: Precision,
) -> Result<()> {
    use crate::morale::World as Morale;
    let Some(member) = member else {
        return Ok(());
    };
    let owner = o.owner;
    if world.slot_record_live(o, slot)? {
        world.recruit_release(o, slot)?;
    }
    for property in world.member_definition_properties(member)? {
        world.remove_property(o, owner, property, member)?;
    }
    let mut index = 0;
    while index < world.owner_property_count(owner)? {
        let entry = world.owner_property(owner, index)?;
        if world.member_has_properties(member)?
            && (!world.member_a4(member)? || entry.kind != 7)
            && !entry.disabled
        {
            world.remove_property(o, member, entry.property, owner)?;
        }
        index += 1;
    }
    world.detach(o, member, slot)?;
    world.clear_slot(o, slot)?;
    if Morale::actor(world, owner)?.flags8 & 1 != 0 {
        return Ok(());
    }
    let loss = F32(n(world.morale_value(member)?)?
        .mul(&n(death_factor)?, p)?
        .store_f32()?);
    let actor = Morale::actor(world, owner)?;
    if Morale::session_mode(world)? != 6 && actor.flags8 & 1 == 0 && actor.flags104 & 4 == 0 {
        // 4CF110 is called here, but its result is not tested.
        let _ = Morale::modifier(world, o)?;
        let busy = Morale::mind(world, owner)?.is_some_and(|m| m.word28 != 0 || m.word2c != 0);
        if !busy {
            crate::morale::lose_amount(o, loss, world, p)?;
        }
    }
    if world.living_slot(o, slot)? {
        o.living = o.living.wrapping_sub(1);
    }
    if let Some(value) = world.member_value6c(member)?
        && n(value)?.compare(&n(o.field_d8)?) == Ordering::Equal
    {
        world.recompute_6c(o)?;
    }
    if o.primary == Some(member) {
        o.primary = None;
        if !removal_blocked(world, member)? && Morale::mind(world, owner)?.is_some() {
            world.mind_primary_lost(o)?;
        }
    }
    if o.leader == Some(member) {
        let (mut attached, mut detached) = (None, None);
        let mut index = 0;
        while index < world.member_count(o)? {
            if let Some(candidate) = world.member(o, index)? {
                if world.member_attached(candidate)? {
                    attached = attached.or(Some(candidate));
                } else {
                    detached = detached.or(Some(candidate));
                }
            }
            index += 1;
        }
        world.slot_refresh(o, slot)?;
        let old = o.leader;
        o.leader = attached.or(detached);
        if let Some(old) = old {
            let old_slot = world.member_slot(old)?;
            world.slot_refresh(o, old_slot)?;
        }
        if let Some(leader) = o.leader {
            let leader_slot = world.member_slot(leader)?;
            world.slot_refresh(o, leader_slot)?;
        }
        if Morale::mind(world, owner)?.is_some() {
            world.mind_leader_changed(o)?;
        }
        if let Some(leader) = o.leader {
            world.follow_leader(o, leader)?;
        }
    }
    world.update_4cf0(o)?;
    let missing_primary = o.leader.is_some() && o.primary.is_none() && world.definition18(o)? >= 0;
    Morale::set_state(world, o, 37, missing_primary)?;
    world.notify_owner(o)?;
    world.update_0f0(o)?;
    if !removal_blocked(world, member)?
        && o.leader.is_none()
        && Morale::actor(world, owner)?.flags8 & 1 == 0
    {
        let flags8 = world.member_flags8(member)?;
        if flags8 & 2 != 0 {
            world.destroy_owner(o)?;
        } else if flags8 & 0x10 != 0 {
            world.begin_owner_death(o)?;
        } else {
            world.request_owner_removal(o)?;
        }
    }
    Ok(())
}

/// `0x57A640` notification target: owner `+70` component virtual `+118`.
pub trait SpeedWorld {
    /// `owner+70 ->virtual+118(old, new)`; a no-op when the owner has no
    /// `+70` component.
    fn speed_changed(&mut self, o: &mut Organization, old: F32, new: F32) -> Result<()>;
}

/// `[0xA3CED4]` (1e-4f) change threshold of `0x57A640`.
pub const SPEED_EPSILON: F32 = F32(0x38d1_b717);

/// Native `0x57A640`: `+D8` becomes the minimum member `+6C` component `+28`
/// over the `0x4364C0` member iteration (all non-null slots, in order). No
/// members, a member without the component, or a minimum still equal to
/// `FLT_MAX` give 0. The field and the notification change only when
/// `|minimum - +D8|` is not below 1e-4.
pub fn recompute_speed(
    o: &mut Organization,
    members_6c: &[Option<F32>],
    world: &mut impl SpeedWorld,
    p: Precision,
) -> Result<()> {
    let mut minimum = F32(0x7f7f_ffff);
    let mut zero = members_6c.is_empty();
    for value in members_6c {
        match value {
            None => {
                zero = true;
                break;
            }
            Some(v) => {
                if n(*v)?.compare(&n(minimum)?) == Ordering::Less {
                    minimum = *v;
                }
            }
        }
    }
    if zero || minimum == F32(0x7f7f_ffff) {
        minimum = F32::ZERO;
    }
    let mut diff = n(minimum)?.sub(&n(o.field_d8)?, p)?;
    if diff.compare(&Binary::from_i32(0)) == Ordering::Less {
        diff = diff.negated();
    }
    if diff.compare(&n(SPEED_EPSILON)?) == Ordering::Less {
        return Ok(());
    }
    let old = std::mem::replace(&mut o.field_d8, minimum);
    world.speed_changed(o, old, minimum)
}

/// Callees of the membership detach `0x570670`.
pub trait DetachWorld {
    /// Member actor `+B0` component present.
    fn has_component_b0(&self, member: u32) -> Result<bool>;
    /// `0x567650` on that component.
    fn detach_component_b0(&mut self, member: u32) -> Result<()>;
    /// Membership `+10 = 0`, `+18 = -1`, `+1C = 0`.
    fn clear_membership(&mut self, member: u32) -> Result<()>;
    /// `0x4CF0F0(member)`.
    fn refresh_member(&mut self, member: u32) -> Result<()>;
    /// Global `[0xB36528]` present.
    fn observer_present(&self) -> Result<bool>;
    /// Member actor `+8` flags.
    fn member_flags8(&self, member: u32) -> Result<u32>;
    /// `0x7A0C20([0xB36528]; member)`.
    fn notify_observer(&mut self, member: u32) -> Result<()>;
}

/// Native `0x570670` (membership component of `member`; both stack
/// arguments unused): `+B0` detach, clear the membership, refresh, then
/// notify the global observer unless the member is dying.
pub fn detach_membership(member: u32, world: &mut impl DetachWorld) -> Result<()> {
    if world.has_component_b0(member)? {
        world.detach_component_b0(member)?;
    }
    world.clear_membership(member)?;
    world.refresh_member(member)?;
    if world.observer_present()? && world.member_flags8(member)? & 1 == 0 {
        world.notify_observer(member)?;
    }
    Ok(())
}

/// Callees of the membership attach `0x5705A0` (mirror of
/// [`detach_membership`]).
pub trait AttachWorld {
    /// Membership `+10` owner, `+18` slot and `+1C` attached byte (1).
    fn set_membership(&mut self, member: u32, owner: u32, slot: i32) -> Result<()>;
    /// `0x4D1350` on the member.
    fn notify_member(&mut self, member: u32) -> Result<()>;
    /// `0x4CF0F0` on the member.
    fn refresh_member(&mut self, member: u32) -> Result<()>;
    /// `[B36528]` present.
    fn observer_present(&self) -> Result<bool>;
    /// `0x7A0C20(member)`.
    fn notify_observer(&mut self, member: u32) -> Result<()>;
}

/// Native `0x5705A0(owner, slot)` on the member's membership component.
pub fn attach_membership(
    member: u32,
    owner: u32,
    slot: i32,
    world: &mut impl AttachWorld,
) -> Result<()> {
    world.set_membership(member, owner, slot)?;
    world.notify_member(member)?;
    world.refresh_member(member)?;
    if world.observer_present()? {
        world.notify_observer(member)?;
    }
    Ok(())
}

/// One owner property instance as read by `0x5784C0`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OwnerProperty {
    /// Instance `+C` definition.
    pub property: u32,
    /// Instance `+8` application kind.
    pub kind: u32,
    /// Instance `+10` disabled byte.
    pub disabled: bool,
    /// Definition `+42` byte (pass_down_members).
    pub pass_down_members: bool,
}

pub trait PropagationWorld {
    /// Owner `+78` Properties `+10/+14` instances; `None` without the component.
    fn owner_properties(&self, owner: u32) -> Result<Option<Vec<OwnerProperty>>>;
    /// Member `+78` Properties present.
    fn has_properties(&self, member: u32) -> Result<bool>;
    /// Member `+A4` component present.
    fn has_component_a4(&self, member: u32) -> Result<bool>;
    /// `0x4CEF10(member; property, kind, source)`
    /// (`aa_body::property_api::apply_property`).
    fn apply_property(&mut self, member: u32, property: u32, kind: u32, source: u32) -> Result<()>;
}

/// Native `0x5784C0(new member)` on an Organization: every enabled owner
/// property whose definition passes down to members is applied to a member
/// with Properties (kind 7 skipped for members with a `+A4` component),
/// with the owner as source. Mirror of the `0x578540` detach in
/// [`remove_member`].
pub fn propagate_properties(
    owner: u32,
    member: Option<u32>,
    world: &mut impl PropagationWorld,
) -> Result<()> {
    let Some(properties) = world.owner_properties(owner)? else {
        return Ok(());
    };
    for entry in properties {
        let Some(member) = member else {
            continue;
        };
        if world.has_properties(member)?
            && entry.pass_down_members
            && (!world.has_component_a4(member)? || entry.kind != 7)
            && !entry.disabled
        {
            world.apply_property(member, entry.property, entry.kind, owner)?;
        }
    }
    Ok(())
}
