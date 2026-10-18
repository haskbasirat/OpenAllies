//! Original non-queue purchases and refunds.
//!
//! * Denizen purchases `0x56D5A0` (air attack on a target), `0x56D7C0` (air
//!   attack at a location) and `0x56DA00` (air recon at a location): copy the
//!   ActorData cost (+0x3F0, +0x3F0, +0x3FC), `0x4E20A0`, telemetry and message
//!   when unaffordable; otherwise create the denizen (`0x56CC10(mode,1,1)`) and
//!   debit only when creation succeeded, then use it (virtual +0x10 with mode
//!   0x10 or 0x40). The owner kingdom is not null-checked natively.
//! * HQ attach cost `0x5735D0`: organization cost (`0x56FA30`) scaled by
//!   `*(cache+0xC0)` with `0x4CD770` (binary32 product per row, rows beyond the
//!   source count stay zero in the registry-length result).
//! * HQ attach gate `0x574A10` and attach `0x574C10` (second quote debited
//!   without another check, attachment record, organization attach, event 0x3C).
//! * Transport unload failure refund `0x58BBA0`.
use crate::{Error, Result};
use aa_economy::{F32Bits, X87Precision};
use serde::{Deserialize, Serialize};

pub trait Purchase {
    /// Actor virtual +0x108.
    fn purchase_owner(&mut self) -> Result<Option<u32>>;
    fn purchase_can_afford(&mut self, kingdom: u32, cost: &[u32]) -> Result<bool>;
    fn purchase_debit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()>;
    fn purchase_credit(&mut self, kingdom: u32, cost: &[u32]) -> Result<()>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DenizenPurchase {
    /// `0x56D5A0(target)`, cost ActorData+0x3F0, use mode 0x10.
    AirAttackTarget,
    /// `0x56D7C0(x, y)`, cost ActorData+0x3F0, use mode 0x10.
    AirAttackLocation,
    /// `0x56DA00(x, y)`, cost ActorData+0x3FC, use mode 0x40.
    AirRecon,
}
impl DenizenPurchase {
    pub fn creation_mode(self) -> u32 {
        match self {
            Self::AirRecon => 0x40,
            _ => 0x10,
        }
    }
}

pub trait Denizen: Purchase {
    fn denizen_cost(&mut self, purchase: DenizenPurchase) -> Result<Vec<u32>>;
    /// Telemetry "Insufficient stockpile" (`0x77DC50`) and denizen_error_affording.
    fn denizen_unaffordable(
        &mut self,
        purchase: DenizenPurchase,
        cost: &[u32],
        kingdom: u32,
    ) -> Result<()>;
    /// `0x56CC10(mode, 1, 1)`.
    fn create_denizen(&mut self, purchase: DenizenPurchase, mode: u32) -> Result<Option<u32>>;
    /// Ability context (`0x4CA330`/`0x4CA760`) and created virtual +0x10(mode).
    fn use_denizen(&mut self, purchase: DenizenPurchase, created: u32, mode: u32) -> Result<()>;
}

/// `0x56D5A0` / `0x56D7C0` / `0x56DA00`. Returns true whenever affordable,
/// even if creation failed (nothing is debited then).
pub fn purchase_denizen(purchase: DenizenPurchase, world: &mut impl Denizen) -> Result<bool> {
    let kingdom = world.purchase_owner()?.ok_or(Error::Invalid(
        "denizen purchase dereferences the owner kingdom without a null check",
    ))?;
    let cost = world.denizen_cost(purchase)?;
    if !world.purchase_can_afford(kingdom, &cost)? {
        world.denizen_unaffordable(purchase, &cost, kingdom)?;
        return Ok(false);
    }
    let mode = purchase.creation_mode();
    if let Some(created) = world.create_denizen(purchase, mode)? {
        world.purchase_debit(kingdom, &cost)?;
        world.use_denizen(purchase, created, mode)?;
    }
    Ok(true)
}

/// `0x5735D0` with `0x4CD770`: `out[i] = binary32(PC(factor * cost[i]))` for
/// the source rows; the result has `registry_count` rows (zero-filled).
pub fn hq_attach_cost(
    organization_cost: &[u32],
    factor: u32,
    registry_count: usize,
    precision: X87Precision,
) -> Result<Vec<u32>> {
    let mut out = vec![0u32; registry_count.max(organization_cost.len())];
    for (row, &value) in organization_cost.iter().enumerate() {
        out[row] = aa_economy::transfers::scale(F32Bits(factor), F32Bits(value), precision)?.0;
    }
    Ok(out)
}

pub trait HqAttach: Purchase {
    /// `0x56FA30(organization)` cost vector.
    fn organization_cost(&mut self, organization: u32) -> Result<Vec<u32>>;
    /// `*(cache+0xC0)` attach cost multiplier bits.
    fn attach_cost_factor(&self) -> u32;
    fn registry_count(&self) -> usize;
    fn precision(&self) -> X87Precision;
    /// hq_error_attach_affording|ss through `0x40C230`.
    fn attach_unaffordable(&mut self, organization: u32, cost: &[u32], kingdom: u32) -> Result<()>;
    /// `0x574810(organization ActorData, notify)`.
    fn attach_allowed(&mut self, organization: u32, notify: bool) -> Result<bool>;
    /// `0x573790` record stored through `0x574260(slot)`.
    fn create_attachment(&mut self, organization: u32, slot: u32) -> Result<()>;
    /// `organization+0x7C -> 0x5783B0(actor)`.
    fn attach_to_organization(&mut self, organization: u32) -> Result<()>;
    /// `0x42D470(0x3C, ...)` attach notification (only when paid).
    fn attach_event(&mut self, organization: u32) -> Result<()>;
}

/// `0x574A10(organization, check_cost, notify)`; `organization` is `None` for
/// a null pointer or a missing Organization component (+0x7C).
pub fn hq_attach_affordable(
    organization: Option<u32>,
    check_cost: bool,
    notify: bool,
    world: &mut impl HqAttach,
) -> Result<bool> {
    let Some(organization) = organization else {
        return Ok(false);
    };
    if check_cost {
        let cost = hq_attach_cost(
            &world.organization_cost(organization)?,
            world.attach_cost_factor(),
            world.registry_count(),
            world.precision(),
        )?;
        if let Some(kingdom) = world.purchase_owner()?
            && !world.purchase_can_afford(kingdom, &cost)?
        {
            if notify {
                world.attach_unaffordable(organization, &cost, kingdom)?;
            }
            return Ok(false);
        }
    }
    world.attach_allowed(organization, notify)
}

/// `0x574C10(organization, pay, slot)`.
pub fn hq_attach(
    organization: Option<u32>,
    pay: bool,
    slot: u32,
    world: &mut impl HqAttach,
) -> Result<bool> {
    if !hq_attach_affordable(organization, pay, false, world)? {
        return Ok(false);
    }
    let organization = organization.ok_or(Error::Invalid("attach without organization"))?;
    if pay {
        let cost = hq_attach_cost(
            &world.organization_cost(organization)?,
            world.attach_cost_factor(),
            world.registry_count(),
            world.precision(),
        )?;
        if let Some(kingdom) = world.purchase_owner()? {
            world.purchase_debit(kingdom, &cost)?;
        }
    }
    world.create_attachment(organization, slot)?;
    world.attach_to_organization(organization)?;
    if pay {
        world.attach_event(organization)?;
    }
    Ok(true)
}

/// TransportComponent payment fields (component 24 at actor+0xC0).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportPayment {
    /// +0x38: no refund.
    pub no_refund: bool,
    /// +8 paid cost and +0x18 upkeep vectors.
    pub cost: Vec<u32>,
    pub upkeep: Vec<u32>,
}

pub trait TransportRefund: Purchase {
    /// transport_error_unloading|s through `0x40C230` (message kingdom reread).
    fn unloading_failed_message(&mut self, kingdom: Option<u32>) -> Result<()>;
    /// `0x4CF0F0` economy refresh of the transport actor.
    fn refresh_economy(&mut self) -> Result<()>;
}

/// `0x58BBA0`: refund (when allowed and owned), then both vectors become zeroed
/// registry-length vectors (`0x4CD250`) and the economy is refreshed.
pub fn transport_unload_refund(
    transport: &mut TransportPayment,
    registry_count: usize,
    world: &mut impl TransportRefund,
) -> Result<()> {
    if !transport.no_refund
        && let Some(kingdom) = world.purchase_owner()?
    {
        let message_kingdom = world.purchase_owner()?;
        world.unloading_failed_message(message_kingdom)?;
        world.purchase_credit(kingdom, &transport.cost)?;
    }
    transport.upkeep = vec![0; registry_count];
    transport.cost = vec![0; registry_count];
    world.refresh_economy()
}
