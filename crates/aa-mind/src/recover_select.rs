//! Original recover destination selection `52BB40` (Source/Sim/Tai), used by
//! the SAI recover goal (`7C8AF0`).
//!
//! For every allied Kingdom in the sparse list at `[B36534]+154` and every
//! supply thing whose supply component is active (`58AC70`), it draws
//! `RecoverSupplyZoneCandidates` points inside 0.9 x the supply radius
//! (direction from the global table `[B39AF8]`, distance `u * 0.9r`), keeps
//! those that `4F46E0` can place and whose tile is friendly supply territory
//! (`54F8B0` with a null subject), scores them with `52A930` in recover mode
//! and returns the strictly best positive score's position and the supply
//! thing's GTID (+14).
use crate::{
    Error, Result,
    rout_score::{self, Candidate, RoutWorld, tile_coordinate},
    x87::{n, num, store, unit_random},
};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// `0xA326BC` binary32 0.9.
const RADIUS_SCALE_BITS: u32 = 0x3f66_6666;
/// Radius argument passed to `4F46E0` (binary32 4.0).
const OPEN_POSITION_RADIUS_BITS: u32 = 0x4080_0000;
/// `58AC70` compares the first entry's +8 word with binary32 -1.0.
const INACTIVE_WORD_BITS: u32 = 0xbf80_0000;

/// Supply component (thing+8C) fields read here.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyComponent {
    /// Component+10 radius (binary32).
    pub radius: u32,
    /// `58AC70` input: component+4 object's first list entry (+148, count
    /// +14C) word +8, or `None` when the list is empty or its first entry
    /// is null.
    pub first_entry_word8: Option<u32>,
}
impl SupplyComponent {
    /// Complete `58AC70`.
    pub fn active(&self) -> bool {
        self.first_entry_word8
            .is_some_and(|word| word != INACTIVE_WORD_BITS)
    }
}

/// One Kingdom+264 list node (the thing pointer may be null).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupplyThing {
    /// Thing+14 GTID returned with the selected position.
    pub gtid: u32,
    pub position: [u32; 2],
    pub component: Option<SupplyComponent>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    /// `52A930` inputs (its Kingdom list is the +148 array).
    pub score: rout_score::Context,
    /// Sparse `[B36534]+154` list: per Kingdom its +264 nodes.
    pub kingdoms: Vec<Option<Vec<Option<SupplyThing>>>>,
    /// SVar `RecoverSupplyZoneCandidates` (int).
    pub candidates: i32,
    /// Global direction table `[B39AF8]`.
    pub directions: Vec<[u32; 2]>,
    /// Native `param_5`, forwarded to `52A930` as its moved-from object.
    pub moved_from: Option<[u32; 2]>,
}

/// Services beyond those of `52A930`.
pub trait RecoverWorld: RoutWorld {
    /// Organization virtual `+4` (x87 result discarded).
    fn organization_virtual_4(&mut self) -> Result<()>;
    /// Element virtual `+138(owner)` for slot `slot` of the +154 list.
    fn recover_kingdom_allied(&mut self, slot: usize, owner: u32) -> Result<bool>;
    /// World `4F46E0(organization, &position, 4.0)`. May move `position`.
    fn find_open_position(&mut self, position: &mut [u32; 2], radius: u32) -> Result<bool>;
}

fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Complete `52BB40`: `Some((position, supply GTID))` or `None`.
pub fn select(
    context: &Context,
    world: &mut impl RecoverWorld,
    p: Precision,
) -> Result<Option<([u32; 2], u32)>> {
    let mut best = 0_u32;
    let mut best_position = [0, 0];
    let mut best_gtid = 0;
    world.organization_virtual_4()?;
    let [ox, oy] = context.score.organization;
    for (slot, nodes) in context.kingdoms.iter().enumerate() {
        let Some(nodes) = nodes else { continue };
        let owner = world.owner()?;
        if !world.recover_kingdom_allied(slot, owner)? {
            continue;
        }
        for thing in nodes.iter().flatten() {
            let Some(component) = thing.component else {
                continue;
            };
            if !component.active() {
                continue;
            }
            let mut k = 0;
            while k < context.candidates {
                let [tx, ty] = thing.position;
                let count = context.directions.len() as u32;
                if count == 0 {
                    return Err(Error::World(
                        "recover direction table is empty (native divide fault)".into(),
                    ));
                }
                let direction = context.directions[(world.random()? % count) as usize];
                let reach = store(num(n(component.radius)?.mul(&n(RADIUS_SCALE_BITS)?, p))?)?;
                let distance = num(unit_random(world.random()?, p)?.mul(&n(reach)?, p))?;
                let offset_y = num(n(direction[1])?.mul(&distance, p))?;
                let offset_x = store(num(n(direction[0])?.mul(&distance, p))?)?;
                let y = num(n(ty)?.add(&offset_y, p))?;
                let x = store(num(n(offset_x)?.add(&n(tx)?, p))?)?;
                let unchanged = host(x) == host(ox) && y.compare(&n(oy)?) == Ordering::Equal;
                k += 1;
                if unchanged {
                    continue;
                }
                let mut position = [x, store(y)?];
                if !world.find_open_position(&mut position, OPEN_POSITION_RADIUS_BITS)? {
                    continue;
                }
                let owner = world.owner()?;
                let ty = tile_coordinate(position[1], context.score.map.shift)?;
                let tx = tile_coordinate(position[0], context.score.map.shift)?;
                let tile = context.score.map.width.wrapping_mul(ty).wrapping_add(tx);
                if !world.friendly_zone(tile, owner, false)? {
                    continue;
                }
                let candidate = Candidate {
                    position,
                    moved_from: context.moved_from,
                    recover: true,
                };
                let value = rout_score::score(&candidate, &context.score, world, p)?;
                if value.compare(&n(best)?) == Ordering::Greater {
                    best = store(value)?;
                    best_position = position;
                    best_gtid = thing.gtid;
                }
            }
        }
    }
    // `fcomp 0; test ah,41; jp`: success for greater (or unordered).
    Ok((host(best) > 0.0).then_some((best_position, best_gtid)))
}
