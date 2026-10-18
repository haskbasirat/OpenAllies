//! Original construction/upgrade affordability, payment and refunds.
//!
//! * `0x562FD0` BuilderComponent construct affordability (no owner -> true).
//! * `0x566110` ConstructionComponent upgrade gate and affordability.
//! * `0x566360` upgrade start: gate with notification, store the second cost
//!   quote at +0x1C, debit it, trigger, modifiers, +0x28 and actor state 0x1D.
//! * `0x565700` upgrade cancel: credit and zero +0x1C, base modifiers, clear
//!   +0x28, actor state 0x1B.
//! * `0x5656B0` construction cancel: unless destroyed (actor+8 bit 0), credit
//!   and zero +0x10, then destroy the actor.
//! * `0x565770` ActorData+0x4C0 upgrade list membership.
//!
//! Cost quotes (`0x45F4D0` with the actor modifier `0x4CF110`), kingdom
//! restrictions (`0x4E1910`), messages, triggers, modifiers, actor state and
//! destruction are explicit callbacks. As in recruitment, the upgrade start
//! quotes twice and debits the second quote without another check.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// ConstructionComponent payment fields.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstructionPayments {
    /// +0x10: cost paid for the structure under construction.
    pub paid: Vec<u32>,
    /// +0x1C: cost paid for the upgrade in progress.
    pub upgrade_paid: Vec<u32>,
    /// +0x28: upgrade definition in progress (0 = none).
    pub upgrade: u32,
}

/// ActorData+0x174 bit 15: the definition can be an upgrade target.
pub const DEFINITION_UPGRADE_FLAG: u32 = 1 << 15;
/// Actor+0x100 bit 21 blocks upgrades; bit 27 is required.
pub const ACTOR_UPGRADE_BLOCKED: u64 = 1 << 21;
pub const ACTOR_UPGRADE_ENABLED: u64 = 1 << 27;

pub trait Construction {
    /// Actor virtual +0x108 (owner kingdom), called at every native call site.
    fn construction_kingdom(&mut self) -> Result<Option<u32>>;
    /// `0x45F4D0(definition, out, actor modifier 0x4CF110())`.
    fn construction_cost(&mut self, definition: u32) -> Result<Vec<u32>>;
    fn construction_can_afford(&mut self, kingdom: u32, cost: &[u32]) -> Result<bool>;
    fn construction_debit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()>;
    fn construction_credit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()>;
    /// construct_error_affording|ss with the kingdom captured first.
    fn notify_construct_unaffordable(
        &mut self,
        definition: u32,
        cost: &[u32],
        kingdom: u32,
    ) -> Result<()>;
    /// construct_error_affording_upgrade|ss, message kingdom reread.
    fn notify_upgrade_unaffordable(
        &mut self,
        definition: u32,
        cost: &[u32],
        kingdom: Option<u32>,
    ) -> Result<()>;
    /// Actor+0x90 (structure component) present.
    fn has_structure(&mut self) -> Result<bool>;
    /// Actor virtual +0x34.
    fn state_active(&mut self, state: u32) -> Result<bool>;
    /// Actor+0x100 64-bit flags.
    fn actor_flags(&mut self) -> Result<u64>;
    /// ActorData+0x174 of the upgrade definition.
    fn definition_flags(&mut self, definition: u32) -> Result<u32>;
    /// `kingdom.0x4E1910(actor, definition)` team/kingdom restrictions.
    fn kingdom_allows(&mut self, kingdom: u32, definition: u32) -> Result<bool>;
    /// ActorData+0x4C0 list of the constructing actor's own definition.
    fn upgrade_list(&mut self) -> Result<Vec<u32>>;
    /// `0x54BFC0(kingdom, definition)` construction-begin trigger.
    fn construction_begun(&mut self, kingdom: Option<u32>, definition: u32) -> Result<()>;
    /// `actor+0x60 -> 0x561D40(modifier, definition, 0)`; 0 restores the base.
    fn apply_upgrade_modifiers(&mut self, definition: u32) -> Result<()>;
    /// `0x4D64F0(state, enabled)`.
    fn construction_actor_state(&mut self, state: u32, enabled: bool) -> Result<()>;
    /// Actor+8 bit 0.
    fn destroyed(&mut self) -> Result<bool>;
    /// `world.0x4F6760(actor, 0)`.
    fn destroy_actor(&mut self) -> Result<()>;
}

/// `0x562FD0`.
pub fn construct_affordable(definition: u32, world: &mut impl Construction) -> Result<bool> {
    let Some(kingdom) = world.construction_kingdom()? else {
        return Ok(true);
    };
    let cost = world.construction_cost(definition)?;
    if world.construction_can_afford(kingdom, &cost)? {
        return Ok(true);
    }
    world.notify_construct_unaffordable(definition, &cost, kingdom)?;
    Ok(false)
}

/// `0x566110(definition, check_cost, notify)`.
pub fn upgrade_affordable(
    definition: u32,
    check_cost: bool,
    notify: bool,
    world: &mut impl Construction,
) -> Result<bool> {
    if !world.has_structure()? && !world.state_active(0x15)? {
        return Ok(false);
    }
    let flags = world.actor_flags()?;
    if flags & ACTOR_UPGRADE_BLOCKED != 0 || flags & ACTOR_UPGRADE_ENABLED == 0 {
        return Ok(false);
    }
    if definition == 0 || world.definition_flags(definition)? & DEFINITION_UPGRADE_FLAG == 0 {
        return Ok(false);
    }
    let kingdom = world.construction_kingdom()?.ok_or(Error::Invalid(
        "0x4E1910 dereferences the owner kingdom; an owner is required",
    ))?;
    if !world.kingdom_allows(kingdom, definition)? {
        return Ok(false);
    }
    if !world.upgrade_list()?.contains(&definition) {
        return Ok(false);
    }
    if !check_cost {
        return Ok(true);
    }
    let cost = world.construction_cost(definition)?;
    if world.construction_kingdom()?.is_none() {
        return Ok(true);
    }
    let payer = world.construction_kingdom()?.ok_or(Error::Invalid(
        "owner kingdom disappeared between native rereads",
    ))?;
    if world.construction_can_afford(payer, &cost)? {
        return Ok(true);
    }
    if notify {
        let message_kingdom = world.construction_kingdom()?;
        world.notify_upgrade_unaffordable(definition, &cost, message_kingdom)?;
    }
    Ok(false)
}

/// `0x566360`. The stored/debited cost is a fresh quote taken after the gate.
pub fn start_upgrade(
    payments: &mut ConstructionPayments,
    definition: u32,
    world: &mut impl Construction,
) -> Result<bool> {
    if !upgrade_affordable(definition, true, true, world)? {
        return Ok(false);
    }
    payments.upgrade_paid = world.construction_cost(definition)?;
    if world.construction_kingdom()?.is_some() {
        let payer = world.construction_kingdom()?.ok_or(Error::Invalid(
            "owner kingdom disappeared between native rereads",
        ))?;
        world.construction_debit(payer, &payments.upgrade_paid)?;
    }
    let trigger_kingdom = world.construction_kingdom()?;
    world.construction_begun(trigger_kingdom, definition)?;
    world.apply_upgrade_modifiers(definition)?;
    payments.upgrade = definition;
    world.construction_actor_state(0x1d, true)?;
    Ok(true)
}

/// `0x565700`.
pub fn cancel_upgrade(
    payments: &mut ConstructionPayments,
    world: &mut impl Construction,
) -> Result<()> {
    if world.construction_kingdom()?.is_some() {
        let payee = world.construction_kingdom()?.ok_or(Error::Invalid(
            "owner kingdom disappeared between native rereads",
        ))?;
        world.construction_credit(payee, &payments.upgrade_paid)?;
        payments.upgrade_paid.fill(0);
    }
    world.apply_upgrade_modifiers(0)?;
    payments.upgrade = 0;
    world.construction_actor_state(0x1b, true)
}

/// `0x5656B0`.
pub fn cancel_construction(
    payments: &mut ConstructionPayments,
    world: &mut impl Construction,
) -> Result<()> {
    if world.destroyed()? {
        return Ok(());
    }
    if world.construction_kingdom()?.is_some() {
        let payee = world.construction_kingdom()?.ok_or(Error::Invalid(
            "owner kingdom disappeared between native rereads",
        ))?;
        world.construction_credit(payee, &payments.paid)?;
        payments.paid.fill(0);
    }
    world.destroy_actor()
}

/// `0x5655C0(cost, replace)` on ConstructionComponent+0x10: replace copies the
/// rows, otherwise each row is binary32(PC(cost + paid)). A size change goes
/// through `0x50B530` first; accumulation across a size change is unsupported.
pub fn record_paid(
    payments: &mut ConstructionPayments,
    cost: &[u32],
    replace: bool,
    precision: aa_economy::X87Precision,
) -> Result<()> {
    if replace {
        payments.paid = cost.to_vec();
        return Ok(());
    }
    if payments.paid.len() != cost.len() {
        return Err(Error::Invalid(
            "accumulating paid cost across a vector resize is unsupported",
        ));
    }
    for (paid, &value) in payments.paid.iter_mut().zip(cost) {
        *paid = aa_economy::transfers::add_bits(value, *paid, precision)?;
    }
    Ok(())
}

/// Settlement build diagnostics written with `0x404F30` (not UI messages).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SettlementLog {
    /// "settlement %d is at max structures building %s".
    MaxStructures,
    /// "settlement %d cannot build structure %s".
    CannotBuild,
    /// "settlement %d already has structure %s".
    AlreadyHas,
}

/// Placement messages shared by BuildActor and settlement structure building.
pub trait PlacementMessages {
    /// construct_error_enemies_nearby (always sent when enemies are found,
    /// whatever the notify flag); `kingdom` is the owner passed to `0x40C230`.
    fn notify_enemies_nearby(&mut self, kingdom: Option<u32>) -> Result<()>;
    /// construct_error_creating|s; `kingdom` is the owner passed to `0x40C230`.
    fn notify_creation_failed(&mut self, definition: u32, kingdom: Option<u32>) -> Result<()>;
}

/// The x87 precision control the simulation thread runs with (Double53 in
/// the game; the caller selects it).
pub trait FpuPrecision {
    fn precision(&self) -> aa_economy::X87Precision;
}

/// Leaves of SettlementComponent structure building.
pub trait SettlementBuild: Construction + PlacementMessages + FpuPrecision {
    /// SettlementComponent+0x1C structure count and the ActorData+0x464 table
    /// maximum for the level at +0x10.
    fn structure_limit(&mut self) -> Result<(i32, i32)>;
    fn settlement_log(&mut self, log: SettlementLog, definition: u32) -> Result<()>;
    /// Shape `definition+0x38` at (x, y) iterated by `0x554DD0`/`0x554330`:
    /// `tile.0x54EFC0(owner, 5)` per covered tile (row-major) until one reports
    /// enemies. `owner` is the creation-parameter owner (+0x28).
    fn enemies_nearby(
        &mut self,
        definition: u32,
        x: u32,
        y: u32,
        owner: Option<u32>,
    ) -> Result<bool>;
    /// `world.0x4F0B70`: the zone at the location belongs to this settlement.
    fn zone_is_own(&mut self, definition: u32, x: u32, y: u32) -> Result<bool>;
    /// settlement_error_within_zone|ss (settlement name, structure name);
    /// `kingdom` is the owner reread for the message.
    fn notify_within_zone(&mut self, definition: u32, kingdom: Option<u32>) -> Result<()>;
    /// ActorData+0x45C buildable list of the settlement.
    fn buildable(&mut self, definition: u32) -> Result<bool>;
    /// `0x585500(definition, 0, 1)`.
    fn settlement_has_structure(&mut self, definition: u32) -> Result<bool>;
    /// Settlement affordability message (construct_error_affording|ss).
    fn notify_settlement_unaffordable(
        &mut self,
        definition: u32,
        cost: &[u32],
        kingdom: Option<u32>,
    ) -> Result<()>;
    /// `world.0x4F9C10(params)` with definition, location and owner.
    fn create_structure(
        &mut self,
        definition: u32,
        x: u32,
        y: u32,
        owner: Option<u32>,
    ) -> Result<Option<u32>>;
    /// created+0x9C ConstructionComponent payment state, when present.
    fn construction_payments(&mut self, created: u32) -> Result<Option<&mut ConstructionPayments>>;
    /// `0x586B40(created)` settlement structure registration.
    fn settlement_add_structure(&mut self, created: u32) -> Result<()>;
}

/// `0x5879B0(definition, notify)`: null definition or no owner -> false.
pub fn settlement_affordable(
    definition: u32,
    notify: bool,
    world: &mut impl SettlementBuild,
) -> Result<bool> {
    if definition == 0 {
        return Ok(false);
    }
    let Some(kingdom) = world.construction_kingdom()? else {
        return Ok(false);
    };
    let cost = world.construction_cost(definition)?;
    if world.construction_can_afford(kingdom, &cost)? {
        return Ok(true);
    }
    if notify {
        let message_kingdom = world.construction_kingdom()?;
        world.notify_settlement_unaffordable(definition, &cost, message_kingdom)?;
    }
    Ok(false)
}

/// `0x587B60(definition, x, y, check_cost, check_location, notify)`.
pub fn can_build_structure(
    definition: u32,
    x: u32,
    y: u32,
    check_cost: bool,
    check_location: bool,
    notify: bool,
    world: &mut impl SettlementBuild,
) -> Result<bool> {
    if !world.state_active(0xd)? {
        return Ok(false);
    }
    if definition == 0 || world.definition_flags(definition)? & (1 << 13) == 0 {
        return Ok(false);
    }
    let (count, maximum) = world.structure_limit()?;
    if count >= maximum {
        if notify {
            world.settlement_log(SettlementLog::MaxStructures, definition)?;
        }
        return Ok(false);
    }
    let kingdom = world.construction_kingdom()?.ok_or(Error::Invalid(
        "0x4E1910 dereferences the owner kingdom; an owner is required",
    ))?;
    if !world.kingdom_allows(kingdom, definition)? {
        return Ok(false);
    }
    if check_location {
        // The creation parameters (0x4C8C70) read the owner once more.
        let owner = world.construction_kingdom()?;
        if world.enemies_nearby(definition, x, y, owner)? {
            let kingdom = world.construction_kingdom()?;
            world.notify_enemies_nearby(kingdom)?;
            return Ok(false);
        }
        if !world.zone_is_own(definition, x, y)? {
            if notify {
                let kingdom = world.construction_kingdom()?;
                world.notify_within_zone(definition, kingdom)?;
            }
            return Ok(false);
        }
    }
    if !world.buildable(definition)? {
        if notify {
            world.settlement_log(SettlementLog::CannotBuild, definition)?;
        }
        return Ok(false);
    }
    if world.settlement_has_structure(definition)? {
        if notify {
            world.settlement_log(SettlementLog::AlreadyHas, definition)?;
        }
        return Ok(false);
    }
    if check_cost && !settlement_affordable(definition, notify, world)? {
        return Ok(false);
    }
    Ok(true)
}

/// `0x587FD0(definition, x, y)`: checks everything, quotes the cost BEFORE
/// creation, creates the structure, then debits the reread owner, stores the
/// paid cost in the new ConstructionComponent (`0x5657A0` -> `0x5655C0(cost,
/// 1)`) and registers it with the settlement. A failed creation debits nothing.
pub fn build_structure(
    definition: u32,
    x: u32,
    y: u32,
    world: &mut impl SettlementBuild,
) -> Result<Option<u32>> {
    if !can_build_structure(definition, x, y, true, true, true, world)? {
        return Ok(None);
    }
    let owner = world.construction_kingdom()?;
    let cost = world.construction_cost(definition)?;
    let Some(created) = world.create_structure(definition, x, y, owner)? else {
        let kingdom = world.construction_kingdom()?;
        world.notify_creation_failed(definition, kingdom)?;
        return Ok(None);
    };
    if world.construction_kingdom()?.is_some() {
        let payer = world.construction_kingdom()?.ok_or(Error::Invalid(
            "owner kingdom disappeared between native rereads",
        ))?;
        world.construction_debit(payer, &cost)?;
    }
    let precision = world.precision();
    if let Some(payments) = world.construction_payments(created)? {
        record_paid(payments, &cost, true, precision)?;
    }
    world.settlement_add_structure(created)?;
    Ok(Some(created))
}

/// Definition fields read by `0x563160` BuildActor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildRules {
    /// ActorData+0x1C: bit 16 snaps the location to the map grid (with the
    /// definition virtual +0x30), bit 17 resets the facing to (0, -1).
    pub placement_flags: u32,
    /// ActorData+0x430 != 0: refuse when enemies are on the footprint.
    pub check_enemies: bool,
    /// ActorData+0x4E0: class of the map object the actor must be built on
    /// (0 = none). A missing site is only logged.
    pub site_class: u32,
}

/// Creation parameters `0x563160` passes to `world.0x4F9C10` (object type
/// 0xA25B28; +0x38 is always 1, the other fields keep their constructor
/// defaults).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildParams {
    /// +0x04.
    pub definition: u32,
    /// +0x08/+0x0C after `0x6939B0`.
    pub location: [u32; 2],
    /// +0x10: the builder actor's field +0x28.
    pub builder_field_28: u32,
    /// +0x14/+0x18 after `0x6939B0`.
    pub facing: [u32; 2],
    /// +0x28: the builder owner read before the affordability check.
    pub owner: Option<u32>,
    /// +0x34: the fourth BuildActor argument.
    pub tag: u32,
    /// +0x3C: GTID (+0x14) of the site found by `0x555830`.
    pub site: Option<u32>,
}

/// Leaves of BuilderComponent::BuildActor.
pub trait BuildActorWorld: Construction + PlacementMessages {
    fn build_rules(&mut self, definition: u32) -> Result<BuildRules>;
    /// Definition virtual +0x30 (`0x6939B0` calls it before testing bit 16).
    fn snaps_to_grid(&mut self, definition: u32) -> Result<bool>;
    /// `(definition+0x38).0x46EE00(&location, map+8)`.
    fn snap_location(&mut self, definition: u32, location: [u32; 2]) -> Result<[u32; 2]>;
    /// Footprint `definition+0x38` at (location, facing), iterated by
    /// `0x554DD0`/`0x554330`, `tile.0x54EFC0(owner, 5)` per tile until one
    /// reports enemies. The owner is the one read at entry (may be null).
    fn build_enemies_nearby(
        &mut self,
        definition: u32,
        location: [u32; 2],
        facing: [u32; 2],
        owner: Option<u32>,
    ) -> Result<bool>;
    /// `map.0x555830(definition, location)`: GTID of the site object.
    fn find_site(&mut self, definition: u32, location: [u32; 2]) -> Result<Option<u32>>;
    /// `0x404F60` diagnostic for a missing site (creation still proceeds).
    fn log_missing_site(&mut self, definition: u32, location: [u32; 2]) -> Result<()>;
    /// Builder actor field +0x28 copied into the creation parameters.
    fn builder_field_28(&mut self) -> Result<u32>;
    /// `world.0x4F9C10(params)`.
    fn create_actor(&mut self, params: &BuildParams) -> Result<Option<u32>>;
}

/// `0x6939B0(definition, &location, &facing)` placement adjustment.
pub fn adjust_placement(
    definition: u32,
    rules: &BuildRules,
    location: [u32; 2],
    facing: [u32; 2],
    world: &mut impl BuildActorWorld,
) -> Result<([u32; 2], [u32; 2])> {
    let mut location = location;
    let mut facing = facing;
    if world.snaps_to_grid(definition)? && rules.placement_flags & (1 << 16) != 0 {
        location = world.snap_location(definition, location)?;
    }
    if rules.placement_flags & (1 << 17) != 0 {
        facing = [0, 0xBF80_0000];
    }
    Ok((location, facing))
}

/// `0x563160(definition, &location, &facing, tag)` BuilderComponent::BuildActor:
/// placement, owner read, `0x562FD0` affordability (no owner -> affordable),
/// `0x4E1910` restrictions when owned, the optional enemy footprint check,
/// the optional site lookup, creation, and only THEN the cost quote and the
/// debit of the entry owner (no second affordability check). Unlike settlement
/// building, the paid cost is not stored in the created ConstructionComponent.
pub fn build_actor(
    definition: u32,
    location: [u32; 2],
    facing: [u32; 2],
    tag: u32,
    world: &mut impl BuildActorWorld,
) -> Result<Option<u32>> {
    let rules = world.build_rules(definition)?;
    let (location, facing) = adjust_placement(definition, &rules, location, facing, world)?;
    let owner = world.construction_kingdom()?;
    if !construct_affordable(definition, world)? {
        return Ok(None);
    }
    if let Some(kingdom) = owner
        && !world.kingdom_allows(kingdom, definition)?
    {
        return Ok(None);
    }
    if rules.check_enemies && world.build_enemies_nearby(definition, location, facing, owner)? {
        let kingdom = world.construction_kingdom()?;
        world.notify_enemies_nearby(kingdom)?;
        return Ok(None);
    }
    let builder_field_28 = world.builder_field_28()?;
    let mut site = None;
    if rules.site_class != 0 {
        site = world.find_site(definition, location)?;
        if site.is_none() {
            world.log_missing_site(definition, location)?;
        }
    }
    let params = BuildParams {
        definition,
        location,
        builder_field_28,
        facing,
        owner,
        tag,
        site,
    };
    let Some(created) = world.create_actor(&params)? else {
        world.notify_creation_failed(definition, owner)?;
        return Ok(None);
    };
    let cost = world.construction_cost(definition)?;
    if let Some(kingdom) = owner {
        world.construction_debit(kingdom, &cost)?;
    }
    Ok(Some(created))
}

/// Actor+0x100 bit 21: under construction.
pub const ACTOR_CONSTRUCTING: u64 = 1 << 21;
/// Actor+0x100 bit 29: upgrade in progress.
pub const ACTOR_UPGRADING: u64 = 1 << 29;
/// Actor+0x100 bit 30: structure disabled (command 0x16 sets state 0x1E).
pub const ACTOR_DISABLED: u64 = 1 << 30;
/// Actor state written every ConstructionComponent tick: construction or
/// upgrade started less than `construction_window` seconds ago.
pub const STATE_RECENTLY_STARTED: u32 = 0x20;

/// `0x565500(out, accumulate)` salvage value of the paid cost (+0x10):
/// `out[i] (+)= floor(double(PC(PC(fraction * paid[i]) + 0.5f)))`, stored as
/// binary32, where `fraction = *(cache+0x13C)`. Vector semantics follow the
/// native: accumulating into a shorter vector zero-resizes it (`0x45F180`)
/// first; otherwise the count becomes `paid.len()`.
pub fn salvage(
    out: &mut Vec<u32>,
    paid: &[u32],
    fraction: u32,
    accumulate: bool,
    precision: aa_economy::X87Precision,
) -> Result<()> {
    use aa_economy::F32Bits;
    use aa_economy::x87::Register;
    let n = paid.len();
    if accumulate && out.len() < n {
        *out = vec![0; n];
    } else {
        out.resize(n, 0);
    }
    let fraction = Register::load(F32Bits(fraction))?;
    let half = Register::load(F32Bits(0x3F00_0000))?;
    for (slot, &value) in out.iter_mut().zip(paid) {
        let mut st = fraction
            .mul(&Register::load(F32Bits(value))?, precision)
            .add(&half, precision)
            .round_f64()
            .floor();
        if accumulate {
            st = st.add(&Register::load(F32Bits(*slot))?, precision);
        }
        *slot = st.store_f32()?.0;
    }
    Ok(())
}

/// Leaves of the ConstructionComponent lifecycle (sell, commands, Tick).
pub trait ConstructionLifecycle: Construction + FpuPrecision {
    /// `*(cache+0x13C)` salvage fraction (binary32 bits).
    fn salvage_fraction(&mut self) -> Result<u32>;
    /// Event 0x17 (`0x42D490` on 0xB39D8C) with the owner, the actor, its
    /// name and the refund text (`0x5C4CB0`).
    fn notify_sold(&mut self, refund: &[u32], kingdom: Option<u32>) -> Result<()>;
    /// `0x565400`: state 0x1B on, `0x565140` state recompute, settlement refresh.
    fn enable_structure(&mut self) -> Result<()>;
    /// `0x564DC0`: state 0x1E on, `0x4D1350`, settlement refresh.
    fn disable_structure(&mut self) -> Result<()>;
    /// Kingdom+0x239, set by `0x4E98C0` (event 0x33, status 2) when the
    /// kingdom is eliminated.
    fn kingdom_dissolving(&mut self, kingdom: u32) -> Result<bool>;
    /// `0x4D6660` actor state refresh.
    fn refresh_actor(&mut self) -> Result<()>;
    /// `0x4CF110` actor modifier token (null skips auto-upgrades).
    fn auto_upgrade_token(&mut self) -> Result<Option<u32>>;
    /// token+0x30 auto-upgrade interval (binary32 bits).
    fn auto_upgrade_interval(&mut self, token: u32) -> Result<u32>;
    /// Actor+0x1C time stamp (binary32 bits).
    fn actor_time(&mut self) -> Result<u32>;
    /// World+0xE0 simulation time (binary32 bits).
    fn world_time(&mut self) -> Result<u32>;
    /// `0x45F4D0(out, 0)`: the cost without the actor modifier.
    fn base_cost(&mut self, definition: u32) -> Result<Vec<u32>>;
    /// Resource registry rows whose kind is STOCKPILED (manager 0xB3A660+0x24).
    fn stockpiled_resources(&mut self) -> Result<Vec<bool>>;
    /// `0x404ED0` diagnostic: an automatic upgrade has a stockpiled cost.
    fn log_auto_upgrade_cost(&mut self, definition: u32) -> Result<()>;
    /// (actor+0x60)+0x20: construction/upgrade start time (binary32 bits).
    fn construction_start_time(&mut self) -> Result<u32>;
    /// `*(cache+0x1CC)`: window during which state 0x20 stays set.
    fn construction_window(&mut self) -> Result<u32>;
}

/// `0x5658C0`: with an owner, credit the salvage value of the paid cost to the
/// reread owner (`0x4E39D0`), zero the paid vector and post event 0x17.
pub fn refund_salvage(
    payments: &mut ConstructionPayments,
    world: &mut impl ConstructionLifecycle,
) -> Result<()> {
    if world.construction_kingdom()?.is_none() {
        return Ok(());
    }
    let fraction = world.salvage_fraction()?;
    let mut refund = Vec::with_capacity(payments.paid.len());
    salvage(
        &mut refund,
        &payments.paid,
        fraction,
        false,
        world.precision(),
    )?;
    let payee = world.construction_kingdom()?.ok_or(Error::Invalid(
        "owner kingdom disappeared between native rereads",
    ))?;
    world.construction_credit(payee, &refund)?;
    payments.paid.fill(0);
    let kingdom = world.construction_kingdom()?;
    world.notify_sold(&refund, kingdom)
}

/// `0x565A20` sell (command 0x14): cancel an upgrade in progress
/// (`0x565700`), then unless destroyed refund the salvage (`0x5658C0`) and
/// destroy the actor (`0x4F6760(actor, 0)`).
pub fn sell_structure(
    payments: &mut ConstructionPayments,
    world: &mut impl ConstructionLifecycle,
) -> Result<()> {
    if world.actor_flags()? & ACTOR_UPGRADING != 0 {
        cancel_upgrade(payments, world)?;
    }
    if world.destroyed()? {
        return Ok(());
    }
    refund_salvage(payments, world)?;
    world.destroy_actor()
}

/// `0x565860` cancel (command 0x13): construction -> `0x5656B0`, else upgrade
/// -> `0x565700`, else disabled -> `0x565400`.
pub fn cancel_command(
    payments: &mut ConstructionPayments,
    world: &mut impl ConstructionLifecycle,
) -> Result<()> {
    let flags = world.actor_flags()?;
    if flags & ACTOR_CONSTRUCTING != 0 {
        cancel_construction(payments, world)
    } else if flags & ACTOR_UPGRADING != 0 {
        cancel_upgrade(payments, world)
    } else if flags & ACTOR_DISABLED != 0 {
        world.enable_structure()
    } else {
        Ok(())
    }
}

/// `0x5667F0` ConstructionComponent command dispatch on the command type
/// (command+8 -> +0xC); 0x15 carries the upgrade definition at command+0x14.
/// Returns whether the command was handled (the two diagnostic traces
/// `0x77DC50`/`0x4D9A40` are not modelled).
pub fn construction_command(
    command_type: u32,
    definition: u32,
    payments: &mut ConstructionPayments,
    world: &mut impl ConstructionLifecycle,
) -> Result<bool> {
    match command_type {
        0x13 => cancel_command(payments, world)?,
        0x14 => sell_structure(payments, world)?,
        0x15 => {
            start_upgrade(payments, definition, world)?;
        }
        0x16 => world.disable_structure()?,
        _ => return Ok(false),
    }
    Ok(true)
}

/// `0x501570(token, actor)`: interval >= 0 and PC(world time - actor+0x1C)
/// > interval.
pub fn auto_upgrade_due(token: u32, world: &mut impl ConstructionLifecycle) -> Result<bool> {
    use aa_economy::F32Bits;
    use aa_economy::x87::Register;
    use std::cmp::Ordering;
    let interval = Register::load(F32Bits(world.auto_upgrade_interval(token)?))?;
    if interval.compare(&Register::load(F32Bits::ZERO)?) == Ordering::Less {
        return Ok(false);
    }
    let elapsed = Register::load(F32Bits(world.world_time()?))?.sub(
        &Register::load(F32Bits(world.actor_time()?))?,
        world.precision(),
    );
    Ok(interval.compare(&elapsed) == Ordering::Less)
}

/// `0x4CD160` + `0x4CD000`: some STOCKPILED cost row is > 0.
fn has_stockpiled_cost(cost: &[u32], stockpiled: &[bool]) -> Result<bool> {
    if cost.len() > stockpiled.len() {
        return Err(Error::Invalid(
            "cost vector longer than the resource registry",
        ));
    }
    Ok(cost
        .iter()
        .zip(stockpiled)
        .any(|(&v, &s)| s && v != 0 && v <= 0x7F80_0000))
}

/// `0x566880` ConstructionComponent::Tick (updatable at component+8; the two
/// tick arguments are unused). A dissolving owner (+0x239) first services the
/// actor states 0x16 (disable) and 0x14 (sell). Otherwise the actor is
/// refreshed (`0x4D6660`); when the modifier token interval has elapsed, the
/// actor has an upgrade list and no upgrade is running, the first upgrade
/// passing `0x566110(def, 1, 0)` is started with `0x566360` (after the
/// diagnostic for a stockpiled base cost). Finally state 0x20 is set when a
/// construction or upgrade started less than the window ago.
pub fn construction_tick(
    payments: &mut ConstructionPayments,
    world: &mut impl ConstructionLifecycle,
) -> Result<()> {
    use aa_economy::F32Bits;
    use aa_economy::x87::Register;
    use std::cmp::Ordering;
    if let Some(kingdom) = world.construction_kingdom()?
        && world.kingdom_dissolving(kingdom)?
    {
        if world.state_active(0x16)? {
            return world.disable_structure();
        }
        if world.state_active(0x14)? {
            return sell_structure(payments, world);
        }
    }
    world.refresh_actor()?;
    if let Some(token) = world.auto_upgrade_token()?
        && auto_upgrade_due(token, world)?
    {
        let upgrades = world.upgrade_list()?;
        if !upgrades.is_empty() && world.actor_flags()? & ACTOR_UPGRADING == 0 {
            for definition in upgrades {
                if upgrade_affordable(definition, true, false, world)? {
                    let base = world.base_cost(definition)?;
                    let stockpiled = world.stockpiled_resources()?;
                    if has_stockpiled_cost(&base, &stockpiled)? {
                        world.log_auto_upgrade_cost(definition)?;
                    }
                    start_upgrade(payments, definition, world)?;
                    break;
                }
            }
        }
    }
    let elapsed = Register::load(F32Bits(world.world_time()?))?.sub(
        &Register::load(F32Bits(world.construction_start_time()?))?,
        world.precision(),
    );
    let flags = world.actor_flags()?;
    let recent = flags & (ACTOR_CONSTRUCTING | ACTOR_UPGRADING) != 0
        && elapsed.compare(&Register::load(F32Bits(world.construction_window()?))?)
            == Ordering::Less;
    world.construction_actor_state(STATE_RECENTLY_STARTED, recent)
}

/// Actor+0x100 bit 20, copied to the upgraded actor as state 0x14.
pub const ACTOR_STATE_20: u64 = 1 << 20;

/// Creation parameters (object type 0xA25B28) passed to `world.0x4F9C10` by
/// `0x565B00` (settlement founding: mode 0, no source) and `0x566400`
/// (upgrade: mode 4, source = old actor GTID).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplaceParams {
    /// +0x04: ActorData+0x430 (settlement) or ConstructionComponent+0x28.
    pub definition: u32,
    /// +0x28: owner read with actor virtual +0x108.
    pub owner: Option<u32>,
    /// +0x08..+0x18: actor+0x20..+0x30.
    pub placement: [u32; 5],
    /// +0x38.
    pub mode: u32,
    /// +0x3C.
    pub source: u32,
}

/// Diagnostics written by the completion functions (`0x404ED0`/`0x404F60`/
/// `0x404F90`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinishLog {
    /// "Unable to create settlement for center (type=%s)".
    SettlementCreationFailed,
    /// "CreateComponents failed for actor %d type %s".
    CreateComponentsFailed,
    /// "Upgraded %s (%d) to %s (%d)."
    Upgraded,
}

/// Actor components consulted by the completion functions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ComponentPresence {
    /// Actor+0x78 (properties).
    pub c78: bool,
    /// Actor+0x88.
    pub c88: bool,
    /// Actor+0x90.
    pub c90: bool,
    /// Actor+0x94 (structure).
    pub c94: bool,
}

/// Component hand-off on completion: `0x588970`/`0x58CAB0` (construction
/// finished) and `0x588AE0`/`0x58CB80`/`0x5698C0`/`0x5814A0` (upgrade, with
/// the created actor).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletionComponent {
    C78,
    C88,
    C90,
    C94,
}

/// World seams of OnConstructionFinished `0x565B00`, OnUpgradeFinished
/// `0x566400` and their dispatch `0x566AC0`. Actor creation/replacement,
/// components, AI, settlements, groups, names and triggers are other owners'
/// code; the economy parts are bound to existing ports (`0x4E4470`
/// `aa_economy::kingdom_registry::actor_completed`, `0x564ED0`/`0x564E10`
/// `aa_economy::statistics`, `0x4CF0F0` economy refresh, `0x565700`
/// [`cancel_upgrade`]).
pub trait ConstructionFinish: Construction + PlacementMessages {
    /// ActorData+0x430: settlement definition founded by this structure.
    fn settlement_definition(&mut self) -> Result<u32>;
    /// Actor virtual +0x100.
    fn actor_virtual_100(&mut self) -> Result<bool>;
    /// Actor+4 (its ActorData).
    fn actor_definition(&mut self) -> Result<u32>;
    /// Actor+0x20..+0x30.
    fn actor_placement(&mut self) -> Result<[u32; 5]>;
    /// Actor+0x14.
    fn actor_gtid(&mut self) -> Result<u32>;
    fn components(&mut self) -> Result<ComponentPresence>;
    /// `world.0x4F9C10(params)`.
    fn replace_create(&mut self, params: &ReplaceParams) -> Result<Option<u32>>;
    /// Created actor+4.
    fn created_definition(&mut self, created: u32) -> Result<u32>;
    /// created+0x98 SettlementComponent present and `0x586470(actor)`.
    fn join_new_settlement(&mut self, created: u32) -> Result<bool>;
    /// `actor.0x4D21A0(1)` CreateComponents.
    fn create_components(&mut self) -> Result<bool>;
    fn log_finish(&mut self, log: FinishLog) -> Result<()>;
    /// `kingdom.0x4E4470(actor)`.
    fn kingdom_actor_completed(&mut self, kingdom: u32) -> Result<()>;
    /// `0x4CF0F0`: economy component refresh (`0x570020` when actor+0x84).
    fn refresh_economy(&mut self) -> Result<()>;
    /// Global 0xB36528 present.
    fn ai_manager_present(&mut self) -> Result<bool>;
    /// `manager.0x79EA90(owner)`.
    fn ai_for(&mut self, owner: Option<u32>) -> Result<Option<u32>>;
    /// `ai.0x7B2350(actor)`.
    fn ai_constructed(&mut self, ai: u32) -> Result<()>;
    /// Actor virtual +0x158.
    fn in_settlement(&mut self) -> Result<bool>;
    /// (virtual +0x15C)+0x98 -> `0x584B40`.
    fn refresh_settlement(&mut self) -> Result<()>;
    /// `0x4D1350` on the actor (`None`) or on the created actor.
    fn refresh_organization(&mut self, created: Option<u32>) -> Result<()>;
    /// `0x564ED0` (bind to `aa_economy::statistics::construction_completed`).
    fn completion_statistics(&mut self) -> Result<()>;
    /// `0x564E10` (bind to `aa_economy::statistics::upgrade_completed`).
    fn upgrade_statistics(&mut self) -> Result<()>;
    /// `0x42D490(kind, owner, actor, name)`: 1 construction complete, 8 from
    /// the dispatch when nothing was under construction or upgrade.
    fn completion_event(&mut self, kind: u32, owner: Option<u32>) -> Result<()>;
    /// `triggers.0x54C050(owner, definition)`.
    fn construction_trigger(&mut self, owner: Option<u32>, definition: u32) -> Result<()>;
    /// Component completion (`created` = None) or hand-off to the upgraded actor.
    fn complete_component(
        &mut self,
        component: CompletionComponent,
        created: Option<u32>,
    ) -> Result<()>;
    /// `0x4CE6C0(*builder)`.
    fn builder_controller(&mut self, builder: u32) -> Result<u32>;
    /// `0x4D11A0(controller)`: controller+0x70 has a live target.
    fn controller_busy(&mut self, controller: u32) -> Result<bool>;
    /// `(controller+0x70)` virtual +0x40 with the actor.
    fn notify_builder(&mut self, controller: u32) -> Result<()>;
    /// `0x42A590(ActorData+0xE4, actor, 0)`.
    fn built_trigger(&mut self) -> Result<()>;
    /// `world.0x4F6760(actor, 1)` (the upgraded actor replaces this one).
    fn destroy_replaced(&mut self) -> Result<()>;
    /// `0x4D04B0`: virtual +0x50(0), actor+8 bit 30.
    fn prepare_upgrade(&mut self) -> Result<()>;
    /// Actor+0x158 resolved through the GTID table.
    fn actor_group(&mut self) -> Result<Option<u32>>;
    /// `actor.0x4D3DD0(0)`.
    fn leave_group(&mut self) -> Result<()>;
    /// `actor.0x4D0000(group)`.
    fn rejoin_group(&mut self, group: u32) -> Result<()>;
    /// `0x4D7EA0` after a failed upgrade creation.
    fn restore_after_failed_upgrade(&mut self) -> Result<()>;
    /// `created.0x4D64F0(state, enabled)`.
    fn created_state(&mut self, created: u32, state: u32, enabled: bool) -> Result<()>;
    /// `created.0x4D5EA0(actor)`.
    fn transfer_state(&mut self, created: u32) -> Result<()>;
    /// `0x4D1930`: actor+0xDC name is empty.
    fn has_default_name(&mut self) -> Result<bool>;
    /// `created.0x4D7F40(actor name)`.
    fn copy_name(&mut self, created: u32) -> Result<()>;
}

/// `0x4D18C0`: ActorData+0x430 set, virtual +0x100 false and under
/// construction.
pub fn founds_settlement(world: &mut impl ConstructionFinish) -> Result<bool> {
    Ok(world.settlement_definition()? != 0
        && !world.actor_virtual_100()?
        && world.actor_flags()? & ACTOR_CONSTRUCTING != 0)
}

/// `0x565B00(builder)` OnConstructionFinished.
pub fn construction_finished(
    builder: Option<u32>,
    world: &mut impl ConstructionFinish,
) -> Result<()> {
    let founds = founds_settlement(world)?;
    world.construction_actor_state(0x15, false)?;
    if founds {
        let definition = world.settlement_definition()?;
        let owner = world.construction_kingdom()?;
        let placement = world.actor_placement()?;
        let params = ReplaceParams {
            definition,
            owner,
            placement,
            mode: 0,
            source: 0,
        };
        let joined = match world.replace_create(&params)? {
            Some(created) => world.join_new_settlement(created)?,
            None => false,
        };
        if !joined {
            world.log_finish(FinishLog::SettlementCreationFailed)?;
            return world.destroy_actor();
        }
    }
    if !world.create_components()? {
        world.log_finish(FinishLog::CreateComponentsFailed)?;
        world.construction_actor_state(0x15, true)?;
        return world.destroy_actor();
    }
    if world.construction_kingdom()?.is_some() {
        let kingdom = world.construction_kingdom()?.ok_or(Error::Invalid(
            "owner kingdom disappeared between native rereads",
        ))?;
        world.kingdom_actor_completed(kingdom)?;
    }
    world.refresh_economy()?;
    if world.ai_manager_present()? {
        let owner = world.construction_kingdom()?;
        if let Some(ai) = world.ai_for(owner)? {
            world.ai_constructed(ai)?;
        }
    }
    if world.in_settlement()? {
        world.refresh_settlement()?;
    }
    world.refresh_organization(None)?;
    world.completion_statistics()?;
    let components = world.components()?;
    if !components.c90 {
        let owner = world.construction_kingdom()?;
        world.completion_event(1, owner)?;
    }
    let definition = world.actor_definition()?;
    let owner = world.construction_kingdom()?;
    world.construction_trigger(owner, definition)?;
    if components.c94 {
        world.complete_component(CompletionComponent::C94, None)?;
    }
    if components.c90 {
        world.complete_component(CompletionComponent::C90, None)?;
    }
    if let Some(builder) = builder {
        let controller = world.builder_controller(builder)?;
        if !world.controller_busy(controller)? && world.in_settlement()? {
            world.notify_builder(controller)?;
        }
    }
    world.built_trigger()
}

/// `0x566400` OnUpgradeFinished: create the upgraded actor (mode 4) and hand
/// everything over, or on failure restore the actor, message and refund the
/// upgrade (`0x565700`). A null upgrade definition is outside the native
/// domain (`0x4F9C10` dereferences it); it is rejected before any effect.
pub fn upgrade_finished(
    payments: &mut ConstructionPayments,
    world: &mut impl ConstructionFinish,
) -> Result<()> {
    if payments.upgrade == 0 {
        return Err(Error::Invalid(
            "0x4F9C10 dereferences the null upgrade definition (+0x28)",
        ));
    }
    world.prepare_upgrade()?;
    let group = world.actor_group()?;
    if group.is_some() {
        world.leave_group()?;
    }
    let owner = world.construction_kingdom()?;
    let params = ReplaceParams {
        definition: payments.upgrade,
        owner,
        placement: world.actor_placement()?,
        mode: 4,
        source: world.actor_gtid()?,
    };
    let Some(created) = world.replace_create(&params)? else {
        if let Some(group) = group {
            world.rejoin_group(group)?;
        }
        world.restore_after_failed_upgrade()?;
        let kingdom = world.construction_kingdom()?;
        world.notify_creation_failed(payments.upgrade, kingdom)?;
        return cancel_upgrade(payments, world);
    };
    world.construction_actor_state(0x1B, true)?;
    let state_20 = world.actor_flags()? & ACTOR_STATE_20 != 0;
    world.created_state(created, 0x14, state_20)?;
    world.log_finish(FinishLog::Upgraded)?;
    world.upgrade_statistics()?;
    world.transfer_state(created)?;
    if !world.has_default_name()? {
        world.copy_name(created)?;
    }
    let definition = world.created_definition(created)?;
    let owner = world.construction_kingdom()?;
    world.construction_trigger(owner, definition)?;
    let components = world.components()?;
    for (present, component) in [
        (components.c94, CompletionComponent::C94),
        (components.c90, CompletionComponent::C90),
        (components.c88, CompletionComponent::C88),
        (components.c78, CompletionComponent::C78),
    ] {
        if present {
            world.complete_component(component, Some(created))?;
        }
    }
    world.refresh_organization(Some(created))?;
    world.destroy_replaced()
}

/// `0x566AC0(builder)`: under construction -> `0x565B00`, upgrading ->
/// `0x566400`, otherwise event 8 unless the actor has component +0x90.
pub fn construction_complete(
    builder: Option<u32>,
    payments: &mut ConstructionPayments,
    world: &mut impl ConstructionFinish,
) -> Result<()> {
    let flags = world.actor_flags()?;
    if flags & ACTOR_CONSTRUCTING != 0 {
        return construction_finished(builder, world);
    }
    if flags & ACTOR_UPGRADING != 0 {
        return upgrade_finished(payments, world);
    }
    if !world.components()?.c90 {
        let owner = world.construction_kingdom()?;
        world.completion_event(8, owner)?;
    }
    Ok(())
}

/// SettlementComponent structure slots.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettlementStructures {
    /// +0x10 level (row of the ActorData+0x464 limit table).
    pub level: u32,
    /// +0x14 center actor.
    pub center: Option<u32>,
    /// +0x18/+0x1C structure actors in insertion order.
    pub structures: Vec<u32>,
}

/// Facts of one structure actor read by `0x5890C0`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructureMatchFacts {
    /// Actor+0x94 StructureComponent present.
    pub structure_component: bool,
    /// Actor+0x100 bit 25.
    pub state_25: bool,
    /// `0x4D1050`: actor+0x100 bit 26 or +0x104 bit 4.
    pub transient: bool,
    /// Actor+4.
    pub definition: u32,
    /// Actor+0x100 bit 21.
    pub constructing: bool,
    /// ActorData+0x4C0 upgrade list.
    pub upgrades: Vec<u32>,
    /// ActorData+0x4D0 list.
    pub list_4d0: Vec<u32>,
}

/// `0x5890C0(definition, require_complete, include_upgrades)` on a
/// StructureComponent: not state 25 or transient; the actor's definition, or
/// with `include_upgrades` one in ActorData+0x4C0, or one in ActorData+0x4D0;
/// with `require_complete` the actor must not be under construction.
pub fn structure_matches(
    facts: &StructureMatchFacts,
    definition: u32,
    require_complete: bool,
    include_upgrades: bool,
) -> bool {
    struct Read<'a>(&'a StructureMatchFacts);
    impl StructureMatchWorld for Read<'_> {
        fn state_25(&mut self)->Result<bool>{Ok(self.0.state_25)}
        fn transient(&mut self)->Result<bool>{Ok(self.0.transient)}
        fn definition(&mut self)->Result<u32>{Ok(self.0.definition)}
        fn contains(&mut self,offset:u32,definition:u32)->Result<bool>{
            Ok((if offset==0x4c0 {&self.0.upgrades}else{&self.0.list_4d0}).contains(&definition))
        }
        fn constructing(&mut self)->Result<bool>{Ok(self.0.constructing)}
    }
    //The explicit-facts wrapper has no fallible children. Live owners use the
    //same kernel below so unvisited source lists are never eagerly demanded.
    structure_matches_with(&mut Read(facts),definition,require_complete,include_upgrades)
        .expect("explicit StructureMatchFacts reads are infallible")
}

///Borrowed current fields of5890C0. Definition is reread independently before
///each reached source-list branch; callbacks must use the same actual owner.
pub trait StructureMatchWorld {
    fn state_25(&mut self)->Result<bool>;
    fn transient(&mut self)->Result<bool>;
    fn definition(&mut self)->Result<u32>;
    fn contains(&mut self,offset:u32,definition:u32)->Result<bool>;
    fn constructing(&mut self)->Result<bool>;
}
///5890C0 with lazy actual source/current-state reads, preserving native
///early returns before upgrades/predecessors and the final complete check.
pub fn structure_matches_with(
    world:&mut impl StructureMatchWorld,definition:u32,
    require_complete:bool,include_upgrades:bool,
)->Result<bool>{
    if world.state_25()?||world.transient()? {return Ok(false)}
    let mut matched=world.definition()?==definition;
    if !matched&&include_upgrades {
        world.definition()?;
        matched=world.contains(0x4c0,definition)?;
    }
    if !matched {
        world.definition()?;
        matched=world.contains(0x4d0,definition)?;
    }
    if !matched{return Ok(false)}
    Ok(!require_complete||!world.constructing()?)
}

/// `0x585500(definition, require_complete, include_upgrades)`: the center
/// (when it has a StructureComponent) first, then the structures in order.
/// A listed structure without a StructureComponent is unsupported (the native
/// dereferences it).
pub fn find_settlement_structure(
    settlement: &SettlementStructures,
    definition: u32,
    require_complete: bool,
    include_upgrades: bool,
    mut facts: impl FnMut(u32) -> Result<StructureMatchFacts>,
) -> Result<Option<u32>> {
    if let Some(center) = settlement.center {
        let f = facts(center)?;
        if f.structure_component
            && structure_matches(&f, definition, require_complete, include_upgrades)
        {
            return Ok(Some(center));
        }
    }
    for &structure in &settlement.structures {
        let f = facts(structure)?;
        if !f.structure_component {
            return Err(Error::Invalid(
                "0x585500 dereferences the StructureComponent of every listed structure",
            ));
        }
        if structure_matches(&f, definition, require_complete, include_upgrades) {
            return Ok(Some(structure));
        }
    }
    Ok(None)
}

/// Seams of `0x586B40`.
pub trait SettlementSlots {
    /// `0x585F60(actor)`: attach the structure (component links, refreshes).
    fn attach_structure(&mut self, actor: u32) -> Result<()>;
    /// "failed to add structure %s" (`0x404F60`).
    fn log_add_failed(&mut self, actor: u32) -> Result<()>;
}

/// `0x586B40(actor)` AddStructure: a null actor fails silently; an actor with
/// a StructureComponent is appended while the count is below the
/// ActorData+0x464 limit of the settlement level, then attached; otherwise
/// the failure is logged.
pub fn add_settlement_structure(
    settlement: &mut SettlementStructures,
    actor: Option<u32>,
    structure_component: bool,
    limit: i32,
    world: &mut impl SettlementSlots,
) -> Result<bool> {
    let Some(actor) = actor else {
        return Ok(false);
    };
    let count = i32::try_from(settlement.structures.len())
        .map_err(|_| Error::Invalid("settlement structure count exceeds i32"))?;
    if structure_component && count < limit {
        settlement.structures.push(actor);
        world.attach_structure(actor)?;
        return Ok(true);
    }
    world.log_add_failed(actor)?;
    Ok(false)
}
