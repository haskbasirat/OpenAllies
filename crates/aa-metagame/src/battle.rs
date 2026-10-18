//! Quick-battle resolution and loser-army selection of the metagame.
//!
//! Ported instruction-by-instruction from the original AA.exe:
//!
//! | native | role |
//! |---|---|
//! | 93EB70 | sum of division damage (x87, native order, no intermediate store) |
//! | 93EBD0 | army quick-battle strength = 93EB70 + army `damage` |
//! | 954FD0 | defender strength (armies the owner does not regard as enemies) |
//! | 955EB0 | defending kingdom (weights: owner 0.5, +1 per army of a vtable+138 kingdom) |
//! | 95F270 | technology bonus: sum(cost * 0.01f * MetaGameQuickBattleTechnologyBonus) |
//! | 95F2F0 | odds clamp: base - defender + attacker, clamped to `[0, 1]` |
//! | 95F370 | attack odds of a contested territory |
//! | 95F480 | `KKC_QuickBattleSolver::SolveBattle` (one RNG draw) |
//! | 954D80 + 93EC40/93ECA0/93F110/93F080 | weakest/strongest/most/least-expensive army of a side |
//! | 93EA90, 961810, 961840 | army upgrade cost via the owner's shop; shop lookups |
//! | 955420 / 9554F0 | n-th attacker-side / defender-side army |
//! | 9564F0 (selection part) | random army of a side (one RNG draw when the side is non-empty) |
//! | 95C7B0 | destroy behaviour enum table |
//!
//! Proven against `research/phase8-metagame/native-battle.json` (original
//! code under Unicorn at x87 precision control 24/53/64) by
//! `tests/native_battle.rs`. The x87 precision control of the native metagame
//! thread is NOT established; callers pass it explicitly.
//!
//! Territory model: `+0x24` is the owning (controlling) kingdom and `+0x1C`
//! counts the armies whose kingdom the owner regards as an enemy (kingdom actor
//! `vtable+0x140`; 954B30 "enemy territory required" tests exactly this). The
//! attacking side of a battle is therefore "enemies of the owner" and the
//! defending side "everyone else"; 954220 passes the first attacker's kingdom
//! and the owner to `KKC_MGCommand::ResolveBattle`.
//!
//! Seams that remain inputs here (not ported in this module):
//! army actor `vtable+0x108` (owning kingdom actor) is `BattleArmy::owner`;
//! kingdom actor `vtable+0x140` / `+0x138` are [`KingdomRelations::enemy`] /
//! [`KingdomRelations::relation_138`].
use crate::rng::MetaGameRng;
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Identity of a kingdom actor (the object behind `MGKingdom+0x0C`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KingdomActor(pub u32);
/// Identity of a technology data object (shop lookups compare identity).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TechnologyRef(pub u32);
/// Identity of a division data object; also the index into
/// [`BattleContext::division_damage_bits`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct DivisionRef(pub u32);
/// Index into the metagame state's kingdom list (`MGState+0x1C`, native order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct KingdomIndex(pub usize);

/// Kingdom-actor relation virtuals used by the battle code. Their native
/// implementations (kingdom actor class) are not ported in this crate.
pub trait KingdomRelations {
    /// Kingdom actor `vtable+0x140` on `subject` with `other`: `subject`
    /// regards `other` as an enemy (954B30/954C00 enemy/friendly territory tests).
    fn enemy(&self, subject: KingdomActor, other: KingdomActor) -> bool;
    /// Kingdom actor `vtable+0x138` on `subject` with `other` (ally-like; used
    /// by 955EB0 and the attacker-win cleanup 956380). Exact meaning unproven.
    fn relation_138(&self, subject: KingdomActor, other: KingdomActor) -> bool;
}

/// One priced shop row (`entry+4` cost, `entry+8` data identity).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShopEntry<T> {
    pub item: T,
    pub cost: i32,
}

/// Shop lists consulted by the battle code (native shop +18/+1C divisions,
/// +24/+28 technologies). The first entry with a matching identity wins.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleShop {
    pub divisions: Vec<ShopEntry<DivisionRef>>,
    pub technologies: Vec<ShopEntry<TechnologyRef>>,
}
impl BattleShop {
    /// `FUN_00961810`.
    pub fn division(&self, d: DivisionRef) -> Option<&ShopEntry<DivisionRef>> {
        self.divisions.iter().find(|e| e.item == d)
    }
    /// `FUN_00961840`.
    pub fn technology(&self, t: TechnologyRef) -> Option<&ShopEntry<TechnologyRef>> {
        self.technologies.iter().find(|e| e.item == t)
    }
}

/// Metagame kingdom state subset read by the battle code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleKingdom {
    pub actor: KingdomActor,
    /// Owned technologies (`MGKingdom+0x20/+0x24`), native order.
    pub owned_technologies: Vec<TechnologyRef>,
    /// The kingdom data's shop (`MGKingdom+8 -> +0x60`).
    pub shop: BattleShop,
}

/// An army actor standing in the contested territory.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleArmy {
    /// Army actor `vtable+0x108`.
    pub owner: KingdomActor,
    /// MetaGameComponent definition `damage` (definition `+0x5B8 -> +0x14`), binary32 bits.
    pub damage_bits: u32,
    /// Live component division list (`component+0x14/+0x18`), native order.
    pub divisions: Vec<DivisionRef>,
}

/// The contested territory as seen by the battle code.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerritoryBattle {
    /// Territory army list (`+0x10/+0x14`), native order.
    pub armies: Vec<BattleArmy>,
    /// Stored count word `+0x1C` (armies the owner regards as enemies), kept
    /// separately by native code (956280 decrements it on removal).
    pub enemy_army_count: i32,
    /// Owning kingdom (`+0x24`), if any.
    pub owner: Option<KingdomIndex>,
}

/// Which side of a battle a selection applies to (byte argument of 954D80/9564F0).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    /// 0: armies the owner regards as enemies (all armies when unowned).
    Attacker,
    /// 1: armies the owner does not regard as enemies (none when unowned).
    Defender,
}

/// Values referenced by the battle code.
pub struct BattleContext<'a, R: KingdomRelations> {
    pub kingdoms: &'a [BattleKingdom],
    /// Division `damage` (division `+0x34`) binary32 bits, indexed by [`DivisionRef`].
    pub division_damage_bits: &'a [u32],
    pub relations: &'a R,
    /// SVars `MetaGameQuickBattleTechnologyBonus` (settings `+0x208`), binary32 bits.
    pub technology_bonus_bits: u32,
    /// SVars `MetaGameQuickBattleAttackWinProbability` (settings `+0x20C`), binary32 bits.
    pub attack_win_probability_bits: u32,
    /// x87 precision control in effect on the calling thread.
    pub precision: Precision,
}

/// `FUN_0095C7B0` enum table (values are the stored campaign words +0x78/+0x7C).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestroyBehavior {
    None = 0,
    First = 1,
    Weakest = 2,
    Strongest = 3,
    LeastExpensive = 4,
    MostExpensive = 5,
    Random = 6,
    All = 7,
}
impl DestroyBehavior {
    /// Registration order and spelling of 95C7B0.
    pub const TABLE: [(&'static str, DestroyBehavior); 8] = [
        ("none", Self::None),
        ("first", Self::First),
        ("weakest", Self::Weakest),
        ("strongest", Self::Strongest),
        ("least_expensive", Self::LeastExpensive),
        ("most_expensive", Self::MostExpensive),
        ("random", Self::Random),
        ("all", Self::All),
    ];
    pub fn native_value(self) -> u32 {
        self as u32
    }
}

/// Result word of 95F480.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolveResult {
    /// 0: no territory or `enemy_army_count <= 0`; no RNG draw.
    NoBattle,
    /// 1: roll < odds.
    AttackerWins,
    /// 2: otherwise (including unordered compares).
    DefenderWins,
}
impl SolveResult {
    pub fn native_value(self) -> u32 {
        match self {
            Self::NoBattle => 0,
            Self::AttackerWins => 1,
            Self::DefenderWins => 2,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SolveOutcome {
    pub result: SolveResult,
    /// Odds after the binary32 store at 95F49C.
    pub odds_bits: Option<u32>,
    /// Roll after the binary32 store at 95F4CC (the compared value).
    pub roll_bits: Option<u32>,
    /// Roll as logged (binary64 store at 95F4D0 of the unrounded x87 value).
    pub roll_log_f64_bits: Option<u64>,
}

const ZERO_F32: u32 = 0x0000_0000; // 9F32DC
const ONE_F32: u32 = 0x3f80_0000; // 9F32E0
const MINUS_ONE_F32: u32 = 0xbf80_0000; // 9FDC48
const HALF_F32: u32 = 0x3f00_0000; // 955F32 immediate
const PERCENT_F32: u32 = 0x3c23_d70a; // A03078 (0.01f)
const TWO_POW_32_F64: u64 = 0x41f0_0000_0000_0000; // 9FDAA0
const INV_U32_MAX_F64: u64 = 0x3df0_0000_0010_0000; // 9FDA98 (1/(2^32-1))

fn f32b(bits: u32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(bits)?)
}
fn f64b(bits: u64) -> Result<Binary> {
    Ok(Binary::from_f64_bits(bits)?)
}

impl<'a, R: KingdomRelations> BattleContext<'a, R> {
    fn owner_actor(&self, t: &TerritoryBattle) -> Result<Option<KingdomActor>> {
        match t.owner {
            None => Ok(None),
            Some(k) => Ok(Some(self.kingdom(k)?.actor)),
        }
    }
    fn kingdom(&self, k: KingdomIndex) -> Result<&'a BattleKingdom> {
        self.kingdoms
            .get(k.0)
            .ok_or_else(|| Error::Unsupported(format!("kingdom index {} outside state list", k.0)))
    }
    /// `FUN_00938B70`: first state kingdom whose actor is `actor`.
    pub fn kingdom_of_actor(&self, actor: KingdomActor) -> Option<KingdomIndex> {
        self.kingdoms
            .iter()
            .position(|k| k.actor == actor)
            .map(KingdomIndex)
    }
    fn division_damage(&self, d: DivisionRef) -> Result<Binary> {
        let bits = self
            .division_damage_bits
            .get(d.0 as usize)
            .copied()
            .ok_or_else(|| Error::Unsupported(format!("unknown division {}", d.0)))?;
        f32b(bits)
    }

    /// `FUN_0093EB70`: `0.0 + d0 + d1 + ...` at the current precision.
    pub fn division_damage_sum(&self, army: &BattleArmy) -> Result<Binary> {
        let mut sum = f32b(ZERO_F32)?;
        for &d in &army.divisions {
            sum = sum.add(&self.division_damage(d)?, self.precision)?;
        }
        Ok(sum)
    }
    /// `FUN_0093EBD0`: unrounded x87 strength (`93EB70 + damage`).
    pub fn army_strength(&self, army: &BattleArmy) -> Result<Binary> {
        Ok(self
            .division_damage_sum(army)?
            .add(&f32b(army.damage_bits)?, self.precision)?)
    }

    /// `FUN_00954FD0`: binary32 sum of strengths of armies the owner does not
    /// regard as enemies; zero for an unowned territory.
    pub fn defender_strength(&self, t: &TerritoryBattle) -> Result<u32> {
        let mut sum = ZERO_F32;
        if let Some(owner) = self.owner_actor(t)? {
            for army in &t.armies {
                if !self.relations.enemy(owner, army.owner) {
                    sum = self
                        .army_strength(army)?
                        .add(&f32b(sum)?, self.precision)?
                        .store_f32()?;
                }
            }
        }
        Ok(sum)
    }

    /// `FUN_00955EB0`: kingdom with the strictly largest weight (first wins ties);
    /// the owner starts at 0.5, every army whose kingdom passes the owner's
    /// `vtable+0x138` relation adds 1.0 to its own state kingdom.
    pub fn defending_kingdom(&self, t: &TerritoryBattle) -> Result<Option<KingdomIndex>> {
        let Some(owner) = t.owner else {
            return Ok(None);
        };
        if t.armies.len() as i64 == i64::from(t.enemy_army_count) {
            return Ok(None);
        }
        let owner_actor = self.kingdom(owner)?.actor;
        let mut weights = vec![ZERO_F32; self.kingdoms.len()];
        weights[owner.0] = HALF_F32;
        for army in &t.armies {
            if !self.relations.relation_138(owner_actor, army.owner) {
                continue;
            }
            if let Some(k) = self.kingdom_of_actor(army.owner) {
                weights[k.0] = f32b(weights[k.0])?
                    .add(&f32b(ONE_F32)?, self.precision)?
                    .store_f32()?;
            }
        }
        let mut best = f32b(ZERO_F32)?;
        let mut result = None;
        for (i, &w) in weights.iter().enumerate() {
            let w = f32b(w)?;
            if w.compare(&best) == Ordering::Greater {
                best = w;
                result = Some(KingdomIndex(i));
            }
        }
        Ok(result)
    }

    /// `FUN_0095F270`: binary32 technology bonus of a kingdom (0 for none).
    pub fn technology_bonus(&self, kingdom: Option<KingdomIndex>) -> Result<u32> {
        let Some(k) = kingdom else {
            return Ok(ZERO_F32);
        };
        let k = self.kingdom(k)?;
        let bonus = f32b(self.technology_bonus_bits)?;
        let percent = f32b(PERCENT_F32)?;
        let mut sum = ZERO_F32;
        for &tech in &k.owned_technologies {
            if let Some(entry) = k.shop.technology(tech) {
                sum = Binary::from_i32(entry.cost)
                    .mul(&percent, self.precision)?
                    .mul(&bonus, self.precision)?
                    .add(&f32b(sum)?, self.precision)?
                    .store_f32()?;
            }
        }
        Ok(sum)
    }

    /// `FUN_0095F2F0(defender, attacker)`: unrounded x87 odds.
    pub fn odds_clamp(&self, defender_bits: u32, attacker_bits: u32) -> Result<Binary> {
        let defender = f32b(defender_bits)?;
        let attacker = f32b(attacker_bits)?;
        if attacker.is_zero() {
            return f32b(ZERO_F32);
        }
        if defender.is_zero() {
            return f32b(ONE_F32);
        }
        let value = f32b(self.attack_win_probability_bits)?
            .sub(&defender, self.precision)?
            .add(&attacker, self.precision)?;
        if value.compare(&f32b(ZERO_F32)?) == Ordering::Less {
            return f32b(ZERO_F32);
        }
        if value.compare(&f32b(ONE_F32)?) == Ordering::Greater {
            return f32b(ONE_F32);
        }
        Ok(value)
    }

    /// `FUN_0095F370`: unrounded x87 attack odds (-1 without attackers, 1 without defenders).
    pub fn attack_odds(&self, t: &TerritoryBattle) -> Result<Binary> {
        if t.enemy_army_count <= 0 {
            return f32b(MINUS_ONE_F32);
        }
        if t.armies.len() as i64 == i64::from(t.enemy_army_count) {
            return f32b(ONE_F32);
        }
        let defender = self.defender_strength(t)?;
        let defending = self.defending_kingdom(t)?;
        let defender_total = f32b(self.technology_bonus(defending)?)?
            .add(&f32b(defender)?, self.precision)?
            .store_f32()?;
        let owner_actor = self.owner_actor(t)?;
        let mut attack = ZERO_F32;
        let mut last = None;
        for army in &t.armies {
            // 955320: enemies of the owner; every army when unowned.
            if let Some(o) = owner_actor
                && !self.relations.enemy(o, army.owner)
            {
                continue;
            }
            attack = self
                .army_strength(army)?
                .add(&f32b(attack)?, self.precision)?
                .store_f32()?;
            last = self.kingdom_of_actor(army.owner);
        }
        let attacker_total = f32b(self.technology_bonus(last)?)?
            .add(&f32b(attack)?, self.precision)?
            .store_f32()?;
        self.odds_clamp(defender_total, attacker_total)
    }

    /// `FUN_0095F480` (`KKC_QuickBattleSolver::SolveBattle`).
    pub fn solve_battle(&self, t: &TerritoryBattle, rng: &mut MetaGameRng) -> Result<SolveOutcome> {
        if t.enemy_army_count <= 0 {
            return Ok(SolveOutcome {
                result: SolveResult::NoBattle,
                odds_bits: None,
                roll_bits: None,
                roll_log_f64_bits: None,
            });
        }
        let odds_bits = self.attack_odds(t)?.store_f32()?;
        let draw = rng.next_u32() as i32;
        let mut roll = Binary::from_i32(draw);
        if draw < 0 {
            roll = roll.add(&f64b(TWO_POW_32_F64)?, self.precision)?;
        }
        roll = roll.mul(&f64b(INV_U32_MAX_F64)?, self.precision)?;
        let roll_bits = roll.store_f32()?;
        let roll_log = store_f64(&roll)?;
        let result = if f32b(roll_bits)?.compare(&f32b(odds_bits)?) == Ordering::Less {
            SolveResult::AttackerWins
        } else {
            SolveResult::DefenderWins
        };
        Ok(SolveOutcome {
            result,
            odds_bits: Some(odds_bits),
            roll_bits: Some(roll_bits),
            roll_log_f64_bits: Some(roll_log),
        })
    }

    fn on_side(&self, owner: Option<KingdomActor>, army: &BattleArmy, side: Side) -> bool {
        match (side, owner) {
            (Side::Attacker, None) => true,
            (Side::Attacker, Some(o)) => self.relations.enemy(o, army.owner),
            (Side::Defender, None) => false,
            (Side::Defender, Some(o)) => !self.relations.enemy(o, army.owner),
        }
    }

    /// `FUN_00955420` (attacker side) / `FUN_009554F0` (defender side).
    pub fn nth_army(&self, t: &TerritoryBattle, side: Side, n: u32) -> Result<Option<usize>> {
        let owner = self.owner_actor(t)?;
        let mut counter = 0u32;
        for (i, army) in t.armies.iter().enumerate() {
            if self.on_side(owner, army, side) {
                if counter == n {
                    return Ok(Some(i));
                }
                counter = counter.wrapping_add(1);
            }
        }
        Ok(None)
    }

    /// `FUN_0093EA90` with the owner's shop: summed upgrade cost of the army's
    /// divisions (0 when the owner has no state kingdom).
    pub fn army_cost(&self, army: &BattleArmy) -> Result<i32> {
        let Some(k) = self.kingdom_of_actor(army.owner) else {
            return Ok(0);
        };
        let shop = &self.kingdom(k)?.shop;
        let mut sum = 0i32;
        for &d in &army.divisions {
            if let Some(e) = shop.division(d) {
                sum = sum.wrapping_add(e.cost);
            }
        }
        Ok(sum)
    }

    /// `FUN_00954D80` with one of the native comparators. Returns the army index.
    pub fn select_army(
        &self,
        t: &TerritoryBattle,
        side: Side,
        rule: SelectionRule,
    ) -> Result<Option<usize>> {
        let owner = self.owner_actor(t)?;
        let mut best: Option<usize> = None;
        for (i, army) in t.armies.iter().enumerate() {
            if !self.on_side(owner, army, side) {
                continue;
            }
            best = Some(match best {
                None => i,
                Some(b) => {
                    if self.keep_first(&t.armies[b], army, rule)? {
                        b
                    } else {
                        i
                    }
                }
            });
        }
        Ok(best)
    }

    /// Comparator result: `true` keeps the earlier army `a`, `false` takes `b`.
    fn keep_first(&self, a: &BattleArmy, b: &BattleArmy, rule: SelectionRule) -> Result<bool> {
        Ok(match rule {
            // 93EC40: a only if strength(b) > strength(a).
            SelectionRule::Weakest => {
                self.army_strength(b)?.compare(&self.army_strength(a)?) == Ordering::Greater
            }
            // 93ECA0: a only if strength(b) < strength(a).
            SelectionRule::Strongest => {
                self.army_strength(b)?.compare(&self.army_strength(a)?) == Ordering::Less
            }
            // 93F080: a only if cost(a) < cost(b).
            SelectionRule::LeastExpensive => self.army_cost(a)? < self.army_cost(b)?,
            // 93F110: a only if cost(a) > cost(b).
            SelectionRule::MostExpensive => self.army_cost(a)? > self.army_cost(b)?,
        })
    }

    /// Selection part of `FUN_009564F0`: draws once when the side count word is
    /// positive (`enemy_army_count`, or `len - enemy_army_count`), then
    /// picks the `draw % count`-th army of that side.
    pub fn random_army(
        &self,
        t: &TerritoryBattle,
        side: Side,
        rng: &mut MetaGameRng,
    ) -> Result<Option<usize>> {
        let count = match side {
            Side::Attacker => t.enemy_army_count,
            Side::Defender => (t.armies.len() as i32).wrapping_sub(t.enemy_army_count),
        };
        if count < 1 {
            return Ok(None);
        }
        let draw = rng.next_u32();
        self.nth_army(t, side, draw % count as u32)
    }
}

/// Comparator behind 954EB0/954ED0/954EF0/954F10.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionRule {
    Weakest,
    Strongest,
    MostExpensive,
    LeastExpensive,
}

/// Exact binary64 store of a finite value (round to nearest even).
fn store_f64(value: &Binary) -> Result<u64> {
    // Re-round to 53 significant bits, then decompose through binary32-safe
    // pieces: values here are in [0, 1], far from binary64 range limits.
    let rounded = value.add(&f32b(ZERO_F32)?, Precision::Double53)?;
    if rounded.is_zero() {
        return Ok(0);
    }
    // Find the binary64 encoding by bisection on the exponent and significand
    // using only exact comparisons.
    let negative = rounded.is_negative_nonzero();
    let magnitude = if negative { rounded.clone().negated() } else { rounded };
    let mut exponent: i32 = 0;
    let pow2 = |e: i32| -> Result<Binary> { f64b(((e + 1023) as u64) << 52) };
    while magnitude.compare(&pow2(exponent)?) == Ordering::Less {
        exponent -= 1;
        if exponent < -1022 {
            return Err(Error::Unsupported("binary64 subnormal log value".into()));
        }
    }
    while magnitude.compare(&pow2(exponent + 1)?) != Ordering::Less {
        exponent += 1;
        if exponent > 1023 {
            return Err(Error::Unsupported("binary64 overflow log value".into()));
        }
    }
    let mut fraction: u64 = 0;
    let base = ((exponent + 1023) as u64) << 52;
    for bit in (0..52).rev() {
        let candidate = base | fraction | (1u64 << bit);
        if f64b(candidate)?.compare(&magnitude) != Ordering::Greater {
            fraction |= 1u64 << bit;
        }
    }
    Ok(u64::from(negative) << 63 | base | fraction)
}
