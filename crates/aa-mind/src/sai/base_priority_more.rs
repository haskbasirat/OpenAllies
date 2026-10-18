//! Original SAI goal-class base priorities (goal virtual `+78`) of the
//! exploration, building and unpacking classes, conventions as in
//! `base_priority`:
//!
//! | class | +78 | Rust |
//! |---|---|---|
//! | EXPLORE | 7C29F0 | [`explore`] |
//! | BUILD | 7BFA30 | [`build`] |
//! | UPGRADE | 7C0870 | [`upgrade`] |
//! | DESTROY | 7C0BD0 | [`destroy`] |
//! | UNPACK | 7CBC40 | [`unpack`] |
//!
//! Ported callees: the exploration-goal getters `4A5EC0`/`4A5EF0`/`4A5F20`/
//! `4A5F50` (ego `+464`/`+528`/`+5EC` sets), the organization lookup
//! `4CEDA0`, the island lookup `7B8FD0`, `4282A0`. Seams: placement searches
//! `557190`/`557010`, `587B60`, `566110`, the region search `7BB1C0`, and the
//! ego/player valuations `7AE4C0`, `7ADA20`, `4A6DB0`, `4A6D40`, `4A6CE0`,
//! `4AC970`, `4A63A0`, `4A9AE0`. The SAI RNG (`[B36528]+2C`, `85D840`) is
//! drawn directly by DESTROY and UNPACK (and handed to the placement seams).
use super::base_priority::{ClassGoal, GeneralPowerInstance, Obj, PlayerContext, bonus, owned_by, targets};
use super::base_priority_misc::{ClassPriorityWorld, point_on_map};
use super::ego::EXPLORATION_GOALS;
use crate::{
    Error, Result,
    x87::{n, num, store, unit_random},
};
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;

/// Services of these bodies beyond [`ClassPriorityWorld`].
pub trait MorePriorityWorld: ClassPriorityWorld {
    /// Field: the word `[[B39AB0]+108]` (exploration enabled when nonzero).
    fn exploration_setting(&mut self) -> Result<u32>;
    /// Field: `[B3652C]+2C` primary island `+4/+8` region array.
    fn primary_island_regions(&mut self) -> Result<Vec<Option<Obj>>>;
    /// Field: `7B8FF0(kingdom)` on a region: the region-kingdom record's `+8`
    /// explored count and its region's `+C` area (`None` = null record).
    fn region_exploration(&mut self, region: Obj, kingdom: Option<Obj>) -> Result<Option<(i32, i32)>>;
    /// Field: `[[obj+98]+14]`, `None` when `+98` is null.
    fn organization_14(&mut self, obj: Obj) -> Result<Option<Option<Obj>>>;
    /// Field: object `+9C` component.
    fn component_9c(&mut self, obj: Obj) -> Result<Obj>;
    /// Field: SAI actor record `+24` table object.
    fn record_table_object(&mut self, record: Obj) -> Result<Option<Obj>>;
    /// Field: SAI actor record `+28` byte.
    fn record_byte_28(&mut self, record: Obj) -> Result<bool>;
    /// Field: binary32 word of a template entry (`4A6D40` result).
    fn entry_word(&mut self, entry: Obj, offset: u32) -> Result<u32>;
    /// Field: the player object's economy arrays `+1A0/+1A4`, `+1B8` and
    /// `+1C4/+1C8`.
    fn economy_arrays(&mut self, kingdom: Obj) -> Result<[Vec<u32>; 3]>;
    /// Field: word `+534` of an object's definition `+4`.
    fn definition_534(&mut self, obj: Obj) -> Result<u32>;
    /// Field: `7B8FD0`: the island of a region, its `+4/+8` regions.
    fn island_regions(&mut self, region: Obj) -> Result<Vec<Option<Obj>>>;
    /// `85D840` on the SAI RNG `[B36528]+2C`.
    fn sai_random(&mut self) -> Result<u32>;
    /// Object virtual `+34(kind)` (is-a test).
    fn virtual_34(&mut self, obj: Obj, kind: u32) -> Result<bool>;
    /// SAI actor record virtual `+8`: its region.
    fn record_region(&mut self, record: Obj) -> Result<Obj>;
    /// `587B60(definition, -1.0, -1.0, 0, 0, 0)` on an organization
    /// component (`+98`).
    fn can_build(&mut self, component: Obj, definition: u32) -> Result<bool>;
    /// `557190(&site, definition, kingdom, builder, rng)` on `[B36534]+30`.
    fn build_site(
        &mut self,
        definition: u32,
        kingdom: Option<Obj>,
        builder: Option<Obj>,
    ) -> Result<Option<[u32; 2]>>;
    /// `557010(&site, layout, kingdom, region, rng, 1, 0)` on `[B36534]+30`.
    fn unpack_site(
        &mut self,
        layout: u32,
        kingdom: Option<Obj>,
        region: Obj,
    ) -> Result<Option<[u32; 2]>>;
    /// `566110(definition, 0, 0)` on a structure component (`+9C`).
    fn can_upgrade(&mut self, component: Obj, definition: u32) -> Result<bool>;
    /// `7BB1C0(&list, depth, 0)` on a region: the regions it collects.
    fn regions_within(&mut self, region: Obj, depth: u32) -> Result<Vec<Obj>>;
    /// `7AE4C0(definition, point, 1, 1)` on the SAI player (ST0).
    fn build_value(&mut self, definition: u32, point: [u32; 2]) -> Result<Binary>;
    /// `7ADA20(actor, definition)` on the SAI player (ST0).
    fn upgrade_value(&mut self, actor: Option<Obj>, definition: u32) -> Result<Binary>;
    /// `4A6DB0(record, definition)` on a template table (ST0).
    fn template_value(&mut self, table: Obj, record: Obj, definition: u32) -> Result<Binary>;
    /// `4A6D40(record)` on a template table: the first entry it lacks.
    fn template_missing(&mut self, table: Obj, record: Obj) -> Result<Option<Obj>>;
    /// `4A6CE0(definition)` on a template table.
    fn template_has(&mut self, table: Obj, definition: Obj) -> Result<bool>;
    /// `4AC970(player, target, &+4C, &+58, differences, economy)` on the ego
    /// (ST0); it fills the goal vectors `+4C`/`+58` (world-owned).
    fn destroy_value(
        &mut self,
        target: Option<Obj>,
        differences: &[u32],
        economy: &[u32],
    ) -> Result<Binary>;
    /// `4A63A0(player, record)` on the ego (ST0).
    fn destroy_record_value(&mut self, record: Option<Obj>) -> Result<Binary>;
    /// `4A9AE0(record, region, player, point, same_region, rng)` on the ego
    /// (ST0).
    fn unpack_value(
        &mut self,
        record: Obj,
        region: Obj,
        point: [u32; 2],
        same_region: bool,
    ) -> Result<Binary>;
}

/// DESTROY's process-wide statics `[B40EC0]` / `B28A98` / `B28AA4`: computed
/// from the first evaluating player's economy and never refreshed (native).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DestroyStatics {
    /// `B28AA4`: `+1A0[i] - +1B8[i]` (binary32).
    pub differences: Vec<u32>,
    /// `B28A98`: copy of `+1C4`.
    pub economy: Vec<u32>,
}

fn null(what: &str) -> Error {
    Error::World(format!("{what} (native null dereference)"))
}
fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
fn add(a: u32, b: u32, p: Precision) -> Result<u32> {
    store(num(n(a)?.add(&n(b)?, p))?)
}
fn positive_or_unordered(bits: u32) -> bool {
    !(host(bits) <= 0.0)
}

/// Ego exploration getters `4A5EC0`/`4A5EF0`/`4A5F20`/`4A5F50`: field
/// `slot` (1..=4) of exploration set `kind` (0, 1, 2; others use set 0).
pub fn exploration_field(ctx: &PlayerContext<'_>, kind: u32, slot: usize) -> u32 {
    let base = match kind {
        1 => EXPLORATION_GOALS[1],
        2 => EXPLORATION_GOALS[2],
        _ => EXPLORATION_GOALS[0],
    };
    ctx.ego.u32(base + 4 * slot)
}

/// `4CEDA0`: `[[obj+98]+14]`, else the first virtual-`+100` ancestor with
/// an organization.
pub fn organization(w: &mut impl MorePriorityWorld, obj: Obj) -> Result<Option<Obj>> {
    let mut at = obj;
    loop {
        if let Some(org) = w.organization_14(at)? {
            return Ok(org);
        }
        match w.virtual_100(at)? {
            Some(next) => at = next,
            None => return Ok(None),
        }
    }
}

/// The general-power scan with an instance predicate.
fn scan_with<W: MorePriorityWorld>(
    w: &mut W,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    kind: u32,
    ignore_additive: bool,
    mut matches: impl FnMut(&mut W, &GeneralPowerInstance) -> Result<bool>,
    p: Precision,
) -> Result<u32> {
    let mut max = 0_u32;
    for list in 0..2 {
        let gps = if list == 0 {
            w.sai_general_powers()?
        } else {
            w.player_general_powers()?
        };
        for gp in gps {
            if !owned_by(w, &gp, ctx.actor)? || !matches(w, &gp)? || gp.kind != kind {
                continue;
            }
            let value = bonus(&gp, ctx.ego);
            if gp.additive {
                if !ignore_additive {
                    goal.priority = add(value, goal.priority, p)?;
                }
            } else if host(max) == 0.0 || host(max) < host(value) {
                max = value;
            }
        }
    }
    Ok(max)
}

/// Complete `7C29F0` (EXPLORE, `+3C` exploration set, `+40` count): the set's
/// base minus count times its decrement; above its floor the excess shrinks
/// by the share of primary-island regions already explored past the set's
/// threshold; exploration bonuses (kind 9, additive ones ignored).
pub fn explore(
    w: &mut impl MorePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    if w.exploration_setting()? == 0 {
        goal.priority = 0;
        return Ok(());
    }
    let kind = goal.word(0x3c)?;
    let base = exploration_field(ctx, kind, 1);
    let step = exploration_field(ctx, kind, 2);
    let floor = exploration_field(ctx, kind, 3);
    let count = Binary::from_i32(goal.word(0x40)? as i32);
    let value = num(n(base)?.sub(&num(count.mul(&n(step)?, p))?, p))?;
    goal.priority = store(value.clone())?;
    if value.compare(&n(floor)?) == Ordering::Greater {
        let regions = w.primary_island_regions()?;
        let total = regions.len() as i32;
        let mut explored = 0_u32;
        for region in regions.into_iter().flatten() {
            let kingdom = ctx.actor.ok_or_else(|| null("7B8FF4 +18C of a null player object"))?;
            let (seen, area) = w
                .region_exploration(region, Some(kingdom))?
                .ok_or_else(|| null("7C2AA6 +8 of a null region record"))?;
            let threshold = exploration_field(ctx, kind, 4);
            // fcomp; test ah,41; jp: counted when the threshold is (ordered)
            // at most the explored fraction. A zero area divides with #Z
            // masked: +inf counts, -inf and 0/0 (NaN) do not.
            let counted = if area == 0 {
                seen > 0
            } else {
                let fraction =
                    store(num(Binary::from_i32(seen).div(&Binary::from_i32(area), p))?)?;
                host(threshold) <= host(fraction)
            };
            if counted {
                explored = add(explored, 0x3f80_0000, p)?;
            }
        }
        if total == 0 {
            return Err(Error::World(
                "explored share of an empty primary island is 0/0 (NaN, unsupported)".into(),
            ));
        }
        let ratio = num(n(explored)?.div(&Binary::from_i32(total), p))?;
        let excess = num(n(goal.priority)?.sub(&n(floor)?, p))?;
        let cut = num(ratio.mul(&excess, p))?;
        goal.priority = store(num(n(goal.priority)?.sub(&cut, p))?)?;
    }
    let max = scan_with(w, goal, ctx, 9, true, |_, _| Ok(true), p)?;
    goal.priority = add(max, goal.priority, p)?;
    Ok(())
}

/// Complete `7BFA30` (BUILD, `+3C` builder GTID, `+40` definition, `+44/+48`
/// site).
pub fn build(
    w: &mut impl MorePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    goal.priority = 0;
    let builder_id = goal.word(0x3c)?;
    let Some(builder) = w.resolve(builder_id)? else {
        return Ok(());
    };
    let Some(record) = w.sai_record(builder)? else {
        return Ok(());
    };
    let definition = goal.word(0x40)?;
    let b = w
        .resolve(builder_id)?
        .ok_or_else(|| null("7BFAF0 +98 of a stale builder"))?;
    let component = w.word_98(b)?;
    if !w.can_build(component, definition)? {
        return Ok(());
    }
    let builder = w.resolve(builder_id)?;
    let Some(site) = w.build_site(definition, ctx.actor, builder)? else {
        return Ok(());
    };
    goal.set_word(0x44, site[0])?;
    goal.set_word(0x48, site[1])?;
    goal.priority = store(w.build_value(definition, site)?)?;
    if let Some(table) = w.record_table_object(record)? {
        let v = w.template_value(table, record, definition)?;
        goal.priority = store(num(v.add(&n(goal.priority)?, p))?)?;
    }
    if !positive_or_unordered(goal.priority) {
        return Ok(());
    }
    let max = scan_with(
        w,
        goal,
        ctx,
        4,
        false,
        |w, gp| {
            let b = w
                .resolve(builder_id)?
                .ok_or_else(|| null("4CEDA0 of a stale builder"))?;
            let org = organization(w, b)?;
            if targets(w, gp, org)? {
                return Ok(true);
            }
            let b = w.resolve(builder_id)?;
            targets(w, gp, b)
        },
        p,
    )?;
    goal.priority = add(max, goal.priority, p)?;
    Ok(())
}

/// Complete `7C0870` (UPGRADE, `+3C` structure GTID, `+40` definition, `+44`
/// template share).
pub fn upgrade(
    w: &mut impl MorePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    goal.priority = 0;
    if w.goal_expired(goal.id)? {
        return Ok(());
    }
    let id = goal.word(0x3c)?;
    let definition = goal.word(0x40)?;
    let t = w.resolve(id)?.ok_or_else(|| null("7C08C7 +9C of a stale structure"))?;
    let component = w.component_9c(t)?;
    if !w.can_upgrade(component, definition)? {
        return Ok(());
    }
    let t = w.resolve(id)?;
    goal.priority = store(w.upgrade_value(t, definition)?)?;
    let t = w.resolve(id)?.ok_or_else(|| null("7C0943 virtual +100 of a stale structure"))?;
    let container = w.virtual_100(t)?;
    let record = match container {
        Some(c) => w.sai_record(c)?,
        None => None,
    };
    let table = match record {
        Some(r) => w.record_table_object(r)?,
        None => None,
    };
    goal.set_word(0x44, 0)?;
    if let (Some(table), Some(record), Some(container)) = (table, record, container) {
        let t = w.resolve(id)?;
        let org = organization(w, container)?;
        let value = if org == t && w.record_byte_28(record)? {
            let entry = w
                .template_missing(table, record)?
                .ok_or_else(|| null("7C09CD +18 of a null template entry"))?;
            let share = w.entry_word(entry, 0x18)?;
            goal.set_word(0x44, share)?;
            n(share)?
        } else {
            w.template_value(table, record, definition)?
        };
        goal.priority = store(num(value.add(&n(goal.priority)?, p))?)?;
    }
    if positive_or_unordered(goal.priority) {
        let max = scan_with(
            w,
            goal,
            ctx,
            4,
            false,
            |w, gp| {
                let t = w.resolve(id)?;
                targets(w, gp, t)
            },
            p,
        )?;
        goal.priority = add(max, goal.priority, p)?;
    }
    Ok(())
}

/// Complete `7C0BD0` (DESTROY, `+3C` target GTID; writes `+40` destroy
/// value, `+44` record value, `+48` template share).
pub fn destroy(
    w: &mut impl MorePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    statics: &mut Option<DestroyStatics>,
    p: Precision,
) -> Result<()> {
    if w.goal_expired(goal.id)? {
        goal.priority = 0;
        return Ok(());
    }
    let ego = ctx.ego;
    goal.priority = ego.u32(0x7ac);
    let scale = ego.u32(0x7b0);
    let unit = unit_random(w.sai_random()?, p)?;
    let jitter = num(unit.mul(&n(scale)?, p))?;
    goal.priority = store(num(jitter.add(&n(goal.priority)?, p))?)?;
    let id = goal.word(0x3c)?;
    let mut kind_ok = false;
    for kind in [0x14, 0x16, 0x0a] {
        let t = w.resolve(id)?.ok_or_else(|| null("7C0C94 virtual +34 of a stale target"))?;
        if w.virtual_34(t, kind)? {
            kind_ok = true;
            break;
        }
    }
    if !kind_ok {
        return Ok(());
    }
    if statics.is_none() {
        let kingdom = ctx.actor.ok_or_else(|| null("7C0D12 +1C8 of a null player"))?;
        let [have, reserved, economy] = w.economy_arrays(kingdom)?;
        let mut differences = Vec::with_capacity(have.len());
        for (i, h) in have.iter().enumerate() {
            let r = *reserved
                .get(i)
                .ok_or_else(|| Error::World("economy +1B8 shorter than +1A0".into()))?;
            differences.push(store(num(n(*h)?.sub(&n(r)?, p))?)?);
        }
        *statics = Some(DestroyStatics {
            differences,
            economy,
        });
    }
    let s = statics.as_ref().expect("statics set");
    let t = w.resolve(id)?;
    let value = w.destroy_value(t, &s.differences, &s.economy)?;
    goal.set_word(0x40, store(value)?)?;
    let t = w.resolve(id)?.ok_or_else(|| null("4D03C0 of a stale target"))?;
    let record = w.sai_record(t)?;
    let v = w.destroy_record_value(record)?;
    goal.set_word(0x44, store(v.clone())?)?;
    let sum = num(n(goal.priority)?.add(&n(goal.word(0x40)?)?, p))?;
    goal.priority = store(num(sum.add(&v, p))?)?;
    let (container, record) = match w.resolve(id)? {
        Some(t) => {
            let c = w.virtual_100(t)?;
            let r = match c {
                Some(c) => w.sai_record(c)?,
                None => None,
            };
            (c, r)
        }
        None => (None, None),
    };
    goal.set_word(0x48, 0)?;
    if let Some(record) = record {
        let table = w.record_table_object(record)?;
        let container = container.ok_or_else(|| null("4CEDA0 of a null container"))?;
        let org = organization(w, container)?;
        if let Some(table) = table
            && w.record_byte_28(record)?
        {
            let org = org.ok_or_else(|| null("7C0FD0 +100 of a null organization"))?;
            if w.flags_100(org)? & 0x2000_0000 == 0 {
                let t = w.resolve(id)?.ok_or_else(|| null("7C1007 +4 of a stale target"))?;
                let definition = w.definition(t)?;
                if !w.template_has(table, definition)? {
                    let entry = w
                        .template_missing(table, record)?
                        .ok_or_else(|| null("7C101E +14 of a null template entry"))?;
                    let share = w.entry_word(entry, 0x14)?;
                    goal.set_word(0x48, share)?;
                    goal.priority = add(share, goal.priority, p)?;
                }
            }
        }
    }
    Ok(())
}

/// Complete `7CBC40` (UNPACK, `+3C` SAI actor record; writes `+40` region,
/// `+44/+48` site): the best `4A9AE0` value over the regions near the
/// record's region, random regions of its island and the previous choice.
pub fn unpack(
    w: &mut impl MorePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
) -> Result<()> {
    goal.priority = 0xff7f_ffff;
    if w.goal_expired(goal.id)? {
        return Ok(());
    }
    let record = goal.word(0x3c)?;
    if record == 0 {
        return Err(null("7CBC75 virtual +C of a null record"));
    }
    if w.record_dead(record)? {
        return Ok(());
    }
    let id = w.record_object(record)?;
    let o = w.resolve(id)?.ok_or_else(|| null("7CBC8C +4 of the record's stale object"))?;
    let mut layout = w.definition_534(o)?;
    if layout == 0 {
        let o = w.resolve(id)?.ok_or_else(|| null("7CBCA6 +7C of the record's stale object"))?;
        let Some(component) = w.component_7c(o)? else {
            return Ok(());
        };
        let Some(packed) = w.component_40(component)? else {
            return Ok(());
        };
        if w.flags(packed)? & 1 != 0 {
            return Ok(());
        }
        layout = w.definition_534(packed)?;
        if layout == 0 {
            return Ok(());
        }
    }
    let ego = ctx.ego;
    let region = w.record_region(record)?;
    let island = w.island_regions(region)?;
    let mut list = w.regions_within(region, ego.u32(0x334))?;
    let draws = ego.u32(0x338) as i32;
    for _ in 0..draws.max(0) {
        let r = w.sai_random()?;
        if island.is_empty() {
            return Err(Error::World(
                "island region draw divides by a zero count (native #DE)".into(),
            ));
        }
        // A null island slot is listed (and searched) as region 0, as natively.
        let candidate = island[(r % island.len() as u32) as usize].unwrap_or(0);
        if !list.contains(&candidate) {
            list.insert(0, candidate);
        }
    }
    // 7BA380 pushes the previous choice at the head as well.
    let previous = goal.word(0x40)?;
    if previous != 0 && !list.contains(&previous) {
        list.insert(0, previous);
    }
    for candidate in list {
        let Some(site) = w.unpack_site(layout, ctx.actor, candidate)? else {
            continue;
        };
        let same = candidate == goal.word(0x40)?;
        let v = w.unpack_value(record, candidate, site, same)?;
        if v.compare(&n(goal.priority)?) == Ordering::Greater {
            goal.set_word(0x40, candidate)?;
            goal.set_word(0x44, site[0])?;
            goal.set_word(0x48, site[1])?;
            goal.priority = store(v)?;
        }
    }
    if host(goal.priority) < 0.0
        && goal.word(0x40)? != 0
        && point_on_map([goal.word(0x44)?, goal.word(0x48)?], w.map_extent()?)
    {
        goal.priority = 0x3a83_126f;
    }
    Ok(())
}
