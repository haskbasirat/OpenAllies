//! Original element Mind Move command538340, after command authorization and
//! selection processing. Actor targets can choose attack/bombard/follow paths;
//! a ground target uses the existing native navigation activation contract.
use crate::{Error, MindId, Result, StateId};
use aa_navigation::Activation;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Command {
    pub flags: u32,
    pub target: u32,
    pub position_bits: [u32; 2],
    pub auxiliary_bits: [u32; 2],
    pub speed_bits: u32,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Element {
    pub actor: u32,
    pub move_state: StateId,
    pub sleep_state: StateId,
    pub kill_state: StateId,
    pub bombard_state: StateId,
    pub move_destination_bits: [u32; 2],
    pub move_target: u32,
    pub update_pending: bool,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Actor {
    pub flags: u32,
    pub position_bits: [u32; 2],
    pub performer: Option<u32>,
}
pub trait World {
    fn command(&self, id: u32) -> Result<Command>;
    fn element(&self, id: MindId) -> Result<Element>;
    /// Granular native reads let authoritative adapters preserve538340 branch
    /// laziness. Existing snapshot hosts retain the same defaults; live hosts
    /// override these without inventing values for fields not yet read.
    fn element_actor(&self, id: MindId) -> Result<u32> { Ok(self.element(id)?.actor) }
    fn move_state(&self, id: MindId) -> Result<StateId> { Ok(self.element(id)?.move_state) }
    fn sleep_state(&self, id: MindId) -> Result<StateId> { Ok(self.element(id)?.sleep_state) }
    fn kill_state(&self, id: MindId) -> Result<StateId> { Ok(self.element(id)?.kill_state) }
    fn bombard_state(&self, id: MindId) -> Result<StateId> { Ok(self.element(id)?.bombard_state) }
    fn move_destination(&self, id: MindId) -> Result<[u32; 2]> { Ok(self.element(id)?.move_destination_bits) }
    fn move_target(&self, id: MindId) -> Result<u32> { Ok(self.element(id)?.move_target) }
    fn update_pending(&self, id: MindId) -> Result<bool> { Ok(self.element(id)?.update_pending) }
    fn actor(&self, id: u32) -> Result<Actor>;
    fn actor_flags(&self, id: u32) -> Result<u32> { Ok(self.actor(id)?.flags) }
    fn actor_position(&self, id: u32) -> Result<[u32; 2]> { Ok(self.actor(id)?.position_bits) }
    fn actor_performer(&self, id: u32) -> Result<Option<u32>> { Ok(self.actor(id)?.performer) }
    /// Generation-checked native GTID lookup; unresolved references stay absent.
    fn resolve(&self, id: u32) -> Result<Option<u32>>;
    /// 52A000, exact active stack-head identity test, not status==2.
    fn active(&self, state: StateId) -> Result<bool>;
    fn schedule(&mut self, mind: MindId, delay_bits: u32, data: u32) -> Result<()>;
    /// Actual Performer57E280 result, required from performer implementation.
    fn indirect_fire(&mut self, performer: u32) -> Result<bool>;
    /// Target actor virtual+16C with actor argument. Repeated calls are retained.
    fn target_accepts(&mut self, target: u32, actor: u32) -> Result<bool>;
    fn activate_sleep(&mut self, state: StateId, mode: u32) -> Result<bool>;
    fn activate_move(&mut self, state: StateId, request: Activation) -> Result<bool>;
    fn activate_kill(&mut self, state: StateId, mode: u32, target: u32) -> Result<bool>;
    fn activate_bombard(&mut self, state: StateId, mode: u32, target: u32) -> Result<bool>;
    /// 532930 target-follow activation, retaining its movement mode and speed.
    fn activate_follow(
        &mut self,
        state: StateId,
        mode: u32,
        target: u32,
        movement_mode: u32,
        speed_bits: u32,
        auxiliary_bits: [u32; 2],
    ) -> Result<bool>;
}
fn number(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| Error::World(format!("native command arithmetic: {e}")))
}
fn numeric<T>(r: aa_numeric::Result<T>) -> Result<T> {
    r.map_err(|e| Error::World(format!("native command arithmetic: {e}")))
}
fn ordered_equal(a: u32, b: u32) -> bool {
    a & 0x7fff_ffff <= 0x7f80_0000
        && b & 0x7fff_ffff <= 0x7f80_0000
        && (a == b || a & 0x7fff_ffff == 0 && b & 0x7fff_ffff == 0)
}
/// Original52A240 stores only delta-X to binary32 before squared-distance
/// comparison with0.01. Delta-Y remains at the selected x87 precision.
pub fn near_actor(actor: [u32; 2], point: [u32; 2], p: Precision) -> Result<bool> {
    let x = numeric(number(point[0])?.sub(&number(actor[0])?, p))?;
    let x = number(numeric(x.store_f32())?)?;
    let y = numeric(number(point[1])?.sub(&number(actor[1])?, p))?;
    let distance = numeric(numeric(y.mul(&y, p))?.add(&numeric(x.mul(&x, p))?, p))?;
    Ok(distance.compare(&number(0x3c23_d70a)?) == Ordering::Less)
}
fn wake<W: World>(mind: MindId, world: &mut W) -> Result<()> {
    if !world.update_pending(mind)? {
        world.schedule(mind, 0x3f19_999a, 0)?;
    }
    Ok(())
}
/// Execute only after the caller's real permission/selection boundary admits
/// the command. This routine does not assign a current player or authorize it.
pub fn move_command<W: World>(
    mind: MindId,
    command: u32,
    world: &mut W,
    p: Precision,
) -> Result<bool> {
    let flags = world.actor_flags(world.element_actor(mind)?)?;
    if flags & (1 | 1 << 29) != 0 {
        return Ok(false);
    }
    let c = world.command(command)?;
    let targeted = c.flags & 1 != 0;
    if targeted && world.resolve(c.target)?.is_none() {
        return Ok(false);
    }
    let auxiliary = if c.flags & 8 != 0 {
        c.auxiliary_bits
    } else {
        [0, 0]
    };
    let speed = if c.flags & 16 != 0 { c.speed_bits } else { 0 };
    if !targeted {
        if world.active(world.move_state(mind)?)? {
            let destination = world.move_destination(mind)?;
            let point = world.command(command)?.position_bits;
            if ordered_equal(destination[0], point[0]) && ordered_equal(destination[1], point[1]) {
                wake(mind, world)?;
                return Ok(true);
            }
        }
        let point = world.command(command)?.position_bits;
        let position = world.actor_position(world.element_actor(mind)?)?;
        if near_actor(position, point, p)? {
            return world.activate_sleep(world.sleep_state(mind)?, 2);
        }
        return world.activate_move(
            world.move_state(mind)?,
            Activation {
                activation_mode: 2,
                destination_bits: world.command(command)?.position_bits,
                movement_mode: 0,
                requested_speed_bits: speed,
                auxiliary_bits: auxiliary,
            },
        );
    }
    let Some(target) = world.resolve(world.command(command)?.target)? else {
        return Ok(false);
    };
    if world.active(world.move_state(mind)?)?
        && world.resolve(world.move_target(mind)?)? == Some(target)
    {
        wake(mind, world)?;
        return Ok(true);
    }
    let performer = world.actor_performer(world.element_actor(mind)?)?;
    let indirect = if let Some(performer) = performer {
        world.indirect_fire(performer)?
    } else {
        false
    };
    if indirect && world.target_accepts(target, world.element_actor(mind)?)? {
        if world.activate_bombard(world.bombard_state(mind)?, 2, target)? {
            return Ok(true);
        }
    } else if world.target_accepts(target, world.element_actor(mind)?)?
        && world.activate_kill(world.kill_state(mind)?, 2, target)?
    {
        return Ok(true);
    }
    world.activate_follow(world.move_state(mind)?, 2, target, 2, speed, auxiliary)
}
