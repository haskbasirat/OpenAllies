//! TGC_AdminConn private join messages, the KKC_Session snapshot and the
//! PlayerJoin decree of the original AA.exe (v14 target; every executed byte
//! window of these functions is identical in the original and v14 images,
//! research/phase8-net/native-lobby.json `executed_windows`).
//!
//! Ported functions (native order and arithmetic kept):
//! * Private message kind 1, session snapshot: wrapper 0x74C270 (1 bit = 1),
//!   writer KKC_Session::SerializeState(O) 0x76D0B0 and reader
//!   KKC_Session::SerializeState(I) 0x77AAC0 with their sub-codecs: launch
//!   settings 0x76B800 / 0x76EBD0, game settings 0x59EB30 / 0x5A1E90, map
//!   settings 0x512CB0 family / 0x513130 family, world conditions 0x511BB0 /
//!   0x512070 (rules 0x511910 / 0x511E50), map header 0x50E200 / 0x510D90
//!   (kingdom setups 0x50DC60 / 0x50FF20), random-map settings 0x505DE0 /
//!   0x506560, launch state 0x784360 / 0x7847B0, game options 0x77B5A0 /
//!   0x77B9D0, property table 0x79C720 / 0x79CC00, address 0x76A7C0 /
//!   0x76A810, player id 0x74EBC0 / 0x752660 (crate::session), 2-bit peer
//!   field 0x74B8D0 / 0x751300, info text 0x77DCF0.
//! * Private message kind 0, admin state: writer 0x74C130 and client handler
//!   0x74DF50 (synchronizer A reset 0x77D900 = aa_orders Synchronizer::reset,
//!   client clock reset 0x75FAC0 and update 0x75FB40).
//! * Client dispatcher of the private history AdminConn+0xAC: 0x7531A0
//!   (kind = 0x411FA0(2): 0 -> 0x74DF50, 1 -> 0x77AAC0).
//! * Host join completion 0x753580 (connected, 0x752300, 0x752490, the two
//!   private messages finalized by 0x74D0A0 and appended to +0xAC).
//! * PlayerJoin decree process 0x75DA80 -> 0x758FD0 with the session-full test
//!   0x76CCA0 (free kingdom count 0x76C850), KKC_Session::CreatePlayer
//!   0x773710 (SessionPlayer ctor 0x7661D0, rated-flag update 0x770280),
//!   duplicate-name rule 0x766560 (0x76CE10 / 0x7664F0) and the host's
//!   PostPlayerJoin request 0x74CA80 (descriptor 307).
//!
//! Native pointers are typed values here: a SessionPlayer's connection is a
//! [`PeerLink`], a definition pointer is `Option<index>` (`None` = null, the
//! writer then sends the registry count), an interned address object
//! (0x783700, map keyed by IPv4 and 16-bit port) is `Option<PeerAddress>`.
//! Node-level effects that the original performs on global objects are
//! returned as [`LobbyAction`]s in native order; effects whose result feeds
//! back into the code (ChangeToState, map status sends, the kingdom
//! auto-assignment, connection lookup for the join decree) go through the
//! [`SnapshotHost`] / [`JoinHost`] traits. Native assertion failures
//! (0x855D30) and null dereferences are typed errors. Errors are not
//! transactional: fields written before the failing read keep their values,
//! as in the original.
//!
//! Native caps found here (all removed or kept as protocol rules): the host
//! completion builds both messages in the static stream 0xB408F4 created with
//! 0x400 bytes (0x7535CD, 0x767110(0x400)) and grown by the writer ->
//! WireWriter; player list, kingdom setups, world rules, game entries and the
//! property table are native linked lists / doubling vectors without a cap ->
//! Vec, bounded on input only by the packet length. The 32-sample client
//! clock array (0x75FAA0, +0xC..+0x8B) is a fixed native field layout (only
//! samples 0 and 1 are ever read), kept as data.
//!
//! Evidence: research/phase8-net/native_lobby.py -> native-lobby.json,
//! replayed by tests/lobby.rs; notes in research/phase8-net/lobby-notes.txt.
use crate::packets::{AdminConn, NodeHistories};
use crate::session::{
    read_player_id, read_text, wcs_eq, wcsicmp_c, write_player_id, write_text, PeerAddress, PlayerId,
    PLAYER_JOIN_DECREE,
};
use crate::{Error, Result};
use aa_orders::commands::WireWriter;
use aa_orders::net::sync::Synchronizer;
use aa_orders::net::transport::{finalize, X87Precision};
use aa_orders::payload::{Field, Registry};
use aa_orders::{registry_bits, Cursor, Packet, Position};
use serde::{Deserialize, Serialize};

/// Private message kinds (0x7531A0: 0x411FA0 with 2 possibilities).
pub const PRIVATE_ADMIN_STATE: u32 = 0;
pub const PRIVATE_SESSION: u32 = 1;
/// TGC_PostPlayerJoinOrderRequest (descriptor object 0xB409A4).
pub const POST_PLAYER_JOIN_REQUEST: usize = 307;
/// 0x40BC50 message kind of "networking_player_joined|s" (0x759300).
pub const PLAYER_JOINED_MESSAGE_KIND: u32 = 0x1e;
pub const PLAYER_JOINED_KEY: &str = "networking_player_joined|s";
/// 0x766560 format key (localized by 0x4B9F40, formatted by 0x405D60).
pub const DUPLICATE_NAME_KEY: &str = "networking_duplicate_name_formate|sd";
/// 0xA8C084: admin tick length 0.0625 s (binary32 0x3D800000).
pub const ADMIN_TICK: f32 = 0.0625;
/// 0x75FB43: clock offset floor -9999999.0 (0xCB18967F).
pub const CLOCK_FLOOR_BITS: u32 = 0xcb18_967f;
/// 0x75FADF: the client clock keeps 4 x 8 binary32 samples.
pub const CLOCK_SAMPLES: usize = 32;
/// World rule name looked up by 0x770280 (0x77036B, wide literal "RATED").
pub const RATED_RULE: &str = "RATED";

/// Definition managers read by these codecs (global pointer addresses).
pub const MAP_DEF_44: Registry = Registry(0xb3aa24);
pub const MAP_DEF_48: Registry = Registry(0xb3aa34);
pub const CONDITIONS: Registry = Registry(0xb3aa70);
pub const WORLD_RULES: Registry = Registry(0xb3aa88);
pub const NATIONS: Registry = Registry(0xb3aa44);
pub const FACTIONS: Registry = Registry(0xb3aa4c);
pub const COLORS: Registry = Registry(0xb3aa60);
pub const AI_HANDICAPS: Registry = Registry(0xb3aa68);
pub const START_PROPERTIES: Registry = Registry(0xb3aa2c);
/// Object definitions; the reader 0x76A960 also requires virtual +0x38.
pub const OBJECT_DEFINITIONS: Registry = Registry(0xb39f58);

// ---------------------------------------------------------------------------
// Game data and primitive codecs
// ---------------------------------------------------------------------------

/// Game data the codecs consult on the node that runs them. Every lookup is
/// explicit; an unavailable value is an error, never a default.
pub trait LobbyData {
    /// `[registry]+0x28` definition count.
    fn registry_count(&self, registry: Registry) -> Result<usize>;
    /// Reader resolution of an index below the count: the array entry is not
    /// null and, for [`OBJECT_DEFINITIONS`], its virtual +0x38 accepts it.
    fn definition(&self, registry: Registry, index: u32) -> bool;
    /// `def+0x20` value kind of a WorldRule definition (0x511910 / 0x511E50).
    fn world_rule_kind(&self, index: u32) -> Result<u32>;
    /// `def+8` name of a WorldRule definition (compared by 0x511A60).
    fn world_rule_name(&self, index: u32) -> Result<Vec<u16>>;
    /// 0x59F570 (0x59F450 with complaints on): scenario by name; the found
    /// object's own name (+0x38, what 0x512DF0 serializes). The key is the
    /// C string (units before the first NUL).
    fn scenario(&mut self, key: &[u16]) -> Option<Vec<u16>>;
    /// 0x41EEC0 random-map configuration by name (its name +8, serialized by
    /// 0x512E30). Called only for a non-empty key (0x41EEC0 returns null for
    /// a leading NUL without searching).
    fn random_map_config(&mut self, key: &[u16]) -> Option<Vec<u16>>;
    /// Caller limit for lists whose entries occupy zero bits (definition
    /// lists over an empty registry); such counts cannot be bounded by the
    /// packet length.
    fn zero_width_list_budget(&self) -> usize;
}

fn c_key(units: &[u16]) -> &[u16] {
    let end = units.iter().position(|&u| u == 0).unwrap_or(units.len());
    &units[..end]
}

fn write_bool(w: &mut WireWriter, value: bool) -> Result<()> {
    w.write_uint(u32::from(value), 1)?;
    Ok(())
}
fn read_bool(r: &mut Cursor<'_>) -> Result<bool> {
    Ok(r.read_uint(1)? == 1)
}
fn write_u32(w: &mut WireWriter, value: u32) -> Result<()> {
    w.write_uint(value, 32)?;
    Ok(())
}
fn read_u32(r: &mut Cursor<'_>) -> Result<u32> {
    Ok(r.read_uint(32)?)
}
/// 0x411FA0(n): `registry_bits(n)` bits; a value >= n trips the native
/// assertion "_data<_n_possibilities" (Serialize.h line 0x18B).
fn read_bounded(r: &mut Cursor<'_>, n: usize) -> Result<u32> {
    Ok(r.read_possibility(n)?)
}

/// Definition reference writer (0x76A880 family): `def+0xC` or, for null,
/// the registry count, in `registry_bits(count + 1)` bits.
pub fn write_def(w: &mut WireWriter, data: &dyn LobbyData, registry: Registry, def: Option<u32>) -> Result<()> {
    let count = data.registry_count(registry)?;
    let width = registry_bits(count.checked_add(1).ok_or(Error::Protocol("registry count".into()))?)?;
    let value = match def {
        None => u32::try_from(count).map_err(|_| Error::Protocol("registry count".into()))?,
        Some(index) if (index as usize) < count => index,
        Some(index) => {
            return Err(Error::Protocol(format!(
                "definition {index} of registry {:#x} is outside its {count} entries",
                registry.0
            )))
        }
    };
    w.write_uint(value, width)?;
    Ok(())
}
/// Definition reference reader: 0x411FA0(count + 1), then the array entry for
/// an index below the count (0x513130, 0x512070, 0x50FF20, 0x50F420, 0x511E50;
/// 0x76A960 for object definitions), null otherwise.
pub fn read_def(r: &mut Cursor<'_>, data: &dyn LobbyData, registry: Registry) -> Result<Option<u32>> {
    let count = data.registry_count(registry)?;
    let raw = read_bounded(r, count.checked_add(1).ok_or(Error::Protocol("registry count".into()))?)?;
    Ok(if (raw as usize) < count && data.definition(registry, raw) { Some(raw) } else { None })
}

fn write_count(w: &mut WireWriter, len: usize) -> Result<()> {
    let value = i32::try_from(len).map_err(|_| Error::Protocol(format!("list of {len} entries")))?;
    w.write_uint(value as u32, 32)?;
    Ok(())
}
/// Native vector count (32 bits). The original allocates and loops over any
/// count; a count the remaining bits cannot hold is a typed error here.
fn read_count(r: &mut Cursor<'_>, min_bits: usize, budget: usize) -> Result<usize> {
    let count = r.read_uint(32)? as usize;
    let remaining = r.remaining_bits();
    if (min_bits == 0 && count > budget) || count.saturating_mul(min_bits) > remaining {
        return Err(Error::Protocol(format!("list of {count} entries does not fit {remaining} remaining bits")));
    }
    Ok(count)
}
fn def_bits(data: &dyn LobbyData, registry: Registry) -> Result<usize> {
    let count = data.registry_count(registry)?;
    Ok(registry_bits(count + 1)? as usize)
}

/// Address word pair 0x76A7C0: ip then port as 32-bit words, 0/0 for a null
/// address object.
pub fn write_address(w: &mut WireWriter, address: Option<PeerAddress>) -> Result<()> {
    let (ip, port) = address.map_or((0, 0), |a| (a.ip, u32::from(a.port)));
    write_u32(w, ip)?;
    write_u32(w, port)
}
/// 0x76A810: both words 0 -> null; otherwise the interned address object
/// 0x783700(ip, port), whose map key is the 16-bit port (0x783240).
pub fn read_address(r: &mut Cursor<'_>) -> Result<Option<PeerAddress>> {
    let ip = read_u32(r)?;
    let port = read_u32(r)?;
    Ok(if ip == 0 && port == 0 { None } else { Some(PeerAddress { ip, port: port as u16 }) })
}

// ---------------------------------------------------------------------------
// Launch settings tree (session +0x54 and +0x88)
// ---------------------------------------------------------------------------

/// Two CStrings at +0 and +4 (0x49AAB0 / 0x49ACD0).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextPair {
    pub first: Vec<u16>,
    pub second: Vec<u16>,
}
impl TextPair {
    fn write(&self, w: &mut WireWriter) -> Result<()> {
        write_text(w, &self.first)?;
        write_text(w, &self.second)
    }
    fn read(&mut self, r: &mut Cursor<'_>) -> Result<()> {
        self.first = read_text(r)?;
        self.second = read_text(r)?;
        Ok(())
    }
}

/// CString with a 32-bit word (8-byte entries of 0x505CA0 / 0x5061C0 and of
/// the session property table 0x79C720 / 0x79CC00).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedWord {
    pub name: Vec<u16>,
    pub value: u32,
}

/// One world rule entry (12 bytes; writer 0x511910, reader 0x511E50).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRule {
    /// +0 WorldRule definition (0xB3AA88); null is dereferenced natively.
    pub rule: Option<u32>,
    /// +4 byte.
    pub enabled: bool,
    /// +8 value; its encoding follows the definition kind (+0x20): 0 bool
    /// (low byte), 1/3 int32 (0x40D7E0 / 0x40D810), 2 binary32 bits, any
    /// other kind is not serialized.
    pub value: u32,
}

/// World conditions object (0x511DF0, 0x1C bytes): +4 kind, +8 condition
/// definition, +0xC rule vector; +0x18 change counter (0x511AF0).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldConditions {
    /// +4, written in 3 bits, read by 0x411FA0(7).
    pub kind: u32,
    /// +8 condition definition (0xB3AA70).
    pub condition: Option<u32>,
    /// +0xC/+0x10/+0x14 rule vector.
    pub rules: Vec<WorldRule>,
    /// +0x18 counter incremented by 0x511AF0 (not serialized).
    pub changes: u32,
}
impl WorldConditions {
    /// 0x511BB0.
    pub fn write(&self, w: &mut WireWriter, data: &dyn LobbyData) -> Result<()> {
        w.write_uint(self.kind, 3)?;
        write_def(w, data, CONDITIONS, self.condition)?;
        write_count(w, self.rules.len())?;
        for rule in &self.rules {
            // 0x511910: the definition is dereferenced for its kind.
            let index = rule.rule.ok_or(Error::Protocol("0x511910 dereferences a null WorldRule".into()))?;
            write_def(w, data, WORLD_RULES, Some(index))?;
            write_bool(w, rule.enabled)?;
            match data.world_rule_kind(index)? {
                0 => write_bool(w, rule.value & 0xff != 0)?,
                1..=3 => write_u32(w, rule.value)?,
                _ => {}
            }
        }
        Ok(())
    }
    /// 0x512070 with the rule vector 0x511F50. The rules are read into one
    /// temporary {null, 1, 0} reused for every entry, so a value that a kind
    /// does not serialize carries over from the previous entry, and a bool
    /// replaces only the low byte.
    pub fn read(&mut self, r: &mut Cursor<'_>, data: &dyn LobbyData) -> Result<()> {
        self.kind = read_bounded(r, 7)?;
        self.condition = read_def(r, data, CONDITIONS)?;
        let min = def_bits(data, WORLD_RULES)? + 1;
        let count = read_count(r, min, data.zero_width_list_budget())?;
        let mut rules = Vec::new();
        rules.try_reserve_exact(count).map_err(|_| Error::Protocol("rule vector allocation".into()))?;
        let mut carry = 0u32;
        for _ in 0..count {
            let rule = read_def(r, data, WORLD_RULES)?;
            let enabled = read_bool(r)?;
            let index = rule.ok_or(Error::Protocol("0x511E50 dereferences a null WorldRule".into()))?;
            match data.world_rule_kind(index)? {
                0 => carry = (carry & !0xff) | u32::from(read_bool(r)?),
                1..=3 => carry = read_u32(r)?,
                _ => {}
            }
            rules.push(WorldRule { rule, enabled, value: carry });
        }
        self.rules = rules;
        Ok(())
    }
    /// 0x511A60: first rule whose definition name equals `name` under
    /// _wcsicmp (C locale).
    fn find_rule(&self, data: &dyn LobbyData, name: &[u16]) -> Result<Option<usize>> {
        for (i, rule) in self.rules.iter().enumerate() {
            let index = rule.rule.ok_or(Error::Protocol("0x511A60 dereferences a null WorldRule".into()))?;
            if wcsicmp_c(name, &data.world_rule_name(index)?) {
                return Ok(Some(i));
            }
        }
        Ok(None)
    }
}

/// Element of the 0x50DF50 / 0x50FB90 vector (12 bytes): CString, pair.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamedPair {
    pub name: Vec<u16>,
    pub pair: TextPair,
}

/// Kingdom setup record (0x6C bytes; writer 0x50DC60, reader 0x50FF20).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomSetup {
    /// +0x00.
    pub name: Vec<u16>,
    /// +0x04 / +0x08.
    pub pair: TextPair,
    /// +0x10 (a set byte excludes the record from 0x76C850 / 0x778D10).
    pub flag_10: bool,
    pub text_14: Vec<u16>,
    /// +0x18 nation (0xB3AA44), +0x1C faction (0xB3AA4C), +0x20 color
    /// (0xB3AA60), +0x24 AI handicap (0xB3AA68).
    pub nation: Option<u32>,
    pub faction: Option<u32>,
    pub color: Option<u32>,
    pub ai_handicap: Option<u32>,
    /// +0x28 (required by 0x76C850 / 0x778D10), +0x29.
    pub flag_28: bool,
    pub flag_29: bool,
    pub text_2c: Vec<u16>,
    pub flag_30: bool,
    /// +0x40 pair vector (0x50D460 / 0x50FD70).
    pub pairs: Vec<TextPair>,
    /// +0x4C start properties (0xB3AA2C; 0x50D4B0 / 0x50F420).
    pub start_properties: Vec<Option<u32>>,
    pub text_58: Vec<u16>,
    /// +0x5C additional actors (object definitions; 0x50D530 / 0x50E050).
    pub additional_actors: Vec<Option<u32>>,
    pub flag_31: bool,
    pub flag_32: bool,
    /// +0x34 32-bit words (0x50D580 / 0x50E230).
    pub words: Vec<u32>,
}
impl KingdomSetup {
    fn write(&self, w: &mut WireWriter, data: &dyn LobbyData) -> Result<()> {
        write_text(w, &self.name)?;
        self.pair.write(w)?;
        write_bool(w, self.flag_10)?;
        write_text(w, &self.text_14)?;
        write_def(w, data, NATIONS, self.nation)?;
        write_def(w, data, FACTIONS, self.faction)?;
        write_def(w, data, COLORS, self.color)?;
        write_def(w, data, AI_HANDICAPS, self.ai_handicap)?;
        write_bool(w, self.flag_28)?;
        write_bool(w, self.flag_29)?;
        write_text(w, &self.text_2c)?;
        write_bool(w, self.flag_30)?;
        write_count(w, self.pairs.len())?;
        for pair in &self.pairs {
            pair.write(w)?;
        }
        write_count(w, self.start_properties.len())?;
        for def in &self.start_properties {
            write_def(w, data, START_PROPERTIES, *def)?;
        }
        write_text(w, &self.text_58)?;
        write_count(w, self.additional_actors.len())?;
        for def in &self.additional_actors {
            write_def(w, data, OBJECT_DEFINITIONS, *def)?;
        }
        write_bool(w, self.flag_31)?;
        write_bool(w, self.flag_32)?;
        write_count(w, self.words.len())?;
        for word in &self.words {
            write_u32(w, *word)?;
        }
        Ok(())
    }
    fn read(&mut self, r: &mut Cursor<'_>, data: &dyn LobbyData) -> Result<()> {
        let budget = data.zero_width_list_budget();
        self.name = read_text(r)?;
        self.pair.read(r)?;
        self.flag_10 = read_bool(r)?;
        self.text_14 = read_text(r)?;
        self.nation = read_def(r, data, NATIONS)?;
        self.faction = read_def(r, data, FACTIONS)?;
        self.color = read_def(r, data, COLORS)?;
        self.ai_handicap = read_def(r, data, AI_HANDICAPS)?;
        self.flag_28 = read_bool(r)?;
        self.flag_29 = read_bool(r)?;
        self.text_2c = read_text(r)?;
        self.flag_30 = read_bool(r)?;
        let n = read_count(r, 32, budget)?;
        let mut pairs = Vec::with_capacity(n);
        for _ in 0..n {
            let mut pair = TextPair::default();
            pair.read(r)?;
            pairs.push(pair);
        }
        self.pairs = pairs;
        let n = read_count(r, def_bits(data, START_PROPERTIES)?, budget)?;
        let mut defs = Vec::with_capacity(n.min(r.remaining_bits() + 1));
        for _ in 0..n {
            defs.push(read_def(r, data, START_PROPERTIES)?);
        }
        self.start_properties = defs;
        self.text_58 = read_text(r)?;
        let n = read_count(r, def_bits(data, OBJECT_DEFINITIONS)?, budget)?;
        let mut defs = Vec::with_capacity(n.min(r.remaining_bits() + 1));
        for _ in 0..n {
            defs.push(read_def(r, data, OBJECT_DEFINITIONS)?);
        }
        self.additional_actors = defs;
        self.flag_31 = read_bool(r)?;
        self.flag_32 = read_bool(r)?;
        let n = read_count(r, 32, budget)?;
        let mut words = Vec::with_capacity(n);
        for _ in 0..n {
            words.push(read_u32(r)?);
        }
        self.words = words;
        Ok(())
    }
}
/// Lower bound of one kingdom record on the wire (definitions excluded).
const KINGDOM_MIN_BITS: usize = 16 + 32 + 1 + 16 + 1 + 1 + 16 + 1 + 32 + 32 + 16 + 32 + 1 + 1 + 32;

/// Map header (0x50E200 / 0x510D90 on the object at +4): +4 name vector,
/// +0x10 kingdom setup vector.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapHeader {
    pub names: Vec<NamedPair>,
    pub kingdoms: Vec<KingdomSetup>,
}
impl MapHeader {
    fn write(&self, w: &mut WireWriter, data: &dyn LobbyData) -> Result<()> {
        write_count(w, self.names.len())?;
        for entry in &self.names {
            write_text(w, &entry.name)?;
            entry.pair.write(w)?;
        }
        write_count(w, self.kingdoms.len())?;
        for kingdom in &self.kingdoms {
            kingdom.write(w, data)?;
        }
        Ok(())
    }
    /// 0x510D90: 0x50FB90 then 0x5104A0. Both vectors take the read size and
    /// their entries are assigned from one reused temporary.
    fn read(&mut self, r: &mut Cursor<'_>, data: &dyn LobbyData) -> Result<()> {
        let budget = data.zero_width_list_budget();
        let n = read_count(r, 48, budget)?;
        let mut names = Vec::with_capacity(n);
        for _ in 0..n {
            let name = read_text(r)?;
            let mut pair = TextPair::default();
            pair.read(r)?;
            names.push(NamedPair { name, pair });
        }
        self.names = names;
        let n = read_count(r, KINGDOM_MIN_BITS, budget)?;
        let mut kingdoms = Vec::with_capacity(n);
        let mut temp = KingdomSetup::default();
        for _ in 0..n {
            temp.read(r, data)?;
            kingdoms.push(temp.clone());
        }
        self.kingdoms = kingdoms;
        Ok(())
    }
}

/// Extended game settings entry (12 bytes at +0x44; 0x5A1360).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameEntry {
    pub flag_4: bool,
    pub text_8: Vec<u16>,
    pub text_0: Vec<u16>,
    pub flag_5: bool,
}

/// Game settings (0x68 bytes at [launch+0x14], ctor 0x5A2210; writer
/// 0x59EB30, reader 0x5A1E90).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameSettings {
    pub pair_20: TextPair,
    pub pair_28: TextPair,
    pub field_10: u32,
    /// +0xC; the extended block is serialized only when it is above 1
    /// (unsigned compare).
    pub version: u32,
    /// [+8] map header, [+4] world conditions.
    pub header: MapHeader,
    pub conditions: WorldConditions,
    pub field_30: u32,
    pub field_34: u32,
    pub text_3c: Vec<u16>,
    pub field_40: u32,
    pub text_38: Vec<u16>,
    /// +0x18/+0x1C: creation time from 0x6FB1F0 (FILETIME) in the ctor.
    pub time_low: u32,
    pub time_high: u32,
    /// +0x44 entries (presence-bit list, terminated by a 0 bit).
    pub entries: Vec<GameEntry>,
}
impl GameSettings {
    /// 0x59EB30.
    pub fn write(&self, w: &mut WireWriter, data: &dyn LobbyData) -> Result<()> {
        self.pair_20.write(w)?;
        self.pair_28.write(w)?;
        write_u32(w, self.field_10)?;
        write_u32(w, self.version)?;
        self.header.write(w, data)?;
        self.conditions.write(w, data)?;
        write_u32(w, self.field_30)?;
        write_u32(w, self.field_34)?;
        if self.version > 1 {
            write_text(w, &self.text_3c)?;
            write_u32(w, self.field_40)?;
            write_text(w, &self.text_38)?;
            write_u32(w, self.time_low)?;
            write_u32(w, self.time_high)?;
            for entry in &self.entries {
                write_bool(w, true)?;
                write_bool(w, entry.flag_4)?;
                write_text(w, &entry.text_8)?;
                write_text(w, &entry.text_0)?;
                write_bool(w, entry.flag_5)?;
            }
            write_bool(w, false)?;
        }
        Ok(())
    }
    /// 0x5A1E90: fields of the extended block keep their values when the
    /// read version is 0 or 1; otherwise the entries are cleared (+0x48 = 0)
    /// and re-read.
    pub fn read(&mut self, r: &mut Cursor<'_>, data: &dyn LobbyData) -> Result<()> {
        self.pair_20.read(r)?;
        self.pair_28.read(r)?;
        self.field_10 = read_u32(r)?;
        self.version = read_u32(r)?;
        self.header.read(r, data)?;
        self.conditions.read(r, data)?;
        self.field_30 = read_u32(r)?;
        self.field_34 = read_u32(r)?;
        if self.version > 1 {
            self.text_3c = read_text(r)?;
            self.field_40 = read_u32(r)?;
            self.text_38 = read_text(r)?;
            self.time_low = read_u32(r)?;
            self.time_high = read_u32(r)?;
            self.entries.clear();
            while read_bool(r)? {
                let flag_4 = read_bool(r)?;
                let text_8 = read_text(r)?;
                let text_0 = read_text(r)?;
                let flag_5 = read_bool(r)?;
                self.entries.push(GameEntry { flag_4, text_8, text_0, flag_5 });
            }
        }
        Ok(())
    }
}

/// Map settings base (0x512CB0 / 0x513130): +4 header, +0x20 conditions,
/// +0x3C/+0x40 map size (SetMapSize 0x7708C0), +0x44 / +0x48 definitions,
/// +0x4C word.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapSettings {
    pub header: MapHeader,
    pub conditions: WorldConditions,
    pub size_x: u32,
    pub size_y: u32,
    /// +0x44 (0xB3AA24), +0x48 (0xB3AA34).
    pub def_44: Option<u32>,
    pub def_48: Option<u32>,
    pub field_4c: u32,
}
impl MapSettings {
    /// 0x512CB0: conditions first, then the header.
    pub fn write(&self, w: &mut WireWriter, data: &dyn LobbyData) -> Result<()> {
        self.conditions.write(w, data)?;
        self.header.write(w, data)?;
        write_u32(w, self.size_x)?;
        write_u32(w, self.size_y)?;
        write_def(w, data, MAP_DEF_44, self.def_44)?;
        write_def(w, data, MAP_DEF_48, self.def_48)?;
        write_u32(w, self.field_4c)
    }
    /// 0x513130.
    pub fn read(&mut self, r: &mut Cursor<'_>, data: &dyn LobbyData) -> Result<()> {
        self.conditions.read(r, data)?;
        self.header.read(r, data)?;
        self.size_x = read_u32(r)?;
        self.size_y = read_u32(r)?;
        self.def_44 = read_def(r, data, MAP_DEF_44)?;
        self.def_48 = read_def(r, data, MAP_DEF_48)?;
        self.field_4c = read_u32(r)?;
        Ok(())
    }
}

/// Random-map settings at +0x54 of the random map settings (0x505DE0 /
/// 0x506560).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RmcSettings {
    pub field_0: u32,
    pub list_4: Vec<NamedWord>,
    pub list_10: Vec<NamedWord>,
    pub field_1c: u32,
}
impl RmcSettings {
    fn write(&self, w: &mut WireWriter) -> Result<()> {
        write_u32(w, self.field_0)?;
        for list in [&self.list_4, &self.list_10] {
            write_count(w, list.len())?;
            for e in list {
                write_text(w, &e.name)?;
                write_u32(w, e.value)?;
            }
        }
        write_u32(w, self.field_1c)
    }
    fn read(&mut self, r: &mut Cursor<'_>, budget: usize) -> Result<()> {
        self.field_0 = read_u32(r)?;
        for list in [&mut self.list_4, &mut self.list_10] {
            let n = read_count(r, 48, budget)?;
            let mut out = Vec::with_capacity(n);
            for _ in 0..n {
                let name = read_text(r)?;
                let value = read_u32(r)?;
                out.push(NamedWord { name, value });
            }
            *list = out;
        }
        self.field_1c = read_u32(r)?;
        Ok(())
    }
}

/// Launch state (0x28 bytes at [launch+0x2C], ctor 0x40F7D0; writer
/// 0x784360, reader 0x7847B0). +0xC is not serialized.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchState {
    /// +0 (4 bits, 0x411FA0(16)).
    pub kind: u32,
    pub flag_04: bool,
    pub text: Vec<u16>,
    pub field_0c: u32,
    pub field_10: u32,
    pub flag_14: bool,
    pub flag_15: bool,
    pub flag_16: bool,
    /// +0x18 (4 bits, 0x411FA0(16)).
    pub field_18: u32,
    pub field_1c: u32,
    pub flag_20: bool,
    /// +0x24 (3 bits, 0x411FA0(7)).
    pub field_24: u32,
}
impl LaunchState {
    /// 0x784360.
    pub fn write(&self, w: &mut WireWriter) -> Result<()> {
        w.write_uint(self.kind, 4)?;
        write_bool(w, self.flag_04)?;
        write_text(w, &self.text)?;
        write_u32(w, self.field_10)?;
        write_bool(w, self.flag_14)?;
        w.write_uint(self.field_24, 3)?;
        write_bool(w, self.flag_15)?;
        write_bool(w, self.flag_16)?;
        w.write_uint(self.field_18, 4)?;
        write_u32(w, self.field_1c)?;
        write_bool(w, self.flag_20)
    }
    /// 0x7847B0.
    pub fn read(&mut self, r: &mut Cursor<'_>) -> Result<()> {
        self.kind = read_bounded(r, 0x10)?;
        self.flag_04 = read_bool(r)?;
        self.text = read_text(r)?;
        self.field_10 = read_u32(r)?;
        self.flag_14 = read_bool(r)?;
        self.field_24 = read_bounded(r, 7)?;
        self.flag_15 = read_bool(r)?;
        self.flag_16 = read_bool(r)?;
        self.field_18 = read_bounded(r, 0x10)?;
        self.field_1c = read_u32(r)?;
        self.flag_20 = read_bool(r)?;
        Ok(())
    }
}

/// Launch settings (session +0x54 and +0x88, ctor 0x76E540; writer
/// 0x76B800, reader 0x76EBD0). +0x10 and +0x30 are not serialized.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchSettings {
    /// +0 `m_session_state` (3 bits, 0x411FA0(6); name from the sync dump
    /// 0x76D6C0): the state KKC_Session::Launch 0x77A950 passes to
    /// ChangeToState; ChangeToState clears it except when it restores the
    /// last settings. 0x770280 / 0x778D10 test it.
    pub game_type: u32,
    /// +4 (3 bits, 0x411FA0(6)): selects the active map settings
    /// (0x76C700: 0 random map +0x20, 2 scenario +0x1C, 5 metagame +0x28,
    /// any other value +0x18).
    pub mode: u32,
    /// +8 map / save file name.
    pub file: Vec<u16>,
    /// +0xC (3 bits, 0x411FA0(7)); 5 skips the session-full kingdom rule.
    pub field_0c: u32,
    /// [+0x14].
    pub game: GameSettings,
    /// [+0x18] (ctor 0x514000; 0x512DA0 adds the +0x50 byte).
    pub map_default: MapSettings,
    pub map_default_flag: bool,
    /// [+0x1C] (ctor 0x5141F0; 0x512DF0 adds the name of the +0x50 scenario).
    pub map_scenario: MapSettings,
    pub scenario: Option<Vec<u16>>,
    /// [+0x20] (ctor 0x514050; 0x512E30 adds the +0x50 random-map
    /// configuration name and the +0x54 settings).
    pub map_random: MapSettings,
    pub random_map: Option<Vec<u16>>,
    pub rmc_settings: RmcSettings,
    /// [+0x24] (ctor 0x5141A0).
    pub map_24: MapSettings,
    /// [+0x28] (ctor 0x944BC0, metagame map).
    pub map_metagame: MapSettings,
    /// [+0x2C].
    pub state: LaunchState,
    /// +0x30 text (0x76DCB0 returns it): posted with the session-state
    /// event of ChangeToState 0x77A750, which then empties it (0x401170).
    /// Not serialized.
    #[serde(default)]
    pub text_30: Vec<u16>,
}
impl LaunchSettings {
    /// 0x76B800.
    pub fn write(&self, w: &mut WireWriter, data: &dyn LobbyData) -> Result<()> {
        w.write_uint(self.game_type, 3)?;
        w.write_uint(self.mode, 3)?;
        write_text(w, &self.file)?;
        self.game.write(w, data)?;
        // 0x512DA0
        self.map_default.write(w, data)?;
        write_bool(w, self.map_default_flag)?;
        // 0x512DF0: (+0x50)+0x38 or the static empty CString 0xB43B40.
        self.map_scenario.write(w, data)?;
        write_text(w, self.scenario.as_deref().unwrap_or(&[]))?;
        // 0x512E30: (+0x50)+8 or 0xB43B40, then 0x505E90(+0x54).
        self.map_random.write(w, data)?;
        write_text(w, self.random_map.as_deref().unwrap_or(&[]))?;
        self.rmc_settings.write(w)?;
        self.map_24.write(w, data)?;
        self.map_metagame.write(w, data)?;
        w.write_uint(self.field_0c, 3)?;
        self.state.write(w)
    }
    /// 0x76EBD0.
    pub fn read(&mut self, r: &mut Cursor<'_>, data: &mut dyn LobbyData) -> Result<()> {
        self.game_type = read_bounded(r, 6)?;
        self.mode = read_bounded(r, 6)?;
        self.file = read_text(r)?;
        self.game.read(r, data)?;
        // 0x513230
        self.map_default.read(r, data)?;
        self.map_default_flag = read_bool(r)?;
        // 0x513290: 0x59F570 on the read name.
        self.map_scenario.read(r, data)?;
        let key = read_text(r)?;
        self.scenario = data.scenario(c_key(&key));
        // 0x513D20: 0x41EEC0 (null for an empty C string) then 0x5065C0.
        self.map_random.read(r, data)?;
        let key = read_text(r)?;
        let key = c_key(&key);
        self.random_map = if key.is_empty() { None } else { data.random_map_config(key) };
        self.rmc_settings.read(r, data.zero_width_list_budget())?;
        self.map_24.read(r, data)?;
        self.map_metagame.read(r, data)?;
        self.field_0c = read_bounded(r, 7)?;
        self.state.read(r)
    }
    /// 0x76C700 / 0x76C6B0 / 0x76C850: map settings selected by the mode.
    pub fn active_map(&self) -> &MapSettings {
        match self.mode {
            0 => &self.map_random,
            2 => &self.map_scenario,
            5 => &self.map_metagame,
            _ => &self.map_default,
        }
    }
    pub fn active_map_mut(&mut self) -> &mut MapSettings {
        match self.mode {
            0 => &mut self.map_random,
            2 => &mut self.map_scenario,
            5 => &mut self.map_metagame,
            _ => &mut self.map_default,
        }
    }
    /// 0x76B490: file of the current game (mode 0: the random-map
    /// configuration name, 2: +8, otherwise the empty CString 0xB43B40).
    pub fn current_file(&self) -> Result<Vec<u16>> {
        match self.mode {
            0 => self
                .random_map
                .clone()
                .ok_or(Error::Protocol("0x76B490 dereferences a null random-map configuration".into())),
            2 => Ok(self.file.clone()),
            _ => Ok(Vec::new()),
        }
    }
}

// ---------------------------------------------------------------------------
// Session model
// ---------------------------------------------------------------------------

/// The AdminPeer a SessionPlayer (+4) points to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PeerLink {
    /// The local TGC_AdminNode itself (vfunc 0 0x74B4D0 = +0x29C).
    Node,
    /// The TGC_AdminConn of the NetConn for this address (vfunc 0 0x74B4E0 =
    /// NetConn+0x9C).
    Conn(PeerAddress),
    /// Null pointer (0x74BC50 on the host for a null address).
    Null,
}

/// AdminPeer fields carried for every player: +0x3C (2 bits, 0x74B8D0 /
/// 0x751300), +0x70 join order (= the joined player's id, 0x759358),
/// +0x74..+0x84 player id object (0x77BA70 layout).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerIdentity {
    pub field_3c: u32,
    pub join_order: u32,
    pub player_id: PlayerId,
}
impl Default for PeerIdentity {
    fn default() -> Self {
        Self { field_3c: 0, join_order: 0, player_id: PlayerId { name: Vec::new(), serial: 0, a: 0, b: 0, c: 0 } }
    }
}
impl crate::packets::PeerBase {
    /// The +0x3C / +0x70 / +0x74 identity fields as the snapshot codecs see them.
    pub fn identity(&self) -> PeerIdentity {
        PeerIdentity { field_3c: self.field_3c as u32, join_order: self.join_order as u32, player_id: self.player_id.clone() }
    }
    /// Writes them (the snapshot reader's +0x74.., +0x70, +0x3C stores).
    pub fn set_identity(&mut self, identity: &PeerIdentity) {
        self.field_3c = identity.field_3c as i32;
        self.join_order = identity.join_order as i32;
        self.player_id = identity.player_id.clone();
    }
}

/// What the snapshot writer reads through a player's AdminPeer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerView {
    /// Virtual 0 of the peer (AdminNode+0x29C or NetConn+0x9C).
    pub address: Option<PeerAddress>,
    pub identity: PeerIdentity,
}

/// AdminPeer lookup of the writing node.
pub trait PeerDirectory {
    fn peer(&self, link: &PeerLink) -> Result<PeerView>;
}

/// The local node's view of the host: AdminNode+0x2C4 is the node itself
/// (`Node`) or a connection (`Conn(address)`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeView {
    pub host: PeerLink,
}
impl NodeView {
    pub fn we_are_host(&self) -> bool {
        self.host == PeerLink::Node
    }
    /// 0x74B900: the peer is AdminNode+0x2C4.
    pub fn is_host(&self, link: &PeerLink) -> bool {
        *link != PeerLink::Null && *link == self.host
    }
    /// 0x74BC50: connection for a (possibly null) address. A non-null
    /// address goes through 0x764970 (connection created on first use).
    pub fn connection(&self, address: Option<PeerAddress>, actions: &mut Vec<LobbyAction>) -> PeerLink {
        match address {
            Some(address) => {
                actions.push(LobbyAction::ConnectToPeer { address });
                PeerLink::Conn(address)
            }
            None if self.we_are_host() => PeerLink::Null,
            None => self.host,
        }
    }
}

/// TGC_SessionPlayer (0x54 bytes, ctor 0x7661D0, vtable 0xA92A28). The
/// +0x44 vector and the +0x50 object are not part of these messages.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionPlayer {
    /// +0x04.
    pub link: PeerLink,
    /// +0x08.
    pub name: Vec<u16>,
    /// +0x0C AI, +0x0D implicit, +0x0E film.
    pub ai: bool,
    pub implicit: bool,
    pub film: bool,
    pub field_10: u32,
    pub text_14: Vec<u16>,
    /// +0x18 (ctor: 2 for AI or the host's player, else 0).
    pub status: u32,
    pub flag_1c: bool,
    /// +0x20.
    pub id: u32,
    /// +0x24 kingdom name.
    pub kingdom: Vec<u16>,
    pub field_28: u32,
    /// +0x2C (set again by the name setter 0x7664F0).
    pub flag_2c: bool,
    pub flag_2d: bool,
    pub text_30: Vec<u16>,
    pub text_34: Vec<u16>,
    pub flag_38: bool,
    pub field_3c: u32,
    pub field_40: u32,
}
impl SessionPlayer {
    /// 0x7661D0(conn, id, ai, implicit, film).
    pub fn new(link: PeerLink, id: u32, ai: bool, implicit: bool, film: bool, node: &NodeView) -> Self {
        let status = if ai || node.is_host(&link) { 2 } else { 0 };
        Self {
            link,
            name: Vec::new(),
            ai,
            implicit,
            film,
            field_10: u32::MAX,
            text_14: Vec::new(),
            status,
            flag_1c: false,
            id,
            kingdom: Vec::new(),
            field_28: 0,
            flag_2c: true,
            flag_2d: ai,
            text_30: Vec::new(),
            text_34: Vec::new(),
            flag_38: true,
            field_3c: u32::MAX,
            field_40: u32::MAX,
        }
    }
}

/// Game options at session +0xCC (0x77B5A0 / 0x77B9D0).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameOptions {
    /// +0xCC game name (SetGameName 0x774700).
    pub name: Vec<u16>,
    /// +0xD0 password (SetGamePassword 0x774610).
    pub password: Vec<u16>,
    /// +0xD4 maximum players (SetMaxPlayers 0x770A80).
    pub max_players: i32,
    pub field_d8: u32,
    /// +0xDC allow observers (0x7709A0).
    pub allow_observers: bool,
}

/// Session property table (0x14-byte object at session+0x170, ctor 0x79C6E0):
/// +4 entry pointers, +8 count, +0x10 flag.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyTable {
    pub flag: bool,
    /// +8 signed count; entries exist only for a positive count.
    pub count: i32,
    pub entries: Vec<NamedWord>,
}
impl PropertyTable {
    /// 0x79C720.
    fn write(&self, w: &mut WireWriter) -> Result<()> {
        if self.entries.len() != self.count.max(0) as usize {
            return Err(Error::Protocol("property table count and entries disagree".into()));
        }
        write_bool(w, self.flag)?;
        write_u32(w, self.count as u32)?;
        for e in &self.entries {
            write_text(w, &e.name)?;
            write_u32(w, e.value)?;
        }
        Ok(())
    }
    /// 0x79CC00: old entries freed (0x79C8D0), count stored, entries read
    /// for a positive count.
    fn read(&mut self, r: &mut Cursor<'_>) -> Result<()> {
        self.flag = read_bool(r)?;
        let count = read_u32(r)? as i32;
        self.entries.clear();
        let n = count.max(0) as usize;
        if n.saturating_mul(48) > r.remaining_bits() {
            return Err(Error::Protocol(format!("property table of {n} entries does not fit the packet")));
        }
        self.count = count;
        for _ in 0..n {
            let name = read_text(r)?;
            let value = read_u32(r)?;
            self.entries.push(NamedWord { name, value });
        }
        Ok(())
    }
    /// 0x79CB60(name, value): an entry whose name equals `name` after
    /// lower-casing both (0x79C9F0: _wcslwr, C locale) and whose |value|
    /// equals |value| (32-bit absolute values, wrapping).
    fn contains(&self, name: &[u16], value: i32) -> bool {
        let lower = |s: &[u16]| -> Vec<u16> {
            c_key(s).iter().map(|&u| if (0x41..=0x5a).contains(&u) { u + 0x20 } else { u }).collect()
        };
        let key = lower(name);
        for e in self.entries.iter().take(self.count.max(0) as usize) {
            if lower(&e.name) == key && (e.value as i32).wrapping_abs() == value.wrapping_abs() {
                return true;
            }
        }
        false
    }
}

/// KKC_Session fields used by the join messages and the PlayerJoin decree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// [+0x14]+0x28 text (0x77DCF0) of the session info object.
    pub info_text: Vec<u16>,
    /// +0x54 and +0x88 launch settings.
    pub launch: LaunchSettings,
    pub launch_88: LaunchSettings,
    /// +0xBC/+0xC0 player list, head first.
    pub players: Vec<SessionPlayer>,
    /// +0xC4 most recently created player (0x773710), by its +0x20 id.
    pub created: Option<u32>,
    /// +0xC8, +0xC9, +0xCA change flags.
    pub changed: bool,
    pub changed_c9: bool,
    pub changed_ca: bool,
    pub options: GameOptions,
    /// +0xE0 counter.
    pub counter_e0: u32,
    /// +0xE4 state (0 reset, 1 staging, 2 in game, 3 editor, 4 film, 5 metagame).
    pub state: u32,
    /// +0xE8 last player id handed out (++ by 0x758FD0).
    pub last_player_id: u32,
    /// +0xF4 multiplayer.
    pub multiplayer: bool,
    /// +0xFC CRC sent with the launch file (0x770B70 / 0x7770B0).
    pub map_crc: u32,
    /// +0x104 / +0x108 / +0x10C: CRC, file and RMC flag checked when
    /// launch +0xC is 5.
    pub load_crc: u32,
    pub load_file: Vec<u16>,
    pub load_rmc: bool,
    pub field_128: u32,
    pub field_154: u32,
    pub field_158: u32,
    pub field_15c: u32,
    pub text_160: Vec<u16>,
    /// +0x164 launch counter (KKC_Session::Launch 0x77A950 increments it).
    pub field_164: u32,
    /// +0xF8: ChangeToState 0x77A750 stores the launch state kind (its +0x18
    /// for kind 6). Not serialized.
    #[serde(default)]
    pub field_f8: u32,
    /// +0x100 (cleared by 0x77A750). Not serialized.
    #[serde(default)]
    pub field_100: u32,
    /// +0x118 launch countdown shown by this node (0x7723F0; 0x76ADE0 > 0
    /// refuses connections; Launch 0x77A950 sets 0).
    #[serde(default)]
    pub launch_countdown: i32,
    /// +0x11C host countdown (AVar LaunchCountdownTime on the host when the
    /// countdown starts, -1 after a stop; > 0 blocks another start).
    #[serde(default)]
    pub host_countdown: i32,
    /// +0x169 (set by 0x77A950 for a launch state kind 0xC, +0x10 == 1,
    /// +0x14 clear): 0x77A6E0 then stores the launch settings as the last
    /// used ones. +0x16A in game after a ready load (0x77A950), +0x16B
    /// cleared by 0x77A950, +0x16C cleared by 0x77A750.
    #[serde(default)]
    pub flag_169: bool,
    #[serde(default)]
    pub flag_16a: bool,
    #[serde(default)]
    pub flag_16b: bool,
    #[serde(default)]
    pub flag_16c: bool,
    /// +0x170 (null when its 0x14-byte allocation failed in the ctor).
    pub properties: Option<PropertyTable>,
}
impl Session {
    pub(crate) fn player_mut(&mut self, id: u32) -> Option<&mut SessionPlayer> {
        self.players.iter_mut().find(|p| p.id == id)
    }
    /// 0x76B8F0(id, film): first player with that id and film byte.
    pub fn live_player(&self, id: u32, film: bool) -> Option<u32> {
        self.players.iter().find(|p| p.id == id && p.film == film).map(|p| p.id)
    }
    /// 0x76C930: players that are neither AI nor film.
    pub fn human_count(&self) -> usize {
        self.players.iter().filter(|p| !p.ai && !p.film).count()
    }
}

impl aa_runtime::load_world_globals::SessionPhase for Session {
    /// Original76AC90 reads this exact retained KKC_Session+F8 field. Launch
    /// transitions77A750 update it in staging::change_to_state; never cache it
    /// from MapResource mode, a constructor value or the final launch kind.
    fn current_phase(&self) -> std::result::Result<u32, String> {
        Ok(self.field_f8)
    }

    /// Original54EF10 reads this exact retained KKC_Session+E4 field;77A750
    /// writes it separately from +F8 during staging::change_to_state.
    fn application_state(&self) -> std::result::Result<u32, String> {
        Ok(self.state)
    }
}

/// Node-level effects in native order (see the module documentation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LobbyAction {
    /// 0x764970 on the address: the NetConn (0x7647A0) and its AdminConn
    /// (0x763990) are created when the address has none yet.
    ConnectToPeer { address: PeerAddress },
    /// 0x74AFD0: peer+8 (joined player) = peer+0x2C (ordering player).
    BindPlayer { link: PeerLink, player: u32 },
    /// 0x74DC10 / 0x45F240: peer appended to AdminNode+0x2AC, +0x2B4 = 1.
    AddPeer { link: PeerLink },
    /// Peer +0x78, +0x74, +0x7C..+0x84, +0x70 and +0x3C written by the
    /// snapshot reader (in that order, one record).
    SetPeerIdentity { link: PeerLink, identity: PeerIdentity },
    /// 0x781DD0 on [session+0x14] (session info object reset).
    ResetSessionInfo,
    /// 0x781250 on [session+0x14] with a non-empty text.
    ApplySessionInfo { text: Vec<u16> },
    /// 0x40BC50(&0xB399B0, kind, CString::Format(0x4B9F40(key), args)).
    UiMessage { kind: u32, key: String, args: Vec<Vec<u16>> },
    /// Order request emitted by the local node (0x74CA80: 0x766C10 with the
    /// default player 0x76AC60, descriptor, payload).
    Request { descriptor: usize, fields: Vec<Field> },
}

// ---------------------------------------------------------------------------
// Private message kind 1: session snapshot
// ---------------------------------------------------------------------------

/// 0x74C270: 1 bit (= 1), then 0x76D0B0.
pub fn write_session_message(
    w: &mut WireWriter,
    session: &Session,
    peers: &dyn PeerDirectory,
    data: &dyn LobbyData,
) -> Result<()> {
    w.write_uint(PRIVATE_SESSION, 1)?;
    write_session_snapshot(w, session, peers, data)
}

/// KKC_Session::SerializeState(O) 0x76D0B0.
pub fn write_session_snapshot(
    w: &mut WireWriter,
    session: &Session,
    peers: &dyn PeerDirectory,
    data: &dyn LobbyData,
) -> Result<()> {
    session.launch.write(w, data)?;
    session.launch_88.write(w, data)?;
    let created = session.created.ok_or(Error::Protocol("0x76D0D4 dereferences a null session+0xC4".into()))?;
    write_u32(w, created)?;
    for player in &session.players {
        write_bool(w, true)?;
        write_bool(w, player.ai)?;
        let peer = match player.link {
            PeerLink::Null => {
                return Err(Error::Protocol("0x76D15B calls through a null player connection".into()))
            }
            ref link => peers.peer(link)?,
        };
        write_address(w, peer.address)?;
        write_bool(w, player.implicit)?;
        write_u32(w, player.id)?;
        write_text(w, &player.kingdom)?;
        write_text(w, &player.name)?;
        write_u32(w, player.status)?;
        write_bool(w, player.flag_2d)?;
        write_text(w, &player.text_14)?;
        write_u32(w, player.field_10)?;
        write_bool(w, player.flag_1c)?;
        write_player_id(w, &peer.identity.player_id)?;
        write_u32(w, peer.identity.join_order)?;
        write_text(w, &player.text_34)?;
        write_text(w, &player.text_30)?;
        // 0x74B8D0: 2 bits.
        w.write_uint(peer.identity.field_3c, 2)?;
        write_bool(w, player.flag_38)?;
        write_u32(w, player.field_40)?;
        write_u32(w, player.field_3c)?;
    }
    write_bool(w, false)?;
    w.write_uint(session.state, 3)?;
    write_u32(w, session.last_player_id)?;
    write_text(w, &session.info_text)?;
    write_u32(w, session.field_154)?;
    write_u32(w, session.map_crc)?;
    write_u32(w, session.load_crc)?;
    write_text(w, &session.load_file)?;
    write_bool(w, session.load_rmc)?;
    write_u32(w, session.field_128)?;
    // 0x77B5A0
    write_text(w, &session.options.name)?;
    write_u32(w, session.options.max_players as u32)?;
    write_text(w, &session.options.password)?;
    write_u32(w, session.options.field_d8)?;
    write_bool(w, session.options.allow_observers)?;
    write_u32(w, session.field_158)?;
    write_u32(w, session.field_15c)?;
    write_text(w, &session.text_160)?;
    write_u32(w, session.field_164)?;
    session
        .properties
        .as_ref()
        .ok_or(Error::Protocol("0x79C720 dereferences a null session+0x170".into()))?
        .write(w)
}

/// Session effects that the original performs inline while reading and whose
/// results feed back into the reader or the decree.
pub trait SnapshotHost {
    /// KKC_Session::ChangeToState 0x77A750(state, arg) on the reading
    /// session: [`crate::staging::change_to_state`] (ported there with its
    /// own host trait for the event post and settings copies).
    fn change_to_state(&mut self, session: &mut Session, state: u32, arg: u32) -> Result<()>;
    /// KKC_Session::SendUpdateRMCMapStatus 0x7770B0 (`rmc`) or
    /// SendUpdateMapStatus 0x770B70 (file, crc, source check); returns the
    /// native bool.
    fn update_map_status(
        &mut self,
        session: &mut Session,
        rmc: bool,
        file: &[u16],
        crc: u32,
        source_check: bool,
    ) -> Result<bool>;
}

/// KKC_Session::SerializeState(I) 0x77AAC0 on the reading session. The
/// record of the joining player (id = the first word) is bound to the node
/// itself and its address is stored in AdminNode+0x29C (0x74B430,
/// `admin.own_address`) inline, as the admin-state message writes the same
/// field.
pub fn read_session_snapshot(
    r: &mut Cursor<'_>,
    session: &mut Session,
    admin: &mut ClientAdmin,
    data: &mut dyn LobbyData,
    host: &mut dyn SnapshotHost,
    node: &NodeView,
) -> Result<Vec<LobbyAction>> {
    let mut actions = Vec::new();
    // 0x77AAF5 / 0x7840B0(1, 1, 0) on [session+0x80] / 0x77A750(1, 0).
    session.launch.game_type = 1;
    let state = &mut session.launch.state;
    state.field_10 = 1;
    state.kind = 0xc;
    state.flag_15 = true;
    state.flag_16 = false;
    state.flag_14 = false;
    host.change_to_state(session, 1, 0)?;
    session.launch.read(r, data)?;
    session.launch_88.read(r, data)?;
    let joining = read_u32(r)?;
    while read_bool(r)? {
        let ai = read_bool(r)?;
        let address = read_address(r)?;
        let implicit = read_bool(r)?;
        let id = read_u32(r)?;
        let link = if id == joining {
            admin.own_address = address;
            PeerLink::Node
        } else {
            node.connection(address, &mut actions)
        };
        let mut player = SessionPlayer::new(link, id, ai, implicit, false, node);
        if !ai {
            if link == PeerLink::Null {
                return Err(Error::Protocol("0x74AFD0 writes through a null player connection".into()));
            }
            actions.push(LobbyAction::BindPlayer { link, player: id });
            actions.push(LobbyAction::AddPeer { link });
        }
        player.kingdom = read_text(r)?;
        player.name = read_text(r)?;
        player.status = read_u32(r)?;
        player.flag_2d = read_bool(r)?;
        player.text_14 = read_text(r)?;
        player.field_10 = read_u32(r)?;
        player.flag_1c = read_bool(r)?;
        if link == PeerLink::Null {
            session.players.push(player);
            session.changed = true;
            return Err(Error::Protocol("0x77AF6B writes through a null player connection".into()));
        }
        let player_id = read_player_id(r)?;
        let join_order = read_u32(r)?;
        player.text_34 = read_text(r)?;
        player.text_30 = read_text(r)?;
        let field_3c = read_bounded(r, 4)?;
        player.flag_38 = read_bool(r)?;
        player.field_40 = read_u32(r)?;
        player.field_3c = read_u32(r)?;
        actions.push(LobbyAction::SetPeerIdentity { link, identity: PeerIdentity { field_3c, join_order, player_id } });
        session.players.push(player);
        session.changed = true;
    }
    session.changed_c9 = true;
    session.changed = true;
    session.counter_e0 = session.counter_e0.wrapping_add(1);
    // 0x77B209: read and discarded.
    read_bounded(r, 6)?;
    session.last_player_id = read_u32(r)?;
    let text = read_text(r)?;
    actions.push(LobbyAction::ResetSessionInfo);
    if !text.is_empty() {
        actions.push(LobbyAction::ApplySessionInfo { text });
    }
    session.field_154 = read_u32(r)?;
    session.map_crc = read_u32(r)?;
    session.load_crc = read_u32(r)?;
    session.load_file = read_text(r)?;
    session.load_rmc = read_bool(r)?;
    let mut checks = true;
    if session.launch.field_0c == 5 {
        let (rmc, file, crc) = (session.load_rmc, session.load_file.clone(), session.load_crc);
        checks = host.update_map_status(session, rmc, &file, crc, true)?;
    }
    if checks {
        if session.launch.mode == 2 {
            let (file, crc) = (session.launch.file.clone(), session.map_crc);
            host.update_map_status(session, false, &file, crc, false)?;
        }
        if session.launch.mode == 0 {
            let (file, crc) = (session.launch.file.clone(), session.map_crc);
            host.update_map_status(session, true, &file, crc, false)?;
        }
    }
    session.field_128 = read_u32(r)?;
    session.options.name = read_text(r)?;
    session.options.max_players = read_u32(r)? as i32;
    session.options.password = read_text(r)?;
    session.options.field_d8 = read_u32(r)?;
    session.options.allow_observers = read_bool(r)?;
    session.changed_ca = true;
    session.field_158 = read_u32(r)?;
    session.field_15c = read_u32(r)?;
    session.text_160 = read_text(r)?;
    session.field_164 = read_u32(r)?;
    session
        .properties
        .as_mut()
        .ok_or(Error::Protocol("0x79CC00 dereferences a null session+0x170".into()))?
        .read(r)?;
    Ok(actions)
}

// ---------------------------------------------------------------------------
// Private message kind 0: admin state
// ---------------------------------------------------------------------------

/// Host-side inputs of 0x74C130.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdminStateSource {
    /// Virtual 0 of the target connection: its NetConn+0x9C address.
    pub conn_address: Option<PeerAddress>,
    /// AdminNode+0x200, +0x204.
    pub field_200: u32,
    pub field_204: u32,
    /// AdminNode+0x208 SessionPlayer (0x765F10 writes its id or 0).
    pub player: Option<u32>,
    /// AdminNode+0x28C admin time (binary32 bits).
    pub admin_time: u32,
    /// AdminNode+0x218 next decree packet index.
    pub decree_end: i32,
}

/// 0x74C130: 1 bit (= 0), address, +0x200, +0x204, player, admin time,
/// +0x218, synchronizer A last index (0x77CBE0: window end - 1) and hash
/// (0x573580: +8).
pub fn write_admin_state(w: &mut WireWriter, source: &AdminStateSource, sync_a: &Synchronizer) -> Result<()> {
    w.write_uint(PRIVATE_ADMIN_STATE, 1)?;
    write_address(w, source.conn_address)?;
    write_u32(w, source.field_200)?;
    write_u32(w, source.field_204)?;
    write_u32(w, source.player.unwrap_or(0))?;
    write_u32(w, source.admin_time)?;
    write_u32(w, source.decree_end as u32)?;
    write_u32(w, sync_a.end().wrapping_sub(1) as u32)?;
    write_u32(w, sync_a.hash as u32)
}

/// Client clock (vtable 0xA91AB4, ctor 0x75FAA0) at AdminNode+0x16C.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientClock {
    /// +4 period, +8 offset (binary32 bits), +0xC.. samples.
    pub period: u32,
    pub offset: u32,
    pub samples: Vec<u32>,
}
impl Default for ClientClock {
    /// 0x75FAA0: everything zero.
    fn default() -> Self {
        Self { period: 0, offset: 0, samples: vec![0; CLOCK_SAMPLES] }
    }
}
impl ClientClock {
    /// Virtual +0xC 0x75FAC0(start, period): +4 = period, +8 =
    /// f32(now - start), every sample = +8.
    pub fn reset(&mut self, start: f32, period: f32, now: f32) {
        self.period = period.to_bits();
        self.offset = x87_sub(now, start).to_bits();
        self.samples = vec![self.offset; CLOCK_SAMPLES];
    }
    /// 0x75FB40(target): +8 = max(old sample 0, -9999999.0) (x87 compare,
    /// NaN keeps the sample), sample 1 = old sample 0, sample 0 = f32(now -
    /// target), +8 = the new sample when +8 <= it (or unordered).
    // The negated comparisons keep the x87 unordered (NaN) outcomes.
    #[allow(clippy::neg_cmp_op_on_partial_ord)]
    pub fn update(&mut self, target: f32, now: f32) {
        if self.samples.len() < 2 {
            self.samples.resize(CLOCK_SAMPLES, 0);
        }
        let floor = f32::from_bits(CLOCK_FLOOR_BITS);
        let old = self.samples[0];
        self.samples[1] = old;
        // fld floor; fcomp old: jump (keep old) when C0|C3 (floor <= old or unordered).
        let edx = if !(floor > f32::from_bits(old)) { old } else { CLOCK_FLOOR_BITS };
        let t = x87_sub(now, target);
        self.samples[0] = t.to_bits();
        // fld edx; fcomp t: C0|C3 (edx <= t or unordered) -> +8 = t.
        self.offset = if !(f32::from_bits(edx) > t) { t.to_bits() } else { edx };
    }
}
/// One binary32 subtraction stored to binary32: identical under every x87
/// precision (double rounding is innocuous for p >= 2*24+2). The x87
/// default NaN is 0xFFC00000.
fn x87_sub(a: f32, b: f32) -> f32 {
    let r = a - b;
    if r.is_nan() && !a.is_nan() && !b.is_nan() {
        f32::from_bits(0xffc0_0000)
    } else {
        r
    }
}
/// 0x74E0B6..0x74E0D2: `fstp f32(fild(k) * 0.0625 + time)` under precision `p`.
pub fn clock_target(k: i32, time: f32, p: X87Precision) -> Result<f32> {
    let product = f64::from(k) * 0.0625; // exact
    match p {
        X87Precision::Single => Ok((product as f32) + time),
        X87Precision::Double => Ok((product + f64::from(time)) as f32),
        X87Precision::Extended => {
            let s = product + f64::from(time);
            let bb = s - product;
            let err = (product - (s - bb)) + (f64::from(time) - bb);
            if err == 0.0 || !s.is_finite() {
                Ok(s as f32)
            } else {
                Err(Error::Orders(aa_orders::Error::X87Unsupported))
            }
        }
    }
}

/// AdminNode fields written by the client handler 0x74DF50.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientAdmin {
    /// +0x29C our address as the host sees it.
    pub own_address: Option<PeerAddress>,
    /// +0x248 / +0x24C decree stream resume position, +0x250 its player.
    pub resume_byte: u32,
    pub resume_bit: u32,
    pub resume_player: Option<u32>,
    /// +0x28C admin time (binary32 bits).
    pub admin_time: u32,
    /// +0x20C next decree packet to execute; +0x218 decree history end
    /// (read only).
    pub decree_next: i32,
    pub decree_end: i32,
    /// +0x158 points to the client clock +0x16C.
    pub clock_selected: bool,
    pub clock: ClientClock,
}

/// Inputs of the client-side handlers that are not AdminNode state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockEnv {
    /// NetNode+0x58 (binary32 seconds).
    pub now: f32,
    pub precision: X87Precision,
}

/// 0x74DF50 TGC_AdminConn private admin-state handler (client).
pub fn read_admin_state(
    r: &mut Cursor<'_>,
    admin: &mut ClientAdmin,
    session: &Session,
    sync_a: &mut Synchronizer,
    env: ClockEnv,
) -> Result<()> {
    admin.own_address = read_address(r)?;
    admin.resume_byte = read_u32(r)?;
    admin.resume_bit = read_u32(r)?;
    // 0x765F70 -> 0x76B8F0(id, 0).
    let id = read_u32(r)?;
    admin.resume_player = session.live_player(id, false);
    admin.admin_time = read_u32(r)?;
    admin.decree_next = read_u32(r)? as i32;
    let index = read_u32(r)? as i32;
    let hash = read_u32(r)? as i32;
    sync_a.reset(index, hash);
    let time = f32::from_bits(admin.admin_time);
    admin.clock.reset(x87_sub(time, ADMIN_TICK), ADMIN_TICK, env.now);
    admin.clock_selected = true;
    let k = admin.decree_end.wrapping_sub(admin.decree_next).wrapping_sub(1);
    let target = clock_target(k, time, env.precision)?;
    admin.clock.update(target, env.now);
    Ok(())
}

/// One processed private message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrivateMessage {
    pub index: i32,
    pub kind: u32,
    pub end: Position,
    pub actions: Vec<LobbyAction>,
}

/// Body of one 0x7531A0 iteration: kind (0x411FA0(2)) then 0x74DF50 or
/// 0x77AAC0 on a finalized private message.
#[allow(clippy::too_many_arguments)]
pub fn apply_private_message(
    bytes: &[u8],
    admin: &mut ClientAdmin,
    session: &mut Session,
    sync_a: &mut Synchronizer,
    data: &mut dyn LobbyData,
    host: &mut dyn SnapshotHost,
    node: &NodeView,
    env: ClockEnv,
) -> Result<(u32, Position, Vec<LobbyAction>)> {
    let mut r = Cursor::for_packet(bytes, None)?;
    let kind = read_bounded(&mut r, 2)?;
    let actions = if kind == PRIVATE_ADMIN_STATE {
        read_admin_state(&mut r, admin, session, sync_a, env)?;
        Vec::new()
    } else {
        read_session_snapshot(&mut r, session, admin, data, host, node)?
    };
    Ok((kind, r.position(), actions))
}

/// 0x7531A0: process every private message of the host connection's
/// history +0xAC in order and retire it (0x750880(begin + 1)).
#[allow(clippy::too_many_arguments)]
pub fn process_private_messages(
    conn: &mut AdminConn,
    admin: &mut ClientAdmin,
    session: &mut Session,
    sync_a: &mut Synchronizer,
    data: &mut dyn LobbyData,
    host: &mut dyn SnapshotHost,
    node: &NodeView,
    env: ClockEnv,
) -> Result<Vec<PrivateMessage>> {
    let mut out = Vec::new();
    while conn.private_history.begin() < conn.private_history.end() {
        let index = conn.private_history.begin();
        let packet = conn
            .private_history
            .get(index)?
            .cloned()
            .ok_or(Error::Protocol(format!("private message {index} is a null packet")))?;
        let (kind, end, actions) = apply_private_message(packet.bytes(), admin, session, sync_a, data, host, node, env)?;
        out.push(PrivateMessage { index, kind, end, actions });
        conn.private_history.retire_before(index.wrapping_add(1))?;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Host join completion 0x753580
// ---------------------------------------------------------------------------

/// The two private messages built by 0x753580, finalized (0x74D0A0).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinMessages {
    /// Kind 1 session snapshot (0x74C270), appended first.
    pub session: Vec<u8>,
    /// Kind 0 admin state (0x74C130), appended second.
    pub admin_state: Vec<u8>,
}

/// TGC_AdminConn host join completion 0x753580: connected (+0x88), 0x752300,
/// 0x752490(AdminNode+0x218), then the session message (0x74C270) and the
/// admin-state message (0x74C130), each built from byte 0 bit 3 in a fresh
/// stream (the native static stream 0xB408F4 is zero-filled before each),
/// finalized by 0x74D0A0 and appended to the private history +0xAC (which
/// 0x752490 has just reset to [0, 0)). In an internet game the original then
/// runs the GameSpy CD-key challenge (0x789620, 0x752990, 0x74E730), which is
/// not ported: Unsupported after both messages are appended.
#[allow(clippy::too_many_arguments)]
pub fn complete_host_join(
    conn: &mut AdminConn,
    node: &mut NodeHistories<'_>,
    session: &Session,
    peers: &dyn PeerDirectory,
    data: &dyn LobbyData,
    admin: &AdminStateSource,
    sync_a: &Synchronizer,
    internet: bool,
) -> Result<JoinMessages> {
    conn.connected = true;
    conn.reset_backup_senders_to_client(node)?;
    conn.reset_senders_to_client(admin.decree_end)?;
    let mut w = WireWriter::new(Position::PACKET_START)?;
    write_session_message(&mut w, session, peers, data)?;
    let session_bytes = append_private(conn, &w)?;
    let mut w = WireWriter::new(Position::PACKET_START)?;
    write_admin_state(&mut w, admin, sync_a)?;
    let admin_bytes = append_private(conn, &w)?;
    if internet {
        return Err(Error::Unsupported(
            "0x753728 internet join: GameSpy CD-key challenge 0x789620 / 0x752990 / 0x74E730",
        ));
    }
    Ok(JoinMessages { session: session_bytes, admin_state: admin_bytes })
}
fn append_private(conn: &mut AdminConn, w: &WireWriter) -> Result<Vec<u8>> {
    let bytes = finalize(w)?;
    let packet = Packet::new(bytes.clone())?;
    let end = conn.private_history.end();
    conn.private_history.put(end, Some(packet))?;
    Ok(bytes)
}

// ---------------------------------------------------------------------------
// PlayerJoin decree 0x758FD0
// ---------------------------------------------------------------------------

/// TGC_PlayerJoinOrder info (0x2C bytes at decree+4, copied by 0x750720;
/// payload schema p306).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinInfo {
    pub name: Vec<u16>,
    pub password: Vec<u16>,
    pub version: Vec<u16>,
    pub build: Vec<u16>,
    pub data_checksum: u32,
    /// +0x14 interned address (0x76A810; null for 0/0).
    pub address: Option<PeerAddress>,
    /// +0x18..+0x28.
    pub player_id: PlayerId,
}
impl JoinInfo {
    /// Decoded decree 317 payload (aa_orders schema p306).
    pub fn from_fields(fields: &[Field]) -> Result<Self> {
        let bad = || Error::Protocol(format!("payload of descriptor {PLAYER_JOIN_DECREE} does not match p306"));
        let text = |f: &Field| match f {
            Field::Text(u) => Ok(u.clone()),
            _ => Err(bad()),
        };
        let word = |f: &Field| match f {
            Field::Uint { width: 32, value } => Ok(*value),
            _ => Err(bad()),
        };
        if fields.len() != 11 {
            return Err(bad());
        }
        let address = match fields[5] {
            Field::NetAddr { ip, port } => {
                if ip == 0 && port == 0 {
                    None
                } else {
                    Some(PeerAddress { ip, port: port as u16 })
                }
            }
            _ => return Err(bad()),
        };
        Ok(Self {
            name: text(&fields[0])?,
            password: text(&fields[1])?,
            version: text(&fields[2])?,
            build: text(&fields[3])?,
            data_checksum: word(&fields[4])?,
            address,
            player_id: PlayerId {
                serial: word(&fields[6])?,
                name: text(&fields[7])?,
                a: word(&fields[8])?,
                b: word(&fields[9])?,
                c: word(&fields[10])?,
            },
        })
    }
}

/// Node state read or written by the decree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinNode {
    pub view: NodeView,
    /// AdminNode+0x29C.
    pub own_address: Option<PeerAddress>,
    /// NetNode+0x58 (binary32 bits) for the disconnect time.
    pub now: u32,
    /// [0xB3654C]+0xBC internet game (0x770280).
    pub internet: bool,
    /// AdminNode+0x2B4 set when a peer is added.
    pub peers_changed: bool,
    /// DAT_00B40930: the IsMultiplayer assertion was answered "ignore".
    pub multiplayer_assert_ignored: bool,
}

/// Connection access and inline effects of the decree.
pub trait JoinHost {
    /// The AdminConn of a link (for a `Conn` link 0x764970 has already been
    /// reported as [`LobbyAction::ConnectToPeer`]) and its peer identity.
    fn conn(&mut self, link: &PeerLink) -> Result<&mut AdminConn>;
    /// KKC_Session::AutoAssignKingdomToPlayer 0x778D10:
    /// [`crate::staging::auto_assign_kingdom`] (its kingdom setters are a
    /// [`crate::staging::KingdomHost`]).
    fn auto_assign_kingdom(&mut self, session: &mut Session, player: u32) -> Result<()>;
    /// `CString::Format(0x4B9F40(DUPLICATE_NAME_KEY), name, n)` for 0x766560.
    fn duplicate_name(&mut self, name: &[u16], n: u32) -> Result<Vec<u16>>;
    /// 0x77E2B0 on [session+0x14] for a kingdom record name: its +5 byte
    /// (a null result is dereferenced natively).
    fn kingdom_open(&mut self, kingdom: &[u16]) -> Result<bool>;
}

/// Outcome of 0x758FD0.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JoinOutcome {
    /// Session full on a non-host (log 0xB9E only).
    FullIgnored,
    /// Session full on the host: the connection is set disconnecting with
    /// reason 0 (game_full) unless it already has a player (log 0xBA8).
    FullDisconnected { link: PeerLink },
    FullAlreadyJoined { link: PeerLink },
    /// The joining address is our own (log 0xBB2).
    RejectedSelf,
    /// The connection already has a player (log 0xBBB).
    RejectedDuplicate { link: PeerLink },
    /// Player created; on the host the caller must now run
    /// [`complete_host_join`] on `link` (0x75939D).
    Joined { player: u32, link: PeerLink, host_completion: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinResult {
    pub outcome: JoinOutcome,
    pub actions: Vec<LobbyAction>,
}

/// 0x76C850: free kingdom records of the active map (no +0x10 byte, +0x28
/// set, 0x77E2B0(name)+5 set, and no session player whose kingdom +0x24
/// equals the name under wcscmp).
pub fn free_kingdoms(session: &Session, host: &mut dyn JoinHost) -> Result<i32> {
    let mut free = 0i32;
    for k in &session.launch.active_map().header.kingdoms {
        if k.flag_10 || !k.flag_28 || !host.kingdom_open(&k.name)? {
            continue;
        }
        if !session.players.iter().any(|p| wcs_eq(&p.kingdom, &k.name)) {
            free += 1;
        }
    }
    Ok(free)
}

/// 0x76CCA0 session full.
pub fn session_full(session: &Session, host: &mut dyn JoinHost) -> Result<bool> {
    let humans = session.human_count() as i64;
    if humans >= i64::from(session.options.max_players) {
        return Ok(true);
    }
    if !session.multiplayer || session.launch.field_0c == 5 || session.options.allow_observers {
        return Ok(false);
    }
    Ok(free_kingdoms(session, host)? <= 0)
}

/// ClientLeave 0x755840's session loop (0x755B58): every player whose
/// connection (+4, 0x8569A0) is `link`, in list order, is destroyed with
/// KKC_Session::DestroyPlayer 0x7737E0 (log 0x485, unlink 0x50C510,
/// deleting destructor, +0xC8 = 1), after which `rated` runs (0x770280,
/// [`update_rated`]) with the destroyed id. +0xC4 is not touched natively
/// (`created` keeps the id). Returns the destroyed ids.
pub fn destroy_link_players(
    session: &mut Session,
    link: &PeerLink,
    mut rated: impl FnMut(&mut Session, u32) -> Result<()>,
) -> Result<Vec<u32>> {
    let mut destroyed = Vec::new();
    let mut k = 0;
    while k < session.players.len() {
        if session.players[k].link != *link {
            k += 1;
            continue;
        }
        let id = session.players.remove(k).id;
        session.changed = true;
        rated(session, id)?;
        destroyed.push(id);
    }
    Ok(destroyed)
}

/// 0x770280 rated flag of the active map's "RATED" world rule.
pub fn update_rated(session: &mut Session, node: &JoinNode, data: &dyn LobbyData) -> Result<()> {
    let Some(properties) = session.properties.as_ref() else { return Ok(()) };
    let mode = session.launch.mode;
    let file_mode = mode == 0 || mode == 2;
    let mut rated = false;
    if node.internet && file_mode {
        let crc = session.map_crc;
        if properties.contains(&session.launch.current_file()?, crc as i32) {
            let humans = session.human_count();
            let players = session.players.len();
            rated = humans == players && players > 1;
        }
    }
    if matches!(mode, 0 | 1 | 2 | 5) && matches!(session.launch.game_type, 1 | 2) {
        let rated_name: Vec<u16> = RATED_RULE.encode_utf16().collect();
        let conditions = &mut session.launch.active_map_mut().conditions;
        if let Some(i) = conditions.find_rule(data, &rated_name)? {
            // 0x77039F: byte compare of entry+8 with the computed flag.
            if (conditions.rules[i].value & 0xff) != u32::from(rated) {
                session.changed_c9 = true;
                // 0x511AF0: same lookup by the rule's own name; only a bool rule changes.
                let name = data.world_rule_name(conditions.rules[i].rule.unwrap_or_default())?;
                if let Some(j) = conditions.find_rule(data, &name)? {
                    let index = conditions.rules[j].rule.unwrap_or_default();
                    if data.world_rule_kind(index)? == 0 {
                        let v = &mut conditions.rules[j].value;
                        *v = (*v & !0xff) | u32::from(rated);
                        conditions.changes = conditions.changes.wrapping_add(1);
                    }
                }
            }
        }
    }
    Ok(())
}

/// KKC_Session::CreatePlayer 0x773710(id, conn, ai, implicit): new player
/// appended to the list, +0xC4 = it, +0xC8 = 1, then 0x770280.
pub fn create_player(
    session: &mut Session,
    id: u32,
    link: PeerLink,
    ai: bool,
    implicit: bool,
    node: &JoinNode,
    data: &dyn LobbyData,
) -> Result<()> {
    session.players.push(SessionPlayer::new(link, id, ai, implicit, false, &node.view));
    session.created = Some(id);
    session.changed = true;
    update_rated(session, node, data)
}

/// 0x766560 SetName with the duplicate rule: while 0x76CE10 finds another
/// player whose name (0x766380, wcscmp) equals the candidate, the candidate
/// becomes Format(DUPLICATE_NAME_KEY, name, n) with n = 1, 2, ...; then
/// 0x7664F0 stores it and sets +0x2C.
pub fn set_player_name(session: &mut Session, player: usize, name: &[u16], host: &mut dyn JoinHost) -> Result<()> {
    let mut candidate = name.to_vec();
    let mut n = 1u32;
    loop {
        let found = session.players.iter().position(|p| wcs_eq(&p.name, &candidate));
        match found {
            Some(i) if i != player => {
                candidate = host.duplicate_name(name, n)?;
                n = n.wrapping_add(1);
            }
            _ => break,
        }
    }
    let p = &mut session.players[player];
    p.name = candidate;
    p.flag_2c = true;
    Ok(())
}

/// PlayerJoin decree process 0x75DA80 -> TGC_AdminNode 0x758FD0.
pub fn process_player_join(
    info: &JoinInfo,
    session: &mut Session,
    node: &mut JoinNode,
    data: &dyn LobbyData,
    host: &mut dyn JoinHost,
) -> Result<JoinResult> {
    let mut actions = Vec::new();
    if !node.multiplayer_assert_ignored && !session.multiplayer {
        return Err(Error::Protocol("assertion g_session->IsMultiplayer() (AdminLayer.cpp 0xB96)".into()));
    }
    let target = |node: &JoinNode, actions: &mut Vec<LobbyAction>| -> Result<PeerLink> {
        match info.address {
            None if node.view.we_are_host() => {
                Err(Error::Protocol("0x759193 dereferences a null connection (host, null address)".into()))
            }
            address => Ok(node.view.connection(address, actions)),
        }
    };
    if session_full(session, host)? {
        if !node.view.we_are_host() {
            return Ok(JoinResult { outcome: JoinOutcome::FullIgnored, actions });
        }
        let link = target(node, &mut actions)?;
        let conn = host.conn(&link)?;
        conn.accepted = false;
        if conn.base.player.is_some() {
            return Ok(JoinResult { outcome: JoinOutcome::FullAlreadyJoined { link }, actions });
        }
        conn.disconnecting = true;
        conn.disconnect_started = node.now;
        conn.disconnect_reason = 0;
        return Ok(JoinResult { outcome: JoinOutcome::FullDisconnected { link }, actions });
    }
    if info.address == node.own_address {
        return Ok(JoinResult { outcome: JoinOutcome::RejectedSelf, actions });
    }
    let link = target(node, &mut actions)?;
    {
        let conn = host.conn(&link)?;
        conn.accepted = false;
        if conn.base.player.is_some() {
            return Ok(JoinResult { outcome: JoinOutcome::RejectedDuplicate { link }, actions });
        }
    }
    session.last_player_id = session.last_player_id.wrapping_add(1);
    let id = session.last_player_id;
    create_player(session, id, link, false, false, node, data)?;
    let index = session.players.len() - 1;
    set_player_name(session, index, &info.name, host)?;
    actions.push(LobbyAction::UiMessage {
        kind: PLAYER_JOINED_MESSAGE_KIND,
        key: PLAYER_JOINED_KEY.into(),
        args: vec![session.players[index].name.clone()],
    });
    {
        let conn = host.conn(&link)?;
        conn.base.player = Some(id);
        conn.base.ordering_player = Some(id);
        conn.base.player_id = info.player_id.clone();
        conn.base.join_order = id as i32;
    }
    actions.push(LobbyAction::AddPeer { link });
    node.peers_changed = true;
    host.auto_assign_kingdom(session, id)?;
    let host_completion = node.view.we_are_host();
    if host_completion {
        actions.push(LobbyAction::Request { descriptor: POST_PLAYER_JOIN_REQUEST, fields: vec![Field::Player(id)] });
    }
    Ok(JoinResult { outcome: JoinOutcome::Joined { player: id, link, host_completion }, actions })
}

/// The created player of a session by id (for callers of the decree).
pub fn player(session: &mut Session, id: u32) -> Option<&mut SessionPlayer> {
    session.player_mut(id)
}
