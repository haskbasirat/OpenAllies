//! Native4EA0F0 soft-shape descriptor and source-first caller range boundaries.
//! Soft actor virtual+10 is distinct from target-shape virtual+14. The ordinary
//! getter4EA020 returns definition+50 without visual/object scale changes.
use crate::{Result, collision::Shape, geometry::Placement, range::ActorTransform};
use aa_numeric::Precision;

pub trait World {
    /// A stable runtime object identity, not necessarily a32-bit GTID.
    type Actor: Copy;
    type ShapeHandle;
    /// Coherent native actor XY/facing fields before the virtual getter.
    fn snapshot_transform(&self, actor: Self::Actor) -> Result<ActorTransform>;
    /// Actual virtual+10; ordinary actors return their source definition+50.
    /// Derived getter effects are visible to the following descriptor capture.
    fn soft_shape(&mut self, actor: Self::Actor) -> Result<Self::ShapeHandle>;
    /// Resolve the retained identity only after both getters. A later getter
    /// may mutate an earlier shape through an alias; an early clone is wrong.
    fn read_shape(&self, handle: &Self::ShapeHandle) -> Result<Shape>;
}
#[derive(Clone, Debug)]
pub struct Descriptor<H> {
    pub shape: H,
    pub transform: ActorTransform,
}
///4EA0F0, retaining shape identity and the transform captured before its getter.
pub fn capture<W: World>(actor: W::Actor, world: &mut W) -> Result<Descriptor<W::ShapeHandle>> {
    let transform = world.snapshot_transform(actor)?;
    let shape = world.soft_shape(actor)?;
    Ok(Descriptor { shape, transform })
}
fn placement<W: World>(descriptor: &Descriptor<W::ShapeHandle>, world: &W) -> Result<Placement> {
    Ok(Placement {
        shape: world.read_shape(&descriptor.shape)?,
        facing: descriptor.transform.facing,
        position: descriptor.transform.position,
    })
}
/// The52FE60 soft-shape branch: capture source then target, and pass target as
/// the46F100 receiver. This does not include52FE60's preceding arrival/ID gates.
/// Earlier getter effects remain committed if a later lookup/comparison fails.
pub fn source_first_in_range<W: World>(
    source: W::Actor,
    target: W::Actor,
    range_bits: u32,
    precision: Precision,
    world: &mut W,
) -> Result<bool> {
    let source = capture(source, world)?;
    let target = capture(target, world)?;
    crate::range::shapes_in_range(
        &placement(&target, world)?,
        &placement(&source, world)?,
        range_bits,
        false,
        precision,
    )
}

/// The5774A0 Mover target branch (577530..57757D): capture source then target,
/// but pass SOURCE as the46F100 receiver. This is not interchangeable with the
///52FE60 arrival branch or4EE800's target-first virtual+14 descriptors.
/// The caller owns the preceding target validity gate and later movement.
pub fn source_receiver_in_range<W: World>(
    source: W::Actor,
    target: W::Actor,
    range_bits: u32,
    precision: Precision,
    world: &mut W,
) -> Result<bool> {
    let source = capture(source, world)?;
    let target = capture(target, world)?;
    crate::range::shapes_in_range(
        &placement(&source, world)?,
        &placement(&target, world)?,
        range_bits,
        false,
        precision,
    )
}
