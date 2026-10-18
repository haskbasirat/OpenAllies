//! Battle resolution of the metagame (`KKC_MGBattleResolver`,
//! `KKC_MGCommand::ResolveBattle`, quick battles) and the deferred waypoint
//! mover, ported from the original AA.exe.
//!
//! | native | role |
//! |---|---|
//! | 966FF0 | resolver ctor: `+4` territory, `+8` attacker won |
//! | 967530 | `Process`: attackers `+0x10 += 1` (93E8F0), battles counter `+0x3C`, win/lose |
//! | 967390 | `ProcessAttackerWin`: winner = MG kingdom (938B70) of the first attacker (955420) |
//! | 967290 | attacker win: liberation (95BEB0, 938BC0, kingdom vtable+0x138), capture 954F30, immobile cleanup 956380, defender destroy 9670D0, retreat 9671A0, owner 955620 |
//! | 9673F0 | `ProcessDefenderWin`: attacker destroy 9670D0, retreat orders to the previous path territory (93EB50, 93F550) |
//! | 9670D0 | destroy behaviour (966F40 first, 954EB0/954ED0/954F10/954EF0 selections, 9564F0 random, 'all' loop) |
//! | 9671A0 / 967040 | retreat to the strongest linked friendly territory (954BE0, 954FD0) or destroy every defender |
//! | 954F30 | capture: capturable armies (`MetaGameComponent data +0x1C`) take the new owner (army vtable+0x160), recount 954CF0 |
//! | 956380 | immobile cleanup: defenders (filter 9553A0) with movement 0, killed in reverse list order |
//! | 956350 | kill (4D7960, engine) + 956280 remove |
//! | 954130 | `ResolveBattle`: messages 953A20, Process, 938E10, 938980(1), result 95EFD0 appended (939410) |
//! | 938E10 | game-over check (95A6B0 per kingdom; kingdom actor `+0x19C`, 4E9720, 50D370 are engine) |
//! | 954220 | `ProcessQuickBattle`: pending battle 93A320, `SolveBattle` 95F480, ResolveBattle |
//! | 93F550 | order: clear the waypoint list, push one territory |
//! | 940020 | deferred per-tick mover: pop next waypoint, 956280 / 956140 / 93F9F0 (as 940100) |
//!
//! Proven by `tests/native_resolve.rs` against
//! `research/phase8-metagame/native-resolve.json` (original code under
//! Unicorn on perturbed InitCreate worlds of all shipped campaigns, PC53 and
//! subsets at PC24/PC64).
//!
//! Model: a killed army (4D7960 + 956280) becomes `None` in
//! [`MgState::armies`]; a captured army only changes [`crate::state::MgArmy::owner`]
//! (`crate::state::MgArmy`). Engine seams are reported through [`Engine`].
//! Every native null dereference reachable from these entry points is an
//! explicit [`Error::Unsupported`]; state may then be partially updated.
use crate::battle::{
    BattleArmy, BattleContext, BattleKingdom, BattleShop, DestroyBehavior, DivisionRef,
    KingdomActor, KingdomIndex, KingdomRelations, SelectionRule, ShopEntry, Side, SolveResult,
    TechnologyRef, TerritoryBattle,
};
use crate::commands::{CommandWorld, Validator};
use crate::data::{EffectKind, MetaGameData};
use crate::state::{ArmyId, BattleRecord, MgState};
use crate::{Error, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Engine calls made by the ported code at its seams, in native order.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum EngineCall {
    /// `FUN_00953A20(attacker, defender, territory, won)`: battle messages.
    BattleMessages {
        attacker: usize,
        defender: Option<usize>,
        territory: usize,
        attacker_won: bool,
    },
    /// Army actor `vtable+0x160` (954F30 capture).
    SetArmyOwner { army: ArmyId, owner: KingdomActor },
    /// `FUN_004D7960` actor kill (956350, 956380, 9564F0).
    KillArmy { army: ArmyId },
    /// `FUN_004E9720(1, 1)` on a winning kingdom actor (938E10).
    KingdomVictory { actor: KingdomActor },
    /// `FUN_0050D370` on world `+0x130` after `MGState+0x50` is set (938E10).
    GameOver,
    /// `FUN_00962880(actor, current, next, 0)`: move visual (940020).
    MoveVisual {
        army: ArmyId,
        from: Option<usize>,
        to: usize,
    },
    /// 952790 block 9527CB..95288D: localized end-turn message of a kingdom.
    EndTurnMessage { kingdom: usize },
    /// `FUN_004FA030` army actor creation by 955B70 (event effect add_actor),
    /// followed by 940100 into the territory.
    CreateArmy {
        army: ArmyId,
        definition: usize,
        owner: KingdomActor,
    },
    /// 95BC80 block 95BD23..95BDF4: event `message` (non-empty) formatted with
    /// the current kingdom's name and posted (category 0x1F).
    EventMessage { event: usize, kingdom: usize },
    /// Effect message block (non-empty effect `message`), formatted with the
    /// objects listed: add_actor 95B2CE (army, kingdom, territory),
    /// remove_actor 95B4E8 (army, kingdom, territory; before the kill),
    /// add_tech 95B6CF / remove_tech 95B88F (kingdom, technology),
    /// change_territory 95BB3C (target kingdom, territory, source kingdom).
    EventEffectMessage {
        event: usize,
        effect: usize,
        kind: EffectKind,
        kingdom: usize,
        army: Option<ArmyId>,
        territory: Option<usize>,
        technology: Option<String>,
        source: Option<usize>,
    },
}

/// Engine-side inputs and the seam call sink.
pub trait Engine {
    /// Kingdom actor field `+0x19C` (938E10 skips a winner whose field is 1).
    fn kingdom_field_19c(&self, actor: KingdomActor) -> u32;
    /// Army actor `+0x70` object `vtable+0x64`: true makes 940020 skip the tick.
    fn army_busy(&self, army: ArmyId) -> bool;
    /// A seam call, in native order.
    fn call(&mut self, call: EngineCall);
    /// Kingdom actor `+0x284` engine list, restricted to actors with a
    /// MetaGame component (938DA0, 95A3C0, 95A410), in list order.
    /// Default (ASSUMPTION, matches the oracles): the live armies the actor
    /// owns, in [`ArmyId`] order.
    fn owned_armies(&self, state: &MgState, actor: KingdomActor) -> Vec<ArmyId> {
        state
            .armies
            .iter()
            .enumerate()
            .filter_map(|(i, a)| a.as_ref().filter(|a| a.owner == actor).map(|_| ArmyId(i as u32)))
            .collect()
    }
}

/// Everything the resolution and turn code reads or writes.
pub struct MgContext<'a, R: KingdomRelations, E: Engine> {
    pub state: &'a mut MgState,
    pub data: &'a MetaGameData,
    /// Kingdom actor `vtable+0x140` / `vtable+0x138`.
    pub relations: &'a R,
    pub engine: &'a mut E,
    /// x87 precision control of the calling thread (default [`crate::GAME_PRECISION`]).
    pub precision: Precision,
}

/// `pending_battle` (93A320) needs no world query; this never answers one.
struct NoCommandWorld;
impl CommandWorld for NoCommandWorld {
    fn controllable(&self, _: ArmyId, _: KingdomActor) -> bool {
        false
    }
    fn reach(&self, _: &MgState, _: ArmyId, _: usize) -> Result<bool> {
        Ok(false)
    }
}

fn unsupported<T>(what: &str) -> Result<T> {
    Err(Error::Unsupported(what.to_string()))
}

impl<R: KingdomRelations, E: Engine> MgContext<'_, R, E> {
    // ------------------------------------------------------------ helpers

    /// Native code dereferences every kingdom actor handle it meets; a kingdom
    /// without actor is outside the proven domain.
    pub(crate) fn require_actors(&self) -> Result<()> {
        if self.state.kingdoms.iter().any(|k| k.actor.is_none()) {
            return unsupported("MG kingdom without kingdom actor (native null dereference)");
        }
        Ok(())
    }
    pub(crate) fn actor(&self, k: usize) -> Result<KingdomActor> {
        self.state.kingdoms[k]
            .actor
            .ok_or_else(|| Error::Unsupported("MG kingdom without kingdom actor".into()))
    }
    /// `FUN_00938BC0`: first MG kingdom whose campaign kingdom IDS equals
    /// that of campaign kingdom `kd` (wcsicmp, C locale).
    pub fn kingdom_of_data(&self, kd: usize) -> Option<usize> {
        let c = &self.data.campaigns[self.state.campaign];
        let ids = &c.kingdoms[kd].ids;
        self.state
            .kingdoms
            .iter()
            .position(|k| c.kingdoms[k.data].ids.eq_ignore_ascii_case(ids))
    }
    /// `FUN_00938B70`: first MG kingdom whose actor is `actor`.
    pub fn kingdom_of_actor(&self, actor: KingdomActor) -> Option<usize> {
        self.state.kingdoms.iter().position(|k| k.actor == Some(actor))
    }
    fn owner_of(&self, army: ArmyId) -> KingdomActor {
        self.state.army(army).owner
    }
    /// Filter 955320 / 955420 (attacker side): unowned, or the owner regards
    /// the army kingdom as an enemy (vtable+0x140).
    fn attacker_side(&self, t: usize, army: ArmyId) -> bool {
        match self.state.owner_actor(t) {
            None => true,
            Some(o) => self.relations.enemy(o, self.owner_of(army)),
        }
    }
    /// 9554F0 (defender side): owned and the owner does not regard the army
    /// kingdom as an enemy.
    fn defender_side(&self, t: usize, army: ArmyId) -> bool {
        match self.state.owner_actor(t) {
            None => false,
            Some(o) => !self.relations.enemy(o, self.owner_of(army)),
        }
    }
    /// Filter 9553A0 (defender iterator 955E80, cleanup 956380): owned and the
    /// owner's vtable+0x138 holds for the army kingdom.
    fn defender_138(&self, t: usize, army: ArmyId) -> bool {
        match self.state.owner_actor(t) {
            None => false,
            Some(o) => self.relations.relation_138(o, self.owner_of(army)),
        }
    }
    /// `FUN_00955420` / `FUN_009554F0`: n-th army of a side.
    pub(crate) fn nth(&self, t: usize, side: Side, n: u32) -> Option<ArmyId> {
        let mut counter = 0u32;
        for r in &self.state.territories[t].armies {
            let on = match side {
                Side::Attacker => self.attacker_side(t, r.army),
                Side::Defender => self.defender_side(t, r.army),
            };
            if on {
                if counter == n {
                    return Some(r.army);
                }
                counter = counter.wrapping_add(1);
            }
        }
        None
    }
    /// Territory `+0x14 - +0x1C` (defender count word).
    fn defender_count(&self, t: usize) -> i32 {
        let tt = &self.state.territories[t];
        (tt.armies.len() as i32).wrapping_sub(tt.enemy_army_count)
    }
    fn side_count(&self, t: usize, side: Side) -> i32 {
        match side {
            Side::Attacker => self.state.territories[t].enemy_army_count,
            Side::Defender => self.defender_count(t),
        }
    }
    /// Battle view of the MG kingdoms. Technology data objects are shared
    /// per IDS (identity compare in 961840), so one [`TechnologyRef`] is
    /// assigned per distinct IDS; a shop row without technology (native NULL
    /// data) and a NULL owned entry (`""`) share the NULL identity.
    fn battle_kingdoms(&self) -> Result<Vec<BattleKingdom>> {
        let c = &self.data.campaigns[self.state.campaign];
        let mut registry: Vec<String> = Vec::new();
        let mut tech = |ids: &str| -> TechnologyRef {
            let i = match registry.iter().position(|x| x == ids) {
                Some(i) => i,
                None => {
                    registry.push(ids.to_string());
                    registry.len() - 1
                }
            };
            TechnologyRef(i as u32)
        };
        let mut out = Vec::with_capacity(self.state.kingdoms.len());
        for (i, k) in self.state.kingdoms.iter().enumerate() {
            let shop = &c.kingdoms[k.data].shop;
            out.push(BattleKingdom {
                actor: self.actor(i)?,
                owned_technologies: k.technologies.iter().map(|t| tech(t)).collect(),
                shop: BattleShop {
                    divisions: shop
                        .divisions
                        .iter()
                        .filter_map(|r| {
                            let ids = r.ids.as_ref()?;
                            let d = self.data.divisions.iter().position(|x| &x.ids == ids)?;
                            Some(ShopEntry { item: DivisionRef(d as u32), cost: r.cost })
                        })
                        .collect(),
                    technologies: shop
                        .technologies
                        .iter()
                        .map(|r| ShopEntry { item: tech(r.ids.as_deref().unwrap_or("")), cost: r.cost })
                        .collect(),
                },
            });
        }
        Ok(out)
    }
    fn territory_battle(&self, t: usize) -> TerritoryBattle {
        let tt = &self.state.territories[t];
        TerritoryBattle {
            armies: tt
                .armies
                .iter()
                .map(|r| {
                    let a = self.state.army(r.army);
                    BattleArmy {
                        owner: a.owner,
                        damage_bits: self.data.armies[a.definition].damage_bits,
                        divisions: a.divisions.clone(),
                    }
                })
                .collect(),
            enemy_army_count: tt.enemy_army_count,
            owner: tt.owner.map(KingdomIndex),
        }
    }
    fn damage_bits(&self) -> Vec<u32> {
        self.data.divisions.iter().map(|d| d.damage_bits).collect()
    }
    fn validator(&self) -> Validator<'_, R, NoCommandWorld> {
        Validator {
            state: &*self.state,
            data: self.data,
            relations: self.relations,
            world: &NoCommandWorld,
            precision: self.precision,
        }
    }
    /// `FUN_00954FD0` on an MG territory (binary32 bits).
    pub fn defender_strength(&self, t: usize) -> Result<u32> {
        self.validator().defender_strength(t)
    }
    /// `FUN_0093A320`: pending battle territory.
    pub fn pending_battle(&self) -> Result<Option<usize>> {
        self.validator().pending_battle()
    }
    /// `FUN_00939B60`: MG territory of a map territory.
    pub(crate) fn territory_of(&self, definition: usize) -> Option<usize> {
        self.state.territories.iter().position(|t| t.definition == definition)
    }

    // ------------------------------------------------------------ kills and selections

    /// `FUN_00956350`: kill (4D7960, engine) then remove (956280); false for no army.
    pub(crate) fn kill(&mut self, t: usize, army: Option<ArmyId>) -> Result<bool> {
        let Some(id) = army else { return Ok(false) };
        self.engine.call(EngineCall::KillArmy { army: id });
        let removed = self.state.remove_from_territory(t, id, self.relations);
        self.state.armies[id.0 as usize] = None;
        Ok(removed)
    }
    fn select(&self, t: usize, side: Side, rule: SelectionRule) -> Result<Option<ArmyId>> {
        let kingdoms = self.battle_kingdoms()?;
        let damage = self.damage_bits();
        let ctx = BattleContext {
            kingdoms: &kingdoms,
            division_damage_bits: &damage,
            relations: self.relations,
            technology_bonus_bits: self.data.settings.technology_bonus_bits,
            attack_win_probability_bits: self.data.settings.attack_win_probability_bits,
            precision: self.precision,
        };
        Ok(ctx
            .select_army(&self.territory_battle(t), side, rule)?
            .map(|i| self.state.territories[t].armies[i].army))
    }
    /// `FUN_009564F0`: random army of a side (one draw when the side count
    /// word is positive), killed and removed.
    fn kill_random(&mut self, t: usize, side: Side) -> Result<bool> {
        let kingdoms = self.battle_kingdoms()?;
        let damage = self.damage_bits();
        let tb = self.territory_battle(t);
        let ctx = BattleContext {
            kingdoms: &kingdoms,
            division_damage_bits: &damage,
            relations: self.relations,
            technology_bonus_bits: self.data.settings.technology_bonus_bits,
            attack_win_probability_bits: self.data.settings.attack_win_probability_bits,
            precision: self.precision,
        };
        let pick = ctx.random_army(&tb, side, &mut self.state.rng)?;
        let victim = pick.map(|i| self.state.territories[t].armies[i].army);
        self.kill(t, victim)
    }

    /// `FUN_009670D0(behaviour, skip, side)`: behaviours 1..6 destroy one army
    /// when `skip == 0`; `all` (7) destroys every army of the side regardless
    /// of `skip`. Returns the native count (1 for a single destroy).
    pub fn destroy(&mut self, t: usize, behaviour: DestroyBehavior, skip: i32, side: Side) -> Result<i32> {
        self.require_actors()?;
        let mut destroyed = 0i32;
        if skip == 0 {
            let killed = match behaviour {
                // 966F40
                DestroyBehavior::First => {
                    let v = self.nth(t, side, 0);
                    self.kill(t, v)?
                }
                // 966F70 -> 954EB0 (93EC40)
                DestroyBehavior::Weakest => {
                    let v = self.select(t, side, SelectionRule::Weakest)?;
                    self.kill(t, v)?
                }
                // 966F90 -> 954ED0 (93ECA0)
                DestroyBehavior::Strongest => {
                    let v = self.select(t, side, SelectionRule::Strongest)?;
                    self.kill(t, v)?
                }
                // 966FB0 -> 954F10 (93F080)
                DestroyBehavior::LeastExpensive => {
                    let v = self.select(t, side, SelectionRule::LeastExpensive)?;
                    self.kill(t, v)?
                }
                // 966FD0 -> 954EF0 (93F110)
                DestroyBehavior::MostExpensive => {
                    let v = self.select(t, side, SelectionRule::MostExpensive)?;
                    self.kill(t, v)?
                }
                // 9564F0
                DestroyBehavior::Random => self.kill_random(t, side)?,
                DestroyBehavior::None | DestroyBehavior::All => false,
            };
            if killed {
                destroyed = 1;
            }
        }
        if behaviour == DestroyBehavior::All {
            while self.side_count(t, side) > 0 {
                let v = self.nth(t, side, 0);
                if v.is_none() {
                    return unsupported("9670D0 'all' with a positive side count and no army of the side (native endless loop)");
                }
                if self.kill(t, v)? {
                    destroyed = destroyed.wrapping_add(1);
                }
            }
        }
        Ok(destroyed)
    }

    // ------------------------------------------------------------ attacker win

    /// `FUN_00954F30`: every capturable army in the territory takes the new
    /// owner (army vtable+0x160); then 954CF0 recounts against the current
    /// (old) territory owner. Returns the number of captured armies.
    pub fn capture_armies(&mut self, t: usize, new_owner: usize) -> Result<i32> {
        let actor = self.actor(new_owner)?;
        let mut count = 0i32;
        let mut i = 0;
        while i < self.state.territories[t].armies.len() {
            let id = self.state.territories[t].armies[i].army;
            let def = self.state.army(id).definition;
            if self.data.armies[def].capturable {
                self.engine.call(EngineCall::SetArmyOwner { army: id, owner: actor });
                self.state.armies[id.0 as usize].as_mut().expect("live army").owner = actor;
                count = count.wrapping_add(1);
            }
            i += 1;
        }
        self.state.recount_enemies(t, self.relations);
        Ok(count)
    }

    /// `FUN_00956380`: defenders (filter 9553A0) whose army type has movement
    /// 0 are killed and removed in reverse list order; returns the number
    /// removed.
    pub fn remove_immobile_defenders(&mut self, t: usize) -> Result<i32> {
        let victims: Vec<ArmyId> = self.state.territories[t]
            .armies
            .iter()
            .map(|r| r.army)
            .filter(|&id| self.defender_138(t, id))
            .filter(|&id| self.data.armies[self.state.army(id).definition].movement == 0)
            .collect();
        let mut count = 0i32;
        for &id in victims.iter().rev() {
            if self.kill(t, Some(id))? {
                count = count.wrapping_add(1);
            }
        }
        Ok(count)
    }

    /// `FUN_00967040`: linked territory (map links of the battle territory,
    /// link order) owned by a kingdom that does not regard the battle
    /// territory's owner as an enemy (954BE0), with the strictly greatest
    /// defender strength (954FD0, binary32); the first candidate is taken
    /// unconditionally.
    pub fn retreat_target(&self, t: usize) -> Result<Option<usize>> {
        let map = self.data.campaigns[self.state.campaign]
            .map
            .ok_or_else(|| Error::Unsupported("campaign without map".into()))?;
        let old_owner = self.state.territories[t].owner;
        let def = self.state.territories[t].definition;
        let mut best: Option<(usize, u32)> = None;
        for link in &self.data.maps[map].territories[def].links {
            let Some(cand) = self.territory_of(link.target) else {
                return unsupported("967040: link target without MG territory (native null dereference)");
            };
            // 954BE0 = !954B70(old owner)
            let friendly = match old_owner {
                None => false,
                Some(k) => match self.state.owner_actor(cand) {
                    None => false,
                    Some(o) => !self.relations.enemy(o, self.actor(k)?),
                },
            };
            if !friendly {
                continue;
            }
            let strength = self.defender_strength(cand)?;
            let take = match best {
                None => true,
                Some((_, b)) => {
                    Binary::from_f32_bits(strength)?.compare(&Binary::from_f32_bits(b)?) == Ordering::Greater
                }
            };
            if take {
                best = Some((cand, strength));
            }
        }
        Ok(best.map(|(c, _)| c))
    }

    /// `FUN_009671A0`: retreat every defender (iterator 955E80, filter 9553A0)
    /// to the retreat target with 93F550, or, without a target, destroy
    /// defenders (9554F0 first, 956350) while the defender count is positive.
    pub fn retreat_or_destroy(&mut self, t: usize) -> Result<bool> {
        let mut ok = true;
        match self.retreat_target(t)? {
            None => {
                while self.defender_count(t) > 0 {
                    let v = self.nth(t, Side::Defender, 0);
                    if v.is_none() {
                        return unsupported("9671A0 with a positive defender count and no defender (native endless loop)");
                    }
                    let killed = self.kill(t, v)?;
                    ok &= killed;
                }
            }
            Some(target) => {
                let list: Vec<ArmyId> = self.state.territories[t]
                    .armies
                    .iter()
                    .map(|r| r.army)
                    .filter(|&id| self.defender_138(t, id))
                    .collect();
                for id in list {
                    // 93F550 always returns 1; once false the call is skipped.
                    ok = if ok { self.set_order(id, target) } else { false };
                }
            }
        }
        Ok(ok)
    }

    /// `FUN_00967290(winner, destroy)`.
    pub fn attacker_win(&mut self, t: usize, winner: Option<usize>, destroy: bool) -> Result<bool> {
        self.require_actors()?;
        let Some(winner) = winner else { return Ok(false) };
        let mut new_owner = winner;
        // 95BEB0: first campaign territory record (kingdom order, record
        // order) of this map territory; its liberates_to kingdom data.
        let c = &self.data.campaigns[self.state.campaign];
        let def = self.state.territories[t].definition;
        let liberation = c
            .kingdoms
            .iter()
            .flat_map(|k| k.territories.iter())
            .find(|r| r.territory == def)
            .and_then(|r| r.liberates_to);
        if let Some(kd) = liberation {
            let Some(l) = self.kingdom_of_data(kd) else {
                return unsupported("967290: liberation kingdom without MG kingdom (native null dereference)");
            };
            if self.relations.relation_138(self.actor(l)?, self.actor(winner)?) {
                new_owner = l;
            }
        }
        self.capture_armies(t, new_owner)?;
        let removed = self.remove_immobile_defenders(t)?;
        if destroy {
            let behaviour = self.data.campaigns[self.state.campaign].defending_destroy_behavior;
            self.destroy(t, behaviour, removed, Side::Defender)?;
        }
        let result = self.retreat_or_destroy(t)?;
        self.state.set_owner(t, new_owner, self.relations);
        Ok(result)
    }

    /// `FUN_00967390`.
    pub fn process_attacker_win(&mut self, t: usize) -> Result<bool> {
        let Some(a0) = self.nth(t, Side::Attacker, 0) else { return Ok(false) };
        let winner = self.kingdom_of_actor(self.owner_of(a0));
        self.attacker_win(t, winner, true)
    }

    /// `FUN_0093EB50`: the territory before the current one in the path
    /// (`None` for a path of at most one entry).
    pub fn previous_territory(&self, army: ArmyId) -> Option<usize> {
        let path = &self.state.army(army).path;
        (path.len() > 1).then(|| path[path.len() - 2])
    }

    /// `FUN_009673F0`.
    pub fn process_defender_win(&mut self, t: usize) -> Result<bool> {
        self.require_actors()?;
        let behaviour = self.data.campaigns[self.state.campaign].attacking_destroy_behavior;
        self.destroy(t, behaviour, 0, Side::Attacker)?;
        let attackers: Vec<ArmyId> = self.state.territories[t]
            .armies
            .iter()
            .map(|r| r.army)
            .filter(|&id| self.attacker_side(t, id))
            .collect();
        let mut targets = Vec::with_capacity(attackers.len());
        for &id in &attackers {
            match self.previous_territory(id) {
                Some(p) => targets.push(p),
                None => {
                    return unsupported(
                        "9673F0: attacker without previous path territory gets a null order (93EB50 = 0; MgArmy cannot hold it)",
                    );
                }
            }
        }
        let mut ok = true;
        for (id, target) in attackers.into_iter().zip(targets) {
            ok = if ok { self.set_order(id, target) } else { false };
        }
        Ok(ok)
    }

    /// `FUN_00967530` (`KKC_MGBattleResolver::Process`).
    pub fn process(&mut self, t: usize, attacker_won: bool) -> Result<bool> {
        self.require_actors()?;
        // 955E50 iterator, 93E8F0
        let attackers: Vec<ArmyId> = self.state.territories[t]
            .armies
            .iter()
            .map(|r| r.army)
            .filter(|&id| self.attacker_side(t, id))
            .collect();
        for id in attackers {
            let a = self.state.armies[id.0 as usize].as_mut().expect("live army");
            a.fought = a.fought.wrapping_add(1);
        }
        let Some(a0) = self.nth(t, Side::Attacker, 0) else { return Ok(false) };
        if self.defender_count(t) > 0 {
            let Some(k) = self.kingdom_of_actor(self.owner_of(a0)) else {
                return unsupported("967530: first attacker without MG kingdom (native null dereference)");
            };
            let kk = &mut self.state.kingdoms[k];
            kk.battles = kk.battles.wrapping_add(1);
        }
        if attacker_won {
            self.process_attacker_win(t)
        } else {
            self.process_defender_win(t)
        }
    }

    // ------------------------------------------------------------ commands

    /// `FUN_00938E10`: game-over check.
    pub fn check_game_over(&mut self) -> Result<()> {
        if self.state.game_over {
            return Ok(());
        }
        let mut any = false;
        for k in 0..self.state.kingdoms.len() {
            if self.state.has_won(self.data, k, self.relations)? {
                let actor = self.actor(k)?;
                if self.engine.kingdom_field_19c(actor) != 1 {
                    any = true;
                    self.engine.call(EngineCall::KingdomVictory { actor });
                }
            }
        }
        if any {
            self.state.game_over = true;
            self.engine.call(EngineCall::GameOver);
        }
        Ok(())
    }

    /// 953A20 (engine messages); it dereferences the attacker kingdom.
    pub(crate) fn battle_messages(&mut self, attacker: Option<usize>, defender: Option<usize>, t: usize, won: bool) -> Result<()> {
        let Some(attacker) = attacker else {
            return unsupported("953A20 with no attacker kingdom (native null dereference)");
        };
        self.engine.call(EngineCall::BattleMessages { attacker, defender, territory: t, attacker_won: won });
        Ok(())
    }

    /// `FUN_00954130` (`KKC_MGCommand::ResolveBattle`).
    pub fn resolve_battle(&mut self, attacker: Option<usize>, defender: Option<usize>, t: usize, attacker_won: bool) -> Result<bool> {
        self.require_actors()?;
        self.battle_messages(attacker, defender, t, attacker_won)?;
        let result = self.process(t, attacker_won)?;
        self.check_game_over()?;
        self.state.battle_resolved = true;
        self.state.results.push(BattleRecord { territory: t, attacker, defender, attacker_won });
        Ok(result)
    }

    /// `FUN_00954220` (`KKC_MGCommand::ProcessQuickBattle`).
    pub fn process_quick_battle(&mut self) -> Result<bool> {
        self.require_actors()?;
        let Some(t) = self.pending_battle()? else {
            return unsupported("954220 without a pending battle (native null dereference)");
        };
        let Some(a0) = self.nth(t, Side::Attacker, 0) else {
            return unsupported("954220: pending battle without attacker (native null dereference)");
        };
        let attacker = self.kingdom_of_actor(self.owner_of(a0));
        let defender = self.state.territories[t].owner;
        let kingdoms = self.battle_kingdoms()?;
        let damage = self.damage_bits();
        let tb = self.territory_battle(t);
        let ctx = BattleContext {
            kingdoms: &kingdoms,
            division_damage_bits: &damage,
            relations: self.relations,
            technology_bonus_bits: self.data.settings.technology_bonus_bits,
            attack_win_probability_bits: self.data.settings.attack_win_probability_bits,
            precision: self.precision,
        };
        let outcome = ctx.solve_battle(&tb, &mut self.state.rng)?;
        let won = outcome.result == SolveResult::AttackerWins;
        self.resolve_battle(attacker, defender, t, won)
    }

    /// `FUN_0093F550`: replace the waypoint list by one territory; returns 1.
    pub fn set_order(&mut self, army: ArmyId, territory: usize) -> bool {
        let a = self.state.armies[army.0 as usize].as_mut().expect("live army");
        a.waypoints.clear();
        a.waypoints.push(territory);
        true
    }

    /// `FUN_00940020` (component `+8` interface): unless the actor is busy,
    /// move to the next waypoint (962880 visual, pop, 956280 / 956140 /
    /// 93F9F0 exactly as 940100).
    pub fn step_orders(&mut self, army: ArmyId) -> Result<()> {
        self.require_actors()?;
        if self.engine.army_busy(army) {
            return Ok(());
        }
        let a = self.state.army(army);
        let Some(&next) = a.waypoints.first() else { return Ok(()) };
        let from = a.territory;
        self.engine.call(EngineCall::MoveVisual { army, from, to: next });
        self.state.armies[army.0 as usize].as_mut().expect("live army").waypoints.remove(0);
        self.state.enter_territory(army, next, self.relations)?;
        Ok(())
    }
}
