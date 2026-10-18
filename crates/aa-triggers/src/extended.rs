//! 546AF0 element cases that no campaign mission uses (developer, trailer and
//! custom maps), ported from the ORIGINAL TriggerElement leaves in round 2 and
//! proven by research/phase8-triggers/native-extended.json.
//!
//! Argument registers are evaluated in the native right-to-left order. Every
//! deeper engine call is an [`Effects`] method naming its native address.
use crate::{
    actions::{Command, destroy_command, fld_fstp},
    decode::Element,
    elements::{Bank, Services},
    manager::{Error, Manager, Result},
    program::{CoreProgram, Diagnostic, compare, text},
    selectors,
};
use aa_navigation::startup::Object;
use aa_numeric::Binary;

/// Engine services reached only by these element types.
pub trait Effects {
    /// 4ED820(a, b) on World+4C (sun dimmer, binary32 arguments).
    fn view_adjust_sun_dimmer(&mut self, a: u32, b: u32) -> Result<()>;
    /// 79F5D0(kingdom, sai) on the SAI manager [B36528].
    fn sai_change_profile(&mut self, kingdom: Object, sai: Object) -> Result<()>;
    /// 747630(data) on the audio system [B36548] (music; data may be null).
    fn play_music(&mut self, data: Option<Object>) -> Result<()>;
    /// 50D230(vfx, actor) on World+130.
    fn ui_vfx_on_actor(&mut self, vfx: Object, actor: Object) -> Result<()>;
    /// [[B39AB0]+24] game-variable cell: its +28 flag word.
    fn fidget_var_flags(&self) -> Result<u32>;
    /// 40FCA0(&kind, B43B40) on B39690 (variable change permission, 4B29B0).
    fn var_permission(&mut self, kind: u32) -> Result<bool>;
    /// Byte store into the [[B39AB0]+24] cell.
    fn set_fidget_var(&mut self, value: u8) -> Result<()>;
    /// 76F370(binary32) on [B36554] (game speed).
    fn set_game_speed(&mut self, bits: u32) -> Result<()>;
    /// 4E2A60(data, 0) on Kingdom+1FC: the Kingdom's hero entry for an ActorData.
    fn kingdom_hero_entry(&mut self, kingdom: Object, data: Object) -> Result<Option<Object>>;
    /// Hero entry +18 store (SetExperienceMax).
    fn set_hero_entry_18(&mut self, entry: Object, bits: u32) -> Result<()>;
    /// 579750(value, 4) on an organization: morale modify. The runtime binds
    /// this to aa_combat::morale::modify.
    fn organization_modify_morale(&mut self, organization: Object, value: u32, mode: u32) -> Result<()>;
    /// Actor virtual +24: display name (CString out-parameter).
    fn actor_display_name(&mut self, actor: Object) -> Result<Vec<u16>>;
    /// Area+84 actor list: the GTID word each node references.
    fn area_actor_ids(&self, area: Object) -> Result<Vec<u32>>;
    /// 4D12C0 on an actor (excluded from area teleports when true).
    fn actor_4d12c0(&mut self, actor: Object) -> Result<bool>;
    /// 4F6760(actor, flag) on World (an area member whose teleport failed).
    fn world_4f6760(&mut self, actor: Object, flag: u32) -> Result<()>;
    /// 4EE5C0(&position) on an actor without organization (x, y, z words).
    fn actor_teleport(&mut self, actor: Object, position: [u32; 3]) -> Result<()>;
    /// 4EEB40(x, y) on an organization member.
    fn member_teleport(&mut self, member: Object, position: [u32; 2]) -> Result<()>;
    /// Object+8 store (teleport bit 30 set and cleared around the move).
    fn store_flags8(&mut self, object: Object, value: u32) -> Result<()>;
    /// 4F03C0 on World: camera shakes are skipped while it is true.
    fn world_4f03c0(&mut self) -> Result<bool>;
    /// 76AC60 on the player manager [B36550].
    fn players_76ac60(&mut self) -> Result<Option<Object>>;
    /// 765E00 on the 76AC60 result (called even when that result is null).
    fn player_765e00(&mut self, player: Option<Object>) -> Result<Option<Object>>;
    /// 416370(name) on GameData+584: audio data lookup without warning.
    fn audio_lookup(&mut self, name: &[u16]) -> Result<Option<Object>>;
    /// 6F7810(0x24) + 4E1130(data): a new hero entry (None if allocation fails).
    fn new_hero_entry(&mut self, data: Object) -> Result<Option<Object>>;
    /// 4E4C30(entry) on Kingdom+1FC.
    fn kingdom_add_hero(&mut self, kingdom: Object, entry: Option<Object>) -> Result<()>;
    /// 4E2CB0(entry, 0) on Kingdom+1FC; a nonnull entry is then deleted (6F7430).
    fn kingdom_remove_hero(&mut self, kingdom: Object, entry: Option<Object>) -> Result<()>;
    /// 542950 tile pass: 554DD0/5541F0/554330 iteration on World+30 over the
    /// query {Area+68 shape, (+2C, +30), (+20, +24)} with 54F930 on every
    /// tile. The iterator belongs to the spatial grid (aa-spatial ports it).
    fn fog_restore_tiles(&mut self, area: Object, query: [u32; 4]) -> Result<()>;
    /// GameData+C ActorData bank: +24 entry array, +28 count (native order).
    fn actor_data_bank(&self) -> Result<Vec<Option<Object>>>;
    /// 4F02B0 on World: the World+D4 random generator (logged by 77DA70).
    fn world_rng(&mut self) -> Result<Object>;
    /// 85D840(rng): the next random word.
    fn random(&mut self, rng: Object) -> Result<u32>;
    /// Kingdom+1FC hero table +1C: entry count.
    fn kingdom_hero_count(&self, kingdom: Object) -> Result<u32>;
    /// 4E13F0(index) on Kingdom+1FC: hero entry by index.
    fn kingdom_hero_at(&mut self, kingdom: Object, index: u32) -> Result<Option<Object>>;
}

pub const SUPPORTED: &[u32] = &[
    4, 11, 20, 21, 24, 25, 26, 32, 33, 35, 37, 38, 39, 41, 45, 48, 49, 65, 68, 84, 88, 89, 90, 106, 111, 113,
];

fn numeric(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| Error::State(e.to_string()))
}
fn arith<T>(v: std::result::Result<T, aa_numeric::Error>) -> Result<T> {
    v.map_err(|e| Error::State(e.to_string()))
}
fn log(core: &mut CoreProgram, address: u32) {
    core.diagnostics.push(Diagnostic::NativeLog(address));
}
fn c_str(s: &[u16]) -> &[u16] {
    let end = s.iter().position(|&c| c == 0).unwrap_or(s.len());
    &s[..end]
}

/// fld v; fmul a; fmul [A03078] (0.01f); fstp dword.
fn percent_of(core: &CoreProgram, value: u32, maximum: u32) -> Result<u32> {
    let pc = core.precision;
    arith(
        numeric(value)?
            .mul(&numeric(maximum)?, pc)
            .and_then(|v| v.mul(&Binary::from_f32_bits(0x3c23_d70a)?, pc))
            .and_then(|v| v.store_f32()),
    )
}

/// 5420D0: log base 2.0 (A01B60) of the speed through two FYL2X, divided at
/// the precision control and stored as binary32.
///
/// FYL2X results are extended precision (not affected by PC) and are not
/// correctly rounded by specification; Unicorn computes them through binary64.
/// The binary32 store is therefore decided from a binary64 log2 with a guard
/// band: a result whose exact value lies within 2^-40 (relative) of a binary32
/// rounding boundary is refused instead of guessed.
pub fn game_speed_bits(value: u32) -> Result<u32> {
    let v = f64::from(f32::from_bits(value));
    if !v.is_finite() || v <= 0.0 {
        return Err(Error::State("5420D0 log of a non-positive or non-finite speed".into()));
    }
    let r = v.log2();
    if r == 0.0 {
        return Ok(0);
    }
    let f = r as f32;
    // Rounding midpoints between f and its two binary32 neighbours.
    let step = |bits: u32, up: bool| -> f32 {
        let positive = bits >> 31 == 0;
        f32::from_bits(match (bits, up) {
            (0, false) => 0x8000_0001,
            (0x8000_0000, true) => 1,
            (b, true) if positive => b + 1,
            (b, true) => b - 1,
            (b, false) if positive => b - 1,
            (b, false) => b + 1,
        })
    };
    for neighbour in [step(f.to_bits(), true), step(f.to_bits(), false)] {
        let boundary = (f64::from(f) + f64::from(neighbour)) / 2.0;
        if (r - boundary).abs() <= r.abs() * (2f64).powi(-40) {
            return Err(Error::State(
                "game speed log2 lies within x87 FYL2X precision of a binary32 rounding boundary".into(),
            ));
        }
    }
    Ok(f.to_bits())
}

fn add(core: &CoreProgram, a: u32, b: u32) -> Result<Binary> {
    arith(numeric(a)?.add(&numeric(b)?, core.precision))
}
fn sub(core: &CoreProgram, a: u32, b: u32) -> Result<Binary> {
    arith(numeric(a)?.sub(&numeric(b)?, core.precision))
}
fn f32_of(v: Binary) -> Result<u32> {
    arith(v.store_f32())
}

/// 5426F0 EffectActorTeleport: 554B30 placement of the actor (radius 8.0f),
/// then either 4EE5C0 for a lone actor or 4EEB40 for every organization
/// member at its formation offset (+78), each placed again; bit 30 of +8 is
/// set around every move. `position` is the location register (x, y, z).
fn teleport<S: Services + ?Sized>(core: &CoreProgram, s: &mut S, actor: Object, position: [u32; 3]) -> Result<bool> {
    let mut place = [fld_fstp(position[0]), fld_fstp(position[1])];
    let mut facing = [s.data_word(actor, 0x2c)?, s.data_word(actor, 0x30)?];
    let data = s
        .data_ref(actor, 4)?
        .ok_or_else(|| Error::State("teleported actor without definition (+4)".into()))?;
    if !s.find_placement(&mut place, &mut facing, data, 0x4100_0000, 0)? {
        return Ok(false);
    }
    // The placed x/y are written back into the location register.
    let position = [place[0], place[1], position[2]];
    const MOVING: u32 = 0x4000_0000;
    let Some(org) = s.data_ref(actor, 0x7c)? else {
        let flags = s.flags8(actor)?;
        s.store_flags8(actor, flags | MOVING)?;
        s.actor_teleport(actor, position)?;
        let flags = s.flags8(actor)?;
        s.store_flags8(actor, flags & !MOVING)?;
        return Ok(true);
    };
    let count = s.data_word(org, 0x2c)? as i32;
    for i in 0..count.max(0) as u32 {
        let members = s
            .data_ref(org, 0x28)?
            .ok_or_else(|| Error::State("organization member array (+28) is null: native faults".into()))?;
        let Some(member) = s.data_ref(members, 4 * i)? else { continue };
        let offsets = s
            .data_ref(org, 0x78)?
            .ok_or_else(|| Error::State("organization offset array (+78) is null: native faults".into()))?;
        let x = f32_of(add(core, s.data_word(offsets, 8 * i)?, place[0])?)?;
        let y = f32_of(add(core, s.data_word(offsets, 8 * i + 4)?, place[1])?)?;
        let mut target = [x, y];
        let mut facing = [s.data_word(member, 0x2c)?, s.data_word(member, 0x30)?];
        let data = s
            .data_ref(member, 4)?
            .ok_or_else(|| Error::State("organization member without definition (+4)".into()))?;
        if !s.find_placement(&mut target, &mut facing, data, 0x4100_0000, 0)? {
            return Ok(false);
        }
        let flags = s.flags8(member)?;
        s.store_flags8(member, flags | MOVING)?;
        let leader = s.data_ref(org, 0x40)? == Some(member);
        if leader {
            let flags = s.flags8(actor)?;
            s.store_flags8(actor, flags | MOVING)?;
        }
        s.member_teleport(member, target)?;
        if leader {
            let flags = s.flags8(actor)?;
            s.store_flags8(actor, flags & !MOVING)?;
        }
        let flags = s.flags8(member)?;
        s.store_flags8(member, flags & !MOVING)?;
    }
    Ok(true)
}

/// GameData+584 audio lookup of a string game variable (4F96D0 / 546A30).
fn music_variable<S: Services + ?Sized>(core: &mut CoreProgram, s: &mut S, variable: &str) -> Result<Option<Object>> {
    let name = s.game_var_string(&variable.encode_utf16().collect::<Vec<_>>())?;
    let name = c_str(&name).to_vec();
    if name.is_empty() {
        return Ok(None);
    }
    let data = s.audio_lookup(&name)?;
    if data.is_none() {
        log(core, 0x410c40);
    }
    Ok(data)
}

/// 54E910 on register `index`: ActorData lookup of the register's +4 string
/// (the element's own text, or the text an earlier element left in that
/// register for a non-text item such as mi_hero_random), then virtual +38.
fn hero_data<S: Services + ?Sized>(manager: &Manager, index: usize, s: &mut S) -> Result<Option<Object>> {
    let name = manager.register_text(index)?.to_vec();
    match s.data_by_ids(Bank::ActorData, &name)? {
        Some(d) if s.is_actor_data(d)? => Ok(Some(d)),
        _ => Ok(None),
    }
}

/// ActorData +174 bits 17 and 8 and byte +2E4: eligible as a random hero.
fn hero_eligible<S: Services + ?Sized>(s: &S, data: Object) -> Result<bool> {
    let flags = s.data_word(data, 0x174)?;
    Ok((flags >> 17) & 1 != 0 && (flags >> 8) & 1 != 0 && s.data_word(data, 0x2e4)? & 0xff != 0)
}

/// 4F02B0 + 85D840, then the unsigned remainder by the candidate count.
fn pick<S: Services + ?Sized, T: Copy>(s: &mut S, candidates: &[T]) -> Result<T> {
    let rng = s.world_rng()?;
    let r = s.random(rng)?;
    Ok(candidates[(r % candidates.len() as u32) as usize])
}

/// 4B29B0: may the game variable cell be changed?
fn var_writable<S: Services + ?Sized>(s: &mut S, flags: u32) -> Result<bool> {
    if flags & 2 != 0 {
        return Ok(false);
    }
    if flags & 5 == 5 && !s.var_permission(2)? {
        return Ok(false);
    }
    if flags & 9 == 9 && !s.var_permission(1)? {
        return Ok(false);
    }
    Ok(true)
}

pub fn evaluate<S: Services + ?Sized>(
    core: &mut CoreProgram,
    manager: &mut Manager,
    element: &Element,
    s: &mut S,
) -> Result<bool> {
    let kind = selectors::kind(element)?;
    let p = &element.parameters;
    Ok(match element.opcode {
        // 54C2F0: 54C240 wcscmp search of manager+40.
        4 => {
            let name = selectors::string(&p[0])?;
            let name = c_str(&name);
            manager.strings().iter().any(|s| c_str(s) == name)
        }
        11 => {
            let value = core.value(manager, &p[2])?;
            let comparison = selectors::comparison(&p[1])?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            // 542580: organization +5C morale against value * +60 * 0.01f.
            match actor.map(|a| s.organization(a)).transpose()?.flatten() {
                Some(org) => {
                    let product = percent_of(core, value, s.data_word(org, 0x60)?)?;
                    compare(s.data_word(org, 0x5c)?, comparison, product)?
                }
                None => false,
            }
        }
        20 => {
            let formation = s.data_by_ids(Bank::Formation, text(&p[1])?)?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            // 541A60: organization +74 -> +C is the formation data.
            match (actor, formation) {
                (Some(actor), Some(formation)) => match s.organization(actor)? {
                    Some(org) => {
                        let current = match s.data_ref(org, 0x74)? {
                            Some(f) => s.data_ref(f, 0xc)?,
                            None => None,
                        };
                        current == Some(formation)
                    }
                    None => false,
                },
                _ => false,
            }
        }
        21 => {
            let value = core.value(manager, &p[2])?;
            let comparison = selectors::comparison(&p[1])?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            // 542BD0: first resource with +18 == 0, Kingdom+1C4[index]
            // + 0.5f, floor (96D5E0), binary32.
            let Some(kingdom) = kingdom else { return Ok(false) };
            let Some(index) = s.resource_flags18()?.iter().position(|&f| f == 0) else {
                return Ok(false);
            };
            let table = s
                .data_ref(kingdom, 0x1c4)?
                .ok_or_else(|| Error::State("Kingdom+1C4 resource array is null: native faults".into()))?;
            let amount = s.data_word(table, 4 * index as u32)?;
            let sum = crate::crt::x87_add_f32_to_f64(amount, 0x3f00_0000, core.precision)?;
            let floored = f64::from_bits(sum).floor().to_bits();
            let left = arith(arith(Binary::from_f64_bits(floored))?.store_f32())?;
            compare(left, comparison, value)?
        }
        24 => match selectors::kingdom(manager, kind, &p[0], s)? {
            // 542340: Kingdom+2D4 (settlement count) <= 0.
            Some(kingdom) => (s.data_word(kingdom, 0x2d4)? as i32) <= 0,
            None => false,
        },
        26 => {
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let data = hero_data(manager, 0, s)?;
            // 542360: the hero entry's +10 GTID does not resolve (401060).
            match (data, kingdom) {
                (Some(data), Some(kingdom)) => match s.kingdom_hero_entry(kingdom, data)? {
                    Some(entry) => {
                        let id = s.data_word(entry, 0x10)?;
                        s.resolve_id(id)?.is_none()
                    }
                    None => false,
                },
                _ => false,
            }
        }
        35 => {
            let value = core.value(manager, &p[2])?;
            let data = hero_data(manager, 1, s)?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            // 541C20: item 27 (specific hero) sets entry+18; item 28 warns.
            let Some(kingdom) = kingdom else { return Ok(false) };
            match p[1].item_index {
                27 => match data {
                    Some(data) => match s.kingdom_hero_entry(kingdom, data)? {
                        Some(entry) => {
                            s.set_hero_entry_18(entry, value)?;
                            true
                        }
                        None => {
                            log(core, 0x404ed0);
                            false
                        }
                    },
                    None => false,
                },
                28 => {
                    log(core, 0x404ed0);
                    false
                }
                _ => false,
            }
        }
        37 => {
            let value = core.value(manager, &p[1])?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            // 543800: fucompp with 0.0 (NaN proceeds); 4CE6C0 unit owner.
            let Some(actor) = actor else { return Ok(false) };
            if value & 0x7fff_ffff == 0 {
                return Ok(false);
            }
            let unit = if s.data_ref(actor, 0x7c)?.is_some() {
                actor
            } else {
                match s.data_ref(actor, 0x80)? {
                    Some(group) => s.data_ref(group, 0x10)?.unwrap_or(actor),
                    None => actor,
                }
            };
            match s.data_ref(unit, 0x7c)? {
                Some(org) => {
                    s.organization_modify_morale(org, value, 4)?;
                    true
                }
                None => {
                    s.actor_display_name(actor)?;
                    log(core, 0x404ed0);
                    false
                }
            }
        }
        41 => {
            let value = selectors::boolean(&p[0])?;
            // 5432A0: 4B29B0 then a byte store; always true.
            let flags = s.fidget_var_flags()?;
            if var_writable(s, flags)? {
                s.set_fidget_var(u8::from(value))?;
            }
            true
        }
        45 => {
            let b = core.value(manager, &p[1])?;
            let a = core.value(manager, &p[0])?;
            s.view_adjust_sun_dimmer(a, b)?;
            true
        }
        65 => {
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let sai = s.data_by_ids(Bank::Sai, text(&p[0])?)?;
            match (sai, kingdom) {
                (Some(sai), Some(kingdom)) => {
                    s.sai_change_profile(kingdom, sai)?;
                    true
                }
                _ => false,
            }
        }
        68 => {
            let value = core.value(manager, &p[0])?;
            // fcomp with 0.0: only v > 0 (or unordered) reaches the FYL2X path.
            let nan = (value >> 23) & 0xff == 0xff && value & 0x7f_ffff != 0;
            if !(nan || (value >> 31 == 0 && value != 0)) {
                return Ok(false);
            }
            s.set_game_speed(game_speed_bits(value)?)?;
            true
        }
        84 => {
            let actor = selectors::actor(manager, kind, &p[1], s)?;
            let vfx = match s.data_by_ids(Bank::ActorData, text(&p[0])?)? {
                Some(d) if s.is_vfx(d)? => Some(d),
                _ => None,
            };
            match (vfx, actor) {
                (Some(vfx), Some(actor)) => {
                    s.ui_vfx_on_actor(vfx, actor)?;
                    true
                }
                _ => false,
            }
        }
        88 => {
            let data = s.data_by_ids(Bank::Audio, text(&p[0])?)?;
            s.play_music(data)?;
            true
        }
        25 => {
            let actor = selectors::actor(manager, kind, &p[1], s)?;
            let data = hero_data(manager, 0, s)?;
            // 543550: any organization member (4364C0, flag 0) whose +4 is the data.
            let (Some(data), Some(actor)) = (data, actor) else { return Ok(false) };
            let Some(org) = s.organization(actor)? else { return Ok(false) };
            let count = s.data_word(org, 0x2c)? as i32;
            let mut found = false;
            for i in 0..count.max(0) as u32 {
                let members = s
                    .data_ref(org, 0x28)?
                    .ok_or_else(|| Error::State("organization member array (+28) is null: native faults".into()))?;
                if let Some(member) = s.data_ref(members, 4 * i)?
                    && s.data_ref(member, 4)? == Some(data)
                {
                    found = true;
                    break;
                }
            }
            found
        }
        32 | 33 => {
            let location = selectors::location(&p[2])?;
            let area = selectors::area(&p[1], s)?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            let (Some(kingdom), Some(area), Some(location)) = (kingdom, area, location) else {
                return Ok(false);
            };
            if element.opcode == 32 {
                // 5436D0: one move command (4CA760, zero vector B279C0) for
                // every Area+84 actor owned by the Kingdom.
                let mut command = Command::new(s.command_definition(5)?);
                command.flags |= 4 | 8 | 0x10;
                command.location = [fld_fstp(location[0]), fld_fstp(location[1])];
                for id in s.area_actor_ids(area)? {
                    let actor = s
                        .resolve_id(id)?
                        .ok_or_else(|| Error::State("stale Area+84 GTID: native calls through null".into()))?;
                    if s.owner(actor)? == Some(kingdom) {
                        if (s.flags8(actor)? >> 6) & 1 != 0 {
                            s.actor_4d4c60(actor, 0)?;
                        }
                        s.process_command(actor, &mut command)?;
                    }
                }
                destroy_command(s, command)?;
            } else {
                // 5430B0: relative teleport of every eligible Area+84 actor.
                let center = [s.data_word(area, 0x20)?, s.data_word(area, 0x24)?, s.data_word(area, 0x28)?];
                for id in s.area_actor_ids(area)? {
                    let Some(actor) = s.resolve_id(id)? else { continue };
                    if s.owner(actor)? != Some(kingdom) || s.actor_4d12c0(actor)? {
                        continue;
                    }
                    if let Some(group) = s.data_ref(actor, 0x80)?
                        && s.data_ref(group, 0x10)?.is_some()
                    {
                        continue;
                    }
                    let a = [s.data_word(actor, 0x20)?, s.data_word(actor, 0x24)?, s.data_word(actor, 0x28)?];
                    let dx = sub(core, a[0], center[0])?;
                    let dy = sub(core, a[1], center[1])?;
                    let dz = f32_of(sub(core, a[2], center[2])?)?;
                    let target = [
                        f32_of(arith(dx.add(&numeric(location[0])?, core.precision))?)?,
                        f32_of(arith(dy.add(&numeric(location[1])?, core.precision))?)?,
                        f32_of(add(core, dz, location[2])?)?,
                    ];
                    if !teleport(core, s, actor, target)? {
                        s.world_4f6760(actor, 1)?;
                    }
                }
            }
            true
        }
        38 => {
            let location = selectors::location(&p[1])?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            // 5431D0: guard command 542A30 (4D9E20 ordinal 2, flags 4|8).
            let (Some(actor), Some(location)) = (actor, location) else { return Ok(false) };
            if (s.flags8(actor)? >> 6) & 1 != 0 {
                s.actor_4d4c60(actor, 0)?;
            }
            let mut command = Command::new(s.command_definition(2)?);
            command.flags |= 4 | 8;
            command.location = [fld_fstp(location[0]), fld_fstp(location[1])];
            let result = s.process_command(actor, &mut command)?;
            destroy_command(s, command)?;
            result
        }
        39 => {
            let location = selectors::location(&p[1])?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            match (actor, location) {
                (Some(actor), Some(location)) => teleport(core, s, actor, location)?,
                _ => false,
            }
        }
        48 | 49 => {
            let data = hero_data(manager, 1, s)?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            // 544CF0 / 544ED0 by the hero item index (+60 of the element).
            let Some(kingdom) = kingdom else { return Ok(false) };
            match p[1].item_index {
                27 => {
                    let Some(data) = data else { return Ok(false) };
                    let existing = s.kingdom_hero_entry(kingdom, data)?;
                    if element.opcode == 48 {
                        if existing.is_some() {
                            return Ok(false);
                        }
                        let entry = s.new_hero_entry(data)?;
                        s.kingdom_add_hero(kingdom, entry)?;
                    } else {
                        let Some(entry) = existing else { return Ok(false) };
                        if s.data_word(entry, 8)? & 0xff == 0
                            && (s.data_word(entry, 0xc)? != 0 || s.data_word(entry, 0x10)? != 0)
                        {
                            return Ok(false);
                        }
                        s.kingdom_remove_hero(kingdom, Some(entry))?;
                    }
                    true
                }
                28 if element.opcode == 48 => {
                    // 544D48: ActorData bank entries that are heroes (+38,
                    // +174 bits 17/8, +2E4) the Kingdom does not have yet.
                    let mut candidates = Vec::new();
                    for data in s.actor_data_bank()? {
                        let data = data.ok_or_else(|| {
                            Error::State("null ActorData bank entry: native calls virtual +38 through null".into())
                        })?;
                        if s.is_actor_data(data)?
                            && hero_eligible(s, data)?
                            && s.kingdom_hero_entry(kingdom, data)?.is_none()
                        {
                            candidates.push(data);
                        }
                    }
                    if candidates.is_empty() {
                        return Ok(false);
                    }
                    let chosen = pick(s, &candidates)?;
                    let entry = s.new_hero_entry(chosen)?;
                    s.kingdom_add_hero(kingdom, entry)?;
                    true
                }
                28 => {
                    // 544F28: the Kingdom's removable heroes (4E13F0 entries
                    // with eligible data; a held entry only when +8 is set
                    // or +C and +10 are both zero).
                    let count = s.kingdom_hero_count(kingdom)? as i32;
                    let mut candidates = Vec::new();
                    for i in 0..count.max(0) as u32 {
                        let Some(entry) = s.kingdom_hero_at(kingdom, i)? else { continue };
                        let Some(data) = s.data_ref(entry, 4)? else { continue };
                        if !hero_eligible(s, data)? {
                            continue;
                        }
                        if s.data_word(entry, 8)? & 0xff == 0
                            && (s.data_word(entry, 0xc)? != 0 || s.data_word(entry, 0x10)? != 0)
                        {
                            continue;
                        }
                        candidates.push(entry);
                    }
                    if candidates.is_empty() {
                        return Ok(false);
                    }
                    let chosen = pick(s, &candidates)?;
                    s.kingdom_remove_hero(kingdom, Some(chosen))?;
                    true
                }
                _ => {
                    if element.opcode == 48 {
                        s.kingdom_add_hero(kingdom, None)?;
                    } else {
                        s.kingdom_remove_hero(kingdom, None)?;
                    }
                    true
                }
            }
        }
        89 => {
            // 4F96D0: the local player's music (+234 -> +38), else the
            // ObserverGameMusic variable; the case always succeeds.
            let player = s.players_76ac60()?;
            let player = s.player_765e00(player)?;
            let music = match player {
                Some(player) => match s.data_ref(player, 0x234)? {
                    Some(set) => s.data_ref(set, 0x38)?,
                    None => return Ok(true),
                },
                None => music_variable(core, s, "ObserverGameMusic")?,
            };
            if music.is_some() {
                s.play_music(music)?;
            }
            true
        }
        90 => {
            // 546A30: AudioMusicSilent, played even when the lookup fails.
            let music = music_variable(core, s, "AudioMusicSilent")?;
            s.play_music(music)?;
            true
        }
        106 => {
            let actor = selectors::actor(manager, kind, &p[1], s)?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            // 544940: "CameraFollow %d %s" (actor +14, Kingdom name) via 50D120.
            let (Some(kingdom), Some(actor)) = (kingdom, actor) else { return Ok(false) };
            let name = s.object_name(kingdom)?;
            let id = s.raw_id(actor)?;
            let mut command: Vec<u16> =
                format!("CameraFollow {} ", crate::crt::format_d(id as i32)).encode_utf16().collect();
            command.extend_from_slice(c_str(&name));
            s.ui_script_command(&command)?;
            true
        }
        111 => {
            // 542950: area rectangle (+20/+24, +2C/+30 copied through x87).
            match selectors::area(&p[0], s)? {
                Some(area) => {
                    let q = [
                        fld_fstp(s.data_word(area, 0x2c)?),
                        fld_fstp(s.data_word(area, 0x30)?),
                        fld_fstp(s.data_word(area, 0x20)?),
                        fld_fstp(s.data_word(area, 0x24)?),
                    ];
                    s.fog_restore_tiles(area, q)?;
                    true
                }
                None => false,
            }
        }
        113 => {
            let value = core.value(manager, &p[1])?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            // 545080: skipped (logged) while 4F03C0; else "ShakeCamera %f %s".
            let Some(kingdom) = kingdom else { return Ok(false) };
            if s.world_4f03c0()? {
                // 404F60 info log ("SKIPPING camera shake in cutscene mode"):
                // info logs are not diagnostics (phase7 harness convention).
                return Ok(true);
            }
            let name = s.object_name(kingdom)?;
            let wide = f64::from(f32::from_bits(fld_fstp(value))).to_bits();
            let mut command: Vec<u16> =
                format!("ShakeCamera {} ", crate::crt::format_f(wide, 6)?).encode_utf16().collect();
            command.extend_from_slice(c_str(&name));
            s.ui_script_command(&command)?;
            true
        }
        _ => return Err(Error::State(format!("opcode {} is not an extended element", element.opcode))),
    })
}

