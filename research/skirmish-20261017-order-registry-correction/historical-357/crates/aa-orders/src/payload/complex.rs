//! Payloads whose native structure depends on decoded values or game data.
//! Each function follows the native reader instruction order cited in its
//! documentation; the matching writer visits the same fields.
use super::{NameKind, Registry, Ser, ThingCheck, def_list};
use crate::{Error, Result};

pub(super) fn schema(descriptor: usize) -> Option<super::Schema> {
    Some(match descriptor {
        22 | 23 => team_command,
        51 | 70 => faction_use_ability,
        64 | 83 | 91 | 93 => load_session_file,
        94 | 95 => load_session_film,
        84 | 86 => selected_command,
        85 | 87 => actor_command,
        117 | 122 => kkc_set_rmc,
        118 | 123 => rmc_settings_order,
        177 | 196 => editor_create_gthing,
        321 | 322 => set_session_rmc,
        355 | 356 => mg_command,
        127 | 131 => host_set_load_map,
        293 | 296 => editor_replace_trigger,
        _ => return None,
    })
}

/// ActorCommand definitions (0xB3AA58; 39 native command types bound by 0x4DA2F0).
pub const ACTOR_COMMANDS: Registry = Registry(0xb3aa58);
/// Object/actor definitions (0xB39F58), resolved by 0x76A960 with virtual +0x38.
pub const OBJECT_DEFINITIONS: Registry = Registry(0xb39f58);

/// 0x4C2C60 parameter set: one null bit (1 = null); otherwise 0x4C2640
/// definition (0xB3AB00), a signed 32-bit count and that many 0x76A960
/// object references. The native loop runs only for `0 < count`; a negative
/// count is stored as the vector size (0x50B530) and re-serialized unchanged
/// by 0x4C2050 with no entries.
pub fn parameter_set(s: &mut dyn Ser) -> Result<()> {
    if s.uint(1)? != 1 {
        s.def(Registry(0xb3ab00))?;
        let raw = s.uint(32)?;
        let count = if (raw as i32) > 0 { raw } else { 0 };
        let bits = s.def_bits(OBJECT_DEFINITIONS)?;
        s.list_len(count, bits)?;
        for _ in 0..count {
            s.def(OBJECT_DEFINITIONS)?;
        }
    }
    Ok(())
}

/// 0x61EBA0 generic command (writer 0x61E6B0). The eleven-bit mask is read
/// first (0x411FA0, 0x800 possibilities); field order is not bit order.
pub fn command(s: &mut dyn Ser) -> Result<()> {
    let flags = s.bounded(0x800)?;
    let has = |bit: u32| flags >> bit & 1 != 0;
    if has(6) {
        s.def(ACTOR_COMMANDS)?;
    }
    if has(0) {
        s.uint(32)?;
    }
    if has(1) {
        s.def(OBJECT_DEFINITIONS)?;
    }
    if has(2) {
        s.uint(32)?;
        s.uint(32)?;
    }
    if has(3) {
        s.uint(32)?;
        s.uint(32)?;
    }
    if has(4) {
        s.uint(32)?;
    }
    if has(7) {
        s.def(Registry(0xb3ab08))?;
    }
    if has(5) {
        parameter_set(s)?;
    }
    if has(8) {
        s.def(Registry(0xb3aa2c))?;
    }
    if has(9) {
        s.uint(1)?;
    }
    s.uint(1)?;
    Ok(())
}

/// 84/86 KKC_TellSelectedCommandOrder (handlers 0x61FB40/0x61F840).
fn selected_command(s: &mut dyn Ser) -> Result<()> {
    command(s)
}

/// 85/87 KKC_TellActorCommandOrder: command, then 0x76A5A0 actor slot.
fn actor_command(s: &mut dyn Ser) -> Result<()> {
    command(s)?;
    s.thing(ThingCheck::B4)?;
    Ok(())
}

/// 22/23 KKC_TeamCommand reader 0x509090 (writer 0x5091B0). The first field
/// selects a TeamCommand definition by raw array index (no bounds check in
/// the native reader); its flag word +0x24 decides the optional fields.
fn team_command(s: &mut dyn Ser) -> Result<()> {
    let index = s.uint(32)?;
    s.thing(ThingCheck::A8)?;
    if s.uint(1)? == 1 {
        s.thing(ThingCheck::A8)?;
    }
    let flags = s.context().team_command_flags(index)?;
    if flags & 1 != 0 {
        s.uint(32)?;
    }
    if flags >> 1 & 1 != 0 {
        s.uint(32)?;
        s.uint(32)?;
    }
    if flags >> 3 & 1 != 0 {
        s.uint(32)?;
    }
    Ok(())
}

/// 51/70 KKC_FactionUseAbilityOrder reader 0x6109F0: ability definition
/// (0xB3AB10), two raw 32-bit words, presence bit, optional 0x76A5A0 actor.
fn faction_use_ability(s: &mut dyn Ser) -> Result<()> {
    s.def(Registry(0xb3ab10))?;
    s.uint(32)?;
    s.uint(32)?;
    if s.uint(1)? == 1 {
        s.thing(ThingCheck::B4)?;
    }
    Ok(())
}

/// 0x49ACD0: two consecutive CStrings.
fn string_pair(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    s.text()?;
    Ok(())
}

/// Native 32-bit-count vector read: count, then `count` elements.
fn vector(s: &mut dyn Ser, min_bits: usize, mut element: impl FnMut(&mut dyn Ser) -> Result<()>) -> Result<()> {
    let count = s.uint(32)?;
    s.list_len(count, min_bits)?;
    for _ in 0..count {
        element(s)?;
    }
    Ok(())
}

/// 0x511E50 world-rule value: rule definition (0xB3AA88), one bool, then a
/// value whose encoding follows the definition's kind (+0x20): 0 bool,
/// 1/3 int32 (0x40D810), 2 binary32, any other kind nothing. A null rule is
/// dereferenced by the native code; Rust reports it.
fn world_rule(s: &mut dyn Ser) -> Result<()> {
    let registry = Registry(0xb3aa88);
    let index = s.def(registry)?;
    let count = s.context().registry_count(registry)?;
    if index as usize >= count {
        return Err(Error::SchemaContext(
            "0x511E50 dereferences a null WorldRule definition",
        ));
    }
    s.uint(1)?;
    match s.context().definition_kind(registry, index)? {
        0 => {
            s.uint(1)?;
        }
        1..=3 => {
            s.uint(32)?;
        }
        _ => {}
    }
    Ok(())
}

/// 0x512070 world conditions: 0x411FA0(7), condition definition (0xB3AA70),
/// then the 0x511F50 world-rule vector.
fn world_conditions(s: &mut dyn Ser) -> Result<()> {
    s.bounded(7)?;
    s.def(Registry(0xb3aa70))?;
    let bits = s.def_bits(Registry(0xb3aa88))? + 1;
    vector(s, bits, world_rule)
}

/// 0x50FF20 one entry of the 0x5104A0 vector (kingdom/player setup record).
fn header_kingdom(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    string_pair(s)?;
    s.uint(1)?;
    s.text()?;
    s.def(Registry(0xb3aa44))?;
    s.def(Registry(0xb3aa4c))?;
    s.def(Registry(0xb3aa60))?;
    s.def(Registry(0xb3aa68))?;
    s.uint(1)?;
    s.uint(1)?;
    s.text()?;
    s.uint(1)?;
    // 0x50FD70: vector of 0x76A4A0 (= 0x49ACD0 string pairs).
    vector(s, 32, string_pair)?;
    // 0x50F420: definition vector over 0xB3AA2C.
    def_list(s, Registry(0xb3aa2c))?;
    s.text()?;
    // 0x50E050: vector of 0x76A960 object-definition references.
    def_list(s, OBJECT_DEFINITIONS)?;
    s.uint(1)?;
    s.uint(1)?;
    // 0x50E230: vector of raw 32-bit words.
    vector(s, 32, |s| s.uint(32).map(drop))?;
    Ok(())
}

/// 0x5A1E90 game file header (map/save/film header carried by load orders).
pub fn file_header(s: &mut dyn Ser) -> Result<()> {
    string_pair(s)?;
    string_pair(s)?;
    s.uint(32)?; // +0x10
    let version = s.uint(32)?; // +0xC
    // 0x510D90: 0x50FB90 vector of (CString, 0x49ACD0) then 0x5104A0 vector.
    vector(s, 48, |s| {
        s.text()?;
        string_pair(s)
    })?;
    vector(s, 0, header_kingdom)?;
    world_conditions(s)?;
    s.uint(32)?; // +0x30
    s.uint(32)?; // +0x34
    if version > 1 {
        s.text()?; // +0x3C
        s.uint(32)?; // +0x40
        s.text()?; // +0x38
        s.uint(32)?; // +0x18
        s.uint(32)?; // +0x1C
        while s.uint(1)? == 1 {
            s.uint(1)?;
            s.text()?;
            s.text()?;
            s.uint(1)?;
        }
    }
    Ok(())
}

/// 64/83 TGC_SetSessionLoadSavedGameOrder and 91/93 TGC_SetSessionLoadMapOrder
/// (readers 0x614B00/0x60FE30, 0x643760/0x641F00): CString, 0x5A1E90 header, 32-bit word.
fn load_session_file(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    file_header(s)?;
    s.uint(32)?;
    Ok(())
}

/// 94/95 KKC_SetSessionLoadFilmOrder reader 0x644D50.
fn load_session_film(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    file_header(s)?;
    s.uint(32)?;
    s.text()?;
    s.uint(32)?;
    s.uint(1)?;
    Ok(())
}

/// 0x5061C0 vector of (CString, 32-bit word).
fn named_words(s: &mut dyn Ser) -> Result<()> {
    vector(s, 48, |s| {
        s.text()?;
        s.uint(32).map(drop)
    })
}

/// 0x506560 random-map settings: word, two 0x5061C0 vectors, word.
pub fn rmc_settings(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    named_words(s)?;
    named_words(s)?;
    s.uint(32)?;
    Ok(())
}

/// 117/122 KKC_SetRMCOrder reader 0x687340: 0x76A9C0 named reference
/// (CString looked up by 0x41EEC0), 0x506560 settings, 0x512070 conditions,
/// two 32-bit words.
fn kkc_set_rmc(s: &mut dyn Ser) -> Result<()> {
    s.named(NameKind::RandomMapConfig)?;
    rmc_settings(s)?;
    world_conditions(s)?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 118/123 TGC_SetRMCSettingsOrder (decree 0x6845B0 -> 0x5065C0).
fn rmc_settings_order(s: &mut dyn Ser) -> Result<()> {
    rmc_settings(s)
}

/// 321/322 TGC_SetSessionRMCOrder reader 0x75E560: named reference, file
/// header, CString, 32-bit word.
fn set_session_rmc(s: &mut dyn Ser) -> Result<()> {
    s.named(NameKind::RandomMapConfig)?;
    file_header(s)?;
    s.text()?;
    s.uint(32)?;
    Ok(())
}

/// 177/196 KKC_EditorCreateGThing reader 0x6AC6C0.
fn editor_create_gthing(s: &mut dyn Ser) -> Result<()> {
    s.def(OBJECT_DEFINITIONS)?;
    for _ in 0..4 {
        s.uint(32)?;
    }
    s.thing(ThingCheck::A8)?;
    parameter_set(s)?;
    s.text()?;
    s.uint(32)?;
    s.uint(32)?;
    s.uint(32)?;
    Ok(())
}

/// 355/356 KKC_TellMGCommandOrder reader 0x953510 (writer 0x951C20): six-bit
/// mask (0x411FA0, 0x40 possibilities), then in this order: bit3 0x9526A0
/// (0xB4A064), bit0 word, bit1 0x76A960 object definition, bit2 0x959070
/// metagame element named by two CStrings, bit4 0x50E860 (0xB3AA2C), bit5
/// 0x9526F0 definition vector (0xB4A06C).
fn mg_command(s: &mut dyn Ser) -> Result<()> {
    let mask = s.bounded(0x40)?;
    if mask >> 3 & 1 != 0 {
        s.def(Registry(0xb4a064))?;
    }
    if mask & 1 != 0 {
        s.uint(32)?;
    }
    if mask >> 1 & 1 != 0 {
        s.def(OBJECT_DEFINITIONS)?;
    }
    if mask >> 2 & 1 != 0 {
        s.named(NameKind::MetagameContainer)?;
        s.named(NameKind::MetagameElement)?;
    }
    if mask >> 4 & 1 != 0 {
        s.def(Registry(0xb3aa2c))?;
    }
    if mask >> 5 & 1 != 0 {
        def_list(s, Registry(0xb4a06c))?;
    }
    Ok(())
}

/// 127/131 KKC_HostSetLoadMapOrder (readers 0x68D000/0x68AD40): CString then
/// 0x512070 world conditions.
fn host_set_load_map(s: &mut dyn Ser) -> Result<()> {
    s.text()?;
    world_conditions(s)
}

/// 0x5489C0: default parameter type of a trigger parameter whose explicit
/// type word is 0, selected by its first word.
pub fn trigger_default_type(first: u32) -> u32 {
    match first {
        1 => 1,
        2 => 4,
        3 => 6,
        4 => 9,
        5 => 0xc,
        6 => 0x13,
        7 => 0x1a,
        8 => 0x1c,
        9 => 0x1d,
        10 => 0x1e,
        0xb => 0x1f,
        0xc => 0x20,
        0xd => 0x21,
        0xe => 0x23,
        0xf => 0x22,
        0x10 => 0x24,
        0x11 => 0x25,
        0x12 => 0x26,
        0x13..=0x16 => 0x27,
        0x17 => 0x28,
        0x18 => 0x29,
        0x19 => 0x2a,
        0x1a => 0x2b,
        0x1b => 0x2c,
        0x1c => 0x2d,
        0x1d => 0x2f,
        0x1e => 0x30,
        0x1f => 0x31,
        0x20 => 0x32,
        0x21 => 0x33,
        0x22 => 0x34,
        0x23 => 0x35,
        0x24 => 0x36,
        0x25 => 0x37,
        0x26 => 0x39,
        0x27 => 0x3a,
        0x28 => 0x3b,
        0x29 => 0x3c,
        0x2a => 0x3d,
        0x2b => 0x3e,
        0x2c => 0x3f,
        _ => 0,
    }
}

/// 0x548920: value storage kind of a trigger parameter type.
pub fn trigger_value_kind(ty: u32) -> u32 {
    match ty {
        1 | 6 | 0xb..=0xe | 0x10 | 0x11 | 0x13 | 0x17 | 0x18 | 0x1c | 0x1e..=0x20 | 0x24 | 0x28
        | 0x2e | 0x39 => 1,
        2 => 2,
        3 | 0xf | 0x12 | 0x1b | 0x21..=0x23 | 0x25 | 0x27 | 0x29..=0x2d | 0x2f..=0x36 | 0x38
        | 0x3a..=0x3f => 6,
        0x26 => 5,
        0x37 => 7,
        _ => 0,
    }
}

/// 0x6B73D0 one trigger parameter: word (+0x28), type word (+0x2C), bool,
/// then the value by kind (0x548B90 -> 0x548920): 1 int32 (0x40D810),
/// 2 binary32 (0x505A80), 5 three words (0x6AA250), 6 CString, 7 CString
/// pair (0x49ACD0), other kinds nothing.
fn trigger_parameter(s: &mut dyn Ser) -> Result<()> {
    let first = s.uint(32)?;
    let ty = s.uint(32)?;
    s.uint(1)?;
    let ty = if ty == 0 { trigger_default_type(first) } else { ty };
    match trigger_value_kind(ty) {
        1 | 2 => {
            s.uint(32)?;
        }
        5 => {
            for _ in 0..3 {
                s.uint(32)?;
            }
        }
        6 => {
            s.text()?;
        }
        7 => string_pair(s)?,
        _ => {}
    }
    Ok(())
}

/// Signed native loop count (`0 < count`): negative counts read nothing.
fn signed_count(s: &mut dyn Ser, min_bits: usize) -> Result<u32> {
    let raw = s.uint(32)?;
    let count = if (raw as i32) > 0 { raw } else { 0 };
    s.list_len(count, min_bits)?;
    Ok(count)
}

/// 0x6B76A0 trigger condition/action: definition word (raw index into the
/// game-data vector [0xB36530]+0x3C8), signed parameter count, parameters,
/// trailing word.
fn trigger_element(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    for _ in 0..signed_count(s, 65)? {
        trigger_parameter(s)?;
    }
    s.uint(32)?;
    Ok(())
}

/// 293/296 KKC_EditorReplaceTrigger: 0x6B84A0 (stream mode 1 = reading builds
/// a new trigger) -> 0x6B7E80: word, two bools, word, two CStrings, word,
/// signed element count, elements (0x6B76A0).
fn editor_replace_trigger(s: &mut dyn Ser) -> Result<()> {
    s.uint(32)?;
    s.uint(1)?;
    s.uint(1)?;
    s.uint(32)?;
    s.text()?;
    s.text()?;
    s.uint(32)?;
    for _ in 0..signed_count(s, 96)? {
        trigger_element(s)?;
    }
    Ok(())
}
