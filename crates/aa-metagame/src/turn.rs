//! Turn advance of the metagame, ported from the original AA.exe.
//!
//! | native | role |
//! |---|---|
//! | 952790 | `KKC_MGCommand::ProcessEndTurn`: end-turn message (engine), then 9391F0 |
//! | 9391F0 | advance (938C90) until the current kingdom can play (938DA0), at most once per kingdom |
//! | 938DA0 | can play: capital not lost (95A350), or an owned army with movement > 0 |
//! | 938C90 | next kingdom: index + 1 mod count, clear results (938A80), battles counters 0, territory turn start (9556D0), new round: round + 1, income (959FB0), event rolls (95ADD0); event fire (95BE20); clear `+0x44` / `+0x46` |
//! | 9556D0 | territory turn start: auto-capture when the defender strength is 0.0f and `+0x1C > 0` (953A20, resolver (T, won) Process 967530, 938E10; no record), then 93FE10 on every army there |
//! | 93FE10 | army turn start: fought = 0, path = [current territory] (93F9F0) |
//! | 959FB0 | income: resources += production of owned territories (i32 wrapping) |
//! | 95ADD0 | event roll (new round): fixed round, or one draw against `round_probability` (x87, unrounded); kingdom = `kingdom_turn` or one draw over the kingdoms |
//! | 95BE20 | event fire: triggered and the current kingdom is the event kingdom -> 95BC80 |
//! | 95BC80 | effects in order (95B630 add_tech, 95B220 add_actor, 95B7F0 remove_tech, 95B400 remove_actor, 95B9B0 change_territory); event message when any effect applied |
//! | 95AED0 / 95AE80 / 95AF40 | effect territory / army type / technology (given, or one draw) |
//!
//! Proven by `tests/native_turn.rs` against
//! `research/phase8-metagame/native-turn.json` (PC53 and subsets at PC24/PC64;
//! synthetic campaign events, since no shipped campaign has any).
//!
//! Localized message blocks (the event / effect `message` formatted with
//! names and posted, category 0x1F) are engine seams: [`EngineCall::EventMessage`]
//! and [`EngineCall::EventEffectMessage`] carry the formatting arguments.
use crate::battle::{DivisionRef, KingdomActor, KingdomRelations, Side};
use crate::data::{EffectKind, EventEffect};
use crate::resolve::{Engine, EngineCall, MgContext};
use crate::state::{ArmyId, MgArmy, MgState};
use crate::{Error, Result};
use aa_numeric::Binary;
use std::cmp::Ordering;

/// 9FDAA0 (2^32, binary64) and 9FDA98 (1 / (2^32 - 1), binary64).
const TWO_POW_32_F64: u64 = 0x41f0_0000_0000_0000;
const INV_U32_MAX_F64: u64 = 0x3df0_0000_0010_0000;

fn unsupported<T>(what: &str) -> Result<T> {
    Err(Error::Unsupported(what.to_string()))
}

impl<R: KingdomRelations, E: Engine> MgContext<'_, R, E> {
    /// `FUN_00952790(issuer)`: always returns true. The message block
    /// (938B70 + 95A8D0 + 40BE60) dereferences the issuer's MG kingdom.
    pub fn process_end_turn(&mut self, issuer: KingdomActor) -> Result<bool> {
        self.require_actors()?;
        let Some(k) = self.kingdom_of_actor(issuer) else {
            return unsupported("952790: issuer without MG kingdom (95A8D0 null dereference)");
        };
        self.engine.call(EngineCall::EndTurnMessage { kingdom: k });
        self.advance_turn()?;
        Ok(true)
    }

    /// `FUN_009391F0`.
    pub fn advance_turn(&mut self) -> Result<()> {
        self.next_kingdom()?;
        let mut i = 1usize;
        while !self.can_play()? && i < self.state.kingdoms.len() {
            self.next_kingdom()?;
            i += 1;
        }
        Ok(())
    }

    /// `FUN_00938DA0`: the current kingdom still holds its capital (95A350
    /// false) or an army of its kingdom actor's engine list (`+0x284`,
    /// [`Engine::owned_armies`]) has a type with movement > 0.
    pub fn can_play(&self) -> Result<bool> {
        let Some(k) = self.state.current else {
            return unsupported("938DA0 without current kingdom (native null dereference)");
        };
        if !self.state.capital_lost(k, self.relations)? {
            return Ok(true);
        }
        let actor = self.actor(k)?;
        Ok(self
            .engine
            .owned_armies(self.state, actor)
            .iter()
            .any(|&a| self.data.armies[self.state.army(a).definition].movement > 0))
    }

    /// `FUN_00938C90`: advance to the next kingdom.
    pub fn next_kingdom(&mut self) -> Result<()> {
        self.require_actors()?;
        let count = self.state.kingdoms.len();
        if count == 0 {
            return unsupported("938C90 with no kingdoms (native division by zero)");
        }
        // index of the current kingdom (-1 when not found) + 1
        let n = match self.state.current {
            Some(c) if c < count => c + 1,
            _ => 0,
        };
        self.state.current = Some(n % count);
        // 938A80
        self.state.results.clear();
        for k in &mut self.state.kingdoms {
            k.battles = 0;
        }
        for t in 0..self.state.territories.len() {
            self.territory_turn_start(t)?;
        }
        if n == count {
            self.state.round = self.state.round.wrapping_add(1);
            for k in 0..count {
                self.income(k);
            }
            for i in 0..self.state.events.len() {
                self.roll_event(i)?;
            }
        }
        for i in 0..self.state.events.len() {
            self.fire_event(i)?;
        }
        self.state.battle_started = false;
        self.state.battle_resolved = false;
        Ok(())
    }

    /// `FUN_00959FB0`.
    pub fn income(&mut self, k: usize) {
        let map = self.data.campaigns[self.state.campaign].map.expect("campaign map");
        let defs = &self.data.maps[map].territories;
        let sum = self.state.kingdoms[k]
            .territories
            .iter()
            .fold(0i32, |s, &t| s.wrapping_add(defs[self.state.territories[t].definition].production));
        let kk = &mut self.state.kingdoms[k];
        kk.resources = kk.resources.wrapping_add(sum);
    }

    /// `FUN_009556D0`: territory turn start.
    pub fn territory_turn_start(&mut self, t: usize) -> Result<()> {
        let strength = self.defender_strength(t)?;
        if Binary::from_f32_bits(strength)?.is_zero() && self.state.territories[t].enemy_army_count > 0 {
            let owner = self.state.territories[t].owner;
            let Some(a0) = self.nth(t, Side::Attacker, 0) else {
                return unsupported("9556D0: attackers counted but none found (native null dereference)");
            };
            let attacker = self.kingdom_of_actor(self.state.army(a0).owner);
            self.battle_messages(attacker, owner, t, true)?;
            self.process(t, true)?;
            self.check_game_over()?;
        }
        let armies: Vec<ArmyId> = self.state.territories[t].armies.iter().map(|r| r.army).collect();
        for id in armies {
            Self::army_turn_start(self.state, id)?;
        }
        Ok(())
    }

    /// `FUN_0093FE10`: fought = 0, path emptied then 93F9F0(current territory).
    fn army_turn_start(state: &mut MgState, id: ArmyId) -> Result<()> {
        let a = state.armies[id.0 as usize].as_mut().expect("live army");
        let Some(current) = a.territory else {
            return unsupported("93FE10 on an army without territory (null path entry)");
        };
        a.fought = 0;
        a.path.clear();
        a.path.push(current);
        Ok(())
    }

    // ------------------------------------------------------------ campaign events

    fn event_of(&self, i: usize) -> Result<usize> {
        let e = self.state.events[i].event;
        if e >= self.data.campaigns[self.state.campaign].events.len() {
            return unsupported("event state without campaign event");
        }
        Ok(e)
    }

    /// `FUN_0095ADD0` (state `+0x0C` triggered, `+0x10` kingdom).
    pub fn roll_event(&mut self, i: usize) -> Result<()> {
        let data = self.data;
        let c = &data.campaigns[self.state.campaign];
        let ev = &c.events[self.event_of(i)?];
        let mut triggered = true;
        if ev.round != -1 {
            if self.state.round != ev.round {
                triggered = false;
            }
        } else {
            // fild / fadd 2^32 when negative / fmul 1/(2^32-1), then fcomp with
            // the binary32 probability: kept when roll <= probability.
            let draw = self.state.rng.next_u32() as i32;
            let mut roll = Binary::from_i32(draw);
            if draw < 0 {
                roll = roll.add(&Binary::from_f64_bits(TWO_POW_32_F64)?, self.precision)?;
            }
            roll = roll.mul(&Binary::from_f64_bits(INV_U32_MAX_F64)?, self.precision)?;
            if roll.compare(&Binary::from_f32_bits(ev.round_probability_bits)?) == Ordering::Greater {
                triggered = false;
            }
        }
        self.state.events[i].triggered = triggered;
        if !triggered {
            self.state.events[i].kingdom = None;
            return Ok(());
        }
        let kingdom = match ev.kingdom_turn {
            Some(k) => k,
            None => {
                // div by the campaign kingdom count, 9389B0 on the MG kingdoms
                let count = c.kingdoms.len() as u32;
                if count == 0 {
                    return unsupported("95ADD0 with no campaign kingdoms (native division by zero)");
                }
                let idx = (self.state.rng.next_u32() % count) as usize;
                let Some(k) = self.state.kingdoms.get(idx) else {
                    return unsupported("95ADD0: kingdom index outside the MG kingdom list");
                };
                k.data
            }
        };
        self.state.events[i].kingdom = Some(kingdom);
        Ok(())
    }

    /// `FUN_0095BE20`: fire when triggered and the current kingdom is the
    /// event kingdom (95BC80); returns whether it fired.
    pub fn fire_event(&mut self, i: usize) -> Result<bool> {
        let st = self.state.events[i];
        if !st.triggered {
            return Ok(false);
        }
        let Some(cur) = self.state.current else {
            return unsupported("95BE20 without current kingdom (native null dereference)");
        };
        if st.kingdom != Some(self.state.kingdoms[cur].data) {
            return Ok(false);
        }
        self.apply_event(i)?;
        Ok(true)
    }

    /// `FUN_0095BC80`: every effect in order; the event message when any applied.
    fn apply_event(&mut self, i: usize) -> Result<()> {
        let data = self.data;
        let e = self.event_of(i)?;
        let ev = &data.campaigns[self.state.campaign].events[e];
        let mut any = false;
        for (j, effect) in ev.effects.iter().enumerate() {
            let applied = match effect.kind {
                EffectKind::AddTech => self.effect_technology(e, j, effect, true)?,
                EffectKind::AddActor => self.effect_add_actor(e, j, effect)?,
                EffectKind::RemoveTech => self.effect_technology(e, j, effect, false)?,
                EffectKind::RemoveActor => self.effect_remove_actor(e, j, effect)?,
                EffectKind::ChangeTerritory => self.effect_change_territory(e, j, effect)?,
                EffectKind::Unknown => false,
            };
            any |= applied;
        }
        if any && !ev.message.is_empty() {
            let kingdom = self.state.current.expect("checked by 95BE20");
            self.engine.call(EngineCall::EventMessage { event: e, kingdom });
        }
        Ok(())
    }

    /// Effect kingdom: the target (938BC0 by IDS) or the current kingdom.
    fn effect_kingdom(&self, target: Option<usize>) -> Option<usize> {
        match target {
            None => self.state.current,
            Some(kd) => self.kingdom_of_data(kd),
        }
    }

    /// `FUN_0095AED0(effect, k)`: one draw over `k`'s owned territories (none
    /// without). With an effect territory the native looks it up (939B60) and
    /// compares its owner with `k`, but both paths return `esi`, which is 0
    /// there (95AED8 `xor esi, esi`; 95AF2B `mov eax, esi`): always none.
    fn effect_territory(&mut self, effect: &EventEffect, k: usize) -> Result<Option<usize>> {
        if let Some(d) = effect.territory {
            if self.territory_of(d).is_none() {
                return unsupported("95AED0: effect territory without MG territory (native null dereference)");
            }
            return Ok(None);
        }
        let n = self.state.kingdoms[k].territories.len() as u32;
        if n == 0 {
            return Ok(None);
        }
        let idx = (self.state.rng.next_u32() % n) as usize;
        Ok(Some(self.state.kingdoms[k].territories[idx]))
    }

    fn army_definition(&self, ids: &str) -> Result<usize> {
        self.data
            .armies
            .iter()
            .position(|a| a.ids.eq_ignore_ascii_case(ids))
            .ok_or_else(|| Error::Unsupported(format!("army type {ids} not loaded")))
    }

    /// `FUN_0095AE80(effect, k)`: the effect army type, or one draw over the
    /// actor rows of `k`'s shop (a row without actor gives none).
    fn effect_army_type(&mut self, effect: &EventEffect, k: usize) -> Result<Option<usize>> {
        if let Some(ids) = &effect.actor_ids {
            return Ok(Some(self.army_definition(ids)?));
        }
        let data = self.data;
        let rows = &data.campaigns[self.state.campaign].kingdoms[self.state.kingdoms[k].data].shop.actors;
        if rows.is_empty() {
            return Ok(None);
        }
        let idx = (self.state.rng.next_u32() % rows.len() as u32) as usize;
        match &rows[idx].ids {
            None => Ok(None),
            Some(ids) => Ok(Some(self.army_definition(ids)?)),
        }
    }

    /// `FUN_0095B220` add_actor: 955B70 creates an army of the type in the
    /// territory for the territory owner (4FA030, then 940100).
    fn effect_add_actor(&mut self, event: usize, index: usize, effect: &EventEffect) -> Result<bool> {
        let Some(k) = self.effect_kingdom(effect.kingdom_target) else { return Ok(false) };
        let Some(t) = self.effect_territory(effect, k)? else { return Ok(false) };
        let Some(definition) = self.effect_army_type(effect, k)? else { return Ok(false) };
        let Some(owner) = self.state.owner_actor(t) else {
            return unsupported("955B70 on an unowned territory (native null dereference)");
        };
        let id = ArmyId(self.state.armies.len() as u32);
        self.engine.call(EngineCall::CreateArmy { army: id, definition, owner });
        self.state.armies.push(Some(MgArmy {
            definition,
            owner,
            // ASSUMPTION (engine component creation, as InitCreate): the type's divisions
            divisions: self.data.armies[definition].divisions.iter().map(|&d| DivisionRef(d as u32)).collect(),
            fought: 0,
            territory: None,
            path: Vec::new(),
            waypoints: Vec::new(),
        }));
        self.state.enter_territory(id, t, self.relations)?;
        if !effect.message.is_empty() {
            self.engine.call(EngineCall::EventEffectMessage {
                event,
                effect: index,
                kind: effect.kind,
                kingdom: k,
                army: Some(id),
                territory: Some(t),
                technology: None,
                source: None,
            });
        }
        Ok(true)
    }

    /// `FUN_0095B400` remove_actor: one draw over the kingdom actor's engine
    /// list (95A3C0 / 95A410), then 956350 kill + remove (after the message).
    fn effect_remove_actor(&mut self, event: usize, index: usize, effect: &EventEffect) -> Result<bool> {
        let Some(k) = self.effect_kingdom(effect.kingdom_target) else { return Ok(false) };
        let Some(actor) = self.state.kingdoms[k].actor else { return Ok(false) };
        let list = self.engine.owned_armies(self.state, actor);
        if list.is_empty() {
            return Ok(false);
        }
        let army = list[(self.state.rng.next_u32() % list.len() as u32) as usize];
        let Some(t) = self.state.army(army).territory else {
            return unsupported("95B400: army without territory (native null dereference)");
        };
        if !effect.message.is_empty() {
            self.engine.call(EngineCall::EventEffectMessage {
                event,
                effect: index,
                kind: effect.kind,
                kingdom: k,
                army: Some(army),
                territory: Some(t),
                technology: None,
                source: None,
            });
        }
        self.kill(t, Some(army))?;
        Ok(true)
    }

    /// `FUN_0095AF40(effect, k, add)`: the effect technology, or one draw
    /// over the candidates: add (95A7D0) = shop technology rows of `k` not
    /// owned (shop order, a NULL row included), remove (95A7C0) = a copy of
    /// the owned list. `""` / `None` stand for NULL.
    fn pick_technology(&mut self, effect: &EventEffect, k: usize, add: bool) -> Option<String> {
        if let Some(t) = &effect.tech_ids {
            return Some(t.clone());
        }
        let data = self.data;
        let owned = &self.state.kingdoms[k].technologies;
        let candidates: Vec<String> = if add {
            data.campaigns[self.state.campaign].kingdoms[self.state.kingdoms[k].data]
                .shop
                .technologies
                .iter()
                .map(|r| r.ids.clone().unwrap_or_default())
                .filter(|t| !owned.contains(t))
                .collect()
        } else {
            owned.clone()
        };
        if candidates.is_empty() {
            return None;
        }
        let pick = candidates[(self.state.rng.next_u32() % candidates.len() as u32) as usize].clone();
        (!pick.is_empty()).then_some(pick)
    }

    /// `FUN_0095B630` add_tech (95A770: append when absent) /
    /// `FUN_0095B7F0` remove_tech (95A020: drop the first match).
    fn effect_technology(&mut self, event: usize, index: usize, effect: &EventEffect, add: bool) -> Result<bool> {
        let Some(k) = self.effect_kingdom(effect.kingdom_target) else { return Ok(false) };
        let Some(tech) = self.pick_technology(effect, k, add) else { return Ok(false) };
        let owned = &mut self.state.kingdoms[k].technologies;
        if add {
            if !owned.contains(&tech) {
                owned.push(tech.clone());
            }
        } else if let Some(p) = owned.iter().position(|t| *t == tech) {
            owned.remove(p);
        }
        if !effect.message.is_empty() {
            self.engine.call(EngineCall::EventEffectMessage {
                event,
                effect: index,
                kind: effect.kind,
                kingdom: k,
                army: None,
                territory: None,
                technology: Some(tech),
                source: None,
            });
        }
        Ok(true)
    }

    /// `FUN_0095B9B0` change_territory: pick the territory (given and held by
    /// the source, a draw over the source's territories, given, or a draw
    /// over every MG territory), refuse one the target already holds, then
    /// the attacker-win capture 967290(target, no destroy).
    fn effect_change_territory(&mut self, event: usize, index: usize, effect: &EventEffect) -> Result<bool> {
        let Some(target) = self.effect_kingdom(effect.kingdom_target) else { return Ok(false) };
        let source = effect.kingdom_source.and_then(|kd| self.kingdom_of_data(kd));
        let given = match effect.territory {
            None => None,
            Some(d) => match self.territory_of(d) {
                Some(t) => Some(t),
                None => return unsupported("95B9B0: effect territory without MG territory"),
            },
        };
        let t = match (source, given) {
            (Some(s), Some(t)) => {
                if self.state.territories[t].owner != Some(s) || s == target {
                    return Ok(false);
                }
                t
            }
            (Some(s), None) => {
                if s == target {
                    return Ok(false);
                }
                match self.effect_territory(effect, s)? {
                    Some(t) => t,
                    None => return Ok(false),
                }
            }
            (None, Some(t)) => {
                if self.state.territories[t].owner == Some(target) {
                    return Ok(false);
                }
                t
            }
            (None, None) => {
                let n = self.state.territories.len() as u32;
                if n == 0 {
                    return unsupported("95B9B0 with no territories (native division by zero)");
                }
                let t = (self.state.rng.next_u32() % n) as usize;
                if self.state.territories[t].owner == Some(target) {
                    return Ok(false);
                }
                t
            }
        };
        self.attacker_win(t, Some(target), false)?;
        if !effect.message.is_empty() {
            if source.is_none() {
                return unsupported("95B9B0 message with no source kingdom: 95A8D0(NULL) (native crash)");
            }
            self.engine.call(EngineCall::EventEffectMessage {
                event,
                effect: index,
                kind: effect.kind,
                kingdom: target,
                army: None,
                territory: Some(t),
                technology: None,
                source,
            });
        }
        Ok(true)
    }
}
