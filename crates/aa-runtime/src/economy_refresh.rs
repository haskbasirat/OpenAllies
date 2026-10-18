//! Original570020 resource contribution refresh on the shared Economy component.
//! Kingdom arithmetic uses the real aa-economy state; source/organization value
//! getters remain explicit services implemented by the native value adapter.
use aa_actors::{
    ActorId,
    ground_states::{EconomyState, ResourceVector},
};
use aa_body::F32;
use aa_economy::{Economy, F32Bits};
use aa_numeric::Precision;
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Economy(#[from] aa_economy::EconomyError),
    #[error("resource refresh service: {0}")]
    World(String),
    #[error("resource vector allocation failed")]
    Allocation,
}
pub type Result<T> = std::result::Result<T, Error>;
pub trait World {
    type Kingdom: Copy;
    fn resource_count(&self) -> usize;
    /// Virtual+108; refresh captures its first result for later ADD operations.
    /// The removal helpers re-query instead of reusing that captured Kingdom.
    fn owner(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
    ) -> Result<Option<Self::Kingdom>>;
    fn upkeep_blocked(&mut self, actor: ActorId, component: &mut EconomyState) -> Result<bool>;
    fn upkeep_values(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
        pc: Precision,
    ) -> Result<Vec<F32>>;
    fn provides(&mut self, actor: ActorId, component: &mut EconomyState) -> Result<bool>;
    fn provided_values(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
        write_member_cache: bool,
        pc: Precision,
    ) -> Result<Vec<F32>>;
    fn cost_values(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
        pc: Precision,
    ) -> Result<Vec<F32>>;
    fn kingdom_economy(&mut self, kingdom: Self::Kingdom) -> Result<&mut Economy>;
    /// Native4E48C0 over the actual owner's consumption arrays. The default
    /// preserves existing live Economy owners; saved loading can override it
    /// to borrow the canonical Kingdom arrays without a detached copy/reset.
    fn adjust_consumption(
        &mut self,
        kingdom: Self::Kingdom,
        values: &[F32Bits],
        add: bool,
    ) -> Result<()> {
        self.kingdom_economy(kingdom)?.adjust_consumption(values, add)?;
        Ok(())
    }
    /// Native4E4930, in ascending resource order, with the same owner captured
    /// by570020. Removal helpers still re-query the owner before calling this.
    fn adjust_providers(
        &mut self,
        kingdom: Self::Kingdom,
        values: &[F32Bits],
        add: bool,
    ) -> Result<()> {
        self.kingdom_economy(kingdom)?.adjust_providers(values, add)?;
        Ok(())
    }
    fn parent_organization(
        &mut self,
        actor: ActorId,
        component: &mut EconomyState,
    ) -> Result<Option<ActorId>>;
    /// Original4CF0F0 (Organization's own Economy refresh), not a UI notification.
    fn refresh_parent(
        &mut self,
        actor: ActorId,
        current: &mut EconomyState,
        pc: Precision,
    ) -> Result<()>;
}
fn clear(v: &mut ResourceVector, count: usize) -> Result<()> {
    if count > v.values.len() {
        v.values
            .try_reserve_exact(count - v.values.len())
            .map_err(|_| Error::Allocation)?;
    }
    v.values.resize(count, F32::ZERO);
    v.values.fill(F32::ZERO);
    Ok(())
}
fn assign(target: &mut ResourceVector, values: Vec<F32>) {
    target.values = values;
}
fn words(values: &[F32]) -> Vec<F32Bits> {
    values.iter().map(|v| F32Bits(v.0)).collect()
}
/// Original4CCE70 uses numeric equality: signed zeros compare equal and NaNs
/// do not; bit equality alone would cause unnecessary remove/re-add rounding.
pub fn equal(a: &[F32], b: &[F32]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.0 & 0x7fffffff <= 0x7f800000
                && b.0 & 0x7fffffff <= 0x7f800000
                && (a.0 == b.0 || (a.0 | b.0) & 0x7fffffff == 0)
        })
}
/// Native56F310 clears the flag BEFORE querying current owner. When owner is
/// null, it retains the prior vector; this matters during ownership changes.
pub fn remove_upkeep(component: &mut EconomyState, world: &mut impl World) -> Result<()> {
    component.flag_11 = false;
    if let Some(owner) = world.owner(component.header.owner, component)? {
        world.adjust_consumption(owner, &words(&component.resources[1].values), false)?;
        clear(&mut component.resources[1], world.resource_count())?;
    }
    Ok(())
}
pub fn remove_provided(component: &mut EconomyState, world: &mut impl World) -> Result<()> {
    component.flag_10 = false;
    if let Some(owner) = world.owner(component.header.owner, component)? {
        world.adjust_providers(owner, &words(&component.resources[0].values), false)?;
        clear(&mut component.resources[0], world.resource_count())?;
    }
    Ok(())
}
pub fn refresh(component: &mut EconomyState, world: &mut impl World, pc: Precision) -> Result<()> {
    let Some(captured_owner) = world.owner(component.header.owner, component)? else {
        return Ok(());
    };
    if world.upkeep_blocked(component.header.owner, component)? {
        if component.flag_11 {
            remove_upkeep(component, world)?;
        }
    } else {
        let values = world.upkeep_values(component.header.owner, component, pc)?;
        if !component.flag_11 || !equal(&component.resources[1].values, &values) {
            if component.flag_11 {
                remove_upkeep(component, world)?;
            }
            assign(&mut component.resources[1], values);
            world.adjust_consumption(captured_owner, &words(&component.resources[1].values), true)?;
            component.flag_11 = true;
        }
    }
    if world.provides(component.header.owner, component)? {
        clear(&mut component.resources[3], world.resource_count())?;
        let values = if world.provides(component.header.owner, component)? {
            world.provided_values(component.header.owner, component, true, pc)?
        } else {
            vec![F32::ZERO; world.resource_count()]
        };
        if !component.flag_10 || !equal(&component.resources[0].values, &values) {
            if component.flag_10 {
                remove_provided(component, world)?;
            }
            assign(&mut component.resources[0], values);
            world.adjust_providers(captured_owner, &words(&component.resources[0].values), true)?;
            component.flag_10 = true;
        }
    } else if component.flag_10 {
        remove_provided(component, world)?;
    }
    let values = world.cost_values(component.header.owner, component, pc)?;
    assign(&mut component.resources[2], values);
    if let Some(parent) = world.parent_organization(component.header.owner, component)? {
        world.refresh_parent(parent, component, pc)?;
    }
    Ok(())
}
