//! Metagame commands (`KKC_MGCommand`, 0x28 bytes, vtable 0xACAF2C) and their
//! validation `FUN_00951D70` with the per-command validators.
//!
//! Proven against `research/phase8-metagame/native-commands.json` by
//! `tests/native_commands.rs` (original code under Unicorn on perturbed
//! InitCreate worlds of every shipped campaign). The path search used by move
//! and attack (93FDC0) comes through [`CommandWorld::reach`] /
//! [`ProcessWorld::path`]; [`NativePaths`] provides the ported `crate::path`.
//! Seam: army actor vtable+0x134 (code 5) is an input (`controllable`).
use crate::battle::{
    BattleArmy, BattleContext, BattleKingdom, BattleShop, DivisionRef, KingdomActor,
    KingdomRelations, Side, ShopEntry, TerritoryBattle,
};
use crate::data::{MetaGameData, TerritoryKind};
use crate::state::{ArmyId, MgState};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};

/// Built-in command data (93BAB0 -> 949AA0): index, flags (+0x24), hot (+0x2C).
pub const BUILTIN_FLAGS: [u32; 12] = [
    0x0100_0840, // 0 move
    0x0100_0820, // 1 attack
    0,           // 2 startbattle
    0,           // 3 playbattle
    0,           // 4 cancelbattle
    0,           // 5 quickbattle
    0x0100_0001, // 6 endturn
    0x0101_0000, // 7 purchasetech
    0x0100_1000, // 8 purchaseactor
    0x0110_0800, // 9 upgradeactor
    0x0100_0800, // 10 upgradeactorall
    0x0000_0020, // 11 take (cheat)
];
pub const BUILTIN_HOT: [bool; 12] = [
    true, true, false, false, false, false, false, false, false, false, false, false,
];

/// Validation result codes of 951D70 (message keys by 952EC0 where it has one).
pub mod code {
    pub const OK: u32 = 0;
    pub const INVALID: u32 = 2;
    pub const BAD_INDEX: u32 = 3;
    pub const NOT_YOUR_TURN: u32 = 4;
    /// metagame_error_army_not_owned
    pub const ARMY_NOT_OWNED: u32 = 5;
    /// metagame_error_army_cannot_move
    pub const ARMY_CANNOT_MOVE: u32 = 6;
    /// metagame_error_army_cannot_move_to
    pub const ARMY_CANNOT_MOVE_TO: u32 = 7;
    /// metagame_error_enemy_territory_required
    pub const ENEMY_TERRITORY_REQUIRED: u32 = 8;
    /// metagame_error_friendly_territory_required
    pub const FRIENDLY_TERRITORY_REQUIRED: u32 = 9;
    pub const ARMY_ALREADY_FOUGHT: u32 = 0xA;
    /// metagame_error_existing_pending_battle
    pub const EXISTING_PENDING_BATTLE: u32 = 0xB;
    /// metagame_error_no_attacks_left
    pub const NO_ATTACKS_LEFT: u32 = 0xC;
    /// metagame_error_no_attacks_left_must_pullout
    pub const NO_ATTACKS_LEFT_MUST_PULLOUT: u32 = 0xD;
    pub const NO_PENDING_BATTLE: u32 = 0xE;
    pub const NO_BATTLE_STARTED: u32 = 0xF;
    pub const PENDING_BATTLE_BLOCKS_END_TURN: u32 = 0x10;
    pub const TECHNOLOGY_NOT_IN_SHOP: u32 = 0x11;
    pub const ACTOR_NOT_IN_SHOP: u32 = 0x12;
    pub const DIVISION_NOT_IN_SHOP: u32 = 0x13;
    /// metagame_error_insufficient_resources
    pub const INSUFFICIENT_RESOURCES: u32 = 0x14;
    /// metagame_error_purchase_unowned_capital
    pub const PURCHASE_UNOWNED_CAPITAL: u32 = 0x15;
}

/// Command data object (`+4`, bit 3): registry index and flag word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandData {
    pub index: u32,
    pub flags: u32,
}
impl CommandData {
    pub fn builtin(index: u32) -> Self {
        Self {
            index,
            flags: BUILTIN_FLAGS.get(index as usize).copied().unwrap_or(0),
        }
    }
}

/// `KKC_MGCommand` fields; `None` = presence bit clear.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    /// +0x04 (bit 3)
    pub data: Option<CommandData>,
    /// +0x0C army handle (bit 0)
    pub army: Option<ArmyId>,
    /// +0x10 actor (army type) definition, index into `MetaGameData::armies` (bit 1)
    pub actor_definition: Option<usize>,
    /// +0x14 map territory, alias already replaced by its partner (bit 2)
    pub territory: Option<usize>,
    /// +0x18 technology data (bit 4), by IDS
    pub technology: Option<String>,
    /// +0x1C division list (bit 5), indices into `MetaGameData::divisions`
    pub divisions: Option<Vec<usize>>,
}
impl Command {
    /// `FUN_00950CF0`: an alias territory is stored as its partner.
    pub fn set_territory(&mut self, data: &MetaGameData, map: usize, definition: usize) {
        let t = &data.maps[map].territories[definition];
        self.territory = Some(match (t.kind, t.alias_partner) {
            (TerritoryKind::Alias, Some(p)) => p,
            _ => definition,
        });
    }
    /// Presence flag word (`+8`): bit0 army, bit1 actor, bit2 territory,
    /// bit3 data, bit4 technology, bit5 divisions.
    pub fn presence(&self) -> u32 {
        u32::from(self.army.is_some())
            | u32::from(self.actor_definition.is_some()) << 1
            | u32::from(self.territory.is_some()) << 2
            | u32::from(self.data.is_some()) << 3
            | u32::from(self.technology.is_some()) << 4
            | u32::from(self.divisions.is_some()) << 5
    }
}

/// Engine-side queries the validation needs.
pub trait CommandWorld {
    /// Army actor vtable+0x134 called with the issuer (code 5 when false).
    fn controllable(&self, army: ArmyId, issuer: KingdomActor) -> bool;
    /// `FUN_0093FDC0` path search from the army to an MG territory succeeds.
    fn reach(&self, state: &MgState, army: ArmyId, territory: usize) -> crate::Result<bool>;
}

/// The ported path search ([`crate::path::army_path`], 93FDC0) as the
/// command world; army vtable+0x134 (`controllable`) stays an engine input.
pub struct NativePaths<'a, R: KingdomRelations, C: Fn(ArmyId, KingdomActor) -> bool> {
    pub data: &'a MetaGameData,
    pub relations: &'a R,
    pub precision: Precision,
    pub controllable: C,
}
impl<R: KingdomRelations, C: Fn(ArmyId, KingdomActor) -> bool> CommandWorld for NativePaths<'_, R, C> {
    fn controllable(&self, army: ArmyId, issuer: KingdomActor) -> bool {
        (self.controllable)(army, issuer)
    }
    fn reach(&self, state: &MgState, army: ArmyId, territory: usize) -> crate::Result<bool> {
        Ok(crate::path::army_path(state, self.data, self.relations, army, territory, self.precision)?.is_some())
    }
}
impl<R: KingdomRelations, C: Fn(ArmyId, KingdomActor) -> bool> ProcessWorld for NativePaths<'_, R, C> {
    fn path(&self, state: &MgState, army: ArmyId, territory: usize) -> crate::Result<Option<Vec<usize>>> {
        crate::path::army_path(state, self.data, self.relations, army, territory, self.precision)
    }
}

/// Read-only view used by the validators.
pub struct Validator<'a, R: KingdomRelations, W: CommandWorld> {
    pub state: &'a MgState,
    pub data: &'a MetaGameData,
    pub relations: &'a R,
    pub world: &'a W,
    pub precision: Precision,
}

impl<R: KingdomRelations, W: CommandWorld> Validator<'_, R, W> {
    fn map(&self) -> usize {
        self.data.campaigns[self.state.campaign].map.expect("campaign map")
    }
    /// `FUN_00939B60`: MG territory of a map territory (hash by IDS).
    pub fn territory_of(&self, definition: usize) -> Option<usize> {
        self.state.territories.iter().position(|t| t.definition == definition)
    }
    fn owner_actor(&self, t: usize) -> Option<KingdomActor> {
        self.state.owner_actor(t)
    }
    /// `FUN_00954B30`: unowned, or the owner regards `k` as an enemy.
    pub fn hostile(&self, t: usize, k: KingdomActor) -> bool {
        match self.owner_actor(t) {
            None => true,
            Some(o) => self.relations.enemy(o, k),
        }
    }
    /// `FUN_00954C00`: owned and the owner does not regard `k` as an enemy.
    pub fn friendly(&self, t: usize, k: KingdomActor) -> bool {
        match self.owner_actor(t) {
            None => false,
            Some(o) => !self.relations.enemy(o, k),
        }
    }

    fn battle_kingdoms(&self) -> Vec<BattleKingdom> {
        let c = &self.data.campaigns[self.state.campaign];
        self.state
            .kingdoms
            .iter()
            .map(|k| BattleKingdom {
                actor: k.actor.unwrap_or(KingdomActor(u32::MAX)),
                owned_technologies: Vec::new(), // not read by 954FD0/955420
                shop: BattleShop {
                    divisions: c.kingdoms[k.data]
                        .shop
                        .divisions
                        .iter()
                        .filter_map(|r| {
                            let ids = r.ids.as_ref()?;
                            let d = self.data.divisions.iter().position(|x| &x.ids == ids)?;
                            Some(ShopEntry { item: DivisionRef(d as u32), cost: r.cost })
                        })
                        .collect(),
                    technologies: Vec::new(),
                },
            })
            .collect()
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
            owner: tt.owner.map(crate::battle::KingdomIndex),
        }
    }
    /// `FUN_00954FD0` on an MG territory, binary32 bits.
    pub fn defender_strength(&self, t: usize) -> crate::Result<u32> {
        let kingdoms = self.battle_kingdoms();
        let damage: Vec<u32> = self.data.divisions.iter().map(|d| d.damage_bits).collect();
        let ctx = BattleContext {
            kingdoms: &kingdoms,
            division_damage_bits: &damage,
            relations: self.relations,
            technology_bonus_bits: 0,
            attack_win_probability_bits: 0,
            precision: self.precision,
        };
        ctx.defender_strength(&self.territory_battle(t))
    }
    fn first_attacker(&self, t: usize) -> crate::Result<Option<ArmyId>> {
        let kingdoms = self.battle_kingdoms();
        let damage: Vec<u32> = self.data.divisions.iter().map(|d| d.damage_bits).collect();
        let ctx = BattleContext {
            kingdoms: &kingdoms,
            division_damage_bits: &damage,
            relations: self.relations,
            technology_bonus_bits: 0,
            attack_win_probability_bits: 0,
            precision: self.precision,
        };
        Ok(ctx
            .nth_army(&self.territory_battle(t), Side::Attacker, 0)?
            .map(|i| self.state.territories[t].armies[i].army))
    }
    /// `FUN_0093A320`: first territory (state order) with attackers, defenders
    /// and positive defender strength; an alias yields its partner.
    pub fn pending_battle(&self) -> crate::Result<Option<usize>> {
        for (i, t) in self.state.territories.iter().enumerate() {
            let defenders = t.armies.len() as i32 - t.enemy_army_count;
            if t.enemy_army_count > 0 && defenders > 0 && f32_positive(self.defender_strength(i)?) {
                let def = &self.data.maps[self.map()].territories[t.definition];
                if def.kind == TerritoryKind::Alias {
                    return Ok(def.alias_partner.and_then(|p| self.territory_of(p)));
                }
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
    fn kingdom_of(&self, actor: KingdomActor) -> Option<usize> {
        self.state.kingdoms.iter().position(|k| k.actor == Some(actor))
    }
    fn current_actor(&self) -> Option<KingdomActor> {
        self.state.current.and_then(|k| self.state.kingdoms[k].actor)
    }
    fn live_army(&self, a: Option<ArmyId>) -> Option<ArmyId> {
        a.filter(|id| self.state.armies.get(id.0 as usize).is_some_and(|x| x.is_some()))
    }
    fn shop(&self, k: usize) -> &crate::data::Shop {
        &self.data.campaigns[self.state.campaign].kingdoms[self.state.kingdoms[k].data].shop
    }
    fn division_cost(&self, k: usize, d: usize) -> Option<i32> {
        let ids = &self.data.divisions[d].ids;
        self.shop(k)
            .divisions
            .iter()
            .find(|r| r.ids.as_ref() == Some(ids))
            .map(|r| r.cost)
    }
    /// `FUN_0093F7A0`: divisions an upgrade-all adds, in shop order, repeated
    /// passes until nothing is added (prerequisites against the growing set).
    pub fn upgrade_all_list(&self, k: usize, army: ArmyId) -> Vec<usize> {
        let mut have: Vec<usize> = self.state.army(army).divisions.iter().map(|d| d.0 as usize).collect();
        let mut out = Vec::new();
        loop {
            let mut added = false;
            for r in &self.shop(k).divisions {
                let Some(ids) = &r.ids else { continue };
                let Some(d) = self.data.divisions.iter().position(|x| &x.ids == ids) else { continue };
                if have.contains(&d) {
                    continue;
                }
                if self.data.divisions[d].prerequisites.iter().all(|p| have.contains(p)) {
                    have.push(d);
                    out.push(d);
                    added = true;
                }
            }
            if !added {
                return out;
            }
        }
    }

    /// `FUN_00951D70` for `issuer`.
    pub fn validate(&self, cmd: &Command, issuer: KingdomActor) -> crate::Result<u32> {
        use code::*;
        // 950D30
        let Some(data) = cmd.data else { return Ok(INVALID) };
        let f = data.flags;
        if (f & 0xF000 != 0 && cmd.actor_definition.is_none())
            || (f & 0x0F00 != 0 && cmd.army.is_none())
            || (f & 0xF_0000 != 0 && cmd.technology.is_none())
            || (f & 0xF0 != 0 && cmd.territory.is_none())
        {
            return Ok(INVALID);
        }
        let target = cmd.territory.and_then(|d| self.territory_of(d));
        if f & 0x20 != 0 && !target.is_some_and(|t| self.hostile(t, issuer)) {
            return Ok(ENEMY_TERRITORY_REQUIRED);
        }
        if f & 0x40 != 0 && !target.is_some_and(|t| self.friendly(t, issuer)) {
            return Ok(FRIENDLY_TERRITORY_REQUIRED);
        }
        if f & 0x600 != 0 {
            return Err(crate::Error::Unsupported("army vtable+0x140/+0x138 command flags".into()));
        }
        let army = self.live_army(cmd.army);
        if f & 0x800 != 0 && !army.is_some_and(|a| self.state.army(a).owner == issuer) {
            return Ok(INVALID);
        }
        let not_current = self.current_actor() != Some(issuer);
        let k = self.kingdom_of(issuer);
        match data.index {
            0 | 1 => {
                if not_current {
                    return Ok(NOT_YOUR_TURN);
                }
                let Some(a) = army else { return Ok(INVALID) };
                if !self.world.controllable(a, issuer) {
                    return Ok(ARMY_NOT_OWNED);
                }
                let mg = self.state.army(a);
                if mg.fought > 0 {
                    return Ok(ARMY_ALREADY_FOUGHT);
                }
                if self.data.armies[mg.definition].movement == 0 {
                    return Ok(ARMY_CANNOT_MOVE);
                }
                let Some(t) = target else { return Ok(ARMY_CANNOT_MOVE_TO) };
                if !self.world.reach(self.state, a, t)? {
                    return Ok(ARMY_CANNOT_MOVE_TO);
                }
                if data.index == 0 {
                    return Ok(OK);
                }
                // 951480 attack
                if !f32_positive(self.defender_strength(t)?) {
                    return Ok(OK);
                }
                if let Some(pb) = self.pending_battle()? {
                    let pb_def = self.state.territories[pb].definition;
                    if Some(pb_def) != cmd.territory
                        && (self.state.territories[pb].enemy_army_count != 1
                            || self.first_attacker(pb)? != Some(a))
                    {
                        return Ok(EXISTING_PENDING_BATTLE);
                    }
                }
                if k.is_some_and(|k| self.state.kingdoms[k].battles == 1) {
                    return Ok(NO_ATTACKS_LEFT);
                }
                Ok(OK)
            }
            2 => {
                if self.pending_battle()?.is_none() {
                    return Ok(NO_PENDING_BATTLE);
                }
                if self.state.current.is_some_and(|c| self.state.kingdoms[c].battles == 1) {
                    return Ok(NO_ATTACKS_LEFT_MUST_PULLOUT);
                }
                Ok(OK)
            }
            3 | 4 => Ok(if self.state.battle_started { OK } else { NO_BATTLE_STARTED }),
            5 => {
                if !self.state.battle_started {
                    return Ok(NO_BATTLE_STARTED);
                }
                if self.pending_battle()?.is_none() {
                    return Ok(NO_PENDING_BATTLE);
                }
                if self.state.current.is_some_and(|c| self.state.kingdoms[c].battles == 1) {
                    return Ok(NO_ATTACKS_LEFT_MUST_PULLOUT);
                }
                Ok(OK)
            }
            6 => {
                if not_current {
                    return Ok(NOT_YOUR_TURN);
                }
                Ok(if self.pending_battle()?.is_some() { PENDING_BATTLE_BLOCKS_END_TURN } else { OK })
            }
            7 => {
                if not_current {
                    return Ok(NOT_YOUR_TURN);
                }
                let k = k.expect("current kingdom");
                let tech = cmd.technology.as_deref().unwrap_or_default();
                let Some(row) = self.shop(k).technologies.iter().find(|r| r.ids.as_deref() == Some(tech)) else {
                    return Ok(TECHNOLOGY_NOT_IN_SHOP);
                };
                Ok(if self.state.kingdoms[k].resources < row.cost { INSUFFICIENT_RESOURCES } else { OK })
            }
            8 => {
                if not_current {
                    return Ok(NOT_YOUR_TURN);
                }
                let k = k.expect("current kingdom");
                let ids = cmd.actor_definition.map(|d| self.data.armies[d].ids.as_str());
                let Some(row) = self.shop(k).actors.iter().find(|r| r.ids.as_deref() == ids) else {
                    return Ok(ACTOR_NOT_IN_SHOP);
                };
                if self.state.kingdoms[k].resources < row.cost {
                    return Ok(INSUFFICIENT_RESOURCES);
                }
                Ok(if self.state.capital_lost(k, self.relations)? { PURCHASE_UNOWNED_CAPITAL } else { OK })
            }
            9 | 10 => {
                if not_current {
                    return Ok(NOT_YOUR_TURN);
                }
                let Some(a) = army else { return Ok(INVALID) };
                let mg = self.state.army(a);
                if !self.data.armies[mg.definition].upgradable {
                    return Ok(INVALID);
                }
                let k = k.expect("current kingdom");
                let resources = self.state.kingdoms[k].resources;
                if data.index == 10 {
                    let cost: i32 = self
                        .upgrade_all_list(k, a)
                        .iter()
                        .map(|&d| self.division_cost(k, d).unwrap_or(0))
                        .fold(0i32, |s, c| s.wrapping_add(c));
                    return Ok(if resources < cost { INSUFFICIENT_RESOURCES } else { OK });
                }
                let mut sum = 0i32;
                for &d in cmd.divisions.as_deref().unwrap_or_default() {
                    let Some(cost) = self.division_cost(k, d) else {
                        return Ok(DIVISION_NOT_IN_SHOP);
                    };
                    sum = sum.wrapping_add(cost);
                    if mg.divisions.contains(&DivisionRef(d as u32)) {
                        return Ok(INVALID);
                    }
                    if !self.data.divisions[d]
                        .prerequisites
                        .iter()
                        .all(|p| mg.divisions.contains(&DivisionRef(*p as u32)))
                    {
                        return Ok(INVALID);
                    }
                }
                Ok(if resources < sum { INSUFFICIENT_RESOURCES } else { OK })
            }
            // 950EF0: cheat gate 40FCA0 (cheats disabled in this port).
            11 => Ok(INVALID),
            _ => Ok(BAD_INDEX),
        }
    }
}

fn f32_positive(bits: u32) -> bool {
    // binary32 compare with 0.0f: positive and non-NaN (fcomp, test ah,0x41)
    bits & 0x8000_0000 == 0 && bits != 0 && (bits & 0x7f80_0000 != 0x7f80_0000 || bits & 0x7f_ffff == 0)
}

/// Message categories posted to the engine message channel (40BE60).
pub mod category {
    pub const EVENT: u32 = 0x1F;
    pub const BATTLE: u32 = 0x20;
    pub const PURCHASE: u32 = 0x21;
    pub const MOVEMENT: u32 = 0x22;
}

/// Engine side of command processing.
pub trait ProcessWorld {
    /// `FUN_0093FDC0` path for an army to an MG territory (start excluded).
    fn path(&self, state: &MgState, army: ArmyId, territory: usize) -> crate::Result<Option<Vec<usize>>>;
}

/// Effects of one `KKC_MGCommand::Process` (954570) call besides the state.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessOutcome {
    /// Returned byte.
    pub result: bool,
    /// Message categories posted, in order (texts are presentation).
    pub messages: Vec<u32>,
    /// Army created by a purchase (actor creation 4FA030 is an engine seam).
    pub created: Option<ArmyId>,
    /// Army type changed by 951AA0 (native respawns the actor 955DD0).
    pub respawned: Option<ArmyId>,
}

/// Mutating counterpart of [`Validator`].
pub struct Processor<'a, R: KingdomRelations> {
    pub state: &'a mut MgState,
    pub data: &'a MetaGameData,
    pub relations: &'a R,
    pub precision: Precision,
}

struct NoWorld;
impl CommandWorld for NoWorld {
    fn controllable(&self, _: ArmyId, _: KingdomActor) -> bool {
        unreachable!()
    }
    fn reach(&self, _: &MgState, _: ArmyId, _: usize) -> crate::Result<bool> {
        unreachable!()
    }
}

impl<R: KingdomRelations> Processor<'_, R> {
    fn view(&self) -> Validator<'_, R, NoWorld> {
        Validator { state: self.state, data: self.data, relations: self.relations, world: &NoWorld, precision: self.precision }
    }
    fn kingdom_of(&self, actor: KingdomActor) -> crate::Result<usize> {
        self.state
            .kingdoms
            .iter()
            .position(|k| k.actor == Some(actor))
            .ok_or_else(|| crate::Error::Unsupported("issuer without MG kingdom (native null deref)".into()))
    }

    /// `FUN_00954570` for a command that passed 951D70 (native runs Process
    /// only after validation, 9525A0/9505F0).
    pub fn process(&mut self, cmd: &Command, issuer: KingdomActor, world: &impl ProcessWorld) -> crate::Result<ProcessOutcome> {
        let mut out = ProcessOutcome::default();
        let index = cmd.data.map(|d| d.index).unwrap_or(u32::MAX);
        match index {
            // 9543C0 / 954400 -> 9537B0
            0 | 1 => {
                let army = cmd.army.expect("validated");
                let target = cmd.territory.and_then(|d| self.view().territory_of(d)).expect("validated");
                if self.state.army(army).territory == Some(target) {
                    out.result = true;
                    return Ok(out);
                }
                let Some(path) = world.path(self.state, army, target)? else {
                    return Ok(out);
                };
                // 93F4B0: replace the waypoint queue with the path
                self.state.armies[army.0 as usize].as_mut().expect("live").waypoints = path;
                out.messages.push(category::MOVEMENT);
                out.result = true;
            }
            // 954440
            2 => {
                if let Some(pb) = self.view().pending_battle()? {
                    self.state.battle_started = true;
                    self.state.battle_resolved = false;
                    let t = &self.state.territories[pb];
                    if t.armies.len() as i64 == i64::from(t.enemy_army_count) {
                        return Err(crate::Error::Unsupported("startbattle -> quickbattle 954220 branch".into()));
                    }
                }
                out.result = true;
            }
            // 950E40
            3 => {
                self.state.battle_play_requested = true;
                out.result = true;
            }
            // 950E70
            4 => {
                self.state.battle_started = false;
                self.state.battle_resolved = false;
                out.result = true;
            }
            // 9528B0
            7 => {
                let k = self.kingdom_of(issuer)?;
                let tech = cmd.technology.clone().expect("validated");
                let cost = self.shop(k).technologies.iter().find(|r| r.ids.as_deref() == Some(tech.as_str())).expect("validated").cost;
                self.state.kingdoms[k].resources = self.state.kingdoms[k].resources.wrapping_sub(cost);
                if !self.state.kingdoms[k].technologies.contains(&tech) {
                    self.state.kingdoms[k].technologies.push(tech);
                }
                out.messages.push(category::PURCHASE);
                out.result = true;
            }
            // 952A00
            8 => {
                let k = self.kingdom_of(issuer)?;
                let def = cmd.actor_definition.expect("validated");
                let ids = self.data.armies[def].ids.clone();
                let cost = self.shop(k).actors.iter().find(|r| r.ids.as_deref() == Some(ids.as_str())).expect("validated").cost;
                self.state.kingdoms[k].resources = self.state.kingdoms[k].resources.wrapping_sub(cost);
                // 955B70 on the capital: owner = the capital territory's owner
                let capital = self.state.kingdoms[k].capital.expect("validated capital");
                let owner = self.state.owner_actor(capital).ok_or_else(|| crate::Error::Unsupported("unowned capital".into()))?;
                let id = ArmyId(self.state.armies.len() as u32);
                self.state.armies.push(Some(crate::state::MgArmy {
                    definition: def,
                    owner,
                    divisions: self.data.armies[def].divisions.iter().map(|&d| DivisionRef(d as u32)).collect(),
                    fought: 0,
                    territory: None,
                    path: Vec::new(),
                    waypoints: Vec::new(),
                }));
                self.state.enter_territory(id, capital, self.relations)?;
                out.messages.push(category::PURCHASE);
                self.state.last_purchased = Some(id);
                out.created = Some(id);
                out.result = true;
            }
            // 952D10 / 952D90
            9 | 10 => {
                let k = self.kingdom_of(issuer)?;
                let army = cmd.army.expect("validated");
                let list = if index == 9 {
                    cmd.divisions.clone().unwrap_or_default()
                } else {
                    self.view().upgrade_all_list(k, army)
                };
                for d in list {
                    // 952B90 -> 93F750 (append when absent); pay only when added
                    let a = self.state.armies[army.0 as usize].as_mut().expect("live");
                    if !a.divisions.contains(&DivisionRef(d as u32)) {
                        a.divisions.push(DivisionRef(d as u32));
                        let cost = self.division_cost(k, d).expect("validated");
                        self.state.kingdoms[k].resources = self.state.kingdoms[k].resources.wrapping_sub(cost);
                    }
                    out.messages.push(category::PURCHASE);
                }
                out.respawned = self.change_type(k, army);
                out.result = true;
            }
            other => {
                return Err(crate::Error::Unsupported(format!("processing of command {other} (endturn/quickbattle: turn/resolve modules; take: cheats)")));
            }
        }
        Ok(out)
    }

    fn shop(&self, k: usize) -> &crate::data::Shop {
        &self.data.campaigns[self.state.campaign].kingdoms[self.state.kingdoms[k].data].shop
    }
    fn division_cost(&self, k: usize, d: usize) -> Option<i32> {
        let ids = &self.data.divisions[d].ids;
        self.shop(k).divisions.iter().find(|r| r.ids.as_ref() == Some(ids)).map(|r| r.cost)
    }
    /// `FUN_00951AA0` ChangeGActor: among the issuer's shop actors (shop
    /// order) the type with the strictly smallest division_matching_preference
    /// below the current one whose divisions are all in the army becomes the
    /// army's type (native respawns the actor through 955DD0).
    fn change_type(&mut self, k: usize, army: ArmyId) -> Option<ArmyId> {
        let current = self.state.army(army).definition;
        let mut best = self.data.armies[current].division_matching_preference;
        let mut pick = None;
        for r in &self.shop(k).actors {
            let Some(ids) = &r.ids else { continue };
            let Some(d) = self.data.armies.iter().position(|a| &a.ids == ids) else { continue };
            let def = &self.data.armies[d];
            let have = &self.state.army(army).divisions;
            if def.division_matching_preference < best && def.divisions.iter().all(|x| have.contains(&DivisionRef(*x as u32))) {
                best = def.division_matching_preference;
                pick = Some(d);
            }
        }
        match pick {
            Some(d) if d != current => {
                // 955DD0 respawns the actor; 93F620 restores the component from
                // the 9402E0 snapshot, which does not hold the waypoint queue.
                let a = self.state.armies[army.0 as usize].as_mut().expect("live");
                a.definition = d;
                a.waypoints.clear();
                Some(army)
            }
            _ => None,
        }
    }
}
