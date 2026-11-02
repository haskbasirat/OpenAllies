//! `KKC_CAIStateRampage` (regiment Mind +0x54) of a herd: `aa_mind::cai_thinks::rampage`
//! over [`Regiment`], and its wander point `0x525100`.
//!
//! A map's herds are saved in this state, so `0x4FAE80`'s Load puts them on it at the
//! first update; on (2v2)Kursk their Thinks are the first getters of the film.
//!
//! **Refusal:** a config with `rampage_aggressive` (+0x1C) hunts through `0x51E450` and
//! `0x51E910`, which need the regiment's kill state, still `crate::combat`'s (census
//! X1). No shipped ambient organization sets it; [`nearest_target`] says so when one
//! does. **Rendition:** `0x4F2220` holds no wall footprints here, as in [`route`].
use super::*;
use aa_mind::cai_thinks::rampage::{self, RampageThinkWorld};

pub(super) const RAMPAGE: u32 = cai_thinks::RAMPAGE_OFFSET;
/// Rampage state +0x10, the rampaging byte, as a Mind offset.
const RAMPAGING: u32 = RAMPAGE + 0x10;
/// binary32 2.0, the radius `0x525100` hands `0x4F46E0`.
const PLACEMENT_RADIUS_BITS: u32 = 0x4000_0000;
/// `[A30DE0]`: an angle outside the range weighs ten times its excess, squared.
const ANGLE_WEIGHT: f64 = 10.0;
/// `[A329E4]` binary32 1e-4 as `0x525100` compares it.
const NEAR_ZERO_BITS: u32 = 0x38d1_b717;
/// The start of the best score (`0x525431`).
const NO_SCORE: f32 = 1e8;

/// Virtual +0xC of the state.
pub(super) fn think(host: &mut Regiment<'_>, state: StateId, a: &mut Arena) -> aa_mind::Result<()> {
    rampage::think(state, a, host, PC).map(|_| ())
}

impl Regiment<'_> {
    fn config(&self) -> crate::data::RampageDef {
        self.world.data.def(self.world.orgs[self.o as usize].def).rampage
    }
    /// The member in layout slot `index`.
    fn member_at(&self, index: u32) -> Option<ActorId> {
        alive_members(self.world, self.o).find(|&m| member_slot(self.world, self.o, m) == index as usize)
    }
    /// Organization +0x2C / +0x30.
    fn organization_facing(&self) -> Vec2 {
        let facing = self.ms.orgs[self.o as usize].facing;
        if facing != Vec2::ZERO { facing } else { facing_vector(self.world.orgs[self.o as usize].heading) }
    }
}

/// `0x45F2A0`'s list and the scoring of `0x525100`: the candidate closest to the
/// preferred distance and direction, in the order they were drawn.
fn best_candidate(candidates: &[[f32; 2]], from: [f32; 2], preferred: [f32; 2], config: &crate::data::RampageDef) -> aa_mind::Result<[f32; 2]> {
    let [distance_min, distance_max] = config.distance_bits.map(f32::from_bits);
    let [direction_min, direction_max] = config.direction_bits.map(f32::from_bits);
    let distance_mid = ((f64::from(distance_min) + f64::from(distance_max)) * 0.5) as f32;
    let near_zero = f32::from_bits(NEAR_ZERO_BITS);
    let mut best = [0.0f32, 0.0];
    let mut best_score = NO_SCORE;
    for c in candidates {
        if best_score <= 0.0 {
            break;
        }
        let mut score = 0.0f32;
        let (dx, dy) = (c[0] - from[0], c[1] - from[1]);
        let distance = (f64::from(dx) * f64::from(dx) + f64::from(dy) * f64::from(dy)).sqrt() as f32;
        if distance != distance_mid {
            score = if distance < distance_min {
                distance_min - distance
            } else if distance > distance_max {
                distance - distance_max
            } else {
                1.0
            };
        }
        let unit = if distance <= near_zero {
            [0.0f32, 0.0]
        } else {
            let inverse = 1.0 / f64::from(distance);
            [(f64::from(dx) * inverse) as f32, (f64::from(dy) * inverse) as f32]
        };
        let angle = aa_trig::signed_angle(preferred.map(f32::to_bits), unit.map(f32::to_bits), PC, false).map_err(err)?;
        let angle = f32::from_bits(angle);
        if angle != 0.0 {
            score = if angle < direction_min {
                let over = (f64::from(direction_min) - f64::from(angle)) * ANGLE_WEIGHT;
                (over * over + f64::from(score)) as f32
            } else if angle > direction_max {
                let over = (f64::from(angle) - f64::from(direction_max)) * ANGLE_WEIGHT;
                (over * over + f64::from(score)) as f32
            } else {
                (f64::from(score) + 1.0) as f32
            };
        }
        if score < best_score {
            best = *c;
            best_score = score;
        }
    }
    Ok(best)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A distance in its range scores 1, an angle in its range 1 more and exactly ahead
    /// nothing; outside a range the excess counts, ten times and squared for an angle.
    /// A tie goes to the first drawn.
    #[test]
    fn the_candidate_in_both_ranges_wins() {
        let config = crate::data::RampageDef {
            distance_bits: [0.5f32.to_bits(), 4.0f32.to_bits()],
            direction_bits: [(-0.5f32).to_bits(), 0.5f32.to_bits()],
            ..Default::default()
        };
        let from = [10.0, 10.0];
        let ahead = [1.0, 0.0];
        let far = [20.0, 10.0];
        let behind = [8.0, 10.0];
        let near = [12.0, 10.0];
        let also_near = [13.0, 10.0];
        assert_eq!(best_candidate(&[far, behind, near, also_near], from, ahead, &config).unwrap(), near);
    }
}

impl RampageThinkWorld for Regiment<'_> {
    fn rampage_wander_chance(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Ok(self.config().wait_chance_bits)
    }
    fn rampage_threshold(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Ok(self.config().max_detached_ratio_bits)
    }
    fn rampage_hunts(&mut self, _: MindId) -> aa_mind::Result<bool> {
        Ok(self.config().aggressive)
    }
    fn component_5c_60(&mut self, _: MindId) -> aa_mind::Result<[u32; 2]> {
        let o = self.o;
        let maximum = self.world.combat.org(o).map_or(0.0, |c| c.morale_max);
        Ok([self.world.orgs[o as usize].morale.to_bits(), maximum.to_bits()])
    }
    fn rampaging(&mut self, _: StateId) -> aa_mind::Result<bool> {
        Ok(self.world.books.minds.byte(key(self.o), RAMPAGING) != 0)
    }
    fn set_rampaging(&mut self, _: StateId, value: bool) -> aa_mind::Result<()> {
        self.world.books.minds.set_byte(key(self.o), RAMPAGING, u8::from(value));
        Ok(())
    }
    /// `0x578790`: of the listed members, the part attached (Element +0x1C).
    fn component_578790(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<Binary> {
        let (mut attached, mut listed) = (0i32, 0i32);
        for m in alive_members(self.world, self.o) {
            listed += 1;
            attached += i32::from(element::element_attached(self.world, m));
        }
        if listed == 0 {
            return Err(err("0x578790 of an organization without members"));
        }
        Binary::from_i32(attached).div(&Binary::from_i32(listed), PC).map_err(err)
    }
    fn component_member_count(&mut self, _: MindId) -> aa_mind::Result<i32> {
        let slots = self.world.data.def(self.world.orgs[self.o as usize].def).slots.len();
        let last = alive_members(self.world, self.o).map(|m| member_slot(self.world, self.o, m) + 1).max().unwrap_or(0);
        Ok(slots.max(last) as i32)
    }
    fn component_member(&mut self, _: MindId, index: u32) -> aa_mind::Result<u32> {
        Ok(self.member_at(index).map_or(0, |m| m + 1))
    }
    fn member_word80(&mut self, member: u32) -> aa_mind::Result<Option<u32>> {
        Ok(Some(member))
    }
    fn word80_byte1c(&mut self, object: u32) -> aa_mind::Result<bool> {
        Ok(element::element_attached(self.world, object - 1))
    }
    fn member_mind(&mut self, member: u32) -> aa_mind::Result<Option<u32>> {
        Ok(Some(member))
    }
    /// Mind +0xFC `0x52F160`.
    fn member_mind_virtual_fc(&mut self, member_mind: u32, a: &mut Arena) -> aa_mind::Result<()> {
        let rules = self.rules;
        element::reattach_in(self.ms, self.world, rules, member_mind - 1, a);
        Ok(())
    }
    fn component_577f20(&mut self, _: MindId, _: &mut Arena) -> aa_mind::Result<u32> {
        Err(err("0x577F20 of a hunting rampage is not hosted"))
    }
    fn nearest_target(&mut self, _: MindId, _: u32, _: &mut Arena) -> aa_mind::Result<Option<u32>> {
        Err(err("0x51E450 of a hunting rampage is not hosted"))
    }
    fn kill_target(&mut self, _: MindId) -> aa_mind::Result<u32> {
        Err(err("the kill state of a hunting rampage is not hosted"))
    }
    fn kill_activate(&mut self, _: MindId, _: u32, _: u32, _: &mut Arena) -> aa_mind::Result<bool> {
        Err(err("the kill state of a hunting rampage is not hosted"))
    }
    /// `0x4F02B0` + `0x85D840`.
    fn random(&mut self) -> aa_mind::Result<u32> {
        Ok(self.world.random_u32_at(crate::trace::sites::RAMPAGE_WAIT))
    }
    /// `0x525100`: one getter, then per candidate two draws. The first pass takes
    /// a table direction and a distance within the config's range, around the
    /// organization; a second pass, when none could stand, takes any offset within
    /// the map. Nothing in two passes: where it stands.
    fn wander_point(&mut self, _: StateId, _: &mut Arena) -> aa_mind::Result<[u32; 2]> {
        let o = self.o;
        let config = self.config();
        let state = self.world.native_rng.state;
        self.world.trace.log(crate::trace::SKind::RngGetter(crate::trace::sites::RAMPAGE_WANDER), state);
        let anchor = self.world.orgs[o as usize].anchor;
        let from = [anchor.x, anchor.y];
        let extent = self.extents().extent_bits.map(f32::from_bits);
        let [distance_min, distance_max] = config.distance_bits.map(|b| f64::from(f32::from_bits(b)));
        let leader = self.leader();
        let mut candidates: Vec<[f32; 2]> = Vec::new();
        let mut pass = 0;
        while candidates.is_empty() {
            if pass > 1 {
                return Ok(bits(anchor));
            }
            for _ in 0..config.points_to_try.max(0) {
                let offset = if pass == 0 {
                    let direction = aa_movement::formation::random_direction(self.world.native_rng.next_u32());
                    let u = crate::trace::native_unit_f64(self.world.native_rng.next_u32());
                    let distance = ((distance_max - distance_min) * u + distance_min) as f32;
                    [direction[0] * distance, direction[1] * distance]
                } else {
                    let ux = crate::trace::native_unit_f64(self.world.native_rng.next_u32()) - 0.5;
                    let uy = crate::trace::native_unit_f64(self.world.native_rng.next_u32()) - 0.5;
                    [((ux + ux) * f64::from(extent[0])) as f32, ((uy + uy) * f64::from(extent[1])) as f32]
                };
                let mut point = [from[0] + offset[0], from[1] + offset[1]];
                if !(point[0] >= 0.0 && point[0] < extent[0] && point[1] >= 0.0 && point[1] < extent[1]) {
                    continue;
                }
                let mut words = point.map(f32::to_bits);
                let placed = leader.is_some_and(|l| route::nearest_placement(self.ms, self.world, l, &mut words, PLACEMENT_RADIUS_BITS).unwrap_or(false));
                if placed {
                    point = words.map(f32::from_bits);
                    candidates.push(point);
                }
            }
            pass += 1;
        }
        // The preferred direction: the facing turned by the middle of the range.
        let [direction_min, direction_max] = config.direction_bits.map(f32::from_bits);
        let middle = ((f64::from(direction_min) + f64::from(direction_max)) * 0.5) as f32;
        let sin = f64::from(f32::from_bits(aa_trig::lookup_sin(middle.to_bits(), PC).map_err(err)?));
        let cos = f64::from(f32::from_bits(aa_trig::lookup_cos(middle.to_bits(), PC).map_err(err)?));
        let facing = self.organization_facing();
        let (fx, fy) = (f64::from(facing.x), f64::from(facing.y));
        let preferred = [(fx * cos - fy * sin) as f32, (sin * fx + fy * cos) as f32];
        let best = best_candidate(&candidates, from, preferred, &config)?;
        Ok(best.map(f32::to_bits))
    }
    /// `[B279C0]`, `[B279C4]`.
    fn null_point(&mut self) -> aa_mind::Result<[u32; 2]> {
        Ok([0, 0])
    }
    /// `0x520440` with the stack words (mode, point x, y, speed, auxiliary x, y).
    fn move_order(&mut self, _: MindId, args: [u32; 6], a: &mut Arena) -> aa_mind::Result<()> {
        self.activate_move_fields(vec2([args[1], args[2]]), args[0], args[3], [args[4], args[5]], a).map(|_| ())
    }
}
