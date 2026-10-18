//! Native saved Mind Load (actor component slot 4), applied to a Mind that the
//! original factory 4CF480 has already constructed. Base Mind Load 52C580 and
//! the Element subclass Load 536340 (with the owned-path Load 52D2C0 at +D4)
//! are ported. Regiment528C20 is decoded throughv21; its contextless application
//! currently supportsv0/v1. Decoding is pure; `apply` validates unported branches before
//! the first mutation, so an error leaves the installed Mind untouched (the
//! original aborts the whole map load on any of these errors).
use crate::{
    Arena, StateId,
    construct::{Installed, Kind, Reference},
};
use aa_navigation::PathNode;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("saved Mind payload ends early: {wanted} bytes needed at byte {offset}")]
    ShortRead { offset: usize, wanted: usize },
    /// 8589B0: `value < 0 || value > max` reports "Bad data version" (41DFB0).
    #[error("Bad data version {value} (maximum {max}) ending at byte {offset}")]
    BadVersion { value: i32, max: i32, offset: usize },
    /// 52C580 reaches 5A0910 `KKC_MapResource::ThrowCancelException`
    /// ("Map Load Canceled") when a saved stack name matches no state in the
    /// Mind's virtual +19C list. `offset` is the byte after that name.
    #[error(
        "Map Load Canceled: saved Mind state {name:?} is not in the Mind's state list (byte {offset})"
    )]
    Canceled { name: String, offset: usize },
    #[error("saved Mind Load requires unported native behaviour: {0}")]
    Unsupported(&'static str),
    #[error("saved Mind Load does not match the installed Mind: {0}")]
    Mismatch(String),
}
pub type Result<T> = std::result::Result<T, Error>;

/// Updatable Load 50BCD0 at Mind+8: version (max 1), then a signed table ID.
/// 50BCD0 first cancels the scheduled update (50B250); nonnegative IDs are
/// registered in the world Updatable table by the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Updatable {
    pub version: u32,
    pub serialized_id: Option<i32>,
}

/// Base Mind Load 52C580.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Base {
    pub updatable: Updatable,
    pub version: u32,
    pub word_24: u32,
    pub word_28: u32,
    /// Signed command count; each positive entry is a 4D9C80 command with its
    /// own virtual+8 Load. Negative counts read no commands.
    pub command_count: i32,
    /// Pending commands in native head-to-tail order (52C580 appends).
    pub commands: Vec<crate::saved_command::Command>,
    /// Version >= 1.
    pub word_2c: Option<u32>,
    /// Version >= 1: the stack, head first, as concrete Mind state offsets resolved
    /// through the state list (names are compared as NUL-terminated strings).
    pub stack: Option<Vec<u32>>,
    /// Version >= 3: nonzero constructs a 4D9C80 current command.
    pub current_command_flag: Option<u8>,
    pub current_command: Option<crate::saved_command::Command>,
}

/// One saved 52D2C0 record: 8+8+4+4 (+4 discarded) +8 bytes, stored by the
/// node constructor 52D000 as the 8-word node payload.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedPathRecord {
    pub node: PathNode,
    /// The fifth read lands in a stack local and is never stored.
    pub discarded: u32,
}
/// Owned path Load 52D2C0 (vtable A34738 +4) on Element+D4.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedPath {
    pub version: u32,
    /// Saved order. Each record is pushed to the front, so the loaded path
    /// is this list reversed.
    pub records: Vec<SavedPathRecord>,
    pub word_18: u32,
}
impl SavedPath {
    /// Head-first loaded nodes (52D000 head insertion).
    pub fn loaded_nodes(&self) -> Vec<PathNode> {
        self.records.iter().rev().map(|r| r.node).collect()
    }
    /// Collection +0C/+10 after the Load: reset to 0, then counted per record.
    pub fn kind_counts(&self) -> (i32, i32) {
        let mut counts = (0i32, 0i32);
        for record in &self.records {
            match record.node.words[4] {
                1 => counts.0 = counts.0.wrapping_add(1),
                2 => counts.1 = counts.1.wrapping_add(1),
                _ => {}
            }
        }
        counts
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Store {
    Word { offset: u32, value: u32 },
    Byte { offset: u32, value: u8 },
}

/// Element Mind Load 536340.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Element {
    pub version: u32,
    /// Absent for version 0 (536340 returns before the base Load).
    pub base: Option<Base>,
    /// Scalar stores in native order at their native Element offsets,
    /// including embedded state status words (state offset + 0x0C) and the
    /// version < 10 movement fix-up. Path and text stores are separate.
    pub stores: Vec<Store>,
    pub path: Option<SavedPath>,
    /// Version >= 4: 41D290 text for the uai_bombard CString at +120.
    pub bombard_text: Option<Vec<u16>>,
    /// Legacy bytes read into stack locals and discarded, in order.
    pub discarded: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Saved {
    Element(Element),
    Regiment(crate::regiment_load::Regiment),
}

/// Element virtual +19C (531C30): the states a saved stack name can resolve
/// to, in list order. uai_unload (+34) is deliberately absent natively.
pub const ELEMENT_STATE_LIST: [(u32, &str); 10] = [
    (0x48, "uai_deploy"),
    (0x68, "uai_sleep"),
    (0x7c, "uai_move"),
    (0xf8, "uai_kill"),
    (0x10c, "uai_bombard"),
    (0x148, "uai_repair"),
    (0x12c, "uai_engage"),
    (0x164, "uai_join"),
    (0x178, "uai_reattach"),
    (0x188, "uai_unpack"),
];
/// 52C580 replaces an empty saved name (`_wcsicmp(name, L"") == 0`) by this.
pub const EMPTY_STATE_NAME: &str = "cai_rampage";
/// Element +D4 owned path (52D2C0) and its fields.
pub const ELEMENT_PATH: u32 = 0xd4;
pub const ELEMENT_BOMBARD_TEXT: u32 = 0x120;

pub(crate) struct Reader<'a> {
    pub(crate) bytes: &'a [u8],
    pub(crate) at: usize,
}
impl<'a> Reader<'a> {
    pub(crate) fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .filter(|end| *end <= self.bytes.len());
        let end = end.ok_or(Error::ShortRead {
            offset: self.at,
            wanted: n,
        })?;
        let value = &self.bytes[self.at..end];
        self.at = end;
        Ok(value)
    }
    pub(crate) fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(
            self.take(2)?.try_into().expect("2 bytes"),
        ))
    }
    pub(crate) fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(
            self.take(4)?.try_into().expect("4 bytes"),
        ))
    }
    pub(crate) fn version(&mut self, max: i32) -> Result<u32> {
        let value = self.u32()? as i32;
        if value < 0 || value > max {
            return Err(Error::BadVersion {
                value,
                max,
                offset: self.at,
            });
        }
        Ok(value as u32)
    }
    /// 41D290 / the 52C580 name loop: u16 unit count, then the units.
    pub(crate) fn units(&mut self) -> Result<Vec<u16>> {
        let count = usize::from(self.u16()?);
        Ok(self
            .take(2 * count)?
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect())
    }
}

/// Native comparison text: the CString as a NUL-terminated wide string.
fn c_string(units: &[u16]) -> &[u16] {
    let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
    &units[..end]
}

fn resolve_state(units: &[u16], list: &[(u32, &str)], offset: usize) -> Result<u32> {
    let mut name = c_string(units).to_vec();
    if name.is_empty() {
        name = EMPTY_STATE_NAME.encode_utf16().collect();
    }
    list.iter()
        .find(|(_, candidate)| candidate.encode_utf16().eq(name.iter().copied()))
        .map(|(state, _)| *state)
        .ok_or_else(|| Error::Canceled {
            name: String::from_utf16_lossy(&name),
            offset,
        })
}

pub(crate) fn base(r: &mut Reader<'_>, list: &[(u32, &str)]) -> Result<Base> {
    let version = r.version(1)?;
    let updatable = Updatable {
        version,
        serialized_id: if version >= 1 {
            Some(r.u32()? as i32)
        } else {
            None
        },
    };
    let version = r.version(3)?;
    let word_24 = r.u32()?;
    let word_28 = r.u32()?;
    let command_count = r.u32()? as i32;
    let mut commands = Vec::new();
    for _ in 0..command_count.max(0) {
        commands.push(crate::saved_command::read(r)?);
    }
    let (mut word_2c, mut stack, mut current_command_flag) = (None, None, None);
    let mut current_command = None;
    if version >= 1 {
        word_2c = Some(r.u32()?);
        let count = r.u32()? as i32;
        let mut states = Vec::new();
        for _ in 0..count.max(0) {
            let units = r.units()?;
            states.push(resolve_state(&units, list, r.at)?);
        }
        stack = Some(states);
    }
    if version >= 3 {
        let flag = r.u8()?;
        if flag != 0 {
            current_command = Some(crate::saved_command::read(r)?);
        }
        current_command_flag = Some(flag);
    }
    Ok(Base {
        updatable,
        version,
        word_24,
        word_28,
        command_count,
        commands,
        word_2c,
        stack,
        current_command_flag,
        current_command,
    })
}

pub(crate) fn path(r: &mut Reader<'_>) -> Result<SavedPath> {
    let version = r.version(0)?;
    let count = r.u32()? as i32;
    let mut records = Vec::new();
    for _ in 0..count.max(0) {
        let mut words = [0u32; 8];
        for word in &mut words[0..4] {
            *word = r.u32()?;
        }
        words[4] = r.u32()?;
        words[5] = r.u32()?;
        let discarded = r.u32()?;
        words[6] = r.u32()?;
        words[7] = r.u32()?;
        records.push(SavedPathRecord {
            node: PathNode { words },
            discarded,
        });
    }
    let word_18 = r.u32()?;
    Ok(SavedPath {
        version,
        records,
        word_18,
    })
}

fn element(r: &mut Reader<'_>) -> Result<Element> {
    let version = r.version(14)?;
    let mut saved = Element {
        version,
        base: None,
        stores: Vec::new(),
        path: None,
        bombard_text: None,
        discarded: Vec::new(),
    };
    if version == 0 {
        return Ok(saved);
    }
    saved.base = Some(base(r, &ELEMENT_STATE_LIST)?);
    if version < 2 {
        return Ok(saved);
    }
    let v = version;
    let stores = &mut saved.stores;
    let words = |r: &mut Reader<'_>, stores: &mut Vec<Store>, offsets: &[u32]| -> Result<()> {
        for offset in offsets {
            stores.push(Store::Word {
                offset: *offset,
                value: r.u32()?,
            });
        }
        Ok(())
    };
    let byte = |r: &mut Reader<'_>, stores: &mut Vec<Store>, offset: u32| -> Result<()> {
        stores.push(Store::Byte {
            offset,
            value: r.u8()?,
        });
        Ok(())
    };
    words(r, stores, &[0x1a8, 0x74])?;
    if v >= 9 {
        words(r, stores, &[0x78])?;
    }
    // +88 4, +8C 4, +90 8, +98 8, +A0, +A4, +A8, +AC, +B0 8, +D0.
    words(
        r,
        stores,
        &[
            0x88, 0x8c, 0x90, 0x94, 0x98, 0x9c, 0xa0, 0xa4, 0xa8, 0xac, 0xb0, 0xb4, 0xd0,
        ],
    )?;
    if v >= 12 {
        words(r, stores, &[0xf4])?;
    }
    saved.path = Some(path(r)?);
    if v >= 10 {
        // 552CC0: +B8 (stored last), +BC, +C0 8; then +C8 8.
        words(r, stores, &[0xb8, 0xbc, 0xc0, 0xc4, 0xc8, 0xcc])?;
    } else {
        let a8 = stores.iter().rev().find_map(|s| match s {
            Store::Word {
                offset: 0xa8,
                value,
            } => Some(*value),
            _ => None,
        });
        if a8 == Some(2) {
            stores.extend([
                Store::Word {
                    offset: 0xa8,
                    value: 1,
                },
                Store::Word {
                    offset: 0xb8,
                    value: 0,
                },
                Store::Word {
                    offset: 0xc8,
                    value: 0,
                },
                Store::Word {
                    offset: 0xcc,
                    value: 0xbf80_0000,
                },
            ]);
        }
    }
    if (5..7).contains(&v) {
        saved.discarded.push(r.u8()?);
    }
    if v >= 7 {
        words(r, stores, &[0xf0])?;
    }
    words(r, stores, &[0x104, 0x108])?;
    if v < 4 {
        saved.discarded.push(r.u8()?);
        // 4, 4, 8, 4, 8 bytes into zeroed stack locals.
        saved.discarded.extend_from_slice(r.take(28)?);
    }
    words(r, stores, &[0x138, 0x13c])?;
    saved.discarded.extend_from_slice(r.take(4)?);
    words(r, stores, &[0x140])?;
    byte(r, stores, 0x144)?;
    words(r, stores, &[0x154])?;
    byte(r, stores, 0x158)?;
    words(r, stores, &[0x15c])?;
    if v >= 8 {
        byte(r, stores, 0x160)?;
    }
    words(r, stores, &[0x170, 0x174, 0x184])?;
    if v >= 3 {
        words(r, stores, &[0x1ac])?;
    }
    if v >= 4 {
        words(r, stores, &[0x118])?;
        if v < 6 {
            saved.discarded.push(r.u8()?);
        }
        words(r, stores, &[0x11c, 0x124, 0x128])?;
        saved.bombard_text = Some(r.units()?);
    }
    if v >= 9 {
        words(r, stores, &[0x194, 0x198, 0x19c])?;
        if v >= 13 {
            words(r, stores, &[0x1a0, 0x1a4])?;
        }
    }
    if v >= 11 {
        words(r, stores, &[0x44, 0x58, 0x5c, 0x60, 0x64])?;
    }
    if v >= 14 {
        words(r, stores, &[0x40, 0x54])?;
    }
    Ok(saved)
}

/// Decode the saved Mind of the kind the original factory constructed
/// (ActorData+174 bit 7 Regiment, bit 23 Aircraft, bit 8 Element). Returns
/// the payload and the bytes consumed.
pub fn decode(kind: Kind, bytes: &[u8]) -> Result<(Saved, usize)> {
    let mut r = Reader { bytes, at: 0 };
    let saved = match kind {
        Kind::Element => Saved::Element(element(&mut r)?),
        Kind::Regiment => Saved::Regiment(crate::regiment_load::read(&mut r)?),
        Kind::Aircraft => {
            return Err(Error::Unsupported(
                "Aircraft Mind Load 51C3C0 (no original map saves one)",
            ));
        }
        Kind::Base => {
            return Err(Error::Unsupported(
                "the factory constructs no plain base Mind",
            ));
        }
    };
    Ok((saved, r.at))
}

impl Saved {
    /// Shared 52C580 payload, independent of the concrete Mind subclass.
    pub fn base(&self) -> Option<&Base> {
        match self {
            Self::Element(e) => e.base.as_ref(),
            Self::Regiment(r) => r.base.as_ref(),
        }
    }
    pub fn updatable(&self) -> Option<&Updatable> {
        self.base().map(|b| &b.updatable)
    }
}

/// Apply a decoded saved Mind to the Mind constructed for the same actor.
/// Returns the serialized Updatable ID for the caller's world table (only
/// nonnegative IDs register; 50BCD0). No callback, RNG draw or scheduling
/// occurs: 50B250's cancel leaves `update_head` empty.
/// What [`apply`] hands back to the caller.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Applied {
    /// Serialized Updatable ID to register (nonnegative only; 50BCD0).
    pub serialized_id: Option<i32>,
    /// Loaded Element +D4 path nodes, head first, when the save holds any.
    /// `Installed` keeps only the collection words (+E0/+E4 counts, +E8
    /// dirty, +EC); the nodes travel to the typed Move owner
    /// (aa-runtime movement_world::Runtime::bind_element_restored).
    pub element_path: Option<Vec<PathNode>>,
}

/// One already-owned live command reference per saved allocation. The caller
/// resolves all source definitions and allocates distinct, nonzero identities
/// before calling `apply_with_commands`. Success transfers these references to
/// the Mind; failure transfers none. No dispatch or update scheduling occurs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CommandBindings {
    pub pending: Vec<u32>,
    pub current: Option<u32>,
}

pub fn apply(saved: &Saved, installed: &mut Installed, arena: &mut Arena) -> Result<Applied> {
    if saved
        .base()
        .is_some_and(|b| !b.commands.is_empty() || b.current_command.is_some())
    {
        return Err(Error::Unsupported(
            "saved commands require resolved owned command bindings",
        ));
    }
    apply_with_commands(saved, installed, arena, &CommandBindings::default())
}

/// Base 52C580 command references finish with one owner each: pending entries
/// append in serialized order; the version-3 current command is independent.
pub fn apply_with_commands(
    saved: &Saved,
    installed: &mut Installed,
    arena: &mut Arena,
    commands: &CommandBindings,
) -> Result<Applied> {
    let saved = match saved {
        Saved::Element(element) => element,
        Saved::Regiment(regiment) => {
            return crate::regiment_load::apply_basic(regiment, installed, arena, commands);
        }
    };
    if installed.kind != Kind::Element {
        return Err(Error::Mismatch(format!(
            "Element payload for a {:?} Mind",
            installed.kind
        )));
    }
    // Validate everything before the first mutation.
    let (count, current) = saved
        .base
        .as_ref()
        .map(|b| (b.commands.len(), b.current_command.is_some()))
        .unwrap_or((0, false));
    if commands.pending.len() != count || commands.current.is_some() != current {
        return Err(Error::Mismatch(
            "saved command bindings do not match the payload".into(),
        ));
    }
    let identities: std::collections::BTreeSet<_> = commands
        .pending
        .iter()
        .copied()
        .chain(commands.current)
        .collect();
    if identities.contains(&0) || identities.len() != count + usize::from(current) {
        return Err(Error::Mismatch(
            "saved command allocations require distinct nonzero identities".into(),
        ));
    }
    let status_state = |offset: u32| -> Option<StateId> {
        offset
            .checked_sub(0x0c)
            .and_then(|state| installed.state_bindings.get(&state).copied())
    };
    for store in &saved.stores {
        match *store {
            Store::Word { offset, .. } => {
                if let Some(state) = status_state(offset) {
                    arena
                        .state(state)
                        .map_err(|e| Error::Mismatch(e.to_string()))?;
                } else if !installed.words.contains_key(&offset) {
                    return Err(Error::Mismatch(format!(
                        "Element field {offset:#x} was not constructed"
                    )));
                }
            }
            Store::Byte { offset, .. } => {
                if !installed.bytes.contains_key(&offset) {
                    return Err(Error::Mismatch(format!(
                        "Element byte {offset:#x} was not constructed"
                    )));
                }
            }
        }
    }
    let mind = arena
        .mind(installed.mind_id)
        .map_err(|e| Error::Mismatch(e.to_string()))?;
    if !mind.pending_commands.is_empty() || mind.current_command.is_some() {
        return Err(Error::Mismatch(
            "constructed Mind already owns commands".into(),
        ));
    }
    let mut stack = Vec::new();
    if let Some(base) = &saved.base
        && let Some(states) = &base.stack
    {
        for offset in states {
            stack.push(Some(*installed.state_bindings.get(offset).ok_or_else(
                || Error::Mismatch(format!("Element state {offset:#x} is not bound")),
            )?));
        }
    }
    if let Some(_path) = &saved.path {
        if installed.references.get(&ELEMENT_PATH)
            != Some(&Reference::NativeSubobject { vtable: 0xa34738 })
        {
            return Err(Error::Mismatch(
                "Element +D4 owned path is not constructed".into(),
            ));
        }
        for offset in [0xd8, 0xdc, 0xe0, 0xe4, 0xec] {
            if !installed.words.contains_key(&offset) {
                return Err(Error::Mismatch(format!(
                    "Element path field {offset:#x} was not constructed"
                )));
            }
        }
        if installed.words[&0xd8] != 0 || installed.words[&0xdc] != 0 {
            return Err(Error::Unsupported(
                "replacing a nonempty constructed Element path",
            ));
        }
        if !installed.bytes.contains_key(&0xe8) {
            return Err(Error::Mismatch(
                "Element path dirty byte was not constructed".into(),
            ));
        }
    }
    if let Some(text) = &saved.bombard_text {
        if !text.is_empty() {
            return Err(Error::Unsupported(
                "nonempty saved uai_bombard text (+120) needs a typed CString owner",
            ));
        }
        if installed.references.get(&ELEMENT_BOMBARD_TEXT) != Some(&Reference::EmptyUtf16) {
            return Err(Error::Mismatch(
                "Element +120 text is not constructed".into(),
            ));
        }
    }

    // Base 52C580.
    let mut serialized_id = None;
    if let Some(base) = &saved.base {
        installed.update_head = None;
        serialized_id = base.updatable.serialized_id.filter(|id| *id >= 0);
        let mind = arena
            .mind_mut(installed.mind_id)
            .map_err(|e| Error::Mismatch(e.to_string()))?;
        mind.words_24_30[0] = base.word_24;
        mind.words_24_30[1] = base.word_28;
        mind.pending_commands
            .extend(commands.pending.iter().copied());
        mind.current_command = commands.current;
        if let Some(word) = base.word_2c {
            mind.words_24_30[2] = word;
            // 763B90 clears the constructed stack before the saved names.
            mind.stack.clear();
            mind.stack.extend(stack);
        }
    }
    // 536340 scalar stores, then the owned path (its position in the native
    // sequence does not interact with any scalar store).
    for store in &saved.stores {
        match *store {
            Store::Word { offset, value } => {
                if let Some(state) = status_state(offset) {
                    arena
                        .state_mut(state)
                        .map_err(|e| Error::Mismatch(e.to_string()))?
                        .status = value;
                } else {
                    installed.words.insert(offset, value);
                }
            }
            Store::Byte { offset, value } => {
                installed.bytes.insert(offset, value);
            }
        }
    }
    let mut element_path = None;
    if let Some(path) = &saved.path {
        // 5189F0 clear, +10 = +0C = 0, +18 = -1, +14 = 1; counted pushes; +18 read.
        let (kind1, kind2) = path.kind_counts();
        installed.words.insert(0xe0, kind1 as u32);
        installed.words.insert(0xe4, kind2 as u32);
        installed.bytes.insert(0xe8, 1);
        installed.words.insert(0xec, path.word_18);
        if !path.records.is_empty() {
            element_path = Some(path.loaded_nodes());
        }
    }
    Ok(Applied {
        serialized_id,
        element_path,
    })
}
