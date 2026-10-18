//! Native5705E0 element notification and57C800/57C680 organization goal updates.
//! Component state stays in the owning runtime; no parallel position cache.
use crate::{
    ActorId,
    ground_states::{ElementState, OrganizationState},
};
use aa_body::F32;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("organization position world failed: {0}; earlier effects remain committed")]
    World(String),
    #[error("native organization callback removed the required Mind")]
    MissingMind,
}
pub type Result<T> = std::result::Result<T, Error>;
#[derive(Clone, Copy, Debug)]
pub struct Actor {
    pub flags: u32,
    pub position: [F32; 2],
    pub mind: Option<u32>,
}
pub trait World {
    type Definition;
    fn world_mode(&self) -> Result<u32>;
    fn actor(&self, id: ActorId) -> Result<Actor>;
    fn organization(&self, id: ActorId) -> Result<&OrganizationState<Self::Definition>>;
    fn organization_mut(&mut self, id: ActorId)
    -> Result<&mut OrganizationState<Self::Definition>>;
    /// Actual typed actor reference corresponding to Organization+40. This is
    /// not a cast of a saved pointer or a guess from member-list ordering.
    fn selected_member(&self, id: ActorId) -> Result<Option<ActorId>>;
    /// Actual Mind virtual+64 or+7C, preserving fresh component lookup between.
    fn mind_predicate(&mut self, mind: u32, slot: u32) -> Result<bool>;
    /// Full actor4EEB40 relocation through SpatialIndex::move_ground and its
    /// required position callbacks. Used immediately for runtime flag30 actors.
    fn relocate_owner(&mut self, actor: ActorId, point: [F32; 2]) -> Result<()>;
    /// Actual57C080 interpolation service; never assume success or clear terrain.
    fn interpolate(&mut self, organization: ActorId) -> Result<bool>;
    ///50B220 on this Organization Updatable, event0/data1.
    fn has_scheduled(&mut self, organization: ActorId, event: u32, data: u32) -> Result<bool>;
    ///50AC00 on this Organization Updatable, data1.
    fn schedule(&mut self, organization: ActorId, data: u32) -> Result<()>;
}
fn equal(a: u32, b: u32) -> bool {
    let nan = |x: u32| x & 0x7fff_ffff > 0x7f80_0000;
    !nan(a) && !nan(b) && (a == b || a & 0x7fff_ffff == 0 && b & 0x7fff_ffff == 0)
}
fn same(a: [F32; 2], b: [F32; 2]) -> bool {
    equal(a[0].0, b[0].0) && equal(a[1].0, b[1].0)
}
fn active(organization: ActorId, world: &mut impl World) -> Result<bool> {
    let owner = world.organization(organization)?.header.owner;
    let Some(mind) = world.actor(owner)?.mind else {
        return Ok(false);
    };
    if world.mind_predicate(mind, 0x64)? {
        return Ok(true);
    }
    let owner = world.organization(organization)?.header.owner;
    let mind = world.actor(owner)?.mind.ok_or(Error::MissingMind)?;
    world.mind_predicate(mind, 0x7c)
}
///5705E0. The element's cached owner is the member notification argument.
pub fn element_moved<D>(
    element: &ElementState<D>,
    world: &mut impl World<Definition = D>,
) -> Result<()> {
    if let Some(organization) = element.organization {
        organization_moved(organization, element.header.owner, world)?;
    }
    Ok(())
}
///57C800. Only the selected member can advance an organization's goal; world
/// mode6 suppresses notifications, and dual flag30 bypasses Mind predicates.
pub fn organization_moved(
    organization: ActorId,
    member: ActorId,
    world: &mut impl World,
) -> Result<()> {
    if world.world_mode()? == 6 || world.selected_member(organization)? != Some(member) {
        return Ok(());
    }
    let owner = world.organization(organization)?.header.owner;
    let direct =
        world.actor(owner)?.flags & (1 << 30) != 0 && world.actor(member)?.flags & (1 << 30) != 0;
    if !direct {
        if !active(organization, world)? {
            return Ok(());
        }
        let owner = world.organization(organization)?.header.owner;
        let selected = world
            .selected_member(organization)?
            .ok_or_else(|| Error::World("selected member became null".into()))?;
        if same(
            world.actor(owner)?.position,
            world.actor(selected)?.position,
        ) {
            return Ok(());
        }
    }
    // Native EDI keeps the original notified member after virtual callbacks.
    set_position(organization, world.actor(member)?.position, world)
}
///57C680 and578E10. Store the changed goal even when interpolation cannot begin;
/// only a previously reached goal starts a new interpolation/scheduler request.
pub fn set_position(organization: ActorId, point: [F32; 2], world: &mut impl World) -> Result<()> {
    let owner = world.organization(organization)?.header.owner;
    if world.actor(owner)?.flags & (1 << 30) != 0 {
        world.organization_mut(organization)?.position_xy = point;
        let owner = world.organization(organization)?.header.owner;
        return world.relocate_owner(owner, point);
    }
    if !active(organization, world)? || same(point, world.organization(organization)?.position_xy) {
        return Ok(());
    }
    let owner = world.organization(organization)?.header.owner;
    let reached = same(
        world.actor(owner)?.position,
        world.organization(organization)?.position_xy,
    );
    world.organization_mut(organization)?.position_xy = point;
    if reached && !world.interpolate(organization)? && !world.has_scheduled(organization, 0, 1)? {
        world.schedule(organization, 1)?;
    }
    Ok(())
}
