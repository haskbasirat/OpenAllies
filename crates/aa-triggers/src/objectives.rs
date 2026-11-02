//! Kingdom objective list (Kingdom+300/+304) and mission result (Kingdom+19C).
//!
//! Ported from the original Kingdom methods used by trigger elements:
//! 4F1AD0 World objective lookup, 4E7050 gain (optional SortObjectives
//! ordering), 4E2120 lose, 4E2020 find, 4E21F0/4E2250/4E22B0 state changes and
//! 4E9720 SetMissionResult.  Notifications (42D4B0), UI flashes and the game
//! variable table are services.  Objective names compare with `wcscmp`
//! semantics: UTF-16 code units, case-sensitive, ending at the first NUL.
use crate::manager::{Error, Result};
use aa_navigation::startup::Object;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// World+250 objective definition: +4 name, +C text, +10 byte flag.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObjectiveDef {
    pub name: Vec<u16>,
    pub text: Vec<u16>,
    pub flag_10: u8,
}
/// Kingdom objective entry {definition, state}: 0 incomplete, 1 complete, 2 failed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Index into the World objective definition table (stable after load).
    pub definition: usize,
    pub state: u32,
}

/// Original notification codes posted through 42D4B0.
pub mod message {
    pub const OBJECTIVE_GAINED_FLAGGED: u32 = 0x2a;
    pub const OBJECTIVE_GAINED: u32 = 0x2b;
    pub const OBJECTIVE_LOST_FLAGGED: u32 = 0x2c;
    pub const OBJECTIVE_LOST: u32 = 0x2d;
    pub const OBJECTIVE_FAILED_FLAGGED: u32 = 0x2e;
    pub const OBJECTIVE_FAILED: u32 = 0x2f;
    pub const OBJECTIVE_COMPLETE_FLAGGED: u32 = 0x30;
    pub const OBJECTIVE_COMPLETE: u32 = 0x31;
    pub const MISSION_LOST: u32 = 0x34;
    pub const MISSION_WON: u32 = 0x35;
}

/// The OBJECTIVES chunk as the map load reads it between TEAMS and KINGDOMS
/// (`4FCB3B`): a signed count, then per World+250 MapString object
/// (`502FC0`, version 0 only): version word, key string, display key string
/// (u16 length + UTF-16 units), the display text resolved by 49AD70 when the
/// display key is not empty, and the +10 flag byte; then the World+25C word.
/// An object is appended only after its whole record was read (4FCBB2).
/// Returns the definitions and World+25C.
pub fn load_chunk(
    bytes: &[u8],
    mut lookup: impl FnMut(&[u16]) -> std::result::Result<Vec<u16>, String>,
) -> std::result::Result<(Vec<ObjectiveDef>, u32), String> {
    let mut at = 0usize;
    let mut take = |n: usize| -> std::result::Result<&[u8], String> {
        let end = at.checked_add(n).ok_or("OBJECTIVES stream position overflow")?;
        let v = bytes.get(at..end).ok_or("OBJECTIVES stream is truncated")?;
        at = end;
        Ok(v)
    };
    fn word(v: &[u8]) -> u32 {
        u32::from_le_bytes(v.try_into().unwrap())
    }
    let count = word(take(4)?) as i32;
    let mut out = Vec::new();
    for _ in 0..count.max(0) {
        if word(take(4)?) != 0 {
            return Err("MapString objective version other than 0 (502FC0 diagnostic path)".into());
        }
        let mut string = || -> std::result::Result<Vec<u16>, String> {
            let n = u16::from_le_bytes(take(2)?.try_into().unwrap()) as usize;
            Ok(take(n * 2)?.chunks_exact(2).map(|b| u16::from_le_bytes([b[0], b[1]])).collect())
        };
        let name = string()?;
        let display_key = string()?;
        let text = if display_key.is_empty() { Vec::new() } else { lookup(&display_key)? };
        let flag_10 = take(1)?[0];
        out.push(ObjectiveDef { name, text, flag_10 });
    }
    let word_25c = word(take(4)?);
    Ok((out, word_25c))
}

pub fn wcscmp(a: &[u16], b: &[u16]) -> Ordering {
    let a = a.iter().copied().take_while(|c| *c != 0);
    let b = b.iter().copied().take_while(|c| *c != 0);
    a.cmp(b)
}

pub trait Services {
    /// World+250/+254 definition table, in native order.
    fn objective_definitions(&self) -> Result<&[ObjectiveDef]>;
    /// Kingdom+300 list (growable; the native array capacity is not a limit).
    fn kingdom_objectives_mut(&mut self, kingdom: Object) -> Result<&mut Vec<Entry>>;
    /// 4BE150 on the bound "SortObjectives" game variable.
    fn sort_objectives(&mut self) -> Result<bool>;
    /// 42D4B0(bus, code, kingdom, text).
    fn post_message(&mut self, code: u32, kingdom: Object, text: &[u16]) -> Result<()>;
}

/// 4F1AD0: first definition whose name equals `name`.
pub fn find_definition<S: Services + ?Sized>(s: &S, name: &[u16]) -> Result<Option<usize>> {
    Ok(s
        .objective_definitions()?
        .iter()
        .position(|d| wcscmp(&d.name, name) == Ordering::Equal))
}

/// 4E7050. Returns whether an entry was created.
pub fn gain<S: Services + ?Sized>(s: &mut S, kingdom: Object, name: &[u16]) -> Result<bool> {
    let Some(definition) = find_definition(s, name)? else {
        return Ok(false);
    };
    let entry = Entry {
        definition,
        state: 0,
    };
    let count = s.kingdom_objectives_mut(kingdom)?.len();
    let mut index = count;
    if count != 0 && s.sort_objectives()? {
        let defs = s.objective_definitions()?.to_vec();
        let list = s.kingdom_objectives_mut(kingdom)?;
        index = list
            .iter()
            .position(|e| wcscmp(&defs[definition].text, &defs[e.definition].text) != Ordering::Greater)
            .unwrap_or(list.len());
    }
    s.kingdom_objectives_mut(kingdom)?.insert(index, entry);
    let def = s.objective_definitions()?[definition].clone();
    let code = if def.flag_10 != 0 {
        message::OBJECTIVE_GAINED_FLAGGED
    } else {
        message::OBJECTIVE_GAINED
    };
    s.post_message(code, kingdom, &def.text)?;
    Ok(true)
}

/// 4E2120: remove the first entry whose definition name equals `name`.
pub fn lose<S: Services + ?Sized>(s: &mut S, kingdom: Object, name: &[u16]) -> Result<bool> {
    let defs = s.objective_definitions()?.to_vec();
    let list = s.kingdom_objectives_mut(kingdom)?;
    let Some(index) = list
        .iter()
        .position(|e| wcscmp(&defs[e.definition].name, name) == Ordering::Equal)
    else {
        return Ok(false);
    };
    let def = &defs[list[index].definition];
    let code = if def.flag_10 != 0 {
        message::OBJECTIVE_LOST_FLAGGED
    } else {
        message::OBJECTIVE_LOST
    };
    s.post_message(code, kingdom, &def.text)?;
    s.kingdom_objectives_mut(kingdom)?.remove(index);
    Ok(true)
}

/// 4E21F0 (1), 4E2250 (2), 4E22B0 (0) through 4E2020.
pub fn set_state<S: Services + ?Sized>(
    s: &mut S,
    kingdom: Object,
    name: &[u16],
    state: u32,
) -> Result<bool> {
    let defs = s.objective_definitions()?.to_vec();
    let list = s.kingdom_objectives_mut(kingdom)?;
    let Some(entry) = list
        .iter_mut()
        .find(|e| wcscmp(&defs[e.definition].name, name) == Ordering::Equal)
    else {
        return Ok(false);
    };
    entry.state = state;
    let def = &defs[entry.definition];
    let flagged = def.flag_10 != 0;
    let code = match (state, flagged) {
        (1, true) => message::OBJECTIVE_COMPLETE_FLAGGED,
        (1, false) => message::OBJECTIVE_COMPLETE,
        (2, true) => message::OBJECTIVE_FAILED_FLAGGED,
        (2, false) => message::OBJECTIVE_FAILED,
        (0, true) => message::OBJECTIVE_GAINED_FLAGGED,
        (0, false) => message::OBJECTIVE_GAINED,
        _ => return Err(Error::State("objective state outside native setters".into())),
    };
    s.post_message(code, kingdom, &def.text)?;
    Ok(true)
}

/// 542630/542690: the first entry whose definition is `definition` has `state`.
pub fn has_state<S: Services + ?Sized>(
    s: &mut S,
    kingdom: Object,
    definition: Option<usize>,
    state: u32,
) -> Result<bool> {
    let list = s.kingdom_objectives_mut(kingdom)?;
    Ok(list
        .iter()
        .find(|e| Some(e.definition) == definition)
        .is_some_and(|e| e.state == state))
}

/// Services used by 4E9720 Kingdom::SetMissionResult.
pub trait MissionServices {
    /// Kingdom+19C.
    fn mission_result(&self, kingdom: Object) -> Result<u32>;
    fn set_mission_result(&mut self, kingdom: Object, result: u32) -> Result<()>;
    /// 405370 game-variable lookup; `None` when the variable is missing.
    fn game_var_byte(&mut self, name: &[u16]) -> Result<Option<u8>>;
    /// Kingdom+1EC team and the length of its +44 member list.
    fn team_member_count(&self, kingdom: Object) -> Result<Option<usize>>;
    /// World+48.
    fn world_48(&self) -> Result<u32>;
    /// Kingdom+239 byte.
    fn kingdom_flag_239(&self, kingdom: Object) -> Result<u8>;
    /// 4E9560(kingdom, 0) result text.
    fn kingdom_result_text(&mut self, kingdom: Object, flag: u32) -> Result<Vec<u16>>;
    fn post_mission_message(&mut self, code: u32, kingdom: Object, text: &[u16]) -> Result<()>;
    /// 4F5290 local player's Kingdom.
    fn local_kingdom(&mut self) -> Result<Option<Object>>;
    /// 50D370 on World+130.
    fn ui_mission_ended(&mut self) -> Result<()>;
    /// 4F4DA0 World mission-result processing.
    fn world_mission_result(&mut self, kingdom: Object) -> Result<()>;
}

const TEAM_VICTORY: &[u16] = &[
    b'T' as u16, b'e' as u16, b'a' as u16, b'm' as u16, b'V' as u16, b'i' as u16, b'c' as u16,
    b't' as u16, b'o' as u16, b'r' as u16, b'y' as u16,
];

/// 4E9720(result, notify). Result 1 won, 2 lost. A missing TeamVictory variable
/// makes the original read through the notify argument as a pointer; that
/// path is an explicit error here.
pub fn set_mission_result<S: MissionServices + ?Sized>(
    s: &mut S,
    kingdom: Object,
    result: u32,
    notify: bool,
) -> Result<()> {
    if s.mission_result(kingdom)? != 0 {
        return Ok(());
    }
    s.set_mission_result(kingdom, result)?;
    let team_victory = s.game_var_byte(TEAM_VICTORY)?.ok_or_else(|| {
        Error::State("TeamVictory game variable missing: native dereferences the notify flag".into())
    })?;
    let suppressed = team_victory != 0
        && s
            .team_member_count(kingdom)?
            .is_some_and(|members| members != 1);
    if !suppressed {
        if result == 1 {
            let text = s.kingdom_result_text(kingdom, 0)?;
            s.post_mission_message(message::MISSION_WON, kingdom, &text)?;
        } else if result == 2 && s.world_48()? != 2 && s.kingdom_flag_239(kingdom)? == 0 {
            let text = s.kingdom_result_text(kingdom, 0)?;
            s.post_mission_message(message::MISSION_LOST, kingdom, &text)?;
        }
    }
    if s.local_kingdom()? == Some(kingdom) {
        s.ui_mission_ended()?;
    }
    if notify {
        s.world_mission_result(kingdom)?;
    }
    Ok(())
}

#[cfg(test)]
mod chunk_tests {
    use super::load_chunk;

    fn string(out: &mut Vec<u8>, s: &str) {
        let u: Vec<u16> = s.encode_utf16().collect();
        out.extend((u.len() as u16).to_le_bytes());
        for c in u {
            out.extend(c.to_le_bytes());
        }
    }

    fn chunk() -> Vec<u8> {
        let mut b = Vec::new();
        b.extend(2i32.to_le_bytes());
        // version 0, key, display key, +10 flag
        b.extend(0u32.to_le_bytes());
        string(&mut b, "Primary1");
        string(&mut b, "obj_text");
        b.push(1);
        // an empty display key resolves nothing
        b.extend(0u32.to_le_bytes());
        string(&mut b, "Secondary1");
        string(&mut b, "");
        b.push(0);
        // World+25C
        b.extend(7u32.to_le_bytes());
        b
    }

    #[test]
    fn load_chunk_reads_the_map_objectives() {
        let (defs, word) = load_chunk(&chunk(), |k| {
            assert_eq!(String::from_utf16_lossy(k), "obj_text");
            Ok("Take the bridge".encode_utf16().collect())
        })
        .unwrap();
        assert_eq!(word, 7);
        assert_eq!(defs.len(), 2);
        assert_eq!(String::from_utf16_lossy(&defs[0].name), "Primary1");
        assert_eq!(String::from_utf16_lossy(&defs[0].text), "Take the bridge");
        assert_eq!(defs[0].flag_10, 1);
        assert_eq!(String::from_utf16_lossy(&defs[1].name), "Secondary1");
        assert!(defs[1].text.is_empty());
        assert_eq!(defs[1].flag_10, 0);
    }

    #[test]
    fn load_chunk_refuses_what_the_original_refuses() {
        // a truncated stream and a newer record version are errors, not a
        // partial list
        let whole = chunk();
        assert!(load_chunk(&whole[..whole.len() - 1], |_| Ok(Vec::new())).is_err());
        let mut newer = whole;
        newer[4] = 1;
        assert!(load_chunk(&newer, |_| Ok(Vec::new())).is_err());
        // a negative count reads no record
        let mut none = (-1i32).to_le_bytes().to_vec();
        none.extend(9u32.to_le_bytes());
        assert_eq!(load_chunk(&none, |_| Ok(Vec::new())).unwrap(), (Vec::new(), 9));
    }
}
