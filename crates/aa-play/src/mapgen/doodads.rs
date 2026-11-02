//! The generator stages after the required objects, as the original runs them:
//! - PlaceGroup 0x594590 (StagePlaceNonRequiredObjects 0x595180 for a group
//!   that is not balanced) and CreateExtraMarkers 0x594260 (the balanced
//!   group's grid scan after CreateMarkers 0x594100);
//! - KKC_RMCPlaceWorldParametersActorsStage::Run 0x594930;
//! - the terrain's automatic doodads (0x552BF0, one PlaceAutoDoodads 0x54FB40
//!   per tile);
//! - CleanupBlockingDoodads 0x592170;
//! - KKC_RMGHeroesStage::Run 0x593040 (0x592D20, AddHeroesToKingdom 0x592E00);
//! - CreateObject 0x5910D0.
//!
//! Arithmetic follows the x87 code at PC53 (binary64 registers, binary32
//! stores where the original stores).
use super::super::rng::{Rng, UNIT};
use super::super::space::FACING;
use super::{Create, Placement, World};

/// The weighted table of 0x4B6FF0, after the caller's getter: last entry of
/// `list` first (0x4B6BA0 pushes at the head), the total a binary32 running
/// sum; no draw on an empty table.
pub(super) fn weighted_pick(rng: &mut Rng, list: &[(usize, f32)]) -> Option<usize> {
    let mut total = 0.0f32;
    for &(_, w) in list {
        total = (f64::from(w) + f64::from(total)) as f32;
    }
    let (&(mut pick, first), rest) = list.split_last()?;
    let u = f64::from(rng.at(0x4B7008).draw_after_getter()) * UNIT;
    let mut r = u * f64::from(total) - f64::from(first);
    for &(t, w) in rest.iter().rev() {
        if r < 0.0 {
            break;
        }
        pick = t;
        r -= f64::from(w);
    }
    Some(pick)
}

impl Placement<'_> {
    /// CreateObject 0x5910D0: a thing at the ground height (0x122), an actor
    /// at height 0 with its owner; then 0x4FB640 for a definition with a
    /// start location priority.
    pub(super) fn create_object(&mut self, world: &mut World, rng: &mut Rng, thing: usize, gi: Option<usize>, owner: Option<usize>, pos: [f32; 2], facing: [f32; 2]) -> Option<usize> {
        let actor = self.is_actor(thing);
        let z = if actor { 0.0 } else { super::super::height::surface(world, self.data, pos, 0x122) as f32 };
        let request = Create { thing, pos, z, facing, owner: if actor { owner } else { None }, group: gi, ..Default::default() };
        let index = self.create_gthing(world, rng, request)?;
        self.fix_start(world, owner, index);
        Some(index)
    }

    /// Definition virtual +0x38: the actor classes.
    pub(super) fn is_actor(&self, thing: usize) -> bool {
        matches!(self.data.things[thing].native_kind, 2 | 19)
    }

    /// PlaceGroup 0x594590. Returns (created, minimum).
    pub(super) fn plain_group_exact(&mut self, world: &mut World, gi: usize, value: f32, rng: &mut Rng) -> (i32, i32) {
        let def = self.groups[gi].def.clone();
        let owner = self.groups[gi].kingdom;
        let scale = self.area_scale_exact(def.slider.area_factor);
        let mut count = super::super::blob::ftol_d(scale * f64::from(value));
        let minimum = super::super::blob::ftol_d(scale * f64::from(def.objects_min));
        let mut created = 0;
        while count >= 1 {
            rng.getter();
            let Some(thing) = self.pick_weighted_after_getter(gi, rng) else { break };
            self.required_facing(thing, rng);
            count -= 1;
            let Some((pos, facing)) = self.find_random_position(rng, gi, thing, false, 90) else {
                if minimum <= created + count {
                    continue;
                }
                break;
            };
            if let Some(index) = self.create_object(world, rng, thing, Some(gi), owner, pos, facing) {
                self.group_add(gi, index);
                created += 1;
            }
        }
        (created, minimum)
    }

    /// CreateExtraMarkers 0x594260: the markers still missing, scanning tile
    /// centres row by row: a new pick (getter, 0x4B6FF0, facing 0x5931C0) only
    /// after the last one was used; CheckPositionValid in place (no conform);
    /// CreateObject with the marker's zeroed facing (+0x1C, so the constructor
    /// replaces it). Returns how many were made.
    pub(super) fn extra_markers(&mut self, world: &mut World, gi: usize, need: i32, rng: &mut Rng) -> i32 {
        let owner = self.groups[gi].kingdom;
        let tile = f64::from(world.tile);
        let (tw, th) = (world.tw as i32, world.th as i32);
        let mut need = need;
        let mut extra = 0;
        let mut pick = true;
        let mut thing = 0;
        let mut facing = FACING.map(f32::from_bits);
        for y in 0..th {
            let py = ((f64::from(y) + 0.5) * tile) as f32;
            if need <= 0 {
                break;
            }
            for x in 0..tw {
                if need <= 0 {
                    break;
                }
                let px = ((f64::from(x) + 0.5) * tile) as f32;
                if pick {
                    rng.getter();
                    let Some(t) = self.pick_weighted_after_getter(gi, rng) else { return extra };
                    thing = t;
                    facing = self.required_facing(thing, rng);
                    pick = false;
                }
                let mut pos = [px, py];
                if !self.check_position(Some(gi), thing, &mut pos, true, facing, false) {
                    continue;
                }
                if let Some(index) = self.create_object(world, rng, thing, Some(gi), owner, pos, [0.0, 0.0]) {
                    self.group_add(gi, index);
                    need -= 1;
                    extra += 1;
                    pick = true;
                }
            }
        }
        extra
    }

    /// One tile of 0x552BF0: PlaceAutoDoodads 0x54FB40.
    fn auto_doodad(&mut self, world: &mut World, rng: &mut Rng, t: usize) {
        let terrain = self.data.variations[world.var[t] as usize].terrain;
        let def = &self.data.terrains[terrain];
        let (frequency, primary) = (def.doodad_frequency, def.primary_doodad_frequency);
        let r = rng.at(0x54FB88).unit_d() as f32;
        if !(r < frequency) {
            return;
        }
        let half = (f64::from(world.tile) * 0.5) as f32;
        let range = (f64::from(world.tile) - f64::from(f32::from_bits(0x38D1_B717))) as f32;
        let [cx, cy] = world.tile_center(t);
        let u = rng.at(0x54FC77).unit_d();
        let x = ((f64::from(cx) - f64::from(half)) + u * f64::from(range)) as f32;
        let u = rng.at(0x54FCB5).unit_d();
        let y = ((f64::from(cy) - f64::from(half)) + u * f64::from(range)) as f32;
        let empty = super::super::data::DoodadLists::default();
        let lists = self.data.doodads.get(&terrain).unwrap_or(&empty);
        let mut thing = None;
        if f64::from(r) / f64::from(frequency) < f64::from(primary) {
            rng.getter();
            thing = weighted_pick(rng, &lists.primary);
        }
        if thing.is_none() {
            rng.getter();
            thing = weighted_pick(rng, &lists.normal);
        }
        let Some(thing) = thing else { return };
        let request = Create { thing, pos: [x, y], ..Default::default() };
        if let Some(index) = self.create_gthing(world, rng, request) {
            // 0x4EE930(1): a doodad of this tile, gone when the terrain changes.
            self.objects[index].auto = true;
        }
    }

    /// 0x552BF0: every tile, rows outer.
    pub(in crate::mapgen) fn place_auto_doodads(&mut self, world: &mut World, rng: &mut Rng) {
        for t in 0..world.tw * world.th {
            self.auto_doodad(world, rng, t);
        }
    }

    /// 0x4F5560 / the body of 0x592170: the doodads overlapping a placement
    /// that do not block (bit 10 clear) and no trigger refers to (a generated
    /// world has no triggers yet) are deleted; the list is collected first.
    pub(super) fn clear_doodads(&mut self, placement: &aa_spatial::geometry::Placement) {
        for member in self.space.overlapping(placement, None) {
            let Some(&index) = self.by_member.get(&member) else { continue };
            let o = &self.objects[index];
            if !o.alive {
                continue;
            }
            let def = &self.data.things[o.thing];
            if matches!(def.runtime_class, 2 | 9) && def.flags & (1 << 10) == 0 {
                self.delete_object(index);
            }
        }
    }

    /// CleanupBlockingDoodads 0x592170: in GTID order, every blocking static
    /// actor clears the doodads under its own shape (0x554350).
    pub(in crate::mapgen) fn cleanup_blocking_doodads(&mut self) {
        let mut order: Vec<usize> = (0..self.objects.len()).filter(|&i| self.objects[i].alive).collect();
        order.sort_by_key(|&i| self.objects[i].id);
        for index in order {
            let o = &self.objects[index];
            if !o.alive || !self.is_actor(o.thing) {
                continue;
            }
            let def = &self.data.things[o.thing];
            if def.flags & (1 << 29) == 0 || def.flags & (1 << 10) == 0 {
                continue;
            }
            let placement = self.blocking_placement(index);
            self.clear_doodads(&placement);
        }
    }

    /// KKC_RMGHeroesStage::Run 0x593040 with the hero count of the settings
    /// (+0x70). No film of the corpus has one; the draws of 0x592E00 are not
    /// ported, so a world asking for heroes says so.
    pub(in crate::mapgen) fn place_heroes(&mut self, _rng: &mut Rng, count: u32) {
        if count != 0 {
            self.notes.push(format!("{count} random heroes per kingdom: AddHeroesToKingdom 0x592E00 is not ported"));
        }
    }
}
