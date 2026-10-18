//! Actual 546AF0 element cases ported in phase8: argument order, selector
//! context writes, TriggerElement leaf null checks/results and the trigger-owned
//! Kingdom objective and mission-result methods.  Every deeper engine call is a
//! named `Services` method carrying the native address; implementations must
//! execute that native behaviour or return an explicit error.
//!
//! `evaluate` returns `None` only for opcodes outside this module, so callers
//! can compose it with the other native modules without inventing success.
use crate::{
    decode::{Element, Parameter},
    manager::{Error, Manager, Result},
    objectives::{self, MissionServices},
    program::{CoreProgram, compare, text, word},
    predicates,
    selectors::{self, Checks, Registry},
};
use aa_navigation::startup::Object;
use serde::{Deserialize, Serialize};

/// KKC_DataManager<T>::GetDataByIDSWarn instances used by trigger registers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Bank {
    /// 54E950 -> 54E680, GameData+584.
    Audio,
    /// 54E910/54E8A0 -> 54DF80, GameData+C.
    ActorData,
    /// 54E880 -> 54E080, GameData+464.
    Property,
    /// 54E8E0 -> 54E480, GameData+274.
    Command,
    /// 54E970 -> 54E280, GameData+6F0.
    Weather,
    /// 54E990 -> 54E380, GameData+73C.
    Sun,
    /// 54E9B0 -> 54E580, GameData+474.
    Formation,
    /// 54E9D0 -> 54E780, GameData+564.
    Sai,
    /// 54E9F0 -> 54E180, GameData+468.
    Ability,
}

pub trait Services:
    Registry + Checks + objectives::Services + MissionServices + crate::actions::Effects + crate::extended::Effects
{
    /// GetDataByIDSWarn(name, "TriggerRegister referencing invalid IDS").
    fn data_by_ids(&mut self, bank: Bank, name: &[u16]) -> Result<Option<Object>>;
    /// Object+8 flag word.
    fn flags8(&self, object: Object) -> Result<u32>;
    /// Actor virtual +108 (owning Kingdom).
    fn owner(&mut self, actor: Object) -> Result<Option<Object>>;
    /// Actor virtual +160 (transfer control to Kingdom).
    fn transfer_control(&mut self, actor: Object, kingdom: Object) -> Result<()>;
    /// 4E64C0 Kingdom default-lose test.
    fn kingdom_default_lose(&mut self, kingdom: Object) -> Result<bool>;
    /// 4F5490 World remove object.
    fn world_remove_object(&mut self, actor: Object) -> Result<()>;
    /// 4D7960 actor destroy.
    fn actor_destroy(&mut self, actor: Object) -> Result<()>;
    /// 7476E0(data, 0, 0, 0, 0) on the audio system [B36548].
    fn play_speech(&mut self, data: Option<Object>) -> Result<()>;
    /// 50C880 on World+130.
    fn flash_ui_button(&mut self, name: &[u16]) -> Result<()>;
    /// 48D500 difficulty table lookup on GameData+140; returns entry+C.
    fn difficulty_by_name(&mut self, name: &[u16]) -> Result<Option<i32>>;
    /// *(*(GameVars+104)) map difficulty.
    fn map_difficulty(&self) -> Result<i32>;
    /// ActorData virtual +38 check applied by 54E910.
    fn is_actor_data(&mut self, data: Object) -> Result<bool>;
    /// Actor virtual +158 and +15C used by 5423F0 before the capture scan.
    fn actor_virtual_158(&mut self, actor: Object) -> Result<bool>;
    fn actor_virtual_15c(&mut self, actor: Object) -> Result<Option<Object>>;
    /// Actor+7C organization and 578D10 membership by GTID.
    fn organization(&self, actor: Object) -> Result<Option<Object>>;
    fn organization_has_member(&mut self, organization: Object, id: u32) -> Result<bool>;
    /// Actor+4 definition (ActorData).
    fn definition(&self, actor: Object) -> Result<Option<Object>>;
    /// Actor+100/+104 64-bit state mask.
    fn state_bits(&self, actor: Object) -> Result<u64>;
    /// Actor+60 body: (+10 health, +14 maximum) binary32 words.
    fn body(&self, actor: Object) -> Result<Option<(u32, u32)>>;
    /// 4F0360 on World: first actor inside the Area (argument may be null).
    fn area_first_actor(&mut self, area: Option<Object>) -> Result<Option<Object>>;
    /// 4F0950(actor, area) inside test (handles its own null arguments).
    fn actor_inside_area(&mut self, actor: Option<Object>, area: Option<Object>) -> Result<bool>;
    /// 76B960 on [B36550]: player owning the Kingdom.
    fn kingdom_player(&mut self, kingdom: Object) -> Result<Option<Object>>;
    /// Player+50 selection.
    fn player_selection(&self, player: Object) -> Result<Option<Object>>;
    /// Selection+4 list node values, in list order.
    fn selection_actors(&self, selection: Object) -> Result<Vec<Option<Object>>>;
    /// 4E0FB0(property, 0) on a Kingdom (Kingdom+318 property list lookup).
    fn kingdom_has_property(&mut self, kingdom: Object, property: Object) -> Result<u32>;
    /// Actor virtual +70 (visible) / +74 (explored) with (Kingdom, 1); returns AL.
    fn actor_seen_by(&mut self, actor: Object, kingdom: Object, explored: bool) -> Result<u32>;
}

/// 76B960 then player+50: the Kingdom's current selection.
fn selection<S: Services + ?Sized>(s: &mut S, kingdom: Object) -> Result<Option<Object>> {
    match s.kingdom_player(kingdom)? {
        Some(player) => s.player_selection(player),
        None => Ok(None),
    }
}

/// 54E910: ActorData bank lookup by register string, then virtual +38.
pub(crate) fn actor_data<S: Services + ?Sized>(p: &Parameter, s: &mut S) -> Result<Option<Object>> {
    match s.data_by_ids(Bank::ActorData, &selectors::string(p)?)? {
        Some(d) if s.is_actor_data(d)? => Ok(Some(d)),
        _ => Ok(None),
    }
}

/// Registry generation lookup + Object+8 bit0 (542CA0/542D00 victim test).
fn alive<S: Services + ?Sized>(s: &S, id: u32) -> Result<bool> {
    Ok(match s.resolve_id(id)? {
        Some(o) => s.flags8(o)? & 1 == 0,
        None => false,
    })
}

const BUTTON_OBJECTIVES: &str = "ButtonObjectives";
fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

/// 54DF10 difficulty register.
fn difficulty<S: Services + ?Sized>(
    core: &mut CoreProgram,
    manager: &Manager,
    p: &Parameter,
    s: &mut S,
) -> Result<i32> {
    Ok(match p.item_index {
        1 => word(p)? as i32,
        2 => selectors::ftol(word(p)?),
        3 => {
            // 54CCE0 then fld [var+4]: a missing/empty variable faults natively.
            let before = core.diagnostics.len();
            let bits = core.value(manager, p)?;
            if core.diagnostics.len() != before || text(p)?.is_empty() {
                return Err(Error::State(
                    "difficulty variable missing: native dereferences a null variable".into(),
                ));
            }
            selectors::ftol(bits)
        }
        57 => s.map_difficulty()?,
        56 => s.difficulty_by_name(text(p)?)?.unwrap_or(-1),
        _ => 0,
    })
}

fn int_to_f32(value: i32) -> Result<u32> {
    aa_numeric::Binary::from_i32(value)
        .store_f32()
        .map_err(|e| Error::State(e.to_string()))
}

/// 545A50/545B00/545BB0 objective state wrappers.
fn objective_state<S: Services + ?Sized>(
    s: &mut S,
    definition: Option<usize>,
    kingdom: Option<Object>,
    state: u32,
) -> Result<bool> {
    let (Some(kingdom), Some(definition)) = (kingdom, definition) else {
        return Ok(false);
    };
    let name = s.objective_definitions()?[definition].name.clone();
    let changed = objectives::set_state(s, kingdom, &name, state)?;
    if changed {
        s.flash_ui_button(&utf16(BUTTON_OBJECTIVES))?;
    }
    Ok(changed)
}

/// Conditions, events and objective/victory/speech actions handled here.
pub const SUPPORTED: &[u32] = &[
    2, 3, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 17, 18, 19, 22, 23, 27, 28, 40, 50, 51, 56, 57, 58,
    59, 60, 79, 87, 97, 98, 99, 100, 107,
];

pub fn evaluate<S: Services + ?Sized>(
    core: &mut CoreProgram,
    manager: &mut Manager,
    element: &Element,
    s: &mut S,
) -> Result<Option<bool>> {
    let action = crate::actions::SUPPORTED.contains(&element.opcode);
    let extended = crate::extended::SUPPORTED.contains(&element.opcode);
    if !SUPPORTED.contains(&element.opcode) && !action && !extended {
        return Ok(None);
    }
    if !crate::program::valid(element) {
        return Ok(Some(false));
    }
    // 5422F0 (idempotent when evaluate_core already converted this element).
    manager.convert_registers(element);
    if action {
        return crate::actions::evaluate(core, manager, element, s).map(Some);
    }
    if extended {
        return crate::extended::evaluate(core, manager, element, s).map(Some);
    }
    let kind = selectors::kind(element)?;
    let p = &element.parameters;
    let value = match element.opcode {
        2 => {
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            match (actor, kingdom) {
                (Some(actor), Some(kingdom)) => {
                    let target = if s.actor_virtual_158(actor)? {
                        s.actor_virtual_15c(actor)?
                    } else {
                        Some(actor)
                    };
                    let actor_gtid = target.map(|t| s.raw_id(t)).transpose()?.unwrap_or(0);
                    let kingdom_gtid = s.raw_id(kingdom)?;
                    predicates::captured(manager, kingdom_gtid, actor_gtid)?
                }
                _ => false,
            }
        }
        3 => match (p[0].item_index == 11, p[1].item_index == 11) {
            (true, true) => predicates::destroyed_any(manager)?,
            (true, false) => {
                let victim = selectors::actor_id(manager, &p[1], s)?;
                if victim != 0 && !alive(s, victim)? {
                    predicates::killer_of(manager, victim)? != 0
                } else {
                    false
                }
            }
            (false, true) => {
                let killer = selectors::actor_id(manager, &p[0], s)?;
                killer != 0 && predicates::killed_by(manager, killer)?
            }
            (false, false) => {
                let victim = selectors::actor_id(manager, &p[1], s)?;
                let killer = selectors::actor_id(manager, &p[0], s)?;
                if (killer == 0 && victim == 0) || alive(s, victim)? {
                    false
                } else if let Some(killer_object) = s.resolve_id(killer)? {
                    let found = predicates::killer_of(manager, victim)?;
                    let member = match s.organization(killer_object)? {
                        Some(org) => s.organization_has_member(org, found)?,
                        None => false,
                    };
                    member || found == killer
                } else {
                    false
                }
            }
        },
        5 => {
            let area = selectors::area(&p[1], s)?;
            if p[0].item_index == 11 {
                match area {
                    Some(area) => predicates::entered_any(manager, area.0)?,
                    None => false,
                }
            } else {
                let id = selectors::actor_id(manager, &p[0], s)?;
                match area {
                    Some(area) if id != 0 => predicates::entered(manager, id, area.0)?,
                    _ => false,
                }
            }
        }
        6..=8 => {
            let any = p[1].item_index == 46;
            let data = if any { None } else { actor_data(&p[1], s)? };
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            match kingdom {
                Some(kingdom) if any || data.is_some() => {
                    let id = s.raw_id(kingdom)?;
                    let wanted = data.map(|d| d.0);
                    if element.opcode == 8 {
                        predicates::recruited(manager, id, wanted)?
                    } else {
                        predicates::construction(manager, id, wanted, element.opcode == 6)?
                    }
                }
                _ => false,
            }
        }
        9 => {
            let name = selectors::string(&p[0])?;
            predicates::script_heard(manager, &name)?
        }
        12 | 13 => {
            let index = word(&p[1])?;
            match selectors::actor(manager, kind, &p[0], s)? {
                Some(actor) => {
                    predicates::has_state_bit(s.state_bits(actor)?, index) != (element.opcode == 13)
                }
                None => false,
            }
        }
        10 => {
            let value = core.value(manager, &p[2])?;
            let comparison = selectors::comparison(&p[1])?;
            match selectors::actor(manager, kind, &p[0], s)? {
                Some(actor) => match s.body(actor)? {
                    // 542890: health compared with value * maximum * 0.01f.
                    Some((health, maximum)) => {
                        let pc = core.precision;
                        let num = |b: u32| {
                            aa_numeric::Binary::from_f32_bits(b).map_err(|e| Error::State(e.to_string()))
                        };
                        let product = num(value)?
                            .mul(&num(maximum)?, pc)
                            .and_then(|v| v.mul(&aa_numeric::Binary::from_f32_bits(0x3c23_d70a)?, pc))
                            .and_then(|v| v.store_f32())
                            .map_err(|e| Error::State(e.to_string()))?;
                        compare(health, comparison, product)?
                    }
                    None => false,
                },
                None => false,
            }
        }
        14 => {
            let area = selectors::area(&p[1], s)?;
            if p[0].item_index == 11 {
                // 5425F0: the first actor found becomes the condition subject.
                match s.area_first_actor(area)? {
                    Some(actor) => {
                        manager.set_context_word(s.raw_id(actor)?);
                        true
                    }
                    None => false,
                }
            } else {
                let actor = selectors::actor(manager, kind, &p[0], s)?;
                s.actor_inside_area(actor, area)?
            }
        }
        18 => {
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            match (actor, kingdom) {
                // 543B60 -> 543A10 list membership.
                (Some(actor), Some(kingdom)) => match selection(s, kingdom)? {
                    Some(sel) => s.selection_actors(sel)?.contains(&Some(actor)),
                    None => false,
                },
                _ => false,
            }
        }
        19 => {
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let data = actor_data(&p[0], s)?;
            match (data, kingdom) {
                // 541A20 -> 4ECD40: any selected actor whose +4 is the data.
                (Some(data), Some(kingdom)) => match selection(s, kingdom)? {
                    Some(sel) => {
                        let mut found = false;
                        for actor in s.selection_actors(sel)?.into_iter().flatten() {
                            if s.definition(actor)? == Some(data) {
                                found = true;
                                break;
                            }
                        }
                        found
                    }
                    None => false,
                },
                _ => false,
            }
        }
        22 => {
            let property = s.data_by_ids(Bank::Property, text(&p[1])?)?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            match (kingdom, property) {
                (Some(k), Some(prop)) => s.kingdom_has_property(k, prop)? != 0,
                _ => false,
            }
        }
        99 | 100 => {
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            match (actor, kingdom) {
                (Some(a), Some(k)) => s.actor_seen_by(a, k, element.opcode == 100)? & 0xff != 0,
                _ => false,
            }
        }
        27 => {
            let data = actor_data(&p[1], s)?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            match (actor, data) {
                (Some(actor), Some(data)) => s.definition(actor)? == Some(data),
                _ => false,
            }
        }
        15 => {
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            match (actor, kingdom) {
                (Some(actor), Some(kingdom)) => s.owner(actor)? == Some(kingdom),
                _ => false,
            }
        }
        17 => {
            let right = int_to_f32(difficulty(core, manager, &p[2], s)?)?;
            let comparison = selectors::comparison(&p[1])?;
            let left = int_to_f32(difficulty(core, manager, &p[0], s)?)?;
            compare(left, comparison, right)?
        }
        23 => match selectors::kingdom(manager, kind, &p[0], s)? {
            Some(kingdom) => s.kingdom_default_lose(kingdom)?,
            None => false,
        },
        28 => match selectors::actor(manager, kind, &p[0], s)? {
            Some(actor) => s.flags8(actor)? & 1 != 0,
            None => true,
        },
        40 => {
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            match (actor, kingdom) {
                (Some(actor), Some(kingdom)) => {
                    s.transfer_control(actor, kingdom)?;
                    true
                }
                _ => false,
            }
        }
        50 | 51 => {
            let name = selectors::string(&p[1])?;
            let Some(kingdom) = selectors::kingdom(manager, kind, &p[0], s)? else {
                return Ok(Some(false));
            };
            let changed = if element.opcode == 50 {
                objectives::gain(s, kingdom, &name)?
            } else {
                objectives::lose(s, kingdom, &name)?
            };
            if changed {
                s.flash_ui_button(&utf16(BUTTON_OBJECTIVES))?;
            }
            changed
        }
        56 | 57 => match selectors::kingdom(manager, kind, &p[0], s)? {
            Some(kingdom) => {
                let result = if element.opcode == 56 { 1 } else { 2 };
                objectives::set_mission_result(s, kingdom, result, true)?;
                true
            }
            None => false,
        },
        58..=60 => {
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let definition = objectives::find_definition(s, text(&p[0])?)?;
            let state = match element.opcode {
                58 => 1,
                59 => 2,
                _ => 0,
            };
            objective_state(s, definition, kingdom, state)?
        }
        79 => match selectors::actor(manager, kind, &p[0], s)? {
            Some(actor) => {
                s.actor_destroy(actor)?;
                true
            }
            None => false,
        },
        87 => {
            let data = s.data_by_ids(Bank::Audio, text(&p[0])?)?;
            s.play_speech(data)?;
            true
        }
        97 | 98 => {
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let definition = objectives::find_definition(s, text(&p[0])?)?;
            match kingdom {
                Some(kingdom) => {
                    objectives::has_state(s, kingdom, definition, if element.opcode == 97 { 1 } else { 2 })?
                }
                None => false,
            }
        }
        107 => match selectors::actor(manager, kind, &p[0], s)? {
            Some(actor) => {
                s.world_remove_object(actor)?;
                true
            }
            None => false,
        },
        _ => unreachable!(),
    };
    Ok(Some(value))
}
