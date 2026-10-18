//! Original Organization component (slot 7, 0xDC bytes) saved Load `57D130`
//! (Organization.cpp), applied to the constructed state of `579BD0`
//! ([`OrganizationState::new`]). Also the shared saved-stream pieces used by
//! the Transport loader (`transport_state`): the stream cursor and its native
//! read granularity, the native throw/log messages, and the
//! `KKC_OrganizationLayout` virtual Load `4C3180`.
//!
//! Native order of `57D130` (every step is ported; world effects are
//! [`OrganizationWorld`] callbacks at their native call sites):
//! Updatable `50BCD0`; version `8589B0` (max 10); layout Load `4C3180`;
//! default-layout replacement (`4C1DB0`, or the 858A00 slot-count throw);
//! resize of members `+28`, recruit items `+34` (`430240`), `+50` (`45F180`),
//! slot offsets `+78` (`45D8A0`); maximum morale `578300`; `+D8 = FLT_MAX`;
//! `4C2910` VerifyDefaultLayoutRestrictions; saved slot count; per slot the
//! nested member record (`4FC200` -> `4FAE80`) and `57CBA0` add-member;
//! recruit items (versions 2..6 by Kingdom index `4E13F0`, version >= 7 by
//! ActorData reference `4E2A60` with the pairing assertions); leader `+40`;
//! morale `59E300`; formations `4C2D90` (`+70`, `+74` for version >= 3);
//! version-0 word; `+78` offsets; `+50` words (version >= 1); `+84/+8C/+8D`
//! and the damage record `4C0BA0` (version >= 4); `+88` (>= 5); `+B4` (>= 6);
//! `+BC/+C0` (>= 8); `+D0` (>= 9); `+D4` (>= 10); the `+B4` clamp; `+74 = 0`;
//! set formation `57BFF0` (`57BF50`, `577F30`).
//!
//! Oracle: research/phase8-saved-components/native-organization.json
//! (original x86 under Unicorn, every seam recorded in call order).
//! Partial failure: native throws (C++ exceptions) leave every earlier store
//! and world callback committed; this port mutates `OrganizationState` in the
//! same order and returns the error at the same point.
use crate::{
    ActorId,
    ground_states::{LayoutDefinition, OrganizationState},
};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use std::cmp::Ordering;
use std::fmt;

// ---------------------------------------------------------------- messages

/// One printf argument of a native message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Arg {
    Text(String),
    Int(i32),
}
/// A native message: the original format literal and its arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Message {
    pub format: &'static str,
    pub args: Vec<Arg>,
}
impl Message {
    pub fn new(format: &'static str, args: Vec<Arg>) -> Self {
        Self { format, args }
    }
}
impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut args = self.args.iter();
        let mut chars = self.format.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '%' {
                write!(f, "{c}")?;
                continue;
            }
            while chars.peek().is_some_and(|c| *c == 'h' || *c == 'l') {
                chars.next();
            }
            match (chars.next(), args.next()) {
                (Some(_), Some(Arg::Text(t))) => write!(f, "{t}")?,
                (Some(_), Some(Arg::Int(v))) => write!(f, "{v}")?,
                (Some(s), None) => write!(f, "%{s}")?,
                (None, _) => write!(f, "%")?,
            }
        }
        Ok(())
    }
}

/// Native failure of a saved-component Load. Every variant is a point where
/// the original stops (throws or faults); nothing is defaulted.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum LoadError {
    /// `858A00` TGC_File::ThrowWarningException ("File Warning: %s").
    #[error("File Warning: {0}")]
    Warning(Message),
    /// `858AD0` TGC_File::ThrowException ("Fatal File Error: %s").
    #[error("Fatal File Error: {0}")]
    Fatal(Message),
    /// `41DFB0` exception (version reader "Bad data version", "Unknown IDS").
    #[error("{0}")]
    Exception(Message),
    /// `4F0110` "Loading Exception" assertion (Organization.cpp line).
    #[error("Loading Exception: Organization.cpp({line}): {expression}")]
    Assertion { line: u32, expression: &'static str },
    /// The stream ended inside a native read of `wanted` bytes at `at`
    /// (the engine stream's own end-of-file behaviour is not established).
    #[error("saved component stream ends inside a {wanted}-byte read at {at}")]
    Truncated { at: usize, wanted: usize },
    /// The original dereferences a null pointer at this instruction.
    #[error("native null dereference at {0:#x}")]
    NullDereference(u32),
    /// Native undefined behaviour (out-of-bounds store/read, negative counts,
    /// non-finite arithmetic outside the proven domain): refused explicitly.
    #[error("unsupported native behaviour: {0}")]
    Unsupported(String),
    /// A world callback failed (runtime adapter error).
    #[error("world callback failed: {0}")]
    World(String),
}
pub type Result<T> = std::result::Result<T, LoadError>;

fn text(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}
fn numeric(e: aa_numeric::Error) -> LoadError {
    LoadError::Unsupported(format!("x87 arithmetic outside the finite domain: {e}"))
}
/// `fstp dword` with the default masked x87 exceptions: binary32 overflow
/// stores the signed infinity (round to nearest).
pub(crate) fn store32(value: &Binary) -> Result<u32> {
    match value.store_f32() {
        Ok(bits) => Ok(bits),
        Err(aa_numeric::Error::Binary32Overflow) => Ok(if value.is_negative_nonzero() {
            0xff80_0000
        } else {
            0x7f80_0000
        }),
        Err(e) => Err(numeric(e)),
    }
}
fn number(bits: u32) -> Result<Binary> {
    Binary::from_f32_bits(bits).map_err(numeric)
}
/// x87 `fcom` of two binary32 values: None when unordered.
pub(crate) fn compare32(a: u32, b: u32) -> Option<Ordering> {
    let nan = |x: u32| x & 0x7fff_ffff > 0x7f80_0000;
    if nan(a) || nan(b) {
        return None;
    }
    let key = |x: u32| {
        if x >> 31 != 0 {
            -i64::from(x & 0x7fff_ffff)
        } else {
            i64::from(x)
        }
    };
    Some(key(a).cmp(&key(b)))
}

// ------------------------------------------------------------------ cursor

/// The component's view of the native file stream (`ctx+3C`): every method
/// is one native read call, so truncation happens at the same read.
pub struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl<'a> Cursor<'a> {
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }
    pub fn position(&self) -> usize {
        self.at
    }
    pub fn rest(&self) -> &'a [u8] {
        &self.bytes[self.at..]
    }
    /// One stream read of `n` bytes (vtable +0).
    pub fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .filter(|&e| e <= self.bytes.len())
            .ok_or(LoadError::Truncated {
                at: self.at,
                wanted: n,
            })?;
        let b = &self.bytes[self.at..end];
        self.at = end;
        Ok(b)
    }
    /// Advance past bytes consumed by a nested loader.
    pub fn skip(&mut self, n: usize) -> Result<()> {
        self.take(n).map(|_| ())
    }
    pub fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }
    pub fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    pub fn pair(&mut self) -> Result<[u32; 2]> {
        let b = self.take(8)?;
        Ok([
            u32::from_le_bytes(b[..4].try_into().expect("4")),
            u32::from_le_bytes(b[4..].try_into().expect("4")),
        ])
    }
    /// `41D290`: one 2-byte read of the UTF-16 unit count, then one read of
    /// all units.
    pub fn string(&mut self) -> Result<Vec<u16>> {
        let n = u16::from_le_bytes(self.take(2)?.try_into().expect("2")) as usize;
        Ok(self
            .take(2 * n)?
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect())
    }
    /// `8589B0`: signed version word, `0 <= v <= max` or the 41DFB0 throw.
    pub fn version(&mut self, max: i32) -> Result<i32> {
        let v = self.i32()?;
        if v < 0 || v > max {
            return Err(LoadError::Exception(Message::new(
                "Bad data version",
                vec![],
            )));
        }
        Ok(v)
    }
}

// ------------------------------------------------------------- definitions

/// One `KKC_OrganizationLayoutData` slot group (definition `+1C`, 16 bytes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutEntry {
    /// `+4` Property object handle and its `+8` IDS (messages only).
    pub property: u32,
    pub property_name: String,
    /// `+C` required, `+D` locked, `+E` morale_line bytes (tested nonzero).
    pub required: bool,
    pub locked: bool,
    pub morale_line: bool,
}
/// `KKC_OrganizationLayoutData` fields read by the loaders.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutInfo {
    /// `+8` IDS.
    pub name: String,
    /// `+18` primary slot (-1 when none).
    pub primary_slot: i32,
    /// `+1C`/`+20`: slot groups (the layout parameter count is their count).
    pub entries: Vec<LayoutEntry>,
    /// `+28`/`+2C`: group index of every slot (the slot count).
    pub slot_groups: Vec<u32>,
    /// `+34`/`+38`: formation handles (nonzero).
    pub formations: Vec<u32>,
    /// `+44`/`+48` formation indices used by state descriptors 2 and 3.
    pub formation_44: i32,
    pub formation_48: i32,
    /// `+50` property handle applied while no formation is selected.
    pub property_50: u32,
}
impl LayoutInfo {
    pub fn parameter_count(&self) -> usize {
        self.entries.len()
    }
    pub fn slot_count(&self) -> usize {
        self.slot_groups.len()
    }
}
/// Definition registries and definition fields used by both loaders.
pub trait Definitions<D> {
    /// `463FA0` on `[B3AB00]` (KKC_OrganizationLayoutData) by IDS.
    fn layout_data(&mut self, ids: &[u16]) -> Result<Option<D>>;
    /// `4BDBA0` on `[B39F58]` with class 0x2B (KKC_ActorData) by IDS.
    fn actor_data(&mut self, ids: &[u16]) -> Result<Option<D>>;
    fn layout_info(&self, layout: &D) -> Result<LayoutInfo>;
    /// Definition `+8` IDS (messages).
    fn name(&self, definition: &D) -> Result<String>;
}

/// `4C3050`: LayoutData reference (empty IDS is null; unknown throws).
pub(crate) fn read_layout_ref<D>(
    c: &mut Cursor<'_>,
    defs: &mut impl Definitions<D>,
) -> Result<Option<D>> {
    let ids = c.string()?;
    if ids.is_empty() {
        return Ok(None);
    }
    match defs.layout_data(&ids)? {
        Some(d) => Ok(Some(d)),
        None => Err(LoadError::Exception(Message::new(
            "Unknown IDS %s for class %hs",
            vec![
                Arg::Text(text(&ids)),
                Arg::Text("KKC_OrganizationLayoutData".into()),
            ],
        ))),
    }
}
/// `4C2E60`: ActorData reference (empty IDS is null; unknown throws).
pub(crate) fn read_actor_ref<D>(
    c: &mut Cursor<'_>,
    defs: &mut impl Definitions<D>,
) -> Result<Option<D>> {
    let ids = c.string()?;
    if ids.is_empty() {
        return Ok(None);
    }
    match defs.actor_data(&ids)? {
        Some(d) => Ok(Some(d)),
        None => Err(LoadError::Exception(Message::new(
            "Unknown IDS %s for class %hs",
            vec![Arg::Text(text(&ids)), Arg::Text("KKC_ActorData".into())],
        ))),
    }
}
/// Layout object `4C3180` (virtual +8 of vtable A232B8): version 0, the
/// LayoutData reference stored to `+8` (null faults at 4C31D0), parameters
/// resized to its count with nulls (`430240`), the saved count (858AD0 when
/// different) and one ActorData reference per parameter.
pub(crate) fn load_layout<D: Clone>(
    c: &mut Cursor<'_>,
    layout: &mut LayoutDefinition<D>,
    defs: &mut impl Definitions<D>,
) -> Result<()> {
    c.version(0)?;
    let definition = read_layout_ref(c, defs)?.ok_or(LoadError::NullDereference(0x4c31d0))?;
    let info = defs.layout_info(&definition)?;
    layout.definition = definition;
    layout.slots = info.slot_count();
    layout.parameters.clear();
    layout.parameters.resize(info.parameter_count(), None);
    let count = c.i32()?;
    if count as i64 != info.parameter_count() as i64 {
        return Err(LoadError::Fatal(Message::new(
            "Data changed number of slots for layout data '%s'",
            vec![Arg::Text(info.name)],
        )));
    }
    for i in 0..info.parameter_count() {
        layout.parameters[i] = read_actor_ref(c, defs)?;
    }
    Ok(())
}

// ------------------------------------------------------------------- world

/// A native diagnostic written while loading (the load continues).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// `0x404EA0`/`0x404F30` (debug log B38B70) or `0x5A1040` (map warning).
    pub function: u32,
    /// Source line of the 404EA0/404F30 call (0 for 5A1040).
    pub line: u32,
    pub message: Message,
}
/// Actor fields of a loaded member read directly by `57D130`/`57CBA0`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberFacts<D> {
    /// `+4` ActorData.
    pub definition: D,
    /// ActorData `+174` flags (bit 8: has an Element component).
    pub definition_flags: u32,
    /// `+14` GTID.
    pub gtid: u32,
    /// `+60` Body present.
    pub has_body: bool,
    /// `+6C` Mover `+28` speed bits, when a Mover is present.
    pub mover_speed: Option<u32>,
    /// `+A4` Experience present.
    pub has_experience: bool,
}
/// Owner (Organization `+4`) fields read directly by the loader.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnerFacts<D> {
    pub definition: D,
    /// `+14` GTID.
    pub gtid: u32,
    /// `+70` Mind present.
    pub has_mind: bool,
    /// `+14C > 4` and `[+148]+10` nonnull: that object.
    pub object_10: Option<u32>,
    /// ActorData `+2F0`: the default `KKC_OrganizationLayout` object.
    pub default_layout: LayoutDefinition<D>,
}
/// GWorld fields read directly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldFacts {
    /// GWorld `+2C` (the map's GWORLD version), read by `4C0BA0`.
    pub world_version: u32,
    /// GWorld `+E4` time bits compared with `+B4`.
    pub time_e4: u32,
}
/// A state-group-4 descriptor returned by `4D49B0(owner, 4)`: its `+20`
/// (formation mode) and `+24` (`577F30` mode) words.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StateWords {
    pub word_20: u32,
    pub word_24: u32,
}

/// Every native call made by `57D130` outside the Organization object, in the
/// original names. Methods documented as "fact" read memory and are not
/// ordered calls natively; all others are ordered callbacks.
pub trait OrganizationWorld<D>: Definitions<D> {
    /// Fact: `[GWorld+284]+0` (the Updatable table bound `50BD26`).
    fn updatable_table_word0(&self) -> Result<i32>;
    /// Fact: the table slot for `id` is already occupied.
    fn updatable_slot_occupied(&self, id: i32) -> Result<bool>;
    /// `50BD37`: store this component's Updatable at `id`.
    fn register_updatable(&mut self, id: i32) -> Result<()>;
    /// Fact `4CF110`: actor `+78` Properties `+1C` modifier, if any.
    fn actor_modifier(&mut self, actor: ActorId) -> Result<Option<u32>>;
    /// `501C90(modifier, value)`: extended (ST0) result.
    fn modifier_apply(&mut self, modifier: u32, value_bits: u32) -> Result<Binary>;
    /// Fact: ActorData `+2C0` (`45D580` null-context morale contribution).
    fn morale_contribution(&self, actor_data: &D) -> Result<u32>;
    /// `45EEF0`: the ActorData has the Property.
    fn has_property(&mut self, actor_data: &D, property: u32) -> Result<bool>;
    /// `404EA0`/`404F30`/`5A1040` diagnostics.
    fn log(&mut self, diagnostic: Diagnostic) -> Result<()>;
    /// `4FAE80(ctx, &out, owner, 0)`: load one nested GObject record from the
    /// start of `bytes` (DYNAMIC_THINGS record format, no length prefix).
    /// Returns the loaded thing (None when the original returns null) and the
    /// bytes consumed. Truncation offsets are relative to `bytes`.
    fn load_member_record(&mut self, bytes: &[u8]) -> Result<(Option<ActorId>, usize)>;
    /// Fact: the 4FC200 assert latch byte `B3CC47`.
    fn assert_latch(&self) -> Result<bool>;
    /// Loaded thing virtual `+B4` (IsGActor).
    fn is_actor(&mut self, thing: ActorId) -> Result<bool>;
    /// `855D30` "thing->IsGActor()" assert dialog; its result is stored to
    /// `B3CC47` by [`OrganizationWorld::set_assert_latch`].
    fn assert_dialog(&mut self) -> Result<bool>;
    fn set_assert_latch(&mut self, value: bool) -> Result<()>;
    /// Fact: member fields.
    fn member_facts(&self, member: ActorId) -> Result<MemberFacts<D>>;
    /// `5705A0(owner, slot)` on the member's Element.
    fn element_set_organization(
        &mut self,
        org: &mut OrganizationState<D>,
        member: ActorId,
        slot: i32,
    ) -> Result<()>;
    /// `561760(owner)` on the member's Body.
    fn body_set_organization(&mut self, member: ActorId) -> Result<()>;
    /// `4CED10` on the owner.
    fn owner_experience_token(&mut self) -> Result<u32>;
    /// `4CEBB0(0, token)` on the member.
    fn set_experience_source(&mut self, member: ActorId, token: u32) -> Result<()>;
    /// `5788B0` combat value (aa-combat organization::combat_value).
    fn combat_value(&mut self, org: &mut OrganizationState<D>) -> Result<()>;
    /// Owner Mind virtual `+118(old, new)` after `+D8` decreased.
    fn mind_speed_changed(&mut self, old_bits: u32, new_bits: u32) -> Result<()>;
    /// `4D1080` on the member (state 0x22 flag).
    fn member_flag_4d1080(&mut self, member: ActorId) -> Result<bool>;
    /// `4D64F0(state, enabled)` on the owner (aa_actors::state::set).
    fn set_owner_state(&mut self, state: u8, enabled: bool) -> Result<()>;
    /// `579A00(member)` leader change (aa-movement formation::set_leader).
    /// It stores Organization `+40`; the loader rereads `words[0x40]`.
    fn set_leader(&mut self, org: &mut OrganizationState<D>, member: ActorId) -> Result<()>;
    /// `524B80(member)` on the owner Mind.
    fn mind_member_added(&mut self, member: ActorId) -> Result<()>;
    /// `57C800(member)` follow the leader.
    fn follow_leader(&mut self, org: &mut OrganizationState<D>, member: ActorId) -> Result<()>;
    /// `4D4CF0`, `4CF0F0`, `4D1350` on the owner (in that order).
    fn owner_4d4cf0(&mut self, org: &OrganizationState<D>) -> Result<()>;
    fn owner_4cf0f0(&mut self, org: &mut OrganizationState<D>) -> Result<()>;
    fn owner_4d1350(&mut self, org: &mut OrganizationState<D>) -> Result<()>;
    /// `5784C0(member)`.
    fn member_added_5784c0(
        &mut self,
        org: &mut OrganizationState<D>,
        member: ActorId,
    ) -> Result<()>;
    /// Fact `45EC60`: ActorData `+2D8` list values, head first.
    fn definition_list_2d8(&self, actor_data: &D) -> Result<Vec<u32>>;
    /// `4CEF10(property, kind, actor)` on the owner.
    fn apply_property(&mut self, property: u32, kind: u32, actor: ActorId) -> Result<()>;
    /// `4CEF70(property, actor)` on the owner.
    fn remove_property(&mut self, property: u32, actor: ActorId) -> Result<()>;
    /// `4CEF80(property, actor)` on the owner.
    fn has_applied_property(&mut self, property: u32, actor: ActorId) -> Result<bool>;
    /// `5147C0` on the owner's `[+148]+10` object.
    fn owner_object_notify(&mut self, object: u32) -> Result<()>;
    /// Owner virtual `+108` (Kingdom); None is the native null.
    fn owner_kingdom(&mut self) -> Result<Option<u32>>;
    /// Fact: Kingdom `+218` recruit-single count.
    fn recruit_single_count(&self, kingdom: u32) -> Result<i32>;
    /// Fact `4E13F0`: Kingdom `+214[index]` (0 for a null entry).
    fn recruit_single_at(&self, kingdom: u32, index: i32) -> Result<u32>;
    /// `4E2A60(actor_data, 0)` on Kingdom `+1FC`: 0 when not found.
    fn find_recruit_single(&mut self, kingdom: u32, actor_data: &D) -> Result<u32>;
    /// Fact: recruit item `+C` (actor GTID) and `+10` (organization GTID).
    fn recruit_item_words(&self, item: u32) -> Result<[u32; 2]>;
    /// `4E1200(member)` on the recruit item.
    fn pair_recruit_item(&mut self, item: u32, member: ActorId) -> Result<()>;
    /// `48D500` on `[B3AAF8]` (KKC_DamageType) by IDS.
    fn damage_type(&mut self, ids: &[u16]) -> Result<Option<u32>>;
    /// Fact: formation `+C` definition `+1A` byte and `+24` property.
    fn formation_facts(&self, formation: u32) -> Result<(u8, u32)>;
    /// `4D49B0(4)` on the owner.
    fn state_group4(&mut self) -> Result<Option<StateWords>>;
    /// `57BEF0` formation refresh (aa-movement formation).
    fn formation_refresh(&mut self, org: &mut OrganizationState<D>) -> Result<()>;
    /// `4D1900` on the owner: its `+164` object, if registered.
    fn owner_object_164(&mut self) -> Result<Option<u32>>;
    /// `4B2830` on that object.
    fn object_refresh(&mut self, object: u32) -> Result<()>;
}

#[path = "organization_add_member.rs"]
mod add_member_core;
pub use add_member_core::{AddMemberFields, add_member_fields};

// --------------------------------------------------------------- the load

fn word(org: &OrganizationState<impl Sized>, offset: u32) -> Result<u32> {
    org.words.get(&offset).copied().ok_or_else(|| {
        LoadError::Unsupported(format!("Organization word {offset:#x} uninitialized"))
    })
}
fn actor_word(id: Option<ActorId>) -> u32 {
    id.map_or(0, ActorId::raw)
}
fn as_actor(raw: u32) -> Option<ActorId> {
    (raw != 0).then(|| ActorId::from_raw(raw))
}

/// `578300`: recompute maximum morale `+60` and rescale `+5C`.
pub fn recompute_maximum_morale<D: Clone>(
    org: &mut OrganizationState<D>,
    info: &LayoutInfo,
    owner: ActorId,
    world: &mut impl OrganizationWorld<D>,
    pc: Precision,
) -> Result<()> {
    recompute_maximum_morale_fields(org, info, owner, &mut MaximumMorale(world), pc)
}
///The actual578300 field kernel also serves stored components without a
///reconstructed OrganizationWorld. Modifier identities remain actor handles.
pub trait MaximumMoraleWorld<D> {
    fn modifier(&mut self, actor: ActorId) -> Result<Option<u32>>;
    fn apply(&mut self, modifier: u32, value: u32) -> Result<Binary>;
    fn contribution(&self, definition: &D) -> Result<u32>;
}
struct MaximumMorale<'a, W>(&'a mut W);
impl<D,W:OrganizationWorld<D>> MaximumMoraleWorld<D> for MaximumMorale<'_,W>{
    fn modifier(&mut self,actor:ActorId)->Result<Option<u32>>{self.0.actor_modifier(actor)}
    fn apply(&mut self,modifier:u32,value:u32)->Result<Binary>{self.0.modifier_apply(modifier,value)}
    fn contribution(&self,definition:&D)->Result<u32>{self.0.morale_contribution(definition)}
}
pub fn recompute_maximum_morale_fields<D:Clone>(
    org:&mut OrganizationState<D>, info:&LayoutInfo, owner:ActorId,
    world:&mut impl MaximumMoraleWorld<D>, pc:Precision,
)->Result<()> {
    let maximum = word(org, 0x60)?;
    let ratio = if compare32(maximum, 0) == Some(Ordering::Greater) {
        store32(
            &number(word(org, 0x5c)?)?
                .div(&number(maximum)?, pc)
                .map_err(numeric)?,
        )?
    } else {
        0
    };
    org.words.insert(0x60, 0);
    for slot in 0..info.slot_count() {
        let group = info.slot_groups[slot] as usize;
        let Some(parameter) = org
            .layout
            .parameters
            .get(group)
            .ok_or_else(|| {
                LoadError::Unsupported(format!("layout group {group} beyond the parameter array"))
            })?
            .clone()
        else {
            continue;
        };
        let member = org.members.get(slot).copied().filter(|m| !m.is_null());
        let modifier = match member {
            Some(m) => world.modifier(m)?,
            None => None,
        };
        let base = world.contribution(&parameter)?;
        let value = match modifier {
            None => number(base)?,
            Some(m) => world.apply(m, base)?,
        };
        let sum = value.add(&number(word(org, 0x60)?)?, pc).map_err(numeric)?;
        org.words.insert(0x60, store32(&sum)?);
    }
    if let Some(m) = world.modifier(owner)? {
        let value = world.apply(m, word(org, 0x60)?)?;
        org.words.insert(0x60, store32(&value)?);
    }
    let morale = number(ratio)?
        .mul(&number(word(org, 0x60)?)?, pc)
        .map_err(numeric)?;
    org.words.insert(0x5c, store32(&morale)?);
    Ok(())
}

/// `4C2910` KKC_OrganizationLayout::VerifyDefaultLayoutRestrictions.
pub fn verify_default_layout<D: PartialEq>(
    loaded: &LayoutDefinition<D>,
    default: &LayoutDefinition<D>,
    world: &mut impl OrganizationWorld<D>,
) -> Result<bool> {
    let info = world.layout_info(&default.definition)?;
    let log = |line, format, args| Diagnostic {
        function: 0x404f30,
        line,
        message: Message::new(format, args),
    };
    if default.definition != loaded.definition {
        world.log(log(
            0x115,
            "Layout mismatches LayoutData '%s' defined by the default",
            vec![Arg::Text(info.name.clone())],
        ))?;
        return Ok(false);
    }
    for (i, entry) in info.entries.iter().enumerate() {
        let (Some(param), Some(default_param)) =
            (loaded.parameters.get(i), default.parameters.get(i))
        else {
            return Err(LoadError::Unsupported(
                "layout parameter array shorter than its LayoutData".into(),
            ));
        };
        if entry.locked && param != default_param {
            world.log(log(
                0x122,
                "Layout is changing a locked slot defined by the default: LayoutData '%s'",
                vec![Arg::Text(info.name.clone())],
            ))?;
            return Ok(false);
        }
        if entry.required && param.is_none() {
            world.log(log(
                0x129,
                "Layout is missing a required slot: Property %s LayoutData '%s'",
                vec![
                    Arg::Text(entry.property_name.clone()),
                    Arg::Text(info.name.clone()),
                ],
            ))?;
            return Ok(false);
        }
        let Some(param) = param else {
            if default_param.is_some() {
                world.log(log(
                    0x132,
                    "Layout is removing an element defaulted by the default (therefore required): Property %s LayoutData '%s'",
                    vec![Arg::Text(entry.property_name.clone()), Arg::Text(info.name.clone())],
                ))?;
                return Ok(false);
            }
            continue;
        };
        if !world.has_property(param, entry.property)? {
            let name = world.name(param)?;
            world.log(log(
                0x13b,
                "Layout element %s missing property %s: LayoutData '%s'",
                vec![
                    Arg::Text(name),
                    Arg::Text(entry.property_name.clone()),
                    Arg::Text(info.name.clone()),
                ],
            ))?;
            return Ok(false);
        }
    }
    Ok(true)
}

/// `4C2D90` (load direction): a formation index into LayoutData `+34`.
fn read_formation(c: &mut Cursor<'_>, info: &LayoutInfo) -> Result<u32> {
    let index = c.i32()?;
    if index < 0 {
        return Ok(0);
    }
    if index as usize >= info.formations.len() {
        return Err(LoadError::Fatal(Message::new(
            "Bad desired formation index %d when only %d formations",
            vec![Arg::Int(index), Arg::Int(info.formations.len() as i32)],
        )));
    }
    Ok(info.formations[index as usize])
}

/// `57CBA0(slot, member)`: add a member (load path).
pub fn add_member<D: Clone>(
    org: &mut OrganizationState<D>,
    slot: i32,
    member: ActorId,
    owner: &OwnerFacts<D>,
    world: &mut impl OrganizationWorld<D>,
) -> Result<()> {
    add_member_fields(
        &mut add_member_core::Held { org, owner, world },
        slot,
        member,
    )
}

/// `577F30(byte)`.
fn formation_byte<D>(
    org: &mut OrganizationState<D>,
    value: u8,
    world: &mut impl OrganizationWorld<D>,
) -> Result<()> {
    org.bytes.insert(0x8c, value);
    let Some(descriptor) = world.state_group4()? else {
        return Ok(());
    };
    let old = org.bytes.get(&0x8d).copied().unwrap_or(0);
    match descriptor.word_24 {
        0 => {
            org.bytes.insert(0x8d, 0);
        }
        1 => {
            org.bytes.insert(0x8d, 1);
        }
        2 => {
            org.bytes.insert(0x8d, value);
        }
        _ => {}
    }
    if org.bytes.get(&0x8d).copied().unwrap_or(0) == 0 {
        org.bytes.insert(0x8e, 0);
    } else if old == 0 {
        org.bytes.insert(0x8e, 1);
    }
    Ok(())
}

/// `57BF50(formation)`: select the secondary formation `+74`.
fn select_secondary<D>(
    org: &mut OrganizationState<D>,
    formation: u32,
    owner: ActorId,
    info: &LayoutInfo,
    world: &mut impl OrganizationWorld<D>,
) -> Result<()> {
    let current = word(org, 0x74)?;
    if formation == current {
        return Ok(());
    }
    if current != 0 {
        let (_, property) = world.formation_facts(current)?;
        if property != 0 {
            world.remove_property(property, owner)?;
        }
        org.words.insert(0x74, 0);
    } else if world.has_applied_property(info.property_50, owner)? {
        world.remove_property(info.property_50, owner)?;
    }
    if formation != 0 {
        org.words.insert(0x74, formation);
        let (_, property) = world.formation_facts(formation)?;
        if property != 0 {
            world.apply_property(property, 4, owner)?;
        }
    } else {
        world.apply_property(info.property_50, 4, owner)?;
    }
    world.formation_refresh(org)?;
    if let Some(object) = world.owner_object_164()? {
        world.object_refresh(object)?;
    }
    Ok(())
}

/// `57BFF0(formation)`: set the desired formation `+70`.
pub fn set_formation<D>(
    org: &mut OrganizationState<D>,
    formation: u32,
    owner: ActorId,
    world: &mut impl OrganizationWorld<D>,
) -> Result<()> {
    org.words.insert(0x70, formation);
    let Some(descriptor) = world.state_group4()? else {
        return Ok(());
    };
    let info = world.layout_info(&org.layout.definition)?;
    let indexed = |index: i32| -> Result<u32> {
        usize::try_from(index)
            .ok()
            .and_then(|i| info.formations.get(i).copied())
            .ok_or_else(|| {
                LoadError::Unsupported(format!("LayoutData formation index {index} outside +34"))
            })
    };
    let secondary = match descriptor.word_20 {
        0 => Some(word(org, 0x70)?),
        1 => Some(0),
        2 => Some(indexed(info.formation_44)?),
        3 => Some(indexed(info.formation_48)?),
        _ => None,
    };
    if let Some(f) = secondary {
        select_secondary(org, f, owner, &info, world)?;
    }
    let desired = word(org, 0x70)?;
    let value = if desired != 0 {
        world.formation_facts(desired)?.0
    } else {
        0
    };
    formation_byte(org, value, world)
}

/// `57D130` Organization Load on the constructed state. Returns the bytes
/// consumed. `owner` and `facts` are memory facts; every ordered native call
/// goes through `world`.
pub fn load<D: Clone + PartialEq>(
    org: &mut OrganizationState<D>,
    bytes: &[u8],
    owner: &OwnerFacts<D>,
    facts: WorldFacts,
    world: &mut impl OrganizationWorld<D>,
    pc: Precision,
) -> Result<usize> {
    let mut c = Cursor::new(bytes);
    let owner_id = org.header.owner;
    let owner_name = world.name(&owner.definition)?;
    // 50BCD0 Updatable Load.
    let updatable = c.version(1)?;
    org.header.scheduled_head = None;
    if updatable >= 1 {
        let id = c.i32()?;
        if id >= 0 {
            let bound = world.updatable_table_word0()?;
            if id > bound || world.updatable_slot_occupied(id)? {
                return Err(LoadError::Fatal(Message::new(
                    "Bad or duplicate updatable ID %d. There are %d.",
                    vec![Arg::Int(id), Arg::Int(bound)],
                )));
            }
            world.register_updatable(id)?;
        }
    }
    let version = c.version(10)?;
    // [+10] virtual +8: 4C3180.
    load_layout(&mut c, &mut org.layout, world)?;
    let default_definition = owner.default_layout.definition.clone();
    if org.layout.definition != default_definition {
        let loaded = world.layout_info(&org.layout.definition)?;
        let default = world.layout_info(&default_definition)?;
        if loaded.parameter_count() != default.parameter_count() {
            return Err(LoadError::Warning(Message::new(
                "Changing number of slots for layout data (%s -> %s)",
                vec![Arg::Text(loaded.name), Arg::Text(default.name)],
            )));
        }
        // 4C1DB0: replace the LayoutData (its log needs a count change).
        if default.parameter_count() != org.layout.parameters.len() {
            world.log(Diagnostic {
                function: 0x404ea0,
                line: 0x1de,
                message: Message::new(
                    "Changing number of slots for layout data (%s -> %s)",
                    vec![Arg::Text(loaded.name), Arg::Text(default.name.clone())],
                ),
            })?;
        }
        org.layout.definition = default_definition;
        org.layout.slots = default.slot_count();
    }
    let info = world.layout_info(&org.layout.definition)?;
    let slots = info.slot_count();
    // 430240 / 430240 / 45F180 / 45D8A0 + count.
    org.members.clear();
    org.members.resize(slots, ActorId::NULL);
    org.words_34.clear();
    org.words_34.resize(slots, 0);
    org.words_50.clear();
    org.words_50.resize(slots, 0);
    org.offsets_78.resize(slots, [F32::ZERO; 2]);
    recompute_maximum_morale(org, &info, owner_id, world, pc)?;
    org.words.insert(0xd8, 0x7f7f_ffff);
    if !verify_default_layout(&org.layout, &owner.default_layout, world)? {
        return Err(LoadError::Warning(Message::new(
            "Organization layout no longer adheres to default layout",
            vec![],
        )));
    }
    let saved_slots = c.i32()?;
    if saved_slots as i64 != slots as i64 {
        return Err(LoadError::Warning(Message::new(
            "Layout size changed from %d -> %d",
            vec![Arg::Int(saved_slots), Arg::Int(slots as i32)],
        )));
    }
    // (+2C == layout slot count holds after 430240: assertion line 0x1F0.)
    for _ in 0..slots {
        let slot = c.i32()?;
        if slot < 0 {
            continue;
        }
        // 4FC200 -> 4FAE80.
        let base = c.position();
        let (thing, consumed) = world.load_member_record(c.rest()).map_err(|e| match e {
            LoadError::Truncated { at, wanted } => LoadError::Truncated {
                at: base + at,
                wanted,
            },
            e => e,
        })?;
        c.skip(consumed)?;
        let Some(member) = thing else {
            return Err(LoadError::Warning(Message::new(
                "Failed to load %s %s",
                vec![
                    Arg::Text("element for organization".into()),
                    Arg::Text(owner_name),
                ],
            )));
        };
        if !world.assert_latch()? && !world.is_actor(member)? {
            let latch = world.assert_dialog()?;
            world.set_assert_latch(latch)?;
        }
        let facts = world.member_facts(member)?;
        if facts.definition_flags & 0x100 == 0 {
            return Err(LoadError::Fatal(Message::new(
                "Element '%s' of organization '%s' has no element component",
                vec![
                    Arg::Text(world.name(&facts.definition)?),
                    Arg::Text(owner_name),
                ],
            )));
        }
        add_member(org, slot, member, owner, world)?;
    }
    // (+38 == +2C: assertion line 0x20A holds after the resizes.)
    if (2..=6).contains(&version) {
        for j in 0..slots {
            let index = c.i32()?;
            if index < 0 {
                continue;
            }
            let kingdom = world
                .owner_kingdom()?
                .ok_or(LoadError::NullDereference(0x57d440))?;
            if index < world.recruit_single_count(kingdom)? {
                let kingdom = world
                    .owner_kingdom()?
                    .ok_or(LoadError::NullDereference(0x4e13f8))?;
                org.words_34[j] = world.recruit_single_at(kingdom, index)?;
            }
            if org.words_34[j] == 0 {
                world.log(Diagnostic {
                    function: 0x5a1040,
                    line: 0,
                    message: Message::new(
                        "org %d recruit item single '%d' not found",
                        vec![Arg::Int(owner.gtid as i32), Arg::Int(index)],
                    ),
                })?;
            }
        }
    }
    if version >= 7 {
        for j in 0..slots {
            let Some(reference) = read_actor_ref(&mut c, world)? else {
                continue;
            };
            let kingdom = world
                .owner_kingdom()?
                .ok_or(LoadError::NullDereference(0x4e2a69))?;
            let item = world.find_recruit_single(kingdom, &reference)?;
            org.words_34[j] = item;
            if item == 0 {
                return Err(LoadError::Warning(Message::new(
                    "Recruit item single '%s' not found",
                    vec![Arg::Text(world.name(&reference)?)],
                )));
            }
            let member = org.members[j];
            if !member.is_null() {
                let [actor, _] = world.recruit_item_words(item)?;
                let gtid = world.member_facts(member)?.gtid;
                if actor == 0 {
                    return Err(LoadError::Warning(Message::new(
                        "Recruit item single '%s' not paired to actor %d",
                        vec![Arg::Text(world.name(&reference)?), Arg::Int(gtid as i32)],
                    )));
                }
                if actor != gtid {
                    return Err(LoadError::Assertion {
                        line: 0x234,
                        expression: "m_recruit_items[i]->GetGTIDActor() == m_elements[i]->GetID()",
                    });
                }
                world.pair_recruit_item(item, member)?;
            }
            let [_, organization] = world.recruit_item_words(item)?;
            if organization != owner.gtid {
                return Err(LoadError::Assertion {
                    line: 0x240,
                    expression: "m_recruit_items[i]->GetGTIDOrganization() == GetActor()->GetID()",
                });
            }
        }
    }
    let leader = c.i32()?;
    if leader as i64 >= slots as i64 {
        return Err(LoadError::Fatal(Message::new(
            "Bad leader position %d when only %d elements for organization '%s'",
            vec![
                Arg::Int(leader),
                Arg::Int(slots as i32),
                Arg::Text(owner_name),
            ],
        )));
    }
    let leader = if leader < 0 {
        None
    } else {
        Some(org.members[leader as usize]).filter(|m| !m.is_null())
    };
    org.words.insert(0x40, actor_word(leader));
    // 59E300: saved multiplier * maximum.
    let multiplier = c.u32()?;
    let morale = number(multiplier)?
        .mul(&number(word(org, 0x60)?)?, pc)
        .map_err(numeric)?;
    org.words.insert(0x5c, store32(&morale)?);
    let desired = read_formation(&mut c, &info)?;
    org.words.insert(0x70, desired);
    if version >= 3 {
        let secondary = read_formation(&mut c, &info)?;
        org.words.insert(0x74, secondary);
    }
    if version == 0 {
        c.u32()?;
    }
    for k in 0..org.offsets_78.len() {
        // Two reads, each straight into the entry.
        org.offsets_78[k][0] = F32(c.u32()?);
        org.offsets_78[k][1] = F32(c.u32()?);
    }
    if version >= 1 {
        for k in 0..org.words_50.len() {
            org.words_50[k] = c.u32()?;
        }
    }
    if version >= 4 {
        let w84 = c.u32()?;
        org.words.insert(0x84, w84);
        let b8c = c.u8()?;
        org.bytes.insert(0x8c, b8c);
        let b8d = c.u8()?;
        org.bytes.insert(0x8d, b8d);
        // 4C0BA0 damage record +90..+AB.
        let w90 = c.u32()?;
        org.words.insert(0x90, w90);
        let w94 = c.u32()?;
        org.words.insert(0x94, w94);
        let ids = c.string()?;
        let damage = if ids.is_empty() {
            0
        } else {
            world.damage_type(&ids)?.ok_or_else(|| {
                LoadError::Exception(Message::new(
                    "Unknown IDS %s for class %hs",
                    vec![Arg::Text(text(&ids)), Arg::Text("KKC_DamageType".into())],
                ))
            })?
        };
        org.words.insert(0x98, damage);
        let w9c = c.u32()?;
        org.words.insert(0x9c, w9c);
        let wa0 = c.u32()?;
        org.words.insert(0xa0, wa0);
        if facts.world_version >= 0xf {
            if facts.world_version >= 0x12 {
                let wa4 = c.u32()?;
                org.words.insert(0xa4, wa4);
            } else {
                c.u8()?;
            }
        }
        if facts.world_version >= 0x14 {
            let wa8 = c.u32()?;
            org.words.insert(0xa8, wa8);
        }
    }
    if version >= 5 {
        let w = c.u32()?;
        org.words.insert(0x88, w);
    }
    if version >= 6 {
        org.created_at = F32(c.u32()?);
    }
    if version >= 8 {
        let [x, y] = c.pair()?;
        org.position_xy = [F32(x), F32(y)];
    }
    if version >= 9 {
        let w = c.u32()?;
        org.words.insert(0xd0, w);
    }
    if version >= 10 {
        let w = c.u32()?;
        org.words.insert(0xd4, w);
    }
    if compare32(org.created_at.0, facts.time_e4) == Some(Ordering::Greater) {
        org.created_at = F32::ZERO;
    }
    org.words.insert(0x74, 0);
    let desired = word(org, 0x70)?;
    set_formation(org, desired, owner_id, world)?;
    Ok(c.position())
}

/// Organization `+B4` and `+BC/+C0` live in the typed fields of
/// [`OrganizationState`] (`created_at`, `position_xy`); this maps a native
/// word offset to the value the loader stores there, for fixtures.
pub fn native_word(org: &OrganizationState<impl Sized>, offset: u32) -> Option<u32> {
    match offset {
        0xb4 => Some(org.created_at.0),
        0xb8 => Some(org.definition_value_308.0),
        0xbc => Some(org.position_xy[0].0),
        0xc0 => Some(org.position_xy[1].0),
        _ => org.words.get(&offset).copied(),
    }
}
/// Leader `+40` as an actor reference.
pub fn leader(org: &OrganizationState<impl Sized>) -> Option<ActorId> {
    org.words.get(&0x40).and_then(|&w| as_actor(w))
}
/// Primary `+44` as an actor reference.
pub fn primary(org: &OrganizationState<impl Sized>) -> Option<ActorId> {
    org.words.get(&0x44).and_then(|&w| as_actor(w))
}
