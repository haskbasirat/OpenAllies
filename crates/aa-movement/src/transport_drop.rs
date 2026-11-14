//! Original TransportComponent drop (actor +0xC0, vtable 0xA40100): one
//! thing out per call (`0x58BCB0`), the lead distance of the cargo
//! (`0x58B180`), the loaded mark (`0x58B160`) and the plane's exit
//! (`0x58B130`).
//!
//! The control flow, the branch order, every read of plane, thing,
//! definition and layout memory, and the binary32 store points follow the
//! original instructions. The arithmetic it does itself is ported here:
//! the drop offset rotation `0x46DA80`, the vector2 normalisation
//! `0x409230`, the fall carry `0x577380` ([`launch_carry`]: its NaN exit around
//! `crate::mover::launch_speed`), the
//! map interior clamp `0x552640` (`crate::falling::clamp_to_interior`) and
//! the sums around them. Every other call is a required world callback,
//! invoked in native order with the native arguments: the placement searches
//! `0x554BF0` / `0x554500` / `0x554850`, creation `0x4F9C10` / `0x4FACA0`,
//! the organization calls on Organization +0x7C, the pose commit `0x576BB0`
//! and the aimed fall `0x5770A0` (the host runs `crate::falling::commit_pose`
//! / `begin_aimed_fall` on the thing's Mover there), and the messages.
//!
//! Nothing here is transactional: a callback error stops the drop at that
//! call with the earlier writes kept, as the original would have them.
use crate::falling::{MapBounds, clamp_to_interior};
use crate::mover::{bin, launch_speed, store};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// A native object (pointer in the original); 0 is null. Values are host keys.
pub type Ref = u32;
/// A generation handle (`0x432E40` / `0x401060` resolve it); 0 is none.
pub type Handle = u32;

/// `[9FD59C]`: the plane's actor virtual +0 is scaled by this for a member's
/// ground placement radius.
pub const MEMBER_RADIUS_SCALE: f32 = 4.0;
/// The `0x554BF0` radius pushed for an organization's first placement.
pub const ORGANIZATION_RADIUS: f32 = 4.0;
/// `0x4E2A60`'s second argument and `0x57D950`'s last one.
pub const ZERO_ARGUMENT: u32 = 0;
/// `0x554850`'s scale and count arguments (`1.0f`, `0x10`).
pub const LANDING_SCALE: f32 = 1.0;
pub const LANDING_COUNT: u32 = 0x10;

/// The TransportComponent fields the drop reads and writes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Transport {
    /// +4, the carrying actor.
    pub plane: Ref,
    /// +0x28, the cargo definition.
    pub cargo: Ref,
    /// +0x2C, the organization layout; 0 is a single thing.
    pub layout: Ref,
    /// +0x30 / +0x34.
    pub point: [f32; 2],
    /// +0x38 byte (set by `0x58B160`).
    pub loaded: u8,
    /// +0x3C.
    pub count: i32,
    /// +0x40.
    pub dropped: i32,
    /// +0x44, the organization made by the first drop.
    pub organization: Handle,
    /// +0x48, the next layout slot.
    pub next_slot: i32,
    /// +0x58 string (opaque to the drop; assigned into the creation block).
    pub name: Ref,
    /// +0x5C handle.
    pub attachment: Handle,
}

/// Actor memory the drop reads.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ActorView {
    /// +4.
    pub definition: Ref,
    /// +8; bit 12 selects the airborne branches.
    pub flags: u32,
    /// +0x20..+0x28.
    pub position: [f32; 3],
    /// +0x2C..+0x34.
    pub facing: [f32; 3],
    /// +0x88, tested for zero only (plane).
    pub word_88: u32,
}

/// ActorDefinition memory the drop reads.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DefinitionView {
    /// +0x10, the name passed to the failure message.
    pub name: Ref,
    /// +0x24, subtracted from the plane height for an airborne drop.
    pub height_24: f32,
    /// +0x1A4: `0x577380`'s subtrahend and `0x5770A0`'s drag argument.
    pub field_1a4: f32,
    /// +0x3E4 / +0x3E8 (plane definition), rotated by the facing (`0x46DA80`).
    pub drop_offset: [f32; 2],
    /// +0x3EC byte (plane definition): a ground transport unloads (`0x4FACA0`).
    pub byte_3ec: u8,
}

/// The organization layout: `+8 -> +0x18` leader slot and `+8 -> +0x2C`
/// count; slot definitions are `(+0xC)[(+8 -> +0x28)[slot]]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutView {
    pub leader_slot: i32,
    pub count: i32,
}

/// The actor creation block (0x54 bytes, `0x4C8C20` + `0x4C8C70` defaults),
/// with the fields the drop writes.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CreateParams {
    /// +4.
    pub definition: Ref,
    /// +8..+0x10 (`0x4BE740`).
    pub position: [f32; 3],
    /// +0x14 / +0x18 (defaults 0, -1.0).
    pub facing: [f32; 2],
    /// +0x1C: `None` is the constructor's empty string, `Some` the
    /// Transport +0x58 name assigned by `0x405B80`.
    pub name: Option<Ref>,
    /// +0x20 (default -1).
    pub variant: u32,
    /// +0x28.
    pub kingdom: Ref,
    /// +0x2C.
    pub layout: Ref,
    /// +0x34 (default -1).
    pub word_34: u32,
    /// +0x40.
    pub organization: Handle,
    /// +0x50.
    pub byte_50: u8,
}
impl Default for CreateParams {
    fn default() -> Self {
        Self {
            definition: 0,
            position: [0.0; 3],
            facing: [0.0, -1.0],
            name: None,
            variant: u32::MAX,
            kingdom: 0,
            layout: 0,
            word_34: u32::MAX,
            organization: 0,
            byte_50: 0,
        }
    }
}

/// The Move command `0x4CA760` builds (type 5) and `0x4D9B60` sends.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MoveCommand {
    /// +0x18 / +0x1C: the Transport point.
    pub point: [f32; 2],
    /// +0x28: Organization virtual +0x104, stored to binary32.
    pub value: f32,
    /// +0x20 / +0x24: the static zero vector `[B279C0]` / `[B279C4]`.
    pub vector: [f32; 2],
    /// +0x3C byte.
    pub byte_3c: u8,
}

/// The two Updatables the loaded mark schedules (`0x4CD250`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Updatable {
    /// Transport +8.
    At08,
    /// Transport +0x18.
    At18,
}

/// Which failure message the drop posts (`0x40C230` type 0x11 with the
/// definition name through `0x4B9F40` key `[B43B48]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Failure {
    /// `0x554BF0` found no place for the organization.
    OrganizationPlacement,
    /// `0x4F9C10` made no organization.
    OrganizationCreation,
    /// `0x554500` found no place for a single thing.
    ThingPlacement,
    /// `0x554500` found no place for a member.
    MemberPlacement,
}

/// Native reads and calls, in the order the drop makes them.
pub trait TransportWorld {
    fn actor(&mut self, actor: Ref) -> Result<ActorView>;
    fn definition(&mut self, definition: Ref) -> Result<DefinitionView>;
    fn layout(&mut self, layout: Ref) -> Result<LayoutView>;
    /// The definition of a layout slot (0: none). The slot search reads one
    /// entry past the count when every later slot is empty.
    fn layout_definition(&mut self, layout: Ref, slot: i32) -> Result<Ref>;
    /// `0x432E40`: the object, 0 when the handle is stale.
    fn resolve(&mut self, handle: Handle) -> Result<Ref>;
    /// `0x401060`.
    fn handle_valid(&mut self, handle: Handle) -> Result<bool>;
    /// Object +0x14 (`0x4CF1B0`).
    fn handle_of(&mut self, object: Ref) -> Result<Handle>;
    /// Organization +0x7C -> +0x28 member array at the slot.
    fn organization_member(&mut self, organization: Ref, slot: i32) -> Result<Ref>;
    /// Organization +0x7C -> +0x44.
    fn organization_leader(&mut self, organization: Ref) -> Result<Ref>;
}

pub trait DropWorld: TransportWorld {
    /// Actor virtual +0x108 on the plane (no arguments).
    fn kingdom(&mut self, plane: Ref) -> Result<Ref>;
    /// Actor virtual +0 on the plane (x87 return).
    fn plane_virtual_0(&mut self, plane: Ref) -> Result<Binary>;
    /// Actor virtual +0x18 on the plane (x87 return).
    fn plane_virtual_18(&mut self, plane: Ref) -> Result<Binary>;
    /// `0x554BF0` on GWorld +0x30: in/out point and facing.
    fn place_organization(
        &mut self,
        point: &mut [f32; 2],
        facing: &mut [f32; 2],
        leader: Ref,
        plane_xy: [f32; 2],
        radius: f32,
    ) -> Result<bool>;
    /// `0x554500` on GWorld +0x30 with two trailing zero arguments.
    fn place_thing(
        &mut self,
        position: &mut [f32; 2],
        facing: &mut [f32; 2],
        definition: Ref,
        radius: f32,
        svar_3c: f32,
    ) -> Result<bool>;
    /// `0x554850` on GWorld +0x30 with definition +0x38, `1.0f`, 16, 0, 0, 0.
    fn landing_search(
        &mut self,
        target: &mut [f32; 2],
        direction: &mut [f32; 2],
        definition: Ref,
    ) -> Result<bool>;
    /// `0x40C230(&B399B0, 0x11, plane, kingdom, extra, message, name)`.
    fn report(&mut self, failure: Failure, plane: Ref, kingdom: Ref, extra: Ref, name: Ref)
    -> Result<()>;
    /// `0x4F9C10` on GWorld.
    fn create(&mut self, params: &CreateParams) -> Result<Ref>;
    /// `0x4FACA0` on GWorld.
    fn create_unloaded(&mut self, params: &CreateParams, plane: Ref) -> Result<Ref>;
    /// `0x573D90` on the attachment's +0xD8 with the plane.
    fn attachment_query(&mut self, attachment: Ref, plane: Ref) -> Result<u32>;
    /// `0x574C10` on the attachment's +0xD8: (created, 0, query result).
    fn attachment_attach(&mut self, attachment: Ref, created: Ref, value: u32) -> Result<()>;
    /// `0x4CD250` on Transport +8 / +0x18.
    fn schedule(&mut self, updatable: Updatable) -> Result<()>;
    /// `0x4CF0F0` on the plane.
    fn plane_loaded(&mut self, plane: Ref) -> Result<()>;
    /// `0x578430` on Organization +0x7C with the plane.
    fn organization_carrier(&mut self, organization: Ref, plane: Ref) -> Result<()>;
    /// `0x4E2A60` on Kingdom +0x1FC with (definition, 0).
    fn kingdom_record(&mut self, kingdom: Ref, definition: Ref) -> Result<Ref>;
    /// `0x4E1220` on that record with (organization, slot).
    fn record_recruit(&mut self, record: Ref, organization: Ref, slot: i32) -> Result<()>;
    /// `0x57D860` on Organization +0x7C with (slot, thing).
    fn organization_add(&mut self, organization: Ref, slot: i32, thing: Ref) -> Result<()>;
    /// `0x57D950` on Organization +0x7C with (slot, position, facing,
    /// airborne byte, 0); fills the member slot.
    fn organization_place(
        &mut self,
        organization: Ref,
        slot: i32,
        position: [f32; 3],
        facing: [f32; 2],
        airborne: bool,
    ) -> Result<()>;
    /// Organization virtual +0x104 (no arguments; x87 return).
    fn organization_virtual_104(&mut self, organization: Ref) -> Result<Binary>;
    /// `0x4CA760` + `0x4D9B60` to the organization + `0x4CA2D0`.
    fn command(&mut self, organization: Ref, command: MoveCommand) -> Result<()>;
    /// `0x4F0E50` on GWorld with flags 0x122 (x87 return).
    fn ground_height(&mut self, xy: [f32; 2]) -> Result<Binary>;
    /// `0x576BB0` on the thing's Mover (+0x6C): position, facing, speed.
    fn commit_pose(&mut self, thing: Ref, position: [f32; 3], facing: [f32; 3], speed: f32)
    -> Result<()>;
    /// `0x5770A0` on the thing's Mover: drag, target, orient byte 0.
    fn begin_aimed_fall(&mut self, thing: Ref, drag: f32, target: [f32; 2]) -> Result<()>;
    /// `0x4F53D0` on GWorld with (thing, 0).
    fn destroy(&mut self, thing: Ref) -> Result<()>;
    /// `0x4F6760` on GWorld with (actor, 1).
    fn remove_actor(&mut self, actor: Ref) -> Result<()>;
}

/// Inputs read from globals.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DropContext {
    pub precision: Precision,
    /// GWorld +0x30 map (`0x552640`).
    pub map: MapBounds,
    /// SVar cache +0x2C (`0x577380`).
    pub gravity: f32,
    /// SVar cache +0x3C, `0x554500`'s sixth argument.
    pub svar_3c: f32,
    /// `[B279C0]` / `[B279C4]` (zero-initialized, no writer).
    pub zero_vector: [f32; 2],
}

/// `0x46DA80(out, offset, facing)`.
pub fn rotate_offset(offset: [f32; 2], facing: [f32; 2], precision: Precision) -> Result<[f32; 2]> {
    let (vx, vy, fx, fy) = (bin(offset[0])?, bin(offset[1])?, bin(facing[0])?, bin(facing[1])?);
    let x = vy.mul(&fx, precision)?.add(&vx.mul(&fy, precision)?, precision)?;
    let y = vy.mul(&fy, precision)?.sub(&vx.mul(&fx, precision)?, precision)?;
    Ok([store(&x)?, store(&y)?])
}

/// `0x409230`: `sqrt(x*x + y*y)`; above binary32 1e-4 both components are
/// scaled by the extended reciprocal and stored, otherwise zeroed. Returns
/// the extended length or +0.0.
pub fn normalize2(vector: &mut [f32; 2], precision: Precision) -> Result<Binary> {
    let (x, y) = (bin(vector[0])?, bin(vector[1])?);
    let length = x
        .mul(&x, precision)?
        .add(&y.mul(&y, precision)?, precision)?
        .sqrt(precision)?;
    if length.compare(&crate::mover::epsilon()?) != Ordering::Greater {
        *vector = [0.0; 2];
        return bin(0.0);
    }
    let inverse = bin(1.0)?.div(&length, precision)?;
    vector[0] = store(&inverse.mul(&x, precision)?)?;
    vector[1] = store(&inverse.mul(&y, precision)?)?;
    Ok(length)
}

/// The binary32 store of the x87 indefinite NaN.
pub const INDEFINITE_F32: u32 = 0xffc0_0000;
/// The x87 indefinite NaN as left in ST0.
pub const INDEFINITE_EXTENDED: u128 = 0xffff_c000_0000_0000_0000;

/// `0x577380` with its unordered path: a negative radicand
/// (`distance / gravity * -2.0`) makes FSQRT produce the x87 indefinite NaN,
/// which the `test ah, 5` / `jp` exit returns unchanged (`None`). Otherwise
/// `crate::mover::launch_speed`.
pub fn launch_carry(
    distance: f32,
    factor: f32,
    offset: f32,
    gravity: f32,
    precision: Precision,
) -> Result<Option<Binary>> {
    let radicand = bin(distance)?
        .div(&bin(gravity)?, precision)?
        .mul(&Binary::from_f32_bits(0xc000_0000)?, precision)?;
    if radicand.is_negative_nonzero() {
        return Ok(None);
    }
    launch_speed(distance, factor, offset, gravity, precision).map(Some)
}

/// `0x58B180(distance, factor)`: without a layout the cargo's `0x577380`
/// unrounded; with one the largest member value kept as binary32, where a
/// value replaces the kept one unless it is below it (an unordered compare
/// replaces too). `None` is the x87 indefinite NaN.
pub fn lead_distance<W: TransportWorld>(
    transport: &Transport,
    distance: f32,
    factor: f32,
    gravity: f32,
    precision: Precision,
    world: &mut W,
) -> Result<Option<Binary>> {
    if transport.layout == 0 {
        let definition = world.definition(transport.cargo)?;
        return launch_carry(distance, factor, definition.field_1a4, gravity, precision);
    }
    // binary32 bits of the kept value; INDEFINITE_F32 once a NaN was kept.
    let mut largest = 0u32;
    let mut slot = 0;
    while slot < world.layout(transport.layout)?.count {
        let member = world.layout_definition(transport.layout, slot)?;
        if member != 0 {
            let offset = world.definition(member)?.field_1a4;
            match launch_carry(distance, factor, offset, gravity, precision)? {
                None => largest = INDEFINITE_F32,
                Some(value) => {
                    if largest == INDEFINITE_F32
                        || Binary::from_f32_bits(largest)?.compare(&value) != Ordering::Greater
                    {
                        largest = value.store_f32()?;
                    }
                }
            }
        }
        slot += 1;
    }
    if largest == INDEFINITE_F32 {
        return Ok(None);
    }
    Ok(Some(Binary::from_f32_bits(largest)?))
}

/// `0x58B160`.
pub fn mark_loaded<W: DropWorld>(transport: &mut Transport, world: &mut W) -> Result<()> {
    transport.loaded = 1;
    world.schedule(Updatable::At08)?;
    world.schedule(Updatable::At18)?;
    world.plane_loaded(transport.plane)
}

/// `0x58B130`: `0x4F6760(plane, 1)`.
pub fn leave<W: DropWorld>(transport: &Transport, world: &mut W) -> Result<()> {
    world.remove_actor(transport.plane)
}

fn null(what: &str) -> Error {
    Error::World(format!("native null dereference: {what}"))
}

fn xy(v: [f32; 3]) -> [f32; 2] {
    [v[0], v[1]]
}

/// Complete `0x58BCB0`: one thing out of the hold.
pub fn drop_one<W: DropWorld>(
    transport: &mut Transport,
    context: &DropContext,
    world: &mut W,
) -> Result<()> {
    let pc = context.precision;
    if transport.dropped >= transport.count {
        return Ok(());
    }
    let plane = transport.plane;
    let airborne = world.actor(plane)?.flags >> 12 & 1 != 0;
    if transport.dropped == 0 && transport.layout != 0 {
        let view = world.actor(plane)?;
        let mut facing = [view.facing[0], view.facing[1]];
        let mut point;
        if airborne {
            point = transport.point;
        } else {
            point = xy(view.position);
            if view.word_88 != 0 {
                let offset = world.definition(view.definition)?.drop_offset;
                let facing_now = xy(world.actor(plane)?.facing);
                let rotated = rotate_offset(offset, facing_now, pc)?;
                point = [
                    store(&bin(rotated[0])?.add(&bin(point[0])?, pc)?)?,
                    store(&bin(rotated[1])?.add(&bin(point[1])?, pc)?)?,
                ];
            }
        }
        let layout = world.layout(transport.layout)?;
        let leader = world.layout_definition(transport.layout, layout.leader_slot)?;
        let plane_xy = xy(world.actor(plane)?.position);
        if !world.place_organization(&mut point, &mut facing, leader, plane_xy, ORGANIZATION_RADIUS)? {
            let name = world.definition(transport.cargo)?.name;
            let kingdom = world.kingdom(plane)?;
            world.report(Failure::OrganizationPlacement, plane, kingdom, 0, name)?;
            transport.dropped = transport.count;
            return Ok(());
        }
        let mut params = CreateParams {
            definition: transport.cargo,
            layout: transport.layout,
            position: [point[0], point[1], 0.0],
            facing,
            ..CreateParams::default()
        };
        params.kingdom = world.kingdom(plane)?;
        params.name = Some(transport.name);
        let organization = world.create(&params)?;
        if organization == 0 {
            let name = world.definition(transport.cargo)?.name;
            let kingdom = world.kingdom(plane)?;
            world.report(Failure::OrganizationCreation, plane, kingdom, 0, name)?;
            transport.dropped = transport.count;
            return Ok(());
        }
        if transport.attachment != 0 {
            let attachment = world.resolve(transport.attachment)?;
            if attachment == 0 {
                return Err(null("Transport +0x5C"));
            }
            let value = world.attachment_query(attachment, plane)?;
            world.attachment_attach(attachment, organization, value)?;
        }
        mark_loaded(transport, world)?;
        transport.organization = world.handle_of(organization)?;
        let resolved = world.resolve(transport.organization)?;
        if resolved == 0 {
            return Err(null("Transport +0x44 after creation"));
        }
        world.organization_carrier(resolved, plane)?;
    }
    if transport.layout == 0 {
        return drop_single(transport, airborne, context, world);
    }
    if !world.handle_valid(transport.organization)? {
        transport.dropped = transport.count;
        return Ok(());
    }
    let mut slot = transport.next_slot;
    let mut definition = world.layout_definition(transport.layout, slot)?;
    let count = world.layout(transport.layout)?.count;
    while definition == 0 && slot < count {
        slot += 1;
        definition = world.layout_definition(transport.layout, slot)?;
    }
    transport.next_slot = slot + 1;
    if slot >= count {
        transport.dropped = transport.count;
        return Ok(());
    }
    if world.kingdom(plane)? != 0 {
        let kingdom = world.kingdom(plane)?;
        if kingdom.wrapping_add(0x1fc) != 0 {
            let record = world.kingdom_record(kingdom, definition)?;
            if record != 0 {
                let organization = world.resolve(transport.organization)?;
                world.record_recruit(record, organization, slot)?;
            }
        }
    }
    let plane_definition = world.actor(plane)?.definition;
    let thing;
    if !airborne && world.definition(plane_definition)?.byte_3ec != 0 {
        let mut params = CreateParams { definition, ..CreateParams::default() };
        params.kingdom = world.kingdom(plane)?;
        params.organization = transport.organization;
        thing = world.create_unloaded(&params, plane)?;
        let organization = resolve_organization(transport, world)?;
        world.organization_add(organization, slot, thing)?;
    } else {
        let view = world.actor(plane)?;
        let mut position = view.position;
        let mut facing = [view.facing[0], view.facing[1]];
        if airborne {
            let height = world.definition(definition)?.height_24;
            position[2] = store(&bin(position[2])?.sub(&bin(height)?, pc)?)?;
        } else {
            let radius =
                store(&world.plane_virtual_0(plane)?.mul(&bin(MEMBER_RADIUS_SCALE)?, pc)?)?;
            let mut at = xy(position);
            normalize2(&mut facing, pc)?;
            if !world.place_thing(&mut at, &mut facing, definition, radius, context.svar_3c)? {
                let name = world.definition(definition)?.name;
                let kingdom = world.kingdom(plane)?;
                world.report(Failure::MemberPlacement, plane, kingdom, 0, name)?;
                return Ok(());
            }
            position[0] = at[0];
            position[1] = at[1];
        }
        let organization = resolve_organization(transport, world)?;
        world.organization_place(organization, slot, position, facing, airborne)?;
        let organization = resolve_organization(transport, world)?;
        thing = world.organization_member(organization, slot)?;
        if thing == 0 {
            transport.dropped += 1;
            return Ok(());
        }
        if airborne {
            return fall(transport, thing, context, world);
        }
    }
    let organization = resolve_organization(transport, world)?;
    if thing == world.organization_leader(organization)? {
        let organization = resolve_organization(transport, world)?;
        let value = store(&world.organization_virtual_104(organization)?)?;
        let command = MoveCommand {
            point: transport.point,
            value,
            vector: context.zero_vector,
            byte_3c: 1,
        };
        let organization = world.resolve(transport.organization)?;
        world.command(organization, command)?;
    }
    transport.dropped += 1;
    Ok(())
}

fn resolve_organization<W: DropWorld>(transport: &Transport, world: &mut W) -> Result<Ref> {
    match world.resolve(transport.organization)? {
        0 => Err(null("Transport +0x44")),
        organization => Ok(organization),
    }
}

/// The single-thing half of `0x58BCB0` (Transport +0x2C null).
fn drop_single<W: DropWorld>(
    transport: &mut Transport,
    airborne: bool,
    context: &DropContext,
    world: &mut W,
) -> Result<()> {
    let plane = transport.plane;
    let mut params = CreateParams { definition: transport.cargo, ..CreateParams::default() };
    params.kingdom = world.kingdom(plane)?;
    let thing;
    if airborne {
        let view = world.actor(plane)?;
        params.position = view.position;
        params.facing = xy(view.facing);
        let height = world.definition(transport.cargo)?.height_24;
        params.byte_50 = 1;
        params.position[2] = store(&bin(params.position[2])?.sub(&bin(height)?, context.precision)?)?;
        thing = world.create(&params)?;
    } else {
        let view = world.actor(plane)?;
        if world.definition(view.definition)?.byte_3ec != 0 {
            thing = world.create_unloaded(&params, plane)?;
        } else {
            let mut at = xy(view.position);
            let mut facing = xy(view.facing);
            let radius = store(&world.plane_virtual_0(plane)?)?;
            if !world.place_thing(&mut at, &mut facing, transport.cargo, radius, context.svar_3c)? {
                let name = world.definition(transport.cargo)?.name;
                let kingdom = world.kingdom(plane)?;
                world.report(Failure::ThingPlacement, plane, kingdom, 0, name)?;
                return Ok(());
            }
            // The original fills the block here and creates nothing: the
            // hold is emptied as when creation fails.
            transport.dropped = transport.count;
            return Ok(());
        }
    }
    if thing == 0 {
        transport.dropped = transport.count;
        return Ok(());
    }
    mark_loaded(transport, world)?;
    if airborne {
        return fall(transport, thing, context, world);
    }
    transport.dropped += 1;
    Ok(())
}

/// `0x58C11F`: an airborne thing takes the plane's speed and falls aimed at
/// where `0x577380` carries it along its facing.
fn fall<W: DropWorld>(
    transport: &mut Transport,
    thing: Ref,
    context: &DropContext,
    world: &mut W,
) -> Result<()> {
    let pc = context.precision;
    let view = world.actor(thing)?;
    let (position, facing) = (view.position, view.facing);
    let speed = store(&world.plane_virtual_18(transport.plane)?)?;
    let height = world.ground_height(xy(position))?;
    let above = store(&bin(position[2])?.sub(&height, pc)?)?;
    let definition = world.actor(thing)?.definition;
    let field_1a4 = world.definition(definition)?.field_1a4;
    world.commit_pose(thing, position, facing, speed)?;
    let carry = launch_carry(above, speed, field_1a4, context.gravity, pc)?;
    let mut direction = [facing[0], facing[1]];
    normalize2(&mut direction, pc)?;
    let mut target;
    match carry {
        // NaN propagates through the products and sums, and 0x552640's
        // unordered compares leave both coordinates unchanged.
        None => target = [f32::from_bits(INDEFINITE_F32); 2],
        Some(carry) => {
            let carry = bin(store(&carry)?)?;
            let along_x = store(&bin(direction[0])?.mul(&carry, pc)?)?;
            let along_y = bin(direction[1])?.mul(&carry, pc)?;
            target = [
                store(&bin(along_x)?.add(&bin(position[0])?, pc)?)?,
                store(&along_y.add(&bin(position[1])?, pc)?)?,
            ];
            clamp_to_interior(context.map, &mut target, pc)?;
        }
    }
    if world.landing_search(&mut target, &mut direction, definition)? {
        world.begin_aimed_fall(thing, field_1a4, target)?;
    } else {
        world.destroy(thing)?;
    }
    transport.dropped += 1;
    Ok(())
}
