//! Actual 546AF0 action cases (TriggerElement Effect* leaves) ported in phase8:
//! camera moves (544540/544780 -> "MoveCamera" UI command text), cutscene mode,
//! technology, banner/chat/UI, audio, VFX, weather, counter, resources,
//! actor commands (542EB0/5435D0) and named actor creation (5452A0).
//!
//! Every deeper engine call is an `Effects` method naming its native address.
//! Implementations must execute that behaviour or return an explicit error.
use crate::{
    crt,
    decode::{Element, Parameter, Value},
    elements::{Bank, Services},
    manager::{Error, Manager, Result},
    program::{CoreProgram, Diagnostic, text},
    selectors,
};
use aa_navigation::startup::Object;
use aa_numeric::Binary;
use serde::{Deserialize, Serialize};

/// Fields of the 0x40-byte command built on the stack by 542EB0/5435D0
/// (4D9DC0/4D9E20 + setters 4D9AB0/4D9BF0/4CA760) and processed by 4D9B60.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Command {
    /// +8 command definition (bank lookup or registry ordinal).
    pub definition: Option<Object>,
    /// +C flags: 0x40 definition, 1 target, 2 data, 4 location, 8|0x10 move.
    pub flags: u32,
    /// +10 target GTID.
    pub target_id: u32,
    /// +14 ActorData.
    pub data: Option<Object>,
    /// +18/+1C binary32 location x/y.
    pub location: [u32; 2],
    /// +20/+24 (B279C0/B279C4 words) and +28 speed for the move command.
    pub move_words: [u32; 3],
    /// +34 shared reference released by the 4CA2D0 destructor (set only by
    /// command processing; never by the trigger code itself).
    pub shared34: Option<u64>,
}
impl Command {
    /// 4D9DC0 / 4D9E20: zero fields, +8 definition, flag 0x40 when nonnull.
    pub fn new(definition: Option<Object>) -> Self {
        Self {
            definition,
            flags: if definition.is_some() { 0x40 } else { 0 },
            target_id: 0,
            data: None,
            location: [0; 2],
            move_words: [0; 3],
            shared34: None,
        }
    }
}

/// 4C8C70 spawn request built by 5452A0 (fields the trigger writes; every other
/// field keeps the constructor value recorded by the oracle).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpawnRequest {
    /// +4 ActorData.
    pub definition: Object,
    /// +8/+C/+10 position (z = 0).
    pub position: [u32; 3],
    /// +14/+18 facing from the placement search.
    pub facing: [u32; 2],
    /// +1C name.
    pub name: Vec<u16>,
    /// +28 Kingdom.
    pub kingdom: Object,
}

/// 4C8300(5) ability request built by 544040: +0 type 5, +C Kingdom,
/// +10..+18 from, +20..+28 at, every other word 0.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityRequest {
    pub kingdom: Object,
    pub from: [u32; 3],
    pub at: [u32; 3],
}

/// Camera fields read by 544540: +20/+24/+28 look-at, +60 zoom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CameraFields {
    pub look_at: [u32; 3],
    pub zoom: u32,
}

pub trait Effects {
    /// [[B39940+104]] invalid-ID warning setting.
    fn warn_invalid_ids(&self) -> Result<bool>;
    /// [[B39940+25C]] camera debug-log setting.
    fn camera_debug_log(&self) -> Result<bool>;
    /// World+E0 game time.
    fn world_time_bits(&self) -> Result<u32>;
    /// 4BEDF0: object+18 name.
    fn object_name(&self, object: Object) -> Result<Vec<u16>>;
    fn camera_fields(&self, camera: Object) -> Result<CameraFields>;
    /// 4DBD80 camera pitch (binary32 result).
    fn camera_pitch(&mut self, camera: Object) -> Result<u32>;
    /// 4DBD40 camera yaw (binary32 result).
    fn camera_yaw(&mut self, camera: Object) -> Result<u32>;
    /// 59ABA0 on [B396E0]: milliseconds of the performance clock.
    fn real_time_ms(&mut self) -> Result<u32>;
    /// *(*(B39990+offset)) binary32 camera default variables.
    fn camera_var(&self, offset: u32) -> Result<u32>;
    /// 50D120 on World+130: execute a UI script command.
    fn ui_script_command(&mut self, command: &[u16]) -> Result<()>;
    /// 4F1080 World::SetCutsceneMode.
    fn world_set_cutscene_mode(&mut self, enabled: bool) -> Result<()>;
    /// 6FE710 / 700B10 / 700A90 on *[B40048].
    fn camera_6fe710(&mut self) -> Result<()>;
    fn camera_700b10(&mut self) -> Result<u32>;
    fn camera_700a90(&mut self, value: u32) -> Result<()>;
    /// 4E3AD0 Kingdom::AddProperty(property, kind, a, notify).
    fn kingdom_add_property(&mut self, kingdom: Object, property: Object, kind: u32, a: u32, notify: u32) -> Result<()>;
    /// 4E23D0 Kingdom remove property(property, 0).
    fn kingdom_remove_property(&mut self, kingdom: Object, property: Object, a: u32) -> Result<()>;
    /// 4CEF70 actor remove property(property, 0).
    fn actor_remove_property(&mut self, actor: Object, property: Object, a: u32) -> Result<()>;
    /// World+154/+158 Kingdom slot array, in native order.
    fn world_kingdom_slots(&self) -> Result<Vec<Option<Object>>>;
    /// 50CD00 on World+130 (resolved localized text, duration).
    fn ui_display_banner(&mut self, text: &[u16], duration: u32) -> Result<()>;
    /// Sun data +18/+24/+30 colour triples.
    fn sun_colors(&self, sun: Object) -> Result<[[u32; 3]; 3]>;
    /// 4EDB10 on World+4C.
    fn view_set_sun_colors(&mut self, colors: &[[u32; 3]; 3], duration: u32) -> Result<()>;
    /// GameData+34/+38 resource table: each entry's +18 word.
    fn resource_flags18(&self) -> Result<Vec<u32>>;
    /// 4E2080 Kingdom add resource(index, amount).
    fn kingdom_add_resource(&mut self, kingdom: Object, index: u32, amount: u32) -> Result<()>;
    /// 4F1340 World show counter({start, duration}).
    fn world_show_counter(&mut self, start: u32, duration: u32) -> Result<()>;
    /// 4F1390 World hide counter.
    fn world_hide_counter(&mut self) -> Result<()>;
    /// World+290 weather write.
    fn set_weather(&mut self, weather: Option<Object>) -> Result<()>;
    /// 7474F0(data, 0, 0, 0, 0, location, 0, 0) on [B36548].
    fn play_sound(&mut self, data: Option<Object>, location: Option<[u32; 3]>) -> Result<()>;
    /// 50C3E0 on World+130.
    fn ui_flash_actor(&mut self, actor: Object) -> Result<()>;
    /// [B3653C]+74 UI root.
    fn ui_root(&self) -> Result<Option<Object>>;
    /// UI root virtual +28 (find widget by name).
    fn ui_find_widget(&mut self, root: Object, name: &[u16]) -> Result<Option<Object>>;
    /// Widget+4 name string.
    fn widget_name(&self, widget: Object) -> Result<Vec<u16>>;
    /// 4F8C30 / 4F5E70 World script listen / unlisten.
    fn world_script_listen(&mut self, name: &[u16]) -> Result<()>;
    fn world_script_unlisten(&mut self, name: &[u16]) -> Result<bool>;
    /// 4CE770 actor gain experience(amount, 0, 0).
    fn actor_gain_experience(&mut self, actor: Object, amount: u32) -> Result<()>;
    /// ActorData virtual +48 (VFX check used by 54E8A0).
    fn is_vfx(&mut self, data: Object) -> Result<bool>;
    /// [B39990+64] -> +4/+8 default VFX orientation words.
    fn default_vfx_orientation(&self) -> Result<[u32; 2]>;
    /// 4454A0(vector, angle radians): table-quantized 2D rotation in place.
    fn rotate_2d(&mut self, vector: [u32; 2], angle: u32) -> Result<[u32; 2]>;
    /// 50D280(vfx, position, orientation).
    fn spawn_vfx(&mut self, vfx: Object, position: [u32; 3], orientation: [u32; 3]) -> Result<()>;
    /// 40BC50(B399B0, 0xF, text): returns the native EAX.
    fn chat_message_all(&mut self, text: &[u16]) -> Result<u32>;
    /// 40BF70(B399B0, 0xF, kingdom, 0, text).
    fn chat_message_kingdom(&mut self, kingdom: Object, text: &[u16]) -> Result<()>;
    /// Actor+4 definition, +1C0 count, +1BC[count-1] animation set.
    fn animation_set(&self, actor: Object) -> Result<Option<Object>>;
    /// 45EBB0 on the animation set.
    fn animation_by_name(&mut self, set: Object, name: &[u16]) -> Result<Option<Object>>;
    /// 4CE730 actor play animation.
    fn actor_play_animation(&mut self, actor: Object, animation: Object) -> Result<bool>;
    /// 4D4C60(flag) on an actor whose flag word bit 6 is set.
    fn actor_4d4c60(&mut self, actor: Object, flag: u8) -> Result<()>;
    /// 4D9B60(actor) on the command (may update the command, e.g. +34).
    fn process_command(&mut self, actor: Object, command: &mut Command) -> Result<bool>;
    /// 4CA2D0 destructor: release of a nonnull +34 shared reference.
    fn release_command_shared(&mut self, shared: u64) -> Result<()>;
    /// World+148/+14C slot array, in native order.
    fn world_slots_148(&self) -> Result<Vec<Option<Object>>>;
    /// Kingdom+284 member list head, node value and next (reread after effects).
    fn member_head(&self, kingdom: Object) -> Result<Option<u64>>;
    fn member_value(&self, node: u64) -> Result<Option<Object>>;
    fn member_next(&self, node: u64) -> Result<Option<u64>>;
    /// Actor+80 nonnull and its +10 nonnull.
    fn actor_80_ready(&self, actor: Object) -> Result<bool>;
    /// 5ADC90 on [B36540]: video path for a cutscene name.
    fn video_path(&mut self, name: &[u16]) -> Result<Vec<u16>>;
    /// 410CB0 on B39988: string game variable.
    fn game_var_string(&mut self, name: &[u16]) -> Result<Vec<u16>>;
    /// GameData+298 command definition registry.
    fn command_definition(&self, ordinal: u32) -> Result<Option<Object>>;
    /// Actor+6C -> +28, else actor+7C -> +D8 movement factor.
    fn move_factor(&self, actor: Object) -> Result<Option<u32>>;
    /// 554B30 on World+30 (position, facing, data, radius, mode, 0); may
    /// rewrite position/facing. Mode is 0 for 5452A0 and 1 for 543BA0.
    fn find_placement(&mut self, position: &mut [u32; 2], facing: &mut [u32; 2], data: Object, radius: u32, mode: u32) -> Result<bool>;
    /// Raw word / object reference of a data or actor field (+184, +1C, +414, +2F0, +C0).
    fn data_word(&self, object: Object, offset: u32) -> Result<u32>;
    fn data_ref(&self, object: Object, offset: u32) -> Result<Option<Object>>;
    /// 58B5E0(out, a.x, a.y, b.x, b.y): transport approach point.
    fn transport_path(&mut self, from: [u32; 2], to: [u32; 2]) -> Result<[u32; 2]>;
    /// 58B760(a.x, a.y, b.x, b.y).
    fn transport_path_valid(&mut self, point: [u32; 2], to: [u32; 2]) -> Result<bool>;
    /// 4F9C10 on World: create the transport from the spawn request.
    fn create_transport(&mut self, request: &SpawnRequest) -> Result<Option<Object>>;
    /// Object+8 |= bits.
    fn or_flags8(&mut self, object: Object, bits: u32) -> Result<()>;
    /// 6F7810(0x18) + 4C2290(data+2F0 word): cargo group; None when allocation fails.
    fn create_group(&mut self, word_2f0: u32) -> Result<Option<Object>>;
    /// 496EF0 (position flare) / 496F10 (actor flare) on flare+57C -> +2C, argument 0.
    fn flare_ready(&mut self, config: Object, on_actor: bool) -> Result<bool>;
    /// 453ED0 on B39AA8 ("SAITriggerTeamCommandPriority"), cached by the original
    /// in a function static (B3D464 / B3D46C) after the first call.
    fn flare_var(&mut self, on_actor: bool) -> Result<u32>;
    /// 572E50(request, kingdom, actor, duration, variable).
    fn create_flare(&mut self, request: &SpawnRequest, kingdom: Object, actor: Option<Object>, duration: u32, variable: u32) -> Result<()>;
    /// 4C86D0(ability) on a stack ability user; 4C9F10(request); 4CE080 release.
    fn ability_user_init(&mut self, ability: Object) -> Result<()>;
    fn ability_use(&mut self, request: &AbilityRequest) -> Result<()>;
    fn ability_user_release(&mut self) -> Result<()>;
    /// 58BA90 on transport+C0 (cargo, name, group, position, destination).
    fn transport_load(&mut self, component: Object, cargo: Object, name: &[u16], group: Option<Object>, position: [u32; 2], destination: [u32; 2]) -> Result<()>;
    /// ActorData+174 flags (bit 7 selects 4FA150).
    fn data_flags174(&self, data: Object) -> Result<u32>;
    /// 4FA030 (or 4FA150 when `alternate`) World create.
    fn create_actor(&mut self, request: &SpawnRequest, alternate: bool) -> Result<Option<Object>>;
}

fn numeric(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(|e| Error::State(e.to_string()))
}
fn arith<T>(v: std::result::Result<T, aa_numeric::Error>) -> Result<T> {
    v.map_err(|e| Error::State(e.to_string()))
}
fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}
fn log(core: &mut CoreProgram, address: u32) {
    core.diagnostics.push(Diagnostic::NativeLog(address));
}

/// 54D9E0 then fstp dword (binary32 bits).
fn value<S: Services + ?Sized>(core: &mut CoreProgram, m: &Manager, p: &Parameter, _s: &mut S) -> Result<u32> {
    core.value(m, p)
}

/// 54DCC0 camera register.
fn camera<S: Services + ?Sized>(core: &mut CoreProgram, p: &Parameter, s: &mut S) -> Result<Option<Object>> {
    let raw = match p.value {
        Value::Word(w) => w,
        _ => return Err(Error::State("camera register without word storage".into())),
    };
    if let Some(o) = s.resolve_id(raw)?
        && s.is_camera(o)?
    {
        return Ok(Some(o));
    }
    if s.warn_invalid_ids()? {
        log(core, 0x404ed0);
    }
    Ok(None)
}

/// 54DED0 localized register: the resolved text (+4 of the 4A0AB0 pair).
/// 49AD70 stores the static empty CStringW B43B48 for an empty key in every
/// stream mode; nonempty external keys are filled by `MapStrings::bind`.
fn localized_text(p: &Parameter) -> Result<Vec<u16>> {
    match &p.value {
        Value::Localized { inline: Some(text), .. } => Ok(text.clone()),
        Value::Localized { key, inline: None } if key.is_empty() => Ok(Vec::new()),
        Value::Localized { inline: None, .. } => Err(Error::State(
            "localized text not resolved: the MAPSTRINGS binding must supply it".into(),
        )),
        _ => Err(Error::State("localized register without localized storage".into())),
    }
}

/// 54E8E0: mi_command(39) -> command bank, else null.
fn command_data<S: Services + ?Sized>(p: &Parameter, s: &mut S) -> Result<Option<Object>> {
    if p.item_index != 39 {
        return Ok(None);
    }
    s.data_by_ids(Bank::Command, text(p)?)
}

/// 54E910 ActorData register.
fn actor_data<S: Services + ?Sized>(p: &Parameter, s: &mut S) -> Result<Option<Object>> {
    match s.data_by_ids(Bank::ActorData, &selectors::string(p)?)? {
        Some(d) if s.is_actor_data(d)? => Ok(Some(d)),
        _ => Ok(None),
    }
}

/// 544540 (pan) / 544780 (return to default): build and run "MoveCamera".
#[allow(clippy::too_many_arguments)]
fn move_camera<S: Services + ?Sized>(
    core: &CoreProgram,
    s: &mut S,
    kingdom: Object,
    start_bits: u32,
    duration: u32,
    realtime: bool,
    view: [u32; 5],
    ease: bool,
) -> Result<()> {
    let name = s.object_name(kingdom)?;
    let time = crt::x87_add_f32_to_f64(start_bits, duration, core.precision)?;
    let f = |bits: u32| crt::format_f(f64::from(f32::from_bits(bits)).to_bits(), 6);
    let mut command = format!(
        "MoveCamera {} {} {} {} {} {} {} {} ",
        crt::format_f(time, 6)?,
        crt::format_d(i32::from(realtime)),
        f(view[0])?,
        f(view[1])?,
        f(view[2])?,
        f(view[3])?,
        f(view[4])?,
        crt::format_d(i32::from(ease)),
    );
    let mut wide = utf16(&command);
    wide.extend(name.iter().copied().take_while(|c| *c != 0));
    command.clear();
    s.ui_script_command(&wide)
}

/// 542EB0 EffectActorGivenCommand.
fn give_command<S: Services + ?Sized>(
    core: &mut CoreProgram,
    s: &mut S,
    actor: Option<Object>,
    definition: Option<Object>,
    data: Option<Object>,
    location: Option<[u32; 3]>,
    target: Option<Object>,
) -> Result<bool> {
    let Some(actor) = actor else {
        log(core, 0x404f30);
        return Ok(false);
    };
    if (s.flags8(actor)? >> 6) & 1 != 0 {
        s.actor_4d4c60(actor, 0)?;
    }
    let mut command = Command::new(definition);
    if data.is_some() {
        command.data = data;
        command.flags |= 2;
    }
    if let Some(loc) = location {
        command.location = [fld_fstp(loc[0]), fld_fstp(loc[1])];
        command.flags |= 4;
    } else if let Some(target) = target {
        command.flags |= 1;
        command.target_id = s.raw_id(target)?;
    } else {
        log(core, 0x404f30);
        return Ok(false);
    }
    s.process_command(actor, &mut command)?;
    destroy_command(s, command)?;
    Ok(true)
}

/// 4CA2D0: release a +34 reference installed during processing.
pub(crate) fn destroy_command<S: Services + ?Sized>(s: &mut S, command: Command) -> Result<()> {
    match command.shared34 {
        Some(shared) => s.release_command_shared(shared),
        None => Ok(()),
    }
}

/// fld dword / fstp dword: x87 quiets a signalling NaN, everything else is exact.
pub(crate) fn fld_fstp(bits: u32) -> u32 {
    if (bits >> 23) & 0xff == 0xff && bits & 0x7f_ffff != 0 {
        bits | 0x40_0000
    } else {
        bits
    }
}

pub const SUPPORTED: &[u32] = &[
    29, 30, 31, 34, 36, 42, 46, 47, 52, 53, 55, 66, 67, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78,
    82, 83, 85, 86, 94, 95, 96, 101, 102, 103, 104, 105, 108, 109, 110, 112,
];

pub fn evaluate<S: Services + ?Sized>(
    core: &mut CoreProgram,
    manager: &mut Manager,
    element: &Element,
    s: &mut S,
) -> Result<bool> {
    let kind = selectors::kind(element)?;
    let p = &element.parameters;
    Ok(match element.opcode {
        71 | 72 => {
            let ease = if element.opcode == 71 { selectors::boolean(&p[3])? } else { false };
            let duration = value(core, manager, &p[2], s)?;
            let camera = camera(core, &p[1], s)?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            let (Some(kingdom), Some(camera)) = (kingdom, camera) else {
                return Ok(false);
            };
            let realtime = element.opcode == 72;
            let fields = s.camera_fields(camera)?;
            if s.camera_debug_log()? {
                s.camera_pitch(camera)?;
                s.camera_yaw(camera)?;
                log(core, 0x404fc0);
            }
            let start = if realtime {
                let ms = s.real_time_ms()?;
                let mut v = Binary::from_i32(ms as i32);
                if (ms as i32) < 0 {
                    v = arith(v.add(&numeric(0x4f80_0000)?, core.precision))?;
                }
                arith(arith(v.mul(&numeric(0x3a83_126f)?, core.precision))?.store_f32())?
            } else {
                s.world_time_bits()?
            };
            let pitch = s.camera_pitch(camera)?;
            let yaw = s.camera_yaw(camera)?;
            let view = [fields.look_at[0], fields.look_at[1], fields.zoom, yaw, pitch];
            move_camera(core, s, kingdom, start, duration, realtime, view, ease)?;
            true
        }
        73 => {
            let duration = value(core, manager, &p[2], s)?;
            let location = selectors::location(&p[1])?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            let (Some(kingdom), Some(location)) = (kingdom, location) else {
                return Ok(false);
            };
            if s.camera_debug_log()? {
                log(core, 0x404fc0);
            }
            let divisor = numeric(s.camera_var(0x2c)?)?;
            let zoom = arith(arith(numeric(0x3f80_0000)?.div(&divisor, core.precision))?.store_f32())?;
            let yaw = s.camera_var(0x50)?;
            let pitch = s.camera_var(0x48)?;
            let start = s.world_time_bits()?;
            let view = [location[0], location[1], zoom, yaw, pitch];
            move_camera(core, s, kingdom, start, duration, false, view, false)?;
            true
        }
        42 => {
            let enabled = selectors::boolean(&p[0])?;
            if enabled {
                s.camera_6fe710()?;
                s.world_set_cutscene_mode(true)?;
            } else {
                s.world_set_cutscene_mode(false)?;
                let v = s.camera_700b10()?;
                s.camera_700a90(v)?;
            }
            true
        }
        52 => {
            let property = s.data_by_ids(Bank::Property, text(&p[1])?)?;
            if p[0].item_index == 17 {
                // 5428D0: every live World+154 Kingdom slot.
                let Some(property) = property else { return Ok(false) };
                for kingdom in s.world_kingdom_slots()?.into_iter().flatten() {
                    s.kingdom_add_property(kingdom, property, 2, 0, 1)?;
                }
                true
            } else {
                let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
                match (kingdom, property) {
                    (Some(k), Some(prop)) => {
                        s.kingdom_add_property(k, prop, 2, 0, 1)?;
                        true
                    }
                    _ => false,
                }
            }
        }
        53 => {
            let property = s.data_by_ids(Bank::Property, text(&p[1])?)?;
            let kingdom = selectors::kingdom(manager, kind, &p[0], s)?;
            match (kingdom, property) {
                (Some(k), Some(prop)) => {
                    s.kingdom_remove_property(k, prop, 0)?;
                    true
                }
                _ => false,
            }
        }
        55 => {
            let property = s.data_by_ids(Bank::Property, text(&p[1])?)?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            match (actor, property) {
                (Some(a), Some(prop)) => {
                    s.actor_remove_property(a, prop, 0)?;
                    true
                }
                _ => false,
            }
        }
        36 => {
            let message = localized_text(&p[0])?;
            let duration = value(core, manager, &p[1], s)?;
            s.ui_display_banner(&message, duration)?;
            true
        }
        46 => {
            let duration = value(core, manager, &p[1], s)?;
            match s.data_by_ids(Bank::Sun, text(&p[0])?)? {
                Some(sun) => {
                    let colors = s.sun_colors(sun)?;
                    s.view_set_sun_colors(&colors, duration)?;
                    true
                }
                None => false,
            }
        }
        112 => {
            let amount = value(core, manager, &p[1], s)?;
            match selectors::kingdom(manager, kind, &p[0], s)? {
                Some(kingdom) => {
                    for (i, flag) in s.resource_flags18()?.into_iter().enumerate() {
                        if flag == 0 {
                            s.kingdom_add_resource(kingdom, i as u32, amount)?;
                        }
                    }
                    true
                }
                None => false,
            }
        }
        69 => {
            let duration = value(core, manager, &p[0], s)?;
            // 542120: fcomp 0; ah&41 parity: <= 0 returns false, > 0 or NaN proceeds.
            let nan = (duration >> 23) & 0xff == 0xff && duration & 0x7f_ffff != 0;
            let positive = duration >> 31 == 0 && duration & 0x7fff_ffff != 0;
            if !(positive || nan) {
                return Ok(false);
            }
            let start = s.world_time_bits()?;
            s.world_show_counter(start, duration)?;
            true
        }
        70 => {
            s.world_hide_counter()?;
            true
        }
        108 => {
            let weather = s.data_by_ids(Bank::Weather, text(&p[0])?)?;
            s.set_weather(weather)?;
            true
        }
        85 | 86 => {
            let location = if element.opcode == 86 { selectors::location(&p[1])? } else { None };
            let data = s.data_by_ids(Bank::Audio, text(&p[0])?)?;
            s.play_sound(data, location)?;
            true
        }
        109 => match selectors::actor(manager, kind, &p[0], s)? {
            Some(actor) => {
                s.ui_flash_actor(actor)?;
                true
            }
            None => false,
        },
        110 => {
            let name = selectors::string(&p[0])?;
            if name.is_empty() {
                return Ok(false);
            }
            let Some(root) = s.ui_root()? else { return Ok(false) };
            match s.ui_find_widget(root, &name)? {
                Some(widget) => {
                    let label = s.widget_name(widget)?;
                    s.flash_ui_button(&label)?;
                    true
                }
                None => false,
            }
        }
        95 => {
            let name = selectors::string(&p[0])?;
            let listen = selectors::boolean(&p[1])?;
            if name.is_empty() {
                return Ok(false);
            }
            if listen {
                s.world_script_listen(&name)?;
                true
            } else {
                s.world_script_unlisten(&name)?
            }
        }
        34 => {
            let amount = value(core, manager, &p[1], s)?;
            match selectors::actor(manager, kind, &p[0], s)? {
                Some(actor) => {
                    s.actor_gain_experience(actor, amount)?;
                    true
                }
                None => false,
            }
        }
        82 | 83 => {
            // 542FF0(vfx, location, rotation degrees); opcode 82 passes literal 0.
            let rotation = if element.opcode == 83 { value(core, manager, &p[2], s)? } else { 0 };
            let location = selectors::location(&p[1])?;
            let vfx = match s.data_by_ids(Bank::ActorData, text(&p[0])?)? {
                Some(d) if s.is_vfx(d)? => Some(d),
                _ => None,
            };
            match (vfx, location) {
                (Some(vfx), Some(location)) => {
                    let o = s.default_vfx_orientation()?;
                    let mut v = [o[0], o[1]];
                    // fucompp with 0: rotate unless equal (both zeros compare equal).
                    if rotation & 0x7fff_ffff != 0 {
                        let angle = arith(
                            arith(numeric(rotation)?.mul(&numeric(0x3c8e_fa35)?, core.precision))?.store_f32(),
                        )?;
                        v = s.rotate_2d(v, angle)?;
                    }
                    // The orientation vector overlaps the position on the stack:
                    // its third word is the location x.
                    s.spawn_vfx(vfx, location, [v[0], v[1], location[0]])?;
                    true
                }
                _ => false,
            }
        }
        47 => {
            let message = localized_text(&p[1])?;
            if p[0].item_index == 17 {
                // 543F40 returns the low byte left by 40BC50.
                s.chat_message_all(&message)? & 0xff != 0
            } else {
                match selectors::kingdom(manager, kind, &p[0], s)? {
                    Some(k) => {
                        s.chat_message_kingdom(k, &message)?;
                        true
                    }
                    None => false,
                }
            }
        }
        101 => {
            let name = selectors::string(&p[1])?;
            let Some(actor) = selectors::actor(manager, kind, &p[0], s)? else {
                return Ok(false);
            };
            if name.is_empty() {
                return Ok(false);
            }
            let Some(set) = s.animation_set(actor)? else { return Ok(false) };
            match s.animation_by_name(set, &name)? {
                Some(animation) => s.actor_play_animation(actor, animation)?,
                None => false,
            }
        }
        102..=105 => {
            let (data, location, target) = match element.opcode {
                102 => (None, None, None),
                103 => (None, selectors::location(&p[2])?, None),
                104 => {
                    let location = selectors::location(&p[3])?;
                    (actor_data(&p[2], s)?, location, None)
                }
                _ => (None, None, selectors::actor(manager, kind, &p[2], s)?),
            };
            let definition = command_data(&p[1], s)?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            give_command(core, s, actor, definition, data, location, target)?
        }
        31 => {
            let factor = value(core, manager, &p[2], s)?;
            let location = selectors::location(&p[1])?;
            let actor = selectors::actor(manager, kind, &p[0], s)?;
            let (Some(actor), Some(location)) = (actor, location) else {
                return Ok(false);
            };
            let Some(base) = s.move_factor(actor)? else { return Ok(false) };
            let speed = arith(arith(numeric(base)?.mul(&numeric(factor)?, core.precision))?.store_f32())?;
            if (s.flags8(actor)? >> 6) & 1 != 0 {
                s.actor_4d4c60(actor, 0)?;
            }
            let mut command = Command::new(s.command_definition(5)?);
            command.flags |= 4 | 8 | 0x10;
            command.location = [fld_fstp(location[0]), fld_fstp(location[1])];
            command.move_words = [0, 0, speed];
            let result = s.process_command(actor, &mut command)?;
            destroy_command(s, command)?;
            result
        }
        66 | 67 => {
            // 543410: one stop command (ordinal 6) shared by every member.
            let lock = element.opcode == 66;
            let mut command = Command::new(s.command_definition(6)?);
            for kingdom in s.world_slots_148()?.into_iter().flatten() {
                let mut node = s.member_head(kingdom)?;
                while let Some(current) = node {
                    let actor = s
                        .member_value(current)?
                        .ok_or_else(|| Error::State("null Kingdom member node value".into()))?;
                    if lock {
                        s.process_command(actor, &mut command)?;
                    }
                    if !s.actor_80_ready(actor)? {
                        s.actor_4d4c60(actor, u8::from(lock))?;
                    }
                    node = s.member_next(current)?;
                }
            }
            destroy_command(s, command)?;
            true
        }
        94 => {
            // 545C60: "PlayVideo %s %s %s %d" through the UI command queue.
            let name = selectors::string(&p[0])?;
            let path = s.video_path(&name)?;
            let widget = s.game_var_string(&utf16("FullscreenVideoWidgetIDS"))?;
            let mut command = utf16("PlayVideo ");
            command.extend(path.iter().copied().take_while(|c| *c != 0));
            command.push(u16::from(b' '));
            command.extend(widget.iter().copied().take_while(|c| *c != 0));
            command.extend(utf16(" SendOrderOnVideoCompletion 0"));
            s.ui_script_command(&command)?;
            true
        }
        77 | 78 => {
            // 544A50 (position) / 544BA0 (actor) team flare command.
            let on_actor = element.opcode == 78;
            let duration = value(core, manager, &p[3], s)?;
            let (location, actor) = if on_actor {
                (None, selectors::actor(manager, kind, &p[2], s)?)
            } else {
                (selectors::location(&p[2])?, None)
            };
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let flare = actor_data(&p[0], s)?;
            let (Some(flare), Some(kingdom)) = (flare, kingdom) else {
                return Ok(false);
            };
            let position = match (on_actor, location, actor) {
                (false, Some(l), _) => [fld_fstp(l[0]), fld_fstp(l[1]), 0],
                (true, _, Some(a)) => [s.data_word(a, 0x20)?, s.data_word(a, 0x24)?, s.data_word(a, 0x28)?],
                _ => return Ok(false),
            };
            let config = s
                .data_ref(flare, 0x57c)?
                .ok_or_else(|| Error::State("flare+57C is null: native would call through it".into()))?;
            if !s.flare_ready(config, on_actor)? {
                return Ok(false);
            }
            let request = SpawnRequest {
                definition: flare,
                position,
                // 4C8C20 constructor facing (0, -1); the flare path keeps it.
                facing: [0, 0xbf80_0000],
                name: Vec::new(),
                kingdom,
            };
            let variable = s.flare_var(on_actor)?;
            s.create_flare(&request, kingdom, actor, duration, variable)?;
            true
        }
        96 => {
            let at = selectors::location(&p[3])?;
            let from = selectors::location(&p[2])?;
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let ability = s.data_by_ids(Bank::Ability, text(&p[0])?)?;
            let (Some(ability), Some(kingdom), Some(from), Some(at)) = (ability, kingdom, from, at) else {
                return Ok(false);
            };
            s.ability_user_init(ability)?;
            s.ability_use(&AbilityRequest { kingdom, from, at })?;
            s.ability_user_release()?;
            true
        }
        30 => {
            // 543BA0 EffectActorCreateTransported.
            let name = selectors::string(&p[4])?;
            let dest = selectors::location(&p[3])?;
            let spawn = selectors::location(&p[2])?;
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let data = actor_data(&p[0], s)?;
            let (Some(data), Some(spawn), Some(dest), Some(kingdom)) = (data, spawn, dest, kingdom) else {
                return Ok(false);
            };
            let Some(transport) = s.data_ref(data, 0x184)? else { return Ok(false) };
            let pc = core.precision;
            let sub = |a: u32, b: u32| -> Result<u32> {
                arith(arith(numeric(a)?.sub(&numeric(b)?, pc))?.store_f32())
            };
            // 4281E0: dest - spawn (all three components are stored).
            let mut dir = [sub(dest[0], spawn[0])?, sub(dest[1], spawn[1])?];
            sub(dest[2], spawn[2])?;
            // 409230: normalize when the length exceeds 1e-4f.
            let (x, y) = (numeric(dir[0])?, numeric(dir[1])?);
            let len = arith(
                arith(arith(x.mul(&x, pc))?.add(&arith(y.mul(&y, pc))?, pc))?.sqrt(pc),
            )?;
            if len.compare(&numeric(0x38d1_b717)?) == std::cmp::Ordering::Greater {
                let inv = arith(numeric(0x3f80_0000)?.div(&len, pc))?;
                dir = [
                    arith(arith(inv.mul(&x, pc))?.store_f32())?,
                    arith(arith(inv.mul(&y, pc))?.store_f32())?,
                ];
            }
            let flags = s.data_word(transport, 0x1c)?;
            let (position, facing) = if (flags >> 12) & 1 != 0 {
                let altitude = numeric(s.data_word(transport, 0x414)?)?;
                let z = arith(arith(altitude.add(&numeric(spawn[2])?, pc))?.store_f32())?;
                ([spawn[0], spawn[1], z], dir)
            } else {
                let mut pos = [fld_fstp(spawn[0]), fld_fstp(spawn[1])];
                let mut facing = dir;
                if !s.find_placement(&mut pos, &mut facing, transport, 0x4100_0000, 1)? {
                    return Ok(false);
                }
                ([pos[0], pos[1], 0], facing)
            };
            let mut target = [fld_fstp(dest[0]), fld_fstp(dest[1])];
            if (flags >> 21) & 1 != 0 {
                let point = s.transport_path([position[0], position[1]], target)?;
                if !s.transport_path_valid(point, target)? {
                    return Ok(false);
                }
                target = point;
            }
            let request = SpawnRequest {
                definition: transport,
                position,
                facing,
                name: Vec::new(),
                kingdom,
            };
            let Some(vehicle) = s.create_transport(&request)? else { return Ok(false) };
            s.or_flags8(vehicle, 4)?;
            let group = if (s.data_word(data, 0x174)? >> 7) & 1 != 0 {
                let word = s.data_word(data, 0x2f0)?;
                s.create_group(word)?
            } else {
                None
            };
            let component = s
                .data_ref(vehicle, 0xc0)?
                .ok_or_else(|| Error::State("transport+C0 component is null: native would call through it".into()))?;
            s.transport_load(component, data, &name, group, [position[0], position[1]], target)?;
            // 4CA760(target, speed 0, [B279C0], [B279C4]) -> 4D9B60(vehicle).
            let mut command = Command::new(s.command_definition(5)?);
            command.flags |= 4 | 8 | 0x10;
            command.location = target;
            s.process_command(vehicle, &mut command)?;
            destroy_command(s, command)?;
            true
        }
        29 | 74 | 75 | 76 => {
            let name = selectors::string(&p[3])?;
            let location = selectors::location(&p[2])?;
            let kingdom = selectors::kingdom(manager, kind, &p[1], s)?;
            let data = actor_data(&p[0], s)?;
            let (Some(data), Some(kingdom), Some(location)) = (data, kingdom, location) else {
                return Ok(false);
            };
            let mut position = [fld_fstp(location[0]), fld_fstp(location[1])];
            let mut facing = [0, 0xbf80_0000];
            if !s.find_placement(&mut position, &mut facing, data, 0x4100_0000, 0)? {
                return Ok(false);
            }
            let request = SpawnRequest {
                definition: data,
                position: [position[0], position[1], 0],
                facing,
                name,
                kingdom,
            };
            let alternate = (s.data_flags174(data)? >> 7) & 1 != 0;
            s.create_actor(&request, alternate)?.is_some()
        }
        _ => unreachable!(),
    })
}
