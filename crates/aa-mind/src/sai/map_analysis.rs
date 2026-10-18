//! Original `KKC_SAIMap::DynamicMapAnalysis` (`7AC0D0`, Source/Sai/
//! SAIMap.cpp) with its callees: the influence reset `7AA5B0`, the region
//! actor-list iterators `7A9C60`/`7A9C80` (`7B9180`, `7BB480`, `7BAC00`),
//! the learning multiplier `7AE350` (`7AD890`, `7ADDA0`, `4A5E40`, CRT pow
//! `96D890`), the settlement spread `7ABA70`, the mobile spread `7AB940`,
//! the recursive propagation `7AA620`, the land test `45D720`, and the
//! accumulators `7B9050`/`7BB780`/`7BB820`.
//!
//! Per region (map +14/+18), every resolvable, not-deleting actor of the
//! region's static lists (+34) is collected (GTID list, newest first), the
//! fiber yields (`79DF70`), then each actor (re-resolved) with a positive
//! combat value (`4D0180`) spreads its value, scaled per SAI player by
//! `7AE350`, over the regions: once within its settlement range (`7ABA70`,
//! into entry +14) and once within the ego's border range (into entry +20);
//! then the fiber yields. The mobile lists (+28) follow with `7AB940` (range
//! = speed (capped by `DynamicMapAnalysisSpeedCap`) * ego
//! cv_influence_max_travel_time, into entry +8). Finally the analysis
//! reschedules itself (`79EFF0(-1, dynamic_map_analysis_think_frequency,
//! [B28A88], 0)`) and clears `[B28A88]`.
//!
//! Influence vectors are indexed by the source owner's +188 index and grow
//! zero-filled; the spread values are binary32 stores at the x87 precision.
//! The world (GTID registry, actor virtuals, the yield) is a service, as the
//! fiber yields let the simulation run between the native steps.
use super::ego::Ego;
use super::native_hash::NativeHash;
use crate::{Error, Result, x87::num};
use aa_numeric::{
    Binary, Precision,
    crt_pow::{CrtEnvironment, ci_pow_x87},
};
use std::cmp::Ordering;

const ONE: u32 = 0x3f80_0000;
/// binary32 1e6: the "unreached" distance filled by `4A8700`.
const FAR: u32 = 0x4974_2400;

/// One record of a SAI player's learning list (+170 table value):
/// `{value +0, time +4, +8, next +C}`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LearningRecord {
    pub value: u32,
    pub time: u32,
    pub word8: u32,
}

/// The SAI player fields the analysis reads (`[region entry]+0`).
#[derive(Clone, Debug, PartialEq)]
pub struct SaiPlayer {
    /// +0: the player's slot in every region's entry array.
    pub slot: u32,
    /// +1C: index of the player's ego in the `egos` slice.
    pub ego: usize,
    /// +170: learning lists by object (null list = `None`), head first.
    pub learning: NativeHash<Option<Vec<LearningRecord>>>,
}

/// Per-region influence of one SAI player (`region +1C` entry): vectors by
/// source owner index, +8 mobile, +14 settlement, +20 border.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegionEntry {
    /// +0: index into the players slice.
    pub player: usize,
    pub mobile: Vec<u32>,
    pub settlement: Vec<u32>,
    pub border: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MapRegion {
    /// +0 region index (indexes the spread vectors and the region array).
    pub index: u32,
    /// +5C: neighbour distances (key region index, binary32 distance).
    pub neighbors: NativeHash<u32>,
    /// +1C/+20: one entry per SAI player slot.
    pub entries: Vec<Option<RegionEntry>>,
    /// +34 (static) and +28 (mobile): the actor lists, each a GTID list in
    /// node order, lists in array order.
    pub static_lists: Vec<Vec<u32>>,
    pub mobile_lists: Vec<Vec<u32>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaiMap {
    /// +4: the static analysis exists.
    pub analysed: bool,
    pub regions: Vec<MapRegion>,
}

/// SVars read through `4B2E60` (binary32 bits).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MapSvars {
    pub speed_cap: u32,
    pub settlement_cv_range_fudge_factor: u32,
    pub min_cv_propogation_factor: u32,
    pub think_frequency: u32,
}

/// World services of `7AC0D0` in native order (handles are opaque words).
pub trait MapWorld {
    /// `[B364C0]` GTID registry: the live object, if any.
    fn resolve(&mut self, gtid: u32) -> Result<Option<u32>>;
    /// Object +8 bit 0.
    fn deleting(&mut self, object: u32) -> Result<bool>;
    /// Object +14.
    fn object_gtid(&mut self, object: u32) -> Result<u32>;
    /// Object virtual +108: the owner whose +188 indexes the influence.
    fn owner(&mut self, object: u32) -> Result<u32>;
    /// Owner +188.
    fn owner_index(&mut self, owner: u32) -> Result<u32>;
    /// `4D0180`: the object's combat value (ST0, unstored).
    fn combat_value(&mut self, object: u32) -> Result<Binary>;
    /// Object +98 is set.
    fn has_organization(&mut self, object: u32) -> Result<bool>;
    /// `4CEDA0` (called only when +98 is set): `[[object+98]+14]`.
    fn organization_actor(&mut self, object: u32) -> Result<u32>;
    /// Object +4: its definition (the speed-cap log reads its +8 name).
    fn definition(&mut self, object: u32) -> Result<u32>;
    /// Object +B0 and its +2C word.
    fn settlement_component_range(&mut self, object: u32) -> Result<Option<u32>>;
    /// Object +14C (signed count) and, when it exceeds 5, `[[object+148]+14]`
    /// with its +8 word (`None` when that slot is null).
    fn slot_count(&mut self, object: u32) -> Result<i32>;
    fn slot5_range(&mut self, object: u32) -> Result<Option<u32>>;
    /// Object virtual +104 (ST0, unstored).
    fn speed(&mut self, object: u32) -> Result<Binary>;
    /// `7B9320` on the region's first rectangle: its tile terrain type.
    fn region_terrain(&mut self, region: u32) -> Result<u32>;
    /// `[B36534]+E0` (binary32).
    fn game_time(&mut self) -> Result<u32>;
    /// `79DF70(0)` on the SAI fiber.
    fn yield_slice(&mut self) -> Result<()>;
    /// `79EFF0(-1, delay, immediate, 0)` on `[B36528]`.
    fn schedule(&mut self, delay: u32, immediate: bool) -> Result<()>;
}

fn f(bits: u32) -> Result<Binary> {
    num(Binary::from_f32_bits(bits))
}
fn store(v: &Binary) -> Result<u32> {
    num(v.store_f32())
}
fn less(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Less
}
fn greater(a: &Binary, b: &Binary) -> bool {
    a.compare(b) == Ordering::Greater
}

/// Region access as the native does it (`[map+14][index]`).
fn region(map: &SaiMap, index: u32) -> Result<&MapRegion> {
    map.regions
        .get(index as usize)
        .ok_or_else(|| Error::World(format!("region index {index} outside the SAI map")))
}

/// `45D720(7B9320(...))`: terrain type 0.
fn land(w: &mut impl MapWorld, region: u32) -> Result<bool> {
    Ok(w.region_terrain(region)? == 0)
}

/// `_CIpow` at an x87 call site (`fld base; fld exponent; call 96D890`).
fn pow(base: &Binary, exponent: &Binary, crt: CrtEnvironment) -> Result<Binary> {
    let (value, outcome) = ci_pow_x87(base, exponent, crt)
        .map_err(|e| Error::World(format!("CRT pow 96D890: {e}")))?;
    if outcome.libm_error.is_some() {
        return Err(Error::World(format!(
            "CRT pow 96D890 reported libm error {:?}",
            outcome.libm_error
        )));
    }
    Ok(value)
}

/// Shared arguments of the spread recursion.
struct Spread<'a> {
    value: u32,
    exponent: u32,
    range: u32,
    raw_range: u32,
    mobile: bool,
    dist: &'a mut Vec<u32>,
    val: &'a mut Vec<u32>,
}

fn slot(v: &mut [u32], index: u32) -> Result<&mut u32> {
    v.get_mut(index as usize).ok_or_else(|| {
        Error::World(format!(
            "spread vector index {index} outside the region count"
        ))
    })
}

/// `7AA620(region index, distance)`.
#[allow(clippy::too_many_arguments)]
fn propagate(
    map: &SaiMap,
    s: &mut Spread<'_>,
    index: u32,
    distance: u32,
    w: &mut impl MapWorld,
    sv: &MapSvars,
    p: Precision,
    crt: CrtEnvironment,
) -> Result<()> {
    let m = f(sv.min_cv_propogation_factor)?;
    let one = f(ONE)?;
    let q = num(f(distance)?.div(&f(s.range)?, p))?;
    let t = num(one.sub(&num(q.mul(&num(one.sub(&m, p))?, p))?, p))?;
    let mut factor = store(&t)?;
    if less(&t, &m) {
        factor = sv.min_cv_propogation_factor;
    } else if greater(&f(factor)?, &one) {
        factor = ONE;
    }
    *slot(s.dist, index)? = distance;
    let weighted = num(pow(&f(factor)?, &f(s.exponent)?, crt)?.mul(&f(s.value)?, p))?;
    *slot(s.val, index)? = store(&weighted)?;
    let neighbors: Vec<(u32, u32)> = region(map, index)?
        .neighbors
        .buckets
        .iter()
        .flatten()
        .copied()
        .collect();
    for (key, step) in neighbors {
        let d2 = store(&num(f(distance)?.add(&f(step)?, p))?)?;
        let within = s.mobile || !greater(&f(d2)?, &f(s.raw_range)?);
        let closer = less(&f(d2)?, &f(*slot(s.dist, key)?)?);
        if closer && less(&f(d2)?, &f(s.range)?) && land(w, key)? && within {
            propagate(map, s, key, d2, w, sv, p, crt)?;
        }
    }
    Ok(())
}

/// The `4A8700(n, 1e6)` / `45F180(n)` vector reset and the seed region.
fn seed(
    n: usize,
    region_index: u32,
    value: u32,
    dist: &mut Vec<u32>,
    val: &mut Vec<u32>,
) -> Result<()> {
    *dist = vec![FAR; n];
    *val = vec![0; n];
    *slot(dist, region_index)? = 0;
    *slot(val, region_index)? = value;
    Ok(())
}

fn neighbors_of(map: &SaiMap, r: usize) -> Vec<(u32, u32)> {
    map.regions[r]
        .neighbors
        .buckets
        .iter()
        .flatten()
        .copied()
        .collect()
}

/// `7AE350(object)`: the player's learning multiplier for an object.
fn learning_multiplier(
    player: &mut SaiPlayer,
    ego: &Ego,
    object: u32,
    w: &mut impl MapWorld,
    p: Precision,
    crt: CrtEnvironment,
) -> Result<Binary> {
    let one = f(ONE)?;
    let time = w.game_time()?;
    let duration = ego.u32(0x940);
    let horizon = store(&num(f(time)?.sub(&f(duration)?, p))?)?;
    let Some(entry) = player.learning.get_mut(object)? else {
        return Ok(one);
    };
    let Some(list) = entry else {
        return Ok(one);
    };
    // 7AD890: drop records not newer than the horizon (head first).
    while let Some(first) = list.first() {
        if greater(&f(first.time)?, &f(horizon)?) {
            break;
        }
        list.remove(0);
    }
    if list.is_empty() {
        // The list is released and 7ADDA0 removes the object's entry.
        player.learning.erase(object)?;
        return Ok(one);
    }
    let scale = ego.image[0x960] != 0;
    let mut sum = Binary::from_i32(0);
    for r in list.iter() {
        let term = if scale {
            let age = num(num(f(time)?.sub(&f(r.time)?, p))?.div(&f(duration)?, p))?;
            num(num(one.sub(&age, p))?.mul(&f(r.value)?, p))?
        } else {
            f(r.value)?
        };
        sum = num(sum.add(&term, p))?;
    }
    let base = num(sum.add(&one, p))?;
    let power = store(&pow(&base, &f(ego.u32(0x958))?, crt)?)?;
    // 4A5E40(cv_ceiling): (ceiling - 1) * learning_factor + 1.
    let ceiling = num(num(f(ego.u32(0x95c))?.sub(&one, p))?.mul(&f(ego.u32(0x93c))?, p))?;
    let ceiling = store(&num(ceiling.add(&one, p))?)?;
    if less(&f(power)?, &one) {
        return Ok(one);
    }
    if greater(&f(power)?, &f(ceiling)?) {
        return f(ceiling);
    }
    f(power)
}

/// `7B9050` + `7BB780` / `7BB820`: add a spread value into the region's
/// entry for the player, at the source owner's index (zero-filled growth).
fn accumulate(
    map: &mut SaiMap,
    region_index: usize,
    player_slot: u32,
    owner_index: u32,
    value: u32,
    which: Vector,
    p: Precision,
) -> Result<()> {
    let entry = map.regions[region_index]
        .entries
        .get_mut(player_slot as usize)
        .and_then(|e| e.as_mut())
        .ok_or_else(|| {
            Error::World(format!(
                "region {region_index} has no entry for SAI player slot {player_slot} (native null this)"
            ))
        })?;
    let v = match which {
        Vector::Mobile => &mut entry.mobile,
        Vector::Settlement => &mut entry.settlement,
        Vector::Border => &mut entry.border,
    };
    while v.len() <= owner_index as usize {
        v.push(0);
    }
    let sum = num(f(value)?.add(&f(v[owner_index as usize])?, p))?;
    v[owner_index as usize] = store(&sum)?;
    Ok(())
}

#[derive(Clone, Copy)]
enum Vector {
    Mobile,
    Settlement,
    Border,
}

/// Non-null entries of a region's player array, in slot order.
fn players_of(map: &SaiMap, r: usize) -> Vec<usize> {
    map.regions[r]
        .entries
        .iter()
        .flatten()
        .map(|e| e.player)
        .collect()
}

/// Resolvable, not-deleting objects of the lists, as the GTID list the
/// native builds (newest first).
fn collect(lists: &[Vec<u32>], w: &mut impl MapWorld) -> Result<Vec<u32>> {
    let mut out = Vec::new();
    for gtid in lists.iter().flatten() {
        if let Some(object) = w.resolve(*gtid)?
            && !w.deleting(object)?
        {
            out.insert(0, w.object_gtid(object)?);
        }
    }
    Ok(out)
}

/// `cv > 0` on the unstored ST0 (`fcomp 0; test ah,41; jnp`).
fn positive(cv: &Binary) -> bool {
    cv.compare(&Binary::from_i32(0)) == Ordering::Greater
}

/// Complete `7AC0D0` (diagnostic log excluded).
#[allow(clippy::too_many_arguments)]
pub fn dynamic_map_analysis(
    map: &mut SaiMap,
    players: &mut [SaiPlayer],
    egos: &[Ego],
    w: &mut impl MapWorld,
    sv: &MapSvars,
    immediate: &mut bool,
    p: Precision,
    crt: CrtEnvironment,
) -> Result<()> {
    if map.analysed {
        let n = map.regions.len();
        let (mut dist, mut val) = (Vec::new(), Vec::new());
        // 7AA5B0.
        for r in map.regions.iter_mut().rev() {
            for e in r.entries.iter_mut().flatten() {
                e.mobile.clear();
                e.settlement.clear();
                e.border.clear();
            }
        }
        for r in 0..n {
            let index = map.regions[r].index;
            let gtids = collect(&map.regions[r].static_lists.clone(), w)?;
            w.yield_slice()?;
            for gtid in gtids {
                let Some(mut object) = w.resolve(gtid)? else {
                    continue;
                };
                if w.deleting(object)? {
                    continue;
                }
                let owner = w.owner(object)?;
                let cv = w.combat_value(object)?;
                let cv32 = store(&cv)?;
                if !positive(&cv) {
                    continue;
                }
                if w.has_organization(object)? {
                    object = w.organization_actor(object)?;
                }
                let raw_range = match w.settlement_component_range(object)? {
                    Some(range) => range,
                    None if w.slot_count(object)? > 5 => w.slot5_range(object)?.unwrap_or(0),
                    None => 0,
                };
                for pi in players_of(map, r) {
                    let ego = egos.get(players[pi].ego).ok_or_else(|| {
                        Error::World("SAI player without an ego (native null +1C)".into())
                    })?;
                    let exponent = ego.u32(0x28);
                    let border = ego.u32(0x30);
                    let learn = learning_multiplier(&mut players[pi], ego, object, w, p, crt)?;
                    let value = store(&num(learn.mul(&f(cv32)?, p))?)?;
                    // 7ABA70.
                    let range = store(&num(
                        f(raw_range)?.mul(&f(sv.settlement_cv_range_fudge_factor)?, p)
                    )?)?;
                    seed(n, index, value, &mut dist, &mut val)?;
                    for (key, step) in neighbors_of(map, r) {
                        let within = !greater(&f(step)?, &f(raw_range)?);
                        if less(&f(step)?, &f(range)?) && land(w, key)? && within {
                            let mut s = Spread {
                                value,
                                exponent,
                                range,
                                raw_range,
                                mobile: false,
                                dist: &mut dist,
                                val: &mut val,
                            };
                            propagate(map, &mut s, key, step, w, sv, p, crt)?;
                        }
                    }
                    let owner_index = w.owner_index(owner)?;
                    let slot_index = players[pi].slot;
                    for i in 0..n {
                        if greater(&f(val[i])?, &Binary::from_i32(0)) {
                            accumulate(
                                map,
                                i,
                                slot_index,
                                owner_index,
                                val[i],
                                Vector::Settlement,
                                p,
                            )?;
                        }
                    }
                    // Border spread from this region.
                    seed(n, index, value, &mut dist, &mut val)?;
                    for (key, step) in neighbors_of(map, r) {
                        if less(&f(step)?, &f(border)?) && land(w, key)? {
                            let mut s = Spread {
                                value,
                                exponent,
                                range: border,
                                raw_range: 0,
                                mobile: true,
                                dist: &mut dist,
                                val: &mut val,
                            };
                            propagate(map, &mut s, key, step, w, sv, p, crt)?;
                        }
                    }
                    for i in 0..n {
                        if greater(&f(val[i])?, &Binary::from_i32(0)) {
                            accumulate(map, i, slot_index, owner_index, val[i], Vector::Border, p)?;
                        }
                    }
                }
                w.yield_slice()?;
            }
            let gtids = collect(&map.regions[r].mobile_lists.clone(), w)?;
            w.yield_slice()?;
            for gtid in gtids {
                let Some(object) = w.resolve(gtid)? else {
                    continue;
                };
                if w.deleting(object)? {
                    continue;
                }
                let definition = w.definition(object)?;
                let owner = w.owner(object)?;
                let cv = store(&w.combat_value(object)?)?;
                let speed = w.speed(object)?;
                let mut speed32 = store(&speed)?;
                if greater(&speed, &f(sv.speed_cap)?) {
                    // "SPEED CAP VIOLATED by %s ..." reads the definition +8
                    // name before the cap is applied.
                    if definition == 0 {
                        return Err(Error::World(
                            "speed-cap log reads the name of a null definition (native null dereference)".into(),
                        ));
                    }
                    speed32 = sv.speed_cap;
                }
                if !greater(&f(cv)?, &Binary::from_i32(0)) {
                    continue;
                }
                for pi in players_of(map, r) {
                    let ego = egos.get(players[pi].ego).ok_or_else(|| {
                        Error::World("SAI player without an ego (native null +1C)".into())
                    })?;
                    let learn = learning_multiplier(&mut players[pi], ego, object, w, p, crt)?;
                    let value = store(&num(learn.mul(&f(cv)?, p))?)?;
                    // 7AB940.
                    let range = store(&num(f(speed32)?.mul(&f(ego.u32(0x2c))?, p))?)?;
                    let exponent = ego.u32(0x28);
                    seed(n, index, value, &mut dist, &mut val)?;
                    for (key, step) in neighbors_of(map, r) {
                        if less(&f(step)?, &f(range)?) && land(w, key)? {
                            let mut s = Spread {
                                value,
                                exponent,
                                range,
                                raw_range: 0,
                                mobile: true,
                                dist: &mut dist,
                                val: &mut val,
                            };
                            propagate(map, &mut s, key, step, w, sv, p, crt)?;
                        }
                    }
                    let owner_index = w.owner_index(owner)?;
                    let slot_index = players[pi].slot;
                    for i in 0..n {
                        if greater(&f(val[i])?, &Binary::from_i32(0)) {
                            accumulate(map, i, slot_index, owner_index, val[i], Vector::Mobile, p)?;
                        }
                    }
                }
                w.yield_slice()?;
            }
        }
    }
    w.schedule(sv.think_frequency, *immediate)?;
    *immediate = false;
    Ok(())
}
