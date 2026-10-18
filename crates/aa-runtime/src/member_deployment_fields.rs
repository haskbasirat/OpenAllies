//! Native4FACA0 over the caller's same mutable ActorParams.
use aa_actors::actor_creation::ActorParams;
use aa_numeric::{Binary, Precision};

pub type Result<T> = std::result::Result<T, String>;

/// Every service borrows the actual current owner. No successful service
/// default is supplied for a positive shape, actor creation or Mind command.
pub trait World<D, L> {
    type Actor: Copy;
    type Definition;
    fn definition(&mut self, actor: Self::Actor) -> Result<Self::Definition>;
    fn enabled_3ec(&mut self, definition: &Self::Definition) -> Result<bool>;
    /// Only3DC/E0/E4/E8 are requested, in the actual native read order.
    fn source_word(&mut self, definition: &Self::Definition, offset: u32) -> Result<u32>;
    /// Current Thing words20/24/2C/30, never a copied viewer transform.
    fn actor_word(&mut self, actor: Self::Actor, offset: u32) -> Result<u32>;
    /// Actor virtualC supplies the actual shape;46EC70 consumes the point.
    fn contains_46ec70(&mut self, actor: Self::Actor, point: [u32; 2]) -> Result<bool>;
    ///Fresh actor+4 Data+8 IDS in the actual404ED0 warning atlineE81.
    fn warn_inside_404ed0(&mut self, actor: Self::Actor) -> Result<()>;
    fn create_4f9c10(&mut self, params: &mut ActorParams<D, L>) -> Result<Option<Self::Actor>>;
    /// Current new member+70 virtual198(point, original structure).
    fn mind_deploy_198(
        &mut self,
        member: Self::Actor,
        point: [u32; 2],
        structure: Self::Actor,
    ) -> Result<bool>;
    ///Fresh member+4 Data+8 IDS and member+14 GTID in404F60 atlineEA5.
    fn log_failed_404f60(&mut self, member: Self::Actor) -> Result<()>;
    ///4F6760(member,0); do not substitute dropping or deregistering a shell.
    fn destroy_4f6760(&mut self, member: Self::Actor, flag: u8) -> Result<()>;
}

fn n(v: aa_numeric::Result<Binary>) -> Result<Binary> {
    v.map_err(|e| e.to_string())
}
fn b(bits: u32) -> Result<Binary> {
    n(Binary::from_f32_bits(bits))
}
fn st(value: Binary) -> Result<u32> {
    value.store_f32().map_err(|e| e.to_string())
}

///4FAD28..57/59..7D. Both products and sum/subtraction remain extended
///until the two actual binary32 stores; no fused operation is introduced.
fn rotate(point: [u32; 2], facing: [u32; 2], pc: Precision) -> Result<[u32; 2]> {
    let a = b(point[0])?;
    let z = b(point[1])?;
    let c = b(facing[0])?;
    let s = b(facing[1])?;
    let x = n(n(z.mul(&c, pc))?.add(&n(s.mul(&a, pc))?, pc))?;
    let y = n(n(z.mul(&s, pc))?.sub(&n(a.mul(&c, pc))?, pc))?;
    Ok([st(x)?, st(y)?])
}
///409230. Equality/below1e-4 clears both components. This finite arithmetic
///domain does not manufacture values for NaNs or infinities.
fn normalize(input: [u32; 2], pc: Precision) -> Result<[u32; 2]> {
    let x = b(input[0])?;
    let y = b(input[1])?;
    let length = n(n(n(x.mul(&x, pc))?.add(&n(y.mul(&y, pc))?, pc))?.sqrt(pc))?;
    if length.compare(&b(0x38d1_b717)?) != std::cmp::Ordering::Greater {
        return Ok([0, 0]);
    }
    let scale = n(Binary::from_u32(1).div(&length, pc))?;
    Ok([st(n(scale.mul(&x, pc))?)?, st(n(scale.mul(&y, pc))?)?])
}

///46EC70 on the actual shape returned by actor virtualC. Circle compares
///radius squared against y*y+x*x+binary32(0.0001); rectangles call46E400
///with radius0 and include their boundary. Unknown kinds returnfalse before
///reading coordinates. Arithmetic is qualified to finite source operands.
pub fn contains_46ec70(
    shape: &aa_spatial::collision::Shape,
    point: [u32; 2],
    pc: Precision,
) -> Result<bool> {
    match shape.kind {
        0 => Ok(point[0] & 0x7fff_ffff == 0 && point[1] & 0x7fff_ffff == 0),
        1 => {
            let y = b(point[1])?;
            let x = b(point[0])?;
            let distance =
                n(n(n(y.mul(&y, pc))?.add(&n(x.mul(&x, pc))?, pc))?.add(&b(0x38d1_b717)?, pc))?;
            let radius = b(shape.radius_bits)?;
            Ok(n(radius.mul(&radius, pc))?.compare(&distance) != std::cmp::Ordering::Less)
        }
        2 | 3 => {
            let x = b(point[0])?;
            let y = b(point[1])?;
            Ok(x.compare(&b(shape.bounds[0])?) != std::cmp::Ordering::Less
                && x.compare(&b(shape.bounds[2])?) != std::cmp::Ordering::Greater
                && y.compare(&b(shape.bounds[1])?) != std::cmp::Ordering::Less
                && y.compare(&b(shape.bounds[3])?) != std::cmp::Ordering::Greater)
        }
        _ => Ok(false),
    }
}

pub fn deploy<W: World<D, L>, D, L>(
    world: &mut W,
    params: &mut ActorParams<D, L>,
    structure: W::Actor,
    pc: Precision,
) -> Result<Option<W::Actor>> {
    let first = world.definition(structure)?;
    if !world.enabled_3ec(&first)? {
        return Ok(None);
    }
    let point = [
        world.source_word(&first, 0x3e4)?,
        world.source_word(&first, 0x3e8)?,
    ];
    if world.contains_46ec70(structure, point)? {
        world.warn_inside_404ed0(structure)?;
        return Ok(None);
    }
    // The shape callback may change the actor's current definition or pose.
    let py = world.actor_word(structure, 0x24)?;
    let facing = [
        world.actor_word(structure, 0x2c)?,
        world.actor_word(structure, 0x30)?,
    ];
    let current = world.definition(structure)?;
    let start_x = world.source_word(&current, 0x3dc)?;
    let px = world.actor_word(structure, 0x20)?;
    let start_y = world.source_word(&current, 0x3e0)?;
    let start = rotate([start_x, start_y], facing, pc)?;
    let end = rotate(
        [
            world.source_word(&current, 0x3e4)?,
            world.source_word(&current, 0x3e8)?,
        ],
        facing,
        pc,
    )?;
    let dy = n(b(end[1])?.sub(&b(start[1])?, pc))?;
    let dx = n(b(end[0])?.sub(&b(start[0])?, pc))?;
    let direction = normalize([st(dx)?, st(dy)?], pc)?;

    // Native stores Z before the additions, then copies the resulting point
    //and normalized direction into this SAME caller-owned parameter block.
    params.position_bits[2] = 0;
    let y = n(b(start[1])?.add(&b(py)?, pc))?;
    let x = n(b(start[0])?.add(&b(px)?, pc))?;
    params.position_bits = [st(x)?, st(y)?, 0];
    params.direction_bits[1] = direction[1];
    params.direction_bits[0] = direction[0];
    params.byte_52 = 1;
    let Some(member) = world.create_4f9c10(params)? else {
        return Ok(None);
    };

    // This arithmetic is conditional on creation success. It uses the actual
    //pre-create captured position/rotated endpoint, including callback edits
    //to the request only where native rereads that request in child4F9C10.
    let y = n(b(end[1])?.add(&b(py)?, pc))?;
    let x = n(b(end[0])?.add(&b(px)?, pc))?;
    if world.mind_deploy_198(member, [st(x)?, st(y)?], structure)? {
        return Ok(Some(member));
    }
    world.log_failed_404f60(member)?;
    world.destroy_4f6760(member, 0)?;
    Ok(None)
}

#[cfg(test)]
#[path = "member_deployment_fields_tests.rs"]
mod tests;
