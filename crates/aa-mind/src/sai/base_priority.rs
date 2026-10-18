//! Original SAI goal-class base priorities: goal virtual `+78`, called by the
//! priority finalization `7A5200` (`goal_priority::finalize`) after the
//! affordability test and before the ego `GoalPriority` record is applied.
//!
//! One entry per SAI goal class (vtables from research/phase8-ai
//! inventory.json `sai_goal_classes`); [`GoalClass::base_priority_va`] is
//! the slot `+78` body:
//!
//! | class | vtable | +78 | Rust |
//! |---|---|---|---|
//! | ATTACK_REGION | A9CAC8 | 7C5AE0 | [`attack_region`] |
//! | ATTACK_ACTOR | A9EDD8 | 7C7460 | [`attack_actor`] |
//! | DEFEND_REGION | A9CB58 | 7C5F80 | [`defend_region`] |
//! | DEFEND_ACTOR | A9EE68 | 7C62D0 | [`defend_actor`] |
//! | AIR_ATTACK | A9ECC8 | 7C7E90 | [`air_attack`] |
//! | RECOVER | A9CC70 | 7C6640 | [`recover`] |
//! | REPAIR | A9CBE8 | 7A4230 | [`repair`] |
//! | BOMBARD | A9ED50 | 7C8750 | [`bombard`] |
//!
//! The remaining classes are in `base_priority_misc` (same conventions).
//!
//! Every military body ends with the *general-power bonus* scan shared by
//! these classes: the SAI's active general powers (`[B36528]+80`) and then
//! the player's own (`SAI player +9C`). An instance counts when it belongs to
//! the goal's player (`572040`), matches the goal by a class predicate
//! (`572810` route hops, `572940` region, `5720C0`/`572120` target actor) and
//! has the class's definition kind (`[[gp+14]+8]+C`: 0 attack, 2 defend,
//! 3 friendly support). Its bonus (`571C60`: the instance's `+18`, or the ego
//! default `+890` when that is zero) is either added to `+38` at once
//! (`+3D` additive) or competes for the single largest non-additive bonus,
//! which is added after both lists.
//!
//! Model: goal fields are [`ClassGoal`] (`+38` and the class words from
//! `+3C`); game objects, regions and SAI actor records are opaque handles of
//! the world (native pointers), GTIDs are the raw registry words. Virtual
//! calls and the non-ported callees are [`BasePriorityWorld`] services,
//! called in native order with the native arguments; plain field reads are
//! world queries too (they never call code). x87 arithmetic is exact at the
//! caller's precision control; every `fstp` to binary32 is reproduced.
use super::ego::Ego;
use super::engine::GoalId;
use crate::{
    Error, Result,
    x87::{n, num, store},
};
use aa_numeric::{
    Binary, Precision,
    crt_pow::{CrtEnvironment, ci_pow_x87},
};
use std::cmp::Ordering;

/// A game object, region or SAI actor record (native pointer identity).
pub type Obj = u32;

const ONE: u32 = 0x3f80_0000;

/// The 22 SAI goal classes (goal type `+4` value in brackets of
/// `tables::GOAL_TYPES`; GENERAL_POWER has no goal class).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GoalClass {
    AttackRegion,
    AttackActor,
    DefendRegion,
    DefendActor,
    AirAttack,
    Recover,
    Repair,
    Explore,
    Build,
    Construct,
    Pack,
    Unpack,
    Recruit,
    Upgrade,
    GiveResources,
    GiveActor,
    Destroy,
    AttachHero,
    MoveHero,
    Research,
    Bombard,
    Attach,
}

impl GoalClass {
    pub const ALL: [GoalClass; 22] = [
        GoalClass::AttackRegion,
        GoalClass::AttackActor,
        GoalClass::DefendRegion,
        GoalClass::DefendActor,
        GoalClass::AirAttack,
        GoalClass::Recover,
        GoalClass::Repair,
        GoalClass::Explore,
        GoalClass::Build,
        GoalClass::Construct,
        GoalClass::Pack,
        GoalClass::Unpack,
        GoalClass::Recruit,
        GoalClass::Upgrade,
        GoalClass::GiveResources,
        GoalClass::GiveActor,
        GoalClass::Destroy,
        GoalClass::AttachHero,
        GoalClass::MoveHero,
        GoalClass::Research,
        GoalClass::Bombard,
        GoalClass::Attach,
    ];
    /// The class's `tables::GOAL_TYPES` name (its goal type).
    pub fn name(self) -> &'static str {
        match self {
            GoalClass::AttackRegion => "ATTACK_REGION",
            GoalClass::AttackActor => "ATTACK_ACTOR",
            GoalClass::DefendRegion => "DEFEND_REGION",
            GoalClass::DefendActor => "DEFEND_ACTOR",
            GoalClass::AirAttack => "AIR_ATTACK",
            GoalClass::Recover => "RECOVER",
            GoalClass::Repair => "REPAIR",
            GoalClass::Explore => "EXPLORE",
            GoalClass::Build => "BUILD",
            GoalClass::Construct => "CONSTRUCT",
            GoalClass::Pack => "PACK",
            GoalClass::Unpack => "UNPACK",
            GoalClass::Recruit => "RECRUIT",
            GoalClass::Upgrade => "UPGRADE",
            GoalClass::GiveResources => "GIVE_RESOURCES",
            GoalClass::GiveActor => "GIVE_ACTOR",
            GoalClass::Destroy => "DESTROY",
            GoalClass::AttachHero => "ATTACH_HERO",
            GoalClass::MoveHero => "MOVE_HERO",
            GoalClass::Research => "RESEARCH",
            GoalClass::Bombard => "BOMBARD",
            GoalClass::Attach => "ATTACH",
        }
    }
    /// Native vtable address.
    pub fn vtable(self) -> u32 {
        match self {
            GoalClass::AttackRegion => 0xa9_cac8,
            GoalClass::AttackActor => 0xa9_edd8,
            GoalClass::DefendRegion => 0xa9_cb58,
            GoalClass::DefendActor => 0xa9_ee68,
            GoalClass::AirAttack => 0xa9_ecc8,
            GoalClass::Recover => 0xa9_cc70,
            GoalClass::Repair => 0xa9_cbe8,
            GoalClass::Explore => 0xa9_e950,
            GoalClass::Build => 0xa9_ccf8,
            GoalClass::Construct => 0xa9_cd80,
            GoalClass::Pack => 0xa9_ce90,
            GoalClass::Unpack => 0xa9_cf18,
            GoalClass::Recruit => 0xa9_f3e8,
            GoalClass::Upgrade => 0xa9_ce08,
            GoalClass::GiveResources => 0xa9_e560,
            GoalClass::GiveActor => 0xa9_e5e8,
            GoalClass::Destroy => 0xa9_e670,
            GoalClass::AttachHero => 0xa9_eb10,
            GoalClass::MoveHero => 0xa9_eb98,
            GoalClass::Research => 0xa9_f578,
            GoalClass::Bombard => 0xa9_ed50,
            GoalClass::Attach => 0xa9_cfa0,
        }
    }
    /// The vtable slot `+78` body.
    pub fn base_priority_va(self) -> u32 {
        match self {
            GoalClass::AttackRegion => 0x7c_5ae0,
            GoalClass::AttackActor => 0x7c_7460,
            GoalClass::DefendRegion => 0x7c_5f80,
            GoalClass::DefendActor => 0x7c_62d0,
            GoalClass::AirAttack => 0x7c_7e90,
            GoalClass::Recover => 0x7c_6640,
            GoalClass::Repair => 0x7a_4230,
            GoalClass::Explore => 0x7c_29f0,
            GoalClass::Build => 0x7b_fa30,
            GoalClass::Construct => 0x7c_0120,
            GoalClass::Pack => 0x7c_b640,
            GoalClass::Unpack => 0x7c_bc40,
            GoalClass::Recruit => 0x4b_2830,
            GoalClass::Upgrade => 0x7c_0870,
            GoalClass::GiveResources => 0x7c_10b0,
            GoalClass::GiveActor => 0x7c_1220,
            GoalClass::Destroy => 0x7c_0bd0,
            GoalClass::AttachHero => 0x7c_4d00,
            GoalClass::MoveHero => 0x7c_4d10,
            GoalClass::Research => 0x7d_0680,
            GoalClass::Bombard => 0x7c_8750,
            GoalClass::Attach => 0x7b_e260,
        }
    }
}

/// The goal fields a `+78` body reads and writes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ClassGoal {
    pub id: GoalId,
    /// `+38` base priority (binary32 bits).
    pub priority: u32,
    /// Class words from `+3C` (index 0 is `+3C`, 1 is `+40`, ...).
    pub words: Vec<u32>,
}

impl ClassGoal {
    fn index(offset: u32) -> Result<usize> {
        if offset < 0x3c || offset % 4 != 0 {
            return Err(Error::World(format!(
                "goal class word +{offset:X} is not a class field"
            )));
        }
        Ok(((offset - 0x3c) / 4) as usize)
    }
    /// The class word at native offset `offset` (>= `+3C`).
    pub fn word(&self, offset: u32) -> Result<u32> {
        self.words
            .get(Self::index(offset)?)
            .copied()
            .ok_or_else(|| Error::World(format!("goal class word +{offset:X} is missing")))
    }
    pub fn set_word(&mut self, offset: u32, value: u32) -> Result<()> {
        let i = Self::index(offset)?;
        let slot = self
            .words
            .get_mut(i)
            .ok_or_else(|| Error::World(format!("goal class word +{offset:X} is missing")))?;
        *slot = value;
        Ok(())
    }
}

/// The goal's SAI player (`[[goal+4]+4]`) fields the bodies read.
#[derive(Clone, Copy, Debug)]
pub struct PlayerContext<'a> {
    /// The SAI player object (argument of its valuation services).
    pub player: Obj,
    /// SAI player `+8`: the player's game (kingdom) object.
    pub actor: Option<Obj>,
    /// SAI player `+1C`.
    pub ego: &'a Ego,
    /// CRT state selecting the `_CIpow` (`96D890`) body.
    pub crt: CrtEnvironment,
}

/// One active general-power instance (`[B36528]+80` / player `+9C` node).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeneralPowerInstance {
    /// `+8` GTID of the casting actor.
    pub caster: u32,
    /// `+10` (the argument of the player virtual `+138` when the owner GTID
    /// is stale).
    pub side: u32,
    /// `[[+14]+8]+C`: definition kind (0 attack, 2 defend, 3 support).
    pub kind: u32,
    /// `+18` bonus (binary32; zero selects the ego default `+890`).
    pub bonus: u32,
    /// `+1C` GTID of the target actor.
    pub target: u32,
    /// `+20` byte: the instance targets an actor rather than a point.
    pub targets_actor: bool,
    /// `+24` GTID of the owning player object.
    pub owner: u32,
    /// `+2C/+30` target point (binary32).
    pub point: [u32; 2],
    /// `+34` byte: the point is set.
    pub has_point: bool,
    /// `+3D` byte: the bonus is added directly instead of competing.
    pub additive: bool,
}

/// Services and object fields of the `+78` bodies, in native call order.
/// Methods documented as fields never call native code; all others are
/// virtual calls or callees kept as seams.
pub trait BasePriorityWorld {
    /// `[B364C0]` GTID registry: the object of a matching serial, if any.
    fn resolve(&mut self, gtid: u32) -> Result<Option<Obj>>;
    /// Field: object `+8` flags.
    fn flags(&mut self, obj: Obj) -> Result<u32>;
    /// Field: object `+14` GTID.
    fn object_gtid(&mut self, obj: Obj) -> Result<u32>;
    /// Field: `[B36528]+80` general powers, head first.
    fn sai_general_powers(&mut self) -> Result<Vec<GeneralPowerInstance>>;
    /// Field: SAI player `+9C` general powers, head first.
    fn player_general_powers(&mut self) -> Result<Vec<GeneralPowerInstance>>;
    /// Field: `[B36534]+E0` game time (binary32).
    fn game_time(&mut self) -> Result<u32>;
    /// Field: `[B3652C]` is set and its `+5` byte (the dynamic map exists).
    fn map_ready(&mut self) -> Result<bool>;
    /// Field: object `+20/+24` position (binary32).
    fn position(&mut self, obj: Obj) -> Result<[u32; 2]>;
    /// Field: object `+100` flags.
    fn flags_100(&mut self, obj: Obj) -> Result<u32>;
    /// Field: object `+104` flags.
    fn flags_104(&mut self, obj: Obj) -> Result<u32>;
    /// Field: object `+118` is set (`4CFE90`).
    fn has_118(&mut self, obj: Obj) -> Result<bool>;
    /// Field: object `+7C` component.
    fn component_7c(&mut self, obj: Obj) -> Result<Option<Obj>>;
    /// Field: object `+80` component's `+10` (`None` when either is null).
    fn component_80_10(&mut self, obj: Obj) -> Result<Option<Obj>>;
    /// Field: byte `+2F4` of the object's definition `+4`.
    fn definition_2f4(&mut self, obj: Obj) -> Result<bool>;
    /// Field: component `+5C` and `+60` (binary32).
    fn component_5c_60(&mut self, component: Obj) -> Result<[u32; 2]>;
    /// Field: SAI actor record `+18/+1C` category mask.
    fn record_mask(&mut self, record: Obj) -> Result<u64>;
    /// Field: SAI actor record `+8` GTID of its game object.
    fn record_object(&mut self, record: Obj) -> Result<u32>;
    /// Field: the word at SAI actor record `+30` (`None` when null).
    fn record_route_state(&mut self, record: Obj) -> Result<Option<u32>>;
    /// Field: `[[region +1C][player slot] +2C] +8` for this SAI player.
    fn entry_state(&mut self, region: Obj) -> Result<u32>;
    /// Goal virtual `+14`.
    fn goal_expired(&mut self, goal: GoalId) -> Result<bool>;
    /// Goal virtual `+18(flag)`.
    fn goal_virtual_18(&mut self, goal: GoalId, flag: bool) -> Result<()>;
    /// Goal virtual `+20`: the target region (`None` = null).
    fn goal_location(&mut self, goal: GoalId) -> Result<Option<Obj>>;
    /// Object virtual `+58(other, flag)`.
    fn virtual_58(&mut self, obj: Obj, other: Option<Obj>, flag: bool) -> Result<bool>;
    /// Object virtual `+70(other, flag)`.
    fn virtual_70(&mut self, obj: Obj, other: Option<Obj>, flag: bool) -> Result<bool>;
    /// Object virtual `+100`.
    fn virtual_100(&mut self, obj: Obj) -> Result<Option<Obj>>;
    /// Object virtual `+108`: its owner.
    fn virtual_108(&mut self, obj: Obj) -> Result<Option<Obj>>;
    /// Object virtual `+138(other)`.
    fn virtual_138(&mut self, obj: Obj, other: Option<Obj>) -> Result<bool>;
    /// Object virtual `+140(other)`.
    fn virtual_140(&mut self, obj: Obj, other: Option<Obj>) -> Result<bool>;
    /// SAI actor record virtual `+C` (dead).
    fn record_dead(&mut self, record: Obj) -> Result<bool>;
    /// `79F390(obj)`.
    fn unit_check(&mut self, obj: Obj) -> Result<bool>;
    /// `4EA430(kingdom)` on `obj`: the seen record (`None` = null).
    fn seen_record(&mut self, obj: Option<Obj>, kingdom: Option<Obj>) -> Result<Option<Obj>>;
    /// `7AA3D0(point)` on `[B3652C]`: the region at a point.
    fn region_at(&mut self, point: [u32; 2]) -> Result<Option<Obj>>;
    /// Field: region `+0` index.
    fn region_index(&mut self, region: Obj) -> Result<u32>;
    /// `53F320(&list, start, goal, 1)` on `[[B36534]+234]`, the node count of
    /// a found route and `5C2CD0`: `None` unless the status is 1.
    fn route_hops(&mut self, start: u32, goal: u32) -> Result<Option<i32>>;
    /// `4D0230(flag)` on `obj` (ST0).
    fn health(&mut self, obj: Obj, flag: bool) -> Result<Binary>;
    /// `7AD9E0(target)` on the SAI player (ST0).
    fn attack_value(&mut self, target: Option<Obj>) -> Result<Binary>;
    /// `7AD340(actor, flag)` on the SAI player (ST0).
    fn threat_value(&mut self, actor: Option<Obj>, flag: bool) -> Result<Binary>;
    /// `7AD3E0(actor)` on the SAI player (ST0).
    fn repair_value(&mut self, actor: Option<Obj>) -> Result<Binary>;
    /// `7BD360(&threat, 0)` on the region's entry for this SAI player (ST0;
    /// writes the threatening object).
    fn region_threat(&mut self, region: Obj) -> Result<(Binary, Option<Obj>)>;
}

fn null(what: &str) -> Error {
    Error::World(format!("{what} (native null dereference)"))
}
fn host(bits: u32) -> f32 {
    f32::from_bits(bits)
}
/// `fcomp 0; test ah,41; jnp skip`: continue when greater or unordered.
fn positive_or_unordered(bits: u32) -> bool {
    !(host(bits) <= 0.0)
}
fn add(a: u32, b: u32, p: Precision) -> Result<u32> {
    store(num(n(a)?.add(&n(b)?, p))?)
}
fn mul(a: u32, b: u32, p: Precision) -> Result<u32> {
    store(num(n(a)?.mul(&n(b)?, p))?)
}

/// `572040(player actor)`: the instance belongs to the player.
pub fn owned_by(
    w: &mut impl BasePriorityWorld,
    gp: &GeneralPowerInstance,
    player_actor: Option<Obj>,
) -> Result<bool> {
    match w.resolve(gp.owner)? {
        Some(owner) => Ok(player_actor == Some(owner)),
        None => {
            let actor = player_actor.ok_or_else(|| null("572040 player virtual +138"))?;
            w.virtual_138(actor, Some(gp.side).filter(|s| *s != 0))
        }
    }
}

/// `5720C0(actor)`: the instance's target resolves to `actor`.
pub fn targets(
    w: &mut impl BasePriorityWorld,
    gp: &GeneralPowerInstance,
    actor: Option<Obj>,
) -> Result<bool> {
    Ok(w.resolve(gp.target)?.is_some_and(|t| Some(t) == actor))
}

/// `572120(actor)`: `5720C0(actor)` and `79F390(target)` fails.
pub fn targets_unit(
    w: &mut impl BasePriorityWorld,
    gp: &GeneralPowerInstance,
    actor: Option<Obj>,
) -> Result<bool> {
    if !targets(w, gp, actor)? {
        return Ok(false);
    }
    let target = w
        .resolve(gp.target)?
        .ok_or_else(|| null("572120 79F390 of a stale target"))?;
    Ok(!w.unit_check(target)?)
}

/// `4CF010`: the object's region (null without the dynamic map, before the
/// object is dereferenced).
pub fn object_region(w: &mut impl BasePriorityWorld, obj: Option<Obj>) -> Result<Option<Obj>> {
    if !w.map_ready()? {
        return Ok(None);
    }
    let point = w.position(obj.ok_or_else(|| null("4CF010 position of a null object"))?)?;
    w.region_at(point)
}

/// `572940(region)`: the instance's point or unit target is in `region`
/// (two null regions compare equal, as the pointer compare does).
pub fn in_region(
    w: &mut impl BasePriorityWorld,
    gp: &GeneralPowerInstance,
    region: Option<Obj>,
) -> Result<bool> {
    if !gp.targets_actor {
        if !gp.has_point {
            return Ok(false);
        }
        return Ok(w.region_at(gp.point)? == region);
    }
    let Some(target) = w.resolve(gp.target)? else {
        return Ok(false);
    };
    if !w.unit_check(target)? {
        return Ok(false);
    }
    Ok(object_region(w, Some(target))? == region)
}

/// `572810(region, ego)`: the instance's region is a route of at most
/// ego `+3F4` regions from `region`.
pub fn near_region(
    w: &mut impl BasePriorityWorld,
    gp: &GeneralPowerInstance,
    region: Option<Obj>,
    ego: &Ego,
) -> Result<bool> {
    let found = if gp.has_point {
        w.region_at(gp.point)?
    } else {
        match w.resolve(gp.target)? {
            Some(target) => object_region(w, Some(target))?,
            None => return Ok(false),
        }
    };
    let Some(found) = found else {
        return Ok(false);
    };
    let start = w.region_index(region.ok_or_else(|| null("572810 goal region +0"))?)?;
    let goal = w.region_index(found)?;
    Ok(match w.route_hops(start, goal)? {
        Some(hops) => hops <= ego.u32(0x3f4) as i32,
        None => false,
    })
}

/// `571C60(ego)`: the instance bonus, or the ego default `+890` when it is
/// (ordered) zero.
pub fn bonus(gp: &GeneralPowerInstance, ego: &Ego) -> u32 {
    if host(gp.bonus) == 0.0 {
        ego.u32(0x890)
    } else {
        gp.bonus
    }
}

/// The largest-bonus update of the scans: a zero maximum or one (ordered)
/// below the bonus takes the bonus.
fn takes(max: u32, value: u32) -> bool {
    host(max) == 0.0 || host(max) < host(value)
}

/// Predicates of the general-power scans.
#[derive(Clone, Copy)]
enum Match {
    /// `572810(goal +20, ego)` (the goal virtual is asked per instance).
    NearLocation,
    /// `572940(goal +20)`.
    InLocation,
    /// `5720C0(actor)`.
    Target(Option<Obj>),
    /// `5720C0` of a GTID resolved per instance.
    TargetGtid(u32),
    /// `5720C0` of the SAI actor record's object, resolved per instance.
    TargetRecord(Obj),
    /// `572120` of a GTID resolved per instance.
    TargetUnitGtid(u32),
}

/// The general-power scan of both lists. Returns the largest non-additive
/// bonus (binary32 bits); additive bonuses are folded into `goal.priority`.
/// With `remember_caster` (DEFEND_REGION, SAI list only) a new maximum also
/// sets `+4C` to the caster's GTID while `+4C` no longer resolves.
fn general_power_scan(
    w: &mut impl BasePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    kind: u32,
    rule: Match,
    remember_caster: bool,
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
            if !owned_by(w, &gp, ctx.actor)? {
                continue;
            }
            let matched = match rule {
                Match::NearLocation => {
                    let region = w.goal_location(goal.id)?;
                    near_region(w, &gp, region, ctx.ego)?
                }
                Match::InLocation => {
                    let region = w.goal_location(goal.id)?;
                    in_region(w, &gp, region)?
                }
                Match::Target(actor) => targets(w, &gp, actor)?,
                Match::TargetGtid(gtid) => {
                    let actor = w.resolve(gtid)?;
                    targets(w, &gp, actor)?
                }
                Match::TargetRecord(record) => {
                    let gtid = w.record_object(record)?;
                    let actor = w.resolve(gtid)?;
                    targets(w, &gp, actor)?
                }
                Match::TargetUnitGtid(gtid) => {
                    let actor = w.resolve(gtid)?;
                    targets_unit(w, &gp, actor)?
                }
            };
            if !matched || gp.kind != kind {
                continue;
            }
            let value = bonus(&gp, ctx.ego);
            if gp.additive {
                goal.priority = add(value, goal.priority, p)?;
            } else if takes(max, value) {
                max = value;
                if remember_caster && list == 0 {
                    let threat = goal.word(0x4c)?;
                    if w.resolve(threat)?.is_none()
                        && let Some(caster) = w.resolve(gp.caster)?
                    {
                        let id = w.object_gtid(caster)?;
                        goal.set_word(0x4c, id)?;
                    }
                }
            }
        }
    }
    Ok(max)
}

/// Complete `7C5AE0` (ATTACK_REGION): the region value comes from the class
/// `+60` (`7C6BB0`); a positive (or unordered) `+38` gains the attack bonus of
/// general powers within ego `+3F4` route hops of the region.
pub fn attack_region(
    w: &mut impl BasePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    if !positive_or_unordered(goal.priority) {
        return Ok(());
    }
    let max = general_power_scan(w, goal, ctx, 0, Match::NearLocation, false, p)?;
    goal.priority = add(max, goal.priority, p)?;
    Ok(())
}

/// Complete `7C5F80` (DEFEND_REGION): `+38` = the region threat `7BD360` of
/// this player's entry, `+4C` = the GTID of the threatening object (0 for
/// none), then the defend bonus of general powers in the region.
pub fn defend_region(
    w: &mut impl BasePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    let region = w
        .goal_location(goal.id)?
        .ok_or_else(|| null("7B9050 of a null goal region"))?;
    let (threat, object) = w.region_threat(region)?;
    goal.priority = store(threat)?;
    let id = match object {
        Some(o) => w.object_gtid(o)?,
        None => 0,
    };
    goal.set_word(0x4c, id)?;
    let max = general_power_scan(w, goal, ctx, 2, Match::InLocation, true, p)?;
    goal.priority = add(max, goal.priority, p)?;
    Ok(())
}

/// The `max + +38 <= 0` test of the actor classes on the unstored sum.
fn sum_positive_or_unordered(max: u32, priority: u32, p: Precision) -> Result<bool> {
    let sum = num(n(max)?.add(&n(priority)?, p))?;
    Ok(sum.compare(&Binary::from_i32(0)) == Ordering::Greater)
}

/// Complete `7C7460` (ATTACK_ACTOR, `+4C` target GTID).
pub fn attack_actor(
    w: &mut impl BasePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    goal.priority = 0;
    let target_id = goal.word(0x4c)?;
    let Some(target) = w.resolve(target_id)? else {
        return Ok(());
    };
    if w.flags(target)? & 1 != 0 {
        return Ok(());
    }
    // The scan target: virtual +100, else the +80 component's +10, else
    // the target itself.
    let mut scan = w.virtual_100(target)?;
    if scan.is_none() {
        let again = w
            .resolve(target_id)?
            .ok_or_else(|| null("7C7522 +80 of a stale target"))?;
        scan = match w.component_80_10(again)? {
            Some(o) => Some(o),
            None => w.resolve(target_id)?,
        };
    }
    let max = general_power_scan(w, goal, ctx, 0, Match::Target(scan), false, p)?;
    if !sum_positive_or_unordered(max, goal.priority, p)? {
        return w.goal_virtual_18(goal.id, true);
    }
    let target = w
        .resolve(goal.word(0x4c)?)?
        .ok_or_else(|| null("7C76B7 virtual +58 of a stale target"))?;
    if w.virtual_58(target, ctx.actor, true)? {
        let value = w.attack_value(Some(target))?;
        let sum = num(value.add(&n(max)?, p))?;
        goal.priority = store(num(sum.add(&n(goal.priority)?, p))?)?;
    }
    Ok(())
}

/// Complete `7C62D0` (DEFEND_ACTOR, `+4C` protected actor GTID).
pub fn defend_actor(
    w: &mut impl BasePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    goal.priority = 0;
    let actor_id = goal.word(0x4c)?;
    if w.resolve(actor_id)?.is_none() {
        return Ok(());
    }
    let max = general_power_scan(w, goal, ctx, 2, Match::TargetUnitGtid(actor_id), false, p)?;
    if !sum_positive_or_unordered(max, goal.priority, p)? {
        return w.goal_virtual_18(goal.id, true);
    }
    let actor = w.resolve(goal.word(0x4c)?)?;
    let value = w.threat_value(actor, false)?;
    let sum = num(value.add(&n(max)?, p))?;
    goal.priority = store(num(sum.add(&n(goal.priority)?, p))?)?;
    Ok(())
}

/// `[B364C0]` resolution of a GTID that the native dereferences.
fn live(w: &mut impl BasePriorityWorld, gtid: u32, site: &str) -> Result<Obj> {
    w.resolve(gtid)?.ok_or_else(|| null(site))
}

/// The visibility test of AIR_ATTACK/BOMBARD: target virtual `+70(player,
/// 1)`, else a seen record `4EA430`.
fn target_known(
    w: &mut impl BasePriorityWorld,
    target_id: u32,
    ctx: &PlayerContext<'_>,
) -> Result<bool> {
    let target = live(w, target_id, "virtual +70 of a stale target")?;
    if w.virtual_70(target, ctx.actor, true)? {
        return Ok(true);
    }
    let again = w.resolve(target_id)?;
    Ok(w.seen_record(again, ctx.actor)?.is_some())
}

/// The hostility test of AIR_ATTACK/BOMBARD: player virtual `+140` of the
/// target's owner (`+108`).
fn target_hostile(
    w: &mut impl BasePriorityWorld,
    target_id: u32,
    ctx: &PlayerContext<'_>,
) -> Result<bool> {
    let target = live(w, target_id, "virtual +108 of a stale target")?;
    let player = ctx.actor.ok_or_else(|| null("player virtual +140"))?;
    let owner = w.virtual_108(target)?;
    w.virtual_140(player, owner)
}

/// Complete `7C7E90` (AIR_ATTACK, `+3C` target GTID, `+40` last attack time).
pub fn air_attack(
    w: &mut impl BasePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    goal.priority = 0;
    if w.goal_expired(goal.id)? {
        return Ok(());
    }
    let target_id = goal.word(0x3c)?;
    let target = live(w, target_id, "7C7ED3 +104 of a stale target")?;
    if w.flags_104(target)? & 0x80 != 0 {
        return Ok(());
    }
    let ego = ctx.ego;
    // Re-attack delay ego +450 after +40 (both only when ordered >= 0 / > 0).
    let last = goal.word(0x40)?;
    if !(host(last) < 0.0 || last_unordered(last)) && host(ego.u32(0x450)) > 0.0 {
        let next = num(n(ego.u32(0x450))?.add(&n(last)?, p))?;
        if next.compare(&n(w.game_time()?)?) == Ordering::Greater {
            return Ok(());
        }
    }
    if !target_known(w, target_id, ctx)? {
        return Ok(());
    }
    if !target_hostile(w, target_id, ctx)? {
        return Ok(());
    }
    let target = w.resolve(target_id)?;
    goal.priority = store(w.attack_value(target)?)?;
    let mut scaled = false;
    let t = live(w, target_id, "4CFE90 of a stale target")?;
    if w.has_118(t)? {
        goal.priority = mul(ego.u32(0x420), goal.priority, p)?;
        scaled = true;
    }
    let t = live(w, target_id, "4D0230 of a stale target")?;
    let health = w.health(t, false)?;
    // fcom +424; test ah,1: at least +424 ...
    if health.compare(&n(ego.u32(0x424))?) != Ordering::Less {
        // ... and at most +428 (fcom; test ah,41; jp).
        if health.compare(&n(ego.u32(0x428))?) != Ordering::Greater {
            goal.priority = mul(ego.u32(0x42c), goal.priority, p)?;
            scaled = true;
        }
    }
    if health.compare(&n(ego.u32(0x438))?) != Ordering::Greater {
        let t = live(w, target_id, "4CFE90 of a stale target")?;
        let factor = if w.has_118(t)? {
            ego.u32(0x43c)
        } else {
            ego.u32(0x440)
        };
        scaled = true;
        goal.priority = mul(factor, goal.priority, p)?;
    }
    let t = live(w, target_id, "7C80B5 +100 of a stale target")?;
    if w.flags_100(t)? & 0x8_0000 == 0 {
        let t = live(w, target_id, "7C80CD +7C of a stale target")?;
        if w.component_7c(t)?.is_some() {
            let t = live(w, target_id, "7C80DB +4 of a stale target")?;
            if w.definition_2f4(t)? {
                goal.priority = mul(ego.u32(0x44c), goal.priority, p)?;
            }
        }
    }
    let t = w.resolve(target_id)?;
    let region = object_region(w, t)?.ok_or_else(|| null("7B9050 of a null region"))?;
    if w.entry_state(region)? == 2 {
        goal.priority = mul(ego.u32(0x430), goal.priority, p)?;
    } else if !scaled {
        goal.priority = mul(ego.u32(0x434), goal.priority, p)?;
    }
    if !positive_or_unordered(goal.priority) {
        return Ok(());
    }
    let max = general_power_scan(w, goal, ctx, 0, Match::TargetGtid(target_id), false, p)?;
    goal.priority = add(max, goal.priority, p)?;
    Ok(())
}

fn last_unordered(bits: u32) -> bool {
    host(bits).is_nan()
}

/// Complete `7C8750` (BOMBARD, `+3C` target GTID).
pub fn bombard(
    w: &mut impl BasePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    goal.priority = 0;
    if w.goal_expired(goal.id)? {
        return Ok(());
    }
    let target_id = goal.word(0x3c)?;
    let target = live(w, target_id, "7C8795 +104 of a stale target")?;
    if w.flags_104(target)? & 0x80 != 0 {
        return Ok(());
    }
    if !target_known(w, target_id, ctx)? {
        return Ok(());
    }
    let t = live(w, target_id, "7C8841 +8 of a stale target")?;
    if (w.flags(t)? >> 29) & 1 == 0 {
        let t = live(w, target_id, "4CFE90 of a stale target")?;
        if !w.has_118(t)? {
            return Ok(());
        }
    }
    if !target_hostile(w, target_id, ctx)? {
        return Ok(());
    }
    let t = w.resolve(target_id)?;
    let Some(region) = object_region(w, t)? else {
        return Ok(());
    };
    let target = w.resolve(target_id)?;
    goal.priority = store(w.attack_value(target)?)?;
    if w.entry_state(region)? != 2 {
        let t = live(w, target_id, "4CFE90 of a stale target")?;
        if !w.has_118(t)? {
            goal.priority = mul(ctx.ego.u32(0x454), goal.priority, p)?;
        }
    }
    let max = general_power_scan(w, goal, ctx, 0, Match::TargetGtid(target_id), false, p)?;
    goal.priority = add(max, goal.priority, p)?;
    Ok(())
}

/// `(1 - x^e) * scale` of RECOVER (`_CIpow` at the x87 call site, then
/// `fsubr 1.0`, `fmul`, `fstp`).
fn recovery_term(x: u32, e: u32, scale: u32, ctx: &PlayerContext<'_>, p: Precision) -> Result<u32> {
    let (power, outcome) = ci_pow_x87(&n(x)?, &n(e)?, ctx.crt)
        .map_err(|err| Error::World(format!("CRT pow 96D890: {err}")))?;
    if outcome.libm_error.is_some() {
        return Err(Error::World(format!(
            "CRT pow 96D890 reported libm error {:?}",
            outcome.libm_error
        )));
    }
    let rest = num(n(ONE)?.sub(&power, p))?;
    store(num(rest.mul(&n(scale)?, p))?)
}

/// Complete `7C6640` (RECOVER, `+3C` SAI actor record): the larger of the
/// health and supply recovery needs, plus the support bonus.
pub fn recover(
    w: &mut impl BasePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    goal.priority = 0;
    let record = goal.word(0x3c)?;
    if record == 0 {
        return Err(null("7C6650 virtual +C of a null record"));
    }
    if w.record_dead(record)? {
        return Ok(());
    }
    // 7A1120(4): category bit 4.
    if w.record_mask(record)? & (1 << 4) == 0 {
        return Ok(());
    }
    let object_id = w.record_object(record)?;
    let object = w
        .resolve(object_id)?
        .ok_or_else(|| null("7C667A +8 of the record's stale object"))?;
    let supply = w.component_7c(object)?;
    if (w.flags(object)? >> 29) & 1 != 0 {
        return Ok(());
    }
    let ego = ctx.ego;
    let mut health_need = 0_u32;
    let health = store(w.health(object, true)?)?;
    let (scale, exponent) = (ego.u32(0x6cc), ego.u32(0x6d0));
    if host(health) < host(ego.u32(0x6d4)) || w.record_route_state(record)? == Some(2) {
        health_need = recovery_term(health, exponent, scale, ctx, p)?;
    }
    let mut supply_need = 0_u32;
    if let Some(component) = supply {
        let [have, capacity] = w.component_5c_60(component)?;
        if host(capacity) > 0.0 {
            let fraction = store(num(n(have)?.div(&n(capacity)?, p))?)?;
            let (scale, exponent) = (ego.u32(0x6d8), ego.u32(0x6dc));
            if host(fraction) < host(ego.u32(0x6e0)) || w.record_route_state(record)? == Some(2)
            {
                supply_need = recovery_term(fraction, exponent, scale, ctx, p)?;
            }
        }
    }
    goal.priority = if host(health_need) > host(supply_need) {
        health_need
    } else {
        supply_need
    };
    if !positive_or_unordered(goal.priority) {
        return Ok(());
    }
    let max = general_power_scan(w, goal, ctx, 3, Match::TargetRecord(record), false, p)?;
    goal.priority = add(max, goal.priority, p)?;
    Ok(())
}

/// Complete `7A4230` (REPAIR, `+3C` damaged actor GTID).
pub fn repair(
    w: &mut impl BasePriorityWorld,
    goal: &mut ClassGoal,
    ctx: &PlayerContext<'_>,
    p: Precision,
) -> Result<()> {
    let target_id = goal.word(0x3c)?;
    let Some(target) = w.resolve(target_id)? else {
        goal.priority = 0;
        return Ok(());
    };
    if w.virtual_108(target)?.is_none() {
        goal.priority = 0;
        return Ok(());
    }
    let owner = w
        .virtual_108(target)?
        .ok_or_else(|| null("7A428D virtual +138 of a null owner"))?;
    if !w.virtual_138(owner, ctx.actor)? {
        goal.priority = 0;
        return Ok(());
    }
    let health = w.health(target, false)?;
    // fcomp 1.0; test ah,1; je: only below one (or unordered) continues.
    if health.compare(&n(ONE)?) != Ordering::Less {
        goal.priority = 0;
        return Ok(());
    }
    goal.priority = store(w.repair_value(Some(target))?)?;
    let health = w.health(target, false)?;
    let missing = num(n(ONE)?.sub(&health, p))?;
    goal.priority = store(num(missing.mul(&n(goal.priority)?, p))?)?;
    let max = general_power_scan(w, goal, ctx, 3, Match::TargetGtid(target_id), false, p)?;
    goal.priority = add(max, goal.priority, p)?;
    Ok(())
}
