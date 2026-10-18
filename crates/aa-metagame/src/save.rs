//! Strategic save format of the metagame: the payload of the `METAGAME`
//! chunk of a saved game, and the army MetaGame component record stored in
//! the army actor record of `DYNAMIC_THINGS`. Ported from the original
//! AA.exe; every layout keeps its original 32-bit form (little endian `u32`
//! counts and handles, `u8` flags, strings as a `u16` UTF-16 unit count
//! followed by the units, no terminator).
//!
//! | native | role |
//! |---|---|
//! | 4F2DB0 / 4FC600 | save / load of a map: open the `METAGAME` chunk (859150 / 859030), then MGState vtable `+0x20` / `+0x24` (the chunk framing is not part of this module) |
//! | 938EE0 / 93A3B0 | `KKC_MGState` Save / Load (load target built by 939930 -> ctor 939690 and published in `B49FA0`) |
//! | 95A470 / 95AAC0 | MG kingdom Save / Load (ctor 95A1C0) |
//! | 955160 / 9565E0 | MG territory Save / Load (ctor 954A40; owner 938BC0, 959F20, 95A850, 954CF0; 956570) |
//! | 954970 / 9549C0 | territory army reference Save / Load (vtable ACBDCC; ctor 954860) |
//! | 93F1A0 / 9409E0 | army snapshot (0x28 bytes) Save / Load (ctor 93F590; captured by 9405E0 -> 9402E0 when 956140 creates a reference) |
//! | 95B020 / 95B0F0 | event state Save / Load (ctor 95AD90; the loader does not store them) |
//! | 95F000 / 95F100 | battle record Save / Load (ctors 95EFD0 / 95EF90) |
//! | 93EEA0 / 940740 | army MetaGame component record (+ updatable id 50BC10 / 50BCD0) |
//! | 8589B0 | data version: write `max`; read, `v < 0 \|\| v > max` -> "Bad data version" (41DFB0) |
//! | 41D290 | wide string reader |
//! | 4E7EC0, 4DAAA0, 95AA00, 940680 | IDS readers: empty -> null, unknown -> "Unknown IDS %s for class %hs" (41DFB0) |
//! | 939A20, 95C0D0, 95C130, 95D900, 938BC0, 939B60, 427BD0 + 4BDBA0, 9389F0 | lookups used by the reader |
//!
//! Proven against `research/phase8-metagame/native-save.json` (original code
//! under Unicorn on all shipped campaigns) by `tests/native_save.rs`.
//!
//! Model. The reader is split like the native object graph:
//! [`parse_metagame`] reproduces 93A3B0 into a [`ParsedMetagame`] whose
//! [`MetagameImage`] is the loaded `KKC_MGState` with IDS/handles instead of
//! pointers, raising the native errors in native order and an explicit
//! [`Error::Unsupported`] where the original dereferences a null pointer while
//! loading. [`ParsedMetagame::into_state`] converts the image into an
//! [`MgState`] and refuses what [`MgState`] cannot represent. The writer goes
//! the other way ([`MetagameImage::from_state`], [`MetagameImage::write`];
//! [`write_metagame`]). Every field the payload carries lives in [`MgState`]
//! (faction, RTS winner, event states, help popups, territory visual handles,
//! army reference snapshots captured by [`MgState::enter_territory`]).
//!
//! Engine identities the payload stores as raw GTID handles (kingdom actors,
//! army actors) and the save-session updatable ids are explicit
//! inputs/outputs ([`EngineIdentities`], [`ArmyComponentImage`]), never
//! invented. Data registries the readers search that `MetaGameData` does not
//! hold (factions, shared properties, help popups, Things) are
//! [`SaveRegistries`].
//!
//! Lookups compare IDS like the native `__wcsicmp` in the C locale (ASCII
//! case folding) except the current kingdom, which 93A3B0 compares with
//! `wcscmp` (case-sensitive) against the resolved kingdom data IDS.
use crate::battle::{DivisionRef, KingdomActor};
use crate::data::{Campaign, MapDefinition, MetaGameData};
use crate::rng::MetaGameRng;
use crate::state::{ArmyId, ArmyRef, ArmySnapshot, BattleRecord, EventState, MgArmy, MgKingdom, MgState, MgTerritory};
use crate::{Error, Result};
use serde::{Deserialize, Serialize};

/// Message thrown by 41DFB0 from the version helper 8589B0.
pub const BAD_DATA_VERSION: &str = "Bad data version";
/// Class names passed to "Unknown IDS %s for class %hs".
pub const CLASS_FACTION: &str = "KKC_FactionData";
pub const CLASS_SHARED_PROPERTY: &str = "KKC_SharedPropertyData";
pub const CLASS_HELP_POPUP: &str = "KKC_MGHelpPopupData";
pub const CLASS_DIVISION: &str = "KKC_MGDivisionData";

/// The C-string view the native comparisons and `%s` use (up to the first
/// zero unit; CStrings keep their full length otherwise).
fn c_str(s: &str) -> &str {
    s.split('\0').next().unwrap_or("")
}

fn unknown_ids(ids: &str, class: &str) -> Error {
    Error::NativeData(format!("Unknown IDS {} for class {class}", c_str(ids)))
}

/// `__wcsicmp` (96CAD6) in the C locale: ASCII case folding, C strings.
fn same(a: &str, b: &str) -> bool {
    c_str(a).eq_ignore_ascii_case(c_str(b))
}

// ---------------------------------------------------------------- engine inputs

/// Engine identities the payload stores.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct EngineIdentities {
    /// GTID handle of each kingdom actor (`MGKingdom+0x0C`). A kingdom
    /// without actor is stored as 0 (939B90 writes `actor ? actor+0x14 : 0`).
    pub kingdom_actors: Vec<(KingdomActor, u32)>,
    /// GTID handle of each army actor (territory army reference `+0x04`,
    /// MGState `+0x4C`). `last_purchased == None` is stored as 0 (the value
    /// of the constructors 9397E0/939690).
    pub armies: Vec<(ArmyId, u32)>,
}

impl EngineIdentities {
    pub fn kingdom_handle(&self, actor: Option<KingdomActor>) -> Result<u32> {
        let Some(actor) = actor else { return Ok(0) };
        let h = self
            .kingdom_actors
            .iter()
            .find(|(a, _)| *a == actor)
            .map(|(_, h)| *h)
            .ok_or_else(|| Error::Unsupported(format!("no GTID handle for kingdom actor {}", actor.0)))?;
        if h == 0 {
            return Err(Error::Unsupported("kingdom actor handle 0 is the no-actor value".into()));
        }
        Ok(h)
    }
    pub fn kingdom_actor(&self, handle: u32) -> Result<Option<KingdomActor>> {
        if handle == 0 {
            return Ok(None);
        }
        let mut found = self.kingdom_actors.iter().filter(|(_, h)| *h == handle);
        match (found.next(), found.next()) {
            (Some((a, _)), None) => Ok(Some(*a)),
            (None, _) => Err(Error::Unsupported(format!("kingdom actor handle {handle:#x} not known to the engine"))),
            _ => Err(Error::Unsupported(format!("kingdom actor handle {handle:#x} listed twice"))),
        }
    }
    pub fn army_handle(&self, army: ArmyId) -> Result<u32> {
        let h = self
            .armies
            .iter()
            .find(|(a, _)| *a == army)
            .map(|(_, h)| *h)
            .ok_or_else(|| Error::Unsupported(format!("no GTID handle for army {}", army.0)))?;
        if h == 0 {
            return Err(Error::Unsupported("army handle 0 is the no-army value".into()));
        }
        Ok(h)
    }
    pub fn army(&self, handle: u32) -> Result<ArmyId> {
        let mut found = self.armies.iter().filter(|(_, h)| *h == handle);
        match (found.next(), found.next()) {
            (Some((a, _)), None) => Ok(*a),
            (None, _) => Err(Error::Unsupported(format!("army handle {handle:#x} not known to the engine"))),
            _ => Err(Error::Unsupported(format!("army handle {handle:#x} listed twice"))),
        }
    }
}

/// KKC data registries searched by the readers that [`MetaGameData`] does
/// not hold. Campaigns, campaign kingdoms, map territories and divisions come
/// from [`MetaGameData`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SaveRegistries {
    /// `KKC_FactionData` IDS (4E7EC0 via `[B3AA4C]`).
    pub factions: Vec<String>,
    /// `KKC_SharedPropertyData` IDS (4DAAA0 via `[B3AA2C]`): technologies.
    pub shared_properties: Vec<String>,
    /// `KKC_MGHelpPopupData` IDS (95AA00 via `[B4A074]`).
    pub help_popups: Vec<String>,
    /// Things (427BD0 via `[[B36530]+0x0C]`) with the result of the Thing
    /// virtual `+0x38` that 4BDBA0 requires (`true` for actors).
    pub things: Vec<(String, bool)>,
}

/// Data-manager lookup (939A20 / 48D500 / 463FA0 / 93E170 / 936CF0 / 427BD0
/// through 4915F0): a name whose first unit is zero -> none; otherwise the
/// case-insensitive match. The native hash chain order is not modelled, so
/// an IDS matching two entries is refused.
fn registry<'a, T>(items: &'a [T], ids: &str, key: impl Fn(&T) -> &str) -> Result<Option<(usize, &'a T)>> {
    if c_str(ids).is_empty() {
        return Ok(None);
    }
    let mut found = items.iter().enumerate().filter(|(_, x)| same(key(x), ids));
    match (found.next(), found.next()) {
        (Some(x), None) => Ok(Some(x)),
        (None, _) => Ok(None),
        _ => Err(Error::Unsupported(format!("registry lookup of {ids} is ambiguous (hash chain order not modelled)"))),
    }
}

/// IDS reader shape (4E7EC0, 4DAAA0, 95AA00, 940680): a zero-length string
/// (`[s-0x0C] == 0`) -> null, otherwise the registry lookup and, when it
/// finds nothing, the native error; the found object's own IDS is returned.
fn read_ids_ref<'a, T>(ids: &str, items: &'a [T], key: impl Fn(&T) -> &str, class: &str) -> Result<Option<(usize, &'a T)>> {
    if ids.is_empty() {
        return Ok(None);
    }
    match registry(items, ids, key)? {
        Some(x) => Ok(Some(x)),
        None => Err(unknown_ids(ids, class)),
    }
}

// ---------------------------------------------------------------- stream

/// Engine stream writer (`S = ctx+0x3C`, vtable `+4 Write`).
#[derive(Default)]
struct Writer {
    out: Vec<u8>,
}

impl Writer {
    fn u32(&mut self, v: u32) {
        self.out.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.u32(v as u32);
    }
    fn u8(&mut self, v: u8) {
        self.out.push(v);
    }
    /// Inline string writer of every Save: the low 16 bits of the CString
    /// length (`[p-0x0C]` read as a word), then that many UTF-16 units.
    fn wstr(&mut self, s: &str) {
        let units: Vec<u16> = s.encode_utf16().collect();
        let n = units.len() as u16;
        self.out.extend_from_slice(&n.to_le_bytes());
        for u in &units[..usize::from(n)] {
            self.out.extend_from_slice(&u.to_le_bytes());
        }
    }
    /// 8589B0 on a writing stream: the version written is `max`.
    fn version(&mut self, max: u32) {
        self.u32(max);
    }
}

/// Engine stream reader (vtable `+0 Read`). Reading past the end is refused:
/// the engine stream's behaviour at the end of a chunk is not established.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if self.bytes.len() - self.pos < n {
            return Err(Error::Unsupported(format!(
                "METAGAME stream: read of {n} bytes at {} past the end ({}) (engine stream end behaviour not established)",
                self.pos,
                self.bytes.len()
            )));
        }
        let s = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("4 bytes")))
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(self.u32()? as i32)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    /// Signed loop count (`jle` on the value read).
    fn count(&mut self) -> Result<usize> {
        Ok(self.i32()?.max(0) as usize)
    }
    /// 41D290: `u16` unit count, then the units (the CString keeps any value,
    /// including embedded zero units).
    fn wstr(&mut self) -> Result<String> {
        let n = u16::from_le_bytes(self.take(2)?.try_into().expect("2 bytes"));
        let raw = self.take(2 * usize::from(n))?;
        let units: Vec<u16> = raw.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16(&units)
            .map_err(|_| Error::Unsupported("METAGAME string with unpaired UTF-16 surrogates".into()))
    }
    /// 8589B0 on a reading stream.
    fn version(&mut self, max: i32) -> Result<i32> {
        let v = self.i32()?;
        if v < 0 || v > max {
            return Err(Error::NativeData(BAD_DATA_VERSION.into()));
        }
        Ok(v)
    }
}

// ---------------------------------------------------------------- image

/// An event state as the payload holds it (95B020 / 95B0F0): event data IDS,
/// triggered byte, target campaign kingdom IDS. 93A3B0 reads these and does
/// not store them ([`ParsedMetagame::discarded_events`]).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedEventState {
    /// IDS of the campaign event data (`[[+0x08]+4]`).
    pub event: String,
    pub flag: u8,
    /// IDS of the target campaign kingdom; `None` is written as empty.
    pub target: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomImage {
    /// `+0x08`: campaign kingdom index.
    pub data: Option<usize>,
    /// `+0x0C`: kingdom actor GTID handle.
    pub actor: u32,
    /// `+0x10`: capital (territory position).
    pub capital: Option<usize>,
    /// `+0x14`: owned territories (rebuilt by the loader in territory order).
    pub territories: Vec<usize>,
    /// `+0x20`: technologies (registered IDS; `None` = NULL entry, written empty).
    pub technologies: Vec<Option<String>>,
    /// `+0x38`, `+0x3C`.
    pub resources: i32,
    pub battles: i32,
    /// `+0x2C`: help popups (registered IDS; `None` = NULL entry, written
    /// empty, skipped by the loader).
    pub help_popups: Vec<Option<String>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmyRefImage {
    /// `+0x04`: army actor GTID handle.
    pub handle: u32,
    /// `+0x08`: display slot.
    pub slot: i32,
    /// `+0x0C`: snapshot.
    pub snapshot: ArmySnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerritoryImage {
    /// `+0x04`: index field.
    pub index: u32,
    /// `+0x0C`: map territory definition.
    pub definition: usize,
    /// `+0x20`: territory visual handle (engine).
    pub visual: u32,
    /// `+0x24`: owner (kingdom position).
    pub owner: Option<usize>,
    /// `+0x10`: army references in list order.
    pub armies: Vec<ArmyRefImage>,
    /// `+0x1C`.
    pub enemy_army_count: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordImage {
    /// `+0x04` territory, `+0x08` attacker, `+0x0C` defender (positions).
    pub territory: Option<usize>,
    pub attacker: Option<usize>,
    pub defender: Option<usize>,
    /// `+0x10`.
    pub attacker_won: u8,
}

/// `KKC_MGState` as written by 938EE0 / rebuilt by 93A3B0.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct MetagameImage {
    /// `+0x08`: campaign index.
    pub campaign: Option<usize>,
    /// `+0x0C`: faction (registered IDS).
    pub faction: Option<String>,
    /// `+0x10`: current kingdom (position).
    pub current: Option<usize>,
    /// `+0x14`, `+0x18`.
    pub round: i32,
    pub rng: u32,
    pub kingdoms: Vec<KingdomImage>,
    pub territories: Vec<TerritoryImage>,
    /// `+0x54`: event states (written; the loader reads and drops them, so a
    /// parsed image has none).
    pub events: Vec<EventState>,
    /// `+0x44`, `+0x45`, `+0x46`.
    pub flags_44_45_46: [u8; 3],
    /// `+0x48`: RTS winner name ([`MgState::rts_winner`]).
    pub rts_winner: String,
    /// `+0x4C`: last purchased army handle (0 = none).
    pub last_purchased: u32,
    /// `+0x50`, `+0x51`.
    pub byte_50: u8,
    pub byte_51: u8,
    /// `+0x60`: battle records.
    pub records: Vec<RecordImage>,
}

fn bool_byte(v: u8, what: &str) -> Result<bool> {
    match v {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(Error::Unsupported(format!("{what} byte {v} is not representable as bool"))),
    }
}

/// Result of [`parse_metagame`]: the loaded `KKC_MGState` image, the event
/// states 93A3B0 read and dropped (95B0F0 objects never stored: the loaded
/// state has no event states), and the bytes consumed (the original reads
/// exactly its fields; trailing chunk bytes are not inspected here).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedMetagame {
    pub image: MetagameImage,
    pub discarded_events: Vec<SavedEventState>,
    pub consumed: usize,
}

impl ParsedMetagame {
    /// [`MetagameImage::into_state`], keeping the dropped event states.
    pub fn into_state(
        self,
        ids: &EngineIdentities,
        armies: Vec<Option<MgArmy>>,
        kingdom_names: Vec<(KingdomActor, String)>,
    ) -> Result<LoadedMetagame> {
        let state = self.image.into_state(ids, armies, kingdom_names)?;
        Ok(LoadedMetagame { state, discarded_events: self.discarded_events })
    }
}

/// Result of [`ParsedMetagame::into_state`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedMetagame {
    pub state: MgState,
    /// Event states present in the payload: 93A3B0 reads them (95B0F0) and
    /// does not store them, so the loaded state has none.
    pub discarded_events: Vec<SavedEventState>,
}

impl MetagameImage {
    /// The image 938EE0 writes for `state`. Engine values the state does not
    /// hold (territory visual handles) are refused.
    pub fn from_state(state: &MgState, ids: &EngineIdentities) -> Result<Self> {
        let opt = |s: &String| (!s.is_empty()).then(|| s.clone());
        let kingdoms = state
            .kingdoms
            .iter()
            .map(|k| {
                Ok(KingdomImage {
                    data: Some(k.data),
                    actor: ids.kingdom_handle(k.actor)?,
                    capital: k.capital,
                    territories: k.territories.clone(),
                    technologies: k.technologies.iter().map(opt).collect(),
                    resources: k.resources,
                    battles: k.battles,
                    help_popups: k.help_popups.iter().map(opt).collect(),
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let territories = state
            .territories
            .iter()
            .enumerate()
            .map(|(t, territory)| {
                let armies = territory
                    .armies
                    .iter()
                    .map(|r| Ok(ArmyRefImage { handle: ids.army_handle(r.army)?, slot: r.slot, snapshot: r.snapshot.clone() }))
                    .collect::<Result<Vec<_>>>()?;
                Ok(TerritoryImage {
                    index: t as u32,
                    definition: territory.definition,
                    visual: territory.visual.ok_or_else(|| {
                        Error::Unsupported(format!("territory {t}: visual handle not supplied by the engine"))
                    })?,
                    owner: territory.owner,
                    armies,
                    enemy_army_count: territory.enemy_army_count,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(MetagameImage {
            campaign: Some(state.campaign),
            faction: state.faction.clone(),
            current: state.current,
            round: state.round,
            rng: state.rng.state,
            kingdoms,
            territories,
            events: state.events.clone(),
            flags_44_45_46: [
                u8::from(state.battle_started),
                u8::from(state.battle_play_requested),
                u8::from(state.battle_resolved),
            ],
            rts_winner: state.rts_winner.clone(),
            last_purchased: match state.last_purchased {
                None => 0,
                Some(a) => ids.army_handle(a)?,
            },
            byte_50: u8::from(state.game_over),
            byte_51: state.fast_ai,
            records: state
                .results
                .iter()
                .map(|r| RecordImage {
                    territory: Some(r.territory),
                    attacker: r.attacker,
                    defender: r.defender,
                    attacker_won: u8::from(r.attacker_won),
                })
                .collect(),
        })
    }

    /// 938EE0. A null pointer the original dereferences while writing is
    /// refused ([`Error::Unsupported`]); null references written as empty
    /// strings are written as empty strings.
    pub fn write(&self, data: &MetaGameData) -> Result<Vec<u8>> {
        let crash = |what: &str| Error::Unsupported(format!("938EE0 dereferences a null {what}"));
        let c = self.campaign.ok_or_else(|| crash("campaign"))?;
        let campaign = data.campaigns.get(c).ok_or_else(|| crash("campaign"))?;
        let map = campaign.map.ok_or_else(|| crash("map"))?;
        let map = &data.maps[map];
        let kingdom_ids = |k: usize| -> Result<&str> {
            let image = self.kingdoms.get(k).ok_or_else(|| crash("kingdom"))?;
            let d = image.data.ok_or_else(|| crash("kingdom data"))?;
            Ok(campaign.kingdoms[d].ids.as_str())
        };
        let territory_ids = |d: usize| map.territories[d].ids.as_str();
        let mut w = Writer::default();
        w.version(0);
        w.wstr(&campaign.ids);
        w.wstr(kingdom_ids(self.current.ok_or_else(|| crash("current kingdom"))?)?);
        w.i32(self.round);
        w.u32(self.rng);
        // kingdoms: 95A470
        w.u32(self.kingdoms.len() as u32);
        for (i, k) in self.kingdoms.iter().enumerate() {
            w.version(0);
            w.wstr(kingdom_ids(i)?);
            w.u32(k.actor);
            w.u32(k.technologies.len() as u32);
            for t in &k.technologies {
                w.wstr(t.as_deref().unwrap_or(""));
            }
            w.i32(k.resources);
            w.i32(k.battles);
            w.u32(k.help_popups.len() as u32);
            for p in &k.help_popups {
                w.wstr(p.as_deref().unwrap_or(""));
            }
        }
        // territories: 955160
        w.u32(self.territories.len() as u32);
        for t in &self.territories {
            w.version(0);
            w.u32(t.index);
            w.wstr(territory_ids(t.definition));
            w.u32(t.visual);
            w.u8(u8::from(t.owner.is_some()));
            if let Some(o) = t.owner {
                w.wstr(kingdom_ids(o)?);
            }
            w.u32(t.armies.len() as u32);
            for r in &t.armies {
                // 954970 + 93F1A0
                w.u32(r.handle);
                w.i32(r.slot);
                let s = &r.snapshot;
                w.version(1);
                w.i32(s.fought);
                w.wstr(territory_ids(s.territory.ok_or_else(|| crash("snapshot territory"))?));
                w.u32(s.path.len() as u32);
                for p in &s.path {
                    w.wstr(territory_ids(p.ok_or_else(|| crash("snapshot path territory"))?));
                }
                w.u32(s.divisions.len() as u32);
                for d in &s.divisions {
                    w.wstr(d.map(|d| data.divisions[d.0 as usize].ids.as_str()).unwrap_or(""));
                }
                w.wstr(s.owner_name.as_deref().ok_or_else(|| {
                    Error::Unsupported("army snapshot without the engine kingdom name (4BEDF0 not supplied)".into())
                })?);
                let def = s.definition.ok_or_else(|| crash("snapshot actor definition"))?;
                w.wstr(&data.armies.get(def).ok_or_else(|| crash("snapshot actor definition"))?.ids);
            }
            w.i32(t.enemy_army_count);
        }
        // capitals: [[k+0x10]+4]
        for k in &self.kingdoms {
            let capital = k.capital.ok_or_else(|| crash("capital"))?;
            w.u32(self.territories.get(capital).ok_or_else(|| crash("capital"))?.index);
        }
        // events: 95B020 (no version): [[+8]+4], +0x0C, [[+0x10]+4] or empty
        w.u32(self.events.len() as u32);
        for e in &self.events {
            w.wstr(&campaign.events.get(e.event).ok_or_else(|| crash("event data"))?.ids);
            w.u8(u8::from(e.triggered));
            w.wstr(match e.kingdom {
                None => "",
                Some(k) => &campaign.kingdoms.get(k).ok_or_else(|| crash("event kingdom"))?.ids,
            });
        }
        for b in self.flags_44_45_46 {
            w.u8(b);
        }
        w.wstr(&self.rts_winner);
        w.u32(self.last_purchased);
        w.u8(self.byte_50);
        w.u8(self.byte_51);
        w.wstr(self.faction.as_deref().unwrap_or(""));
        // records: 95F000
        w.u32(self.records.len() as u32);
        for r in &self.records {
            w.version(0);
            let t = r.territory.ok_or_else(|| crash("record territory"))?;
            w.wstr(territory_ids(self.territories.get(t).ok_or_else(|| crash("record territory"))?.definition));
            w.wstr(kingdom_ids(r.attacker.ok_or_else(|| crash("record attacker"))?)?);
            w.wstr(kingdom_ids(r.defender.ok_or_else(|| crash("record defender"))?)?);
            w.u8(r.attacker_won);
        }
        Ok(w.out)
    }

    /// Convert a loaded image into the Rust state. `armies` is the army arena
    /// rebuilt from `DYNAMIC_THINGS` (army actors and their component
    /// records; METAGAME is loaded before DYNAMIC_THINGS by 4FC600, the
    /// payload only references armies by handle) and `kingdom_names` the
    /// engine names of the kingdom actors ([`MgState::kingdom_names`]).
    pub fn into_state(
        self,
        ids: &EngineIdentities,
        armies: Vec<Option<MgArmy>>,
        kingdom_names: Vec<(KingdomActor, String)>,
    ) -> Result<MgState> {
        let campaign = self
            .campaign
            .ok_or_else(|| Error::Unsupported("loaded MGState without campaign (native keeps null)".into()))?;
        let kingdoms = self
            .kingdoms
            .iter()
            .map(|k| {
                Ok(MgKingdom {
                    data: k
                        .data
                        .ok_or_else(|| Error::Unsupported("kingdom without campaign kingdom data".into()))?,
                    actor: ids.kingdom_actor(k.actor)?,
                    capital: k.capital,
                    territories: k.territories.clone(),
                    technologies: k.technologies.iter().map(|t| t.clone().unwrap_or_default()).collect(),
                    help_popups: k.help_popups.iter().map(|p| p.clone().unwrap_or_default()).collect(),
                    resources: k.resources,
                    battles: k.battles,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let mut territories = Vec::with_capacity(self.territories.len());
        for (pos, t) in self.territories.iter().enumerate() {
            if t.index as usize != pos {
                return Err(Error::Unsupported(format!(
                    "territory {pos} has index field {} (not representable)",
                    t.index
                )));
            }
            let mut refs: Vec<ArmyRef> = Vec::with_capacity(t.armies.len());
            for r in &t.armies {
                let army = ids.army(r.handle)?;
                if refs.iter().any(|x| x.army == army) {
                    return Err(Error::Unsupported(format!("army {} referenced twice by territory {pos}", army.0)));
                }
                refs.push(ArmyRef { army, slot: r.slot, snapshot: r.snapshot.clone() });
            }
            territories.push(MgTerritory {
                record: None,
                definition: t.definition,
                armies: refs,
                enemy_army_count: t.enemy_army_count,
                owner: t.owner,
                visual: Some(t.visual),
            });
        }
        let results = self
            .records
            .iter()
            .map(|r| {
                Ok(BattleRecord {
                    territory: r.territory.ok_or_else(|| {
                        Error::Unsupported("battle record with a null territory (939B60 found none)".into())
                    })?,
                    attacker: r.attacker,
                    defender: r.defender,
                    attacker_won: bool_byte(r.attacker_won, "battle record +0x10")?,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let state = MgState {
            campaign,
            current: self.current,
            rng: MetaGameRng::new(self.rng),
            kingdoms,
            territories,
            armies,
            round: self.round,
            battle_started: bool_byte(self.flags_44_45_46[0], "MGState +0x44")?,
            battle_play_requested: bool_byte(self.flags_44_45_46[1], "MGState +0x45")?,
            battle_resolved: bool_byte(self.flags_44_45_46[2], "MGState +0x46")?,
            game_over: bool_byte(self.byte_50, "MGState +0x50")?,
            fast_ai: self.byte_51,
            last_purchased: match self.last_purchased {
                0 => None,
                h => Some(ids.army(h)?),
            },
            results,
            faction: self.faction,
            rts_winner: self.rts_winner,
            events: self.events,
            kingdom_names,
        };
        for t in &state.territories {
            for r in &t.armies {
                if state.armies.get(r.army.0 as usize).is_none_or(Option::is_none) {
                    return Err(Error::Unsupported(format!("army {} is not in the supplied arena", r.army.0)));
                }
            }
        }
        Ok(state)
    }
}

/// 938EE0 for a Rust state.
pub fn write_metagame(state: &MgState, data: &MetaGameData, ids: &EngineIdentities) -> Result<Vec<u8>> {
    MetagameImage::from_state(state, ids)?.write(data)
}

/// [`parse_metagame`] followed by [`ParsedMetagame::into_state`].
pub fn read_metagame(
    bytes: &[u8],
    data: &MetaGameData,
    registries: &SaveRegistries,
    ids: &EngineIdentities,
    armies: Vec<Option<MgArmy>>,
    kingdom_names: Vec<(KingdomActor, String)>,
) -> Result<LoadedMetagame> {
    parse_metagame(bytes, data, registries)?.into_state(ids, armies, kingdom_names)
}

/// 93A3B0 on a state fresh from 939690.
pub fn parse_metagame(bytes: &[u8], data: &MetaGameData, registries: &SaveRegistries) -> Result<ParsedMetagame> {
    let mut p = Parser { r: Reader { bytes, pos: 0 }, data, registries, image: None, discarded_events: Vec::new() };
    let image = p.mgstate()?;
    Ok(ParsedMetagame { image, discarded_events: p.discarded_events, consumed: p.r.pos })
}

fn null(what: &str) -> Error {
    Error::Unsupported(format!("93A3B0 dereferences a null {what} (native crash)"))
}

/// 95D900 (hash 405030 + `__wcsicmp`) on the campaign map.
fn map_territory(map: &MapDefinition, ids: &str) -> Option<usize> {
    map.territories.iter().position(|t| same(&t.ids, ids))
}

struct Parser<'a> {
    r: Reader<'a>,
    data: &'a MetaGameData,
    registries: &'a SaveRegistries,
    image: Option<MetagameImage>,
    discarded_events: Vec<SavedEventState>,
}

impl<'a> Parser<'a> {
    fn image(&self) -> &MetagameImage {
        self.image.as_ref().expect("image under construction")
    }
    fn image_mut(&mut self) -> &mut MetagameImage {
        self.image.as_mut().expect("image under construction")
    }
    /// `[MGState+0x08]`, dereferenced by 95C0D0 / 95C130 / 938970.
    fn campaign(&self) -> Result<&'a Campaign> {
        let data = self.data;
        self.image().campaign.map(|c| &data.campaigns[c]).ok_or_else(|| null("campaign"))
    }
    /// 938970: `[[MGState+0x08]+0x18]`.
    fn map(&self) -> Result<&'a MapDefinition> {
        let data = self.data;
        let c = self.campaign()?;
        Ok(&data.maps[c.map.ok_or_else(|| null("campaign map"))?])
    }
    /// `[[k+0x08]+4]` of a loaded kingdom (non-null once the kingdom loaded).
    fn kingdom_ids(&self, k: usize) -> &'a str {
        let data = self.data;
        let c = self.image().campaign.expect("kingdoms exist only with a campaign");
        let d = self.image().kingdoms[k].data.expect("kingdom data checked at load");
        data.campaigns[c].kingdoms[d].ids.as_str()
    }
    /// 938BC0 on the MGState being loaded: first case-insensitive match.
    fn find_kingdom(&self, ids: &str) -> Option<usize> {
        (0..self.image().kingdoms.len()).find(|&k| same(self.kingdom_ids(k), ids))
    }

    fn mgstate(&mut self) -> Result<MetagameImage> {
        self.r.version(0)?;
        // campaign: 939A20 on the campaign registry ([[B36530]+0x820]+0x18)
        let campaign_ids = self.r.wstr()?;
        let campaign = registry(&self.data.campaigns, &campaign_ids, |c| c.ids.as_str())?.map(|(i, _)| i);
        let current_ids = self.r.wstr()?;
        let round = self.r.i32()?;
        let rng = self.r.u32()?;
        // the 939690 state the load fills
        self.image = Some(MetagameImage {
            campaign,
            faction: None,
            current: None,
            round,
            rng,
            kingdoms: Vec::new(),
            territories: Vec::new(),
            events: Vec::new(),
            flags_44_45_46: [0; 3],
            rts_winner: String::new(),
            last_purchased: 0,
            byte_50: 0,
            byte_51: 1,
            records: Vec::new(),
        });
        // kingdoms: new 0x40 + 95A1C0 + 95AAC0, appended, then compared with
        // the current kingdom IDS by wcscmp (last match wins)
        for _ in 0..self.r.count()? {
            let k = self.kingdom()?;
            let position = self.image().kingdoms.len();
            self.image_mut().kingdoms.push(k);
            if c_str(self.kingdom_ids(position)) == c_str(&current_ids) {
                self.image_mut().current = Some(position);
            }
        }
        // territories: new 0x28 + 954A40(i) + 9565E0, appended, hash insert
        for _ in 0..self.r.count()? {
            let t = self.territory()?;
            self.image_mut().territories.push(t);
        }
        // capitals: for every kingdom, territories[u32] without a bounds check
        for k in 0..self.image().kingdoms.len() {
            let index = self.r.u32()? as usize;
            let n = self.image().territories.len();
            if index >= n {
                return Err(Error::Unsupported(format!(
                    "capital index {index} outside the {n} territories (native reads past the vector)"
                )));
            }
            self.image_mut().kingdoms[k].capital = Some(index);
        }
        // event states: new 0x14 + 95AD90 + 95B0F0, never stored
        for _ in 0..self.r.count()? {
            let event = self.r.wstr()?;
            self.campaign()?; // 95C130 on [state+0x08]
            let flag = self.r.u8()?;
            let target = self.r.wstr()?;
            if !target.is_empty() {
                self.campaign()?; // 95C0D0
            }
            let target = (!target.is_empty()).then_some(target);
            self.discarded_events.push(SavedEventState { event, flag, target });
        }
        let flags = [self.r.u8()?, self.r.u8()?, self.r.u8()?];
        let rts_winner = self.r.wstr()?;
        let last_purchased = self.r.u32()?;
        let byte_50 = self.r.u8()?;
        let byte_51 = self.r.u8()?;
        // faction: 4E7EC0
        let faction_ids = self.r.wstr()?;
        let faction = read_ids_ref(&faction_ids, &self.registries.factions, |s| s.as_str(), CLASS_FACTION)?
            .map(|(_, s)| s.clone());
        {
            let image = self.image_mut();
            image.flags_44_45_46 = flags;
            image.rts_winner = rts_winner;
            image.last_purchased = last_purchased;
            image.byte_50 = byte_50;
            image.byte_51 = byte_51;
            image.faction = faction;
        }
        // battle records: new 0x14 + 95EF90 + 95F100 (lookups through B49FA0,
        // which 939930 set to the state being loaded)
        for _ in 0..self.r.count()? {
            let record = self.record()?;
            self.image_mut().records.push(record);
        }
        Ok(self.image.take().expect("image"))
    }

    /// 95AAC0.
    fn kingdom(&mut self) -> Result<KingdomImage> {
        self.r.version(0)?;
        let data_ids = self.r.wstr()?;
        let actor = self.r.u32()?;
        // 573580 + 95C0D0: first case-insensitive match in the campaign kingdoms
        let data = self.campaign()?.kingdoms.iter().position(|k| same(&k.ids, &data_ids));
        let mut technologies = Vec::new();
        for _ in 0..self.r.count()? {
            let ids = self.r.wstr()?;
            let found = read_ids_ref(&ids, &self.registries.shared_properties, |s| s.as_str(), CLASS_SHARED_PROPERTY)?;
            technologies.push(found.map(|(_, s)| s.clone()));
        }
        let resources = self.r.i32()?;
        let battles = self.r.i32()?;
        let mut help_popups = Vec::new();
        for _ in 0..self.r.count()? {
            let ids = self.r.wstr()?;
            if let Some((_, s)) = read_ids_ref(&ids, &self.registries.help_popups, |s| s.as_str(), CLASS_HELP_POPUP)? {
                help_popups.push(Some(s.clone()));
            }
        }
        // 93A55C reads [[k+8]+4] right after the kingdom loaded
        let data = data.ok_or_else(|| null("campaign kingdom data (95C0D0 found none)"))?;
        Ok(KingdomImage {
            data: Some(data),
            actor,
            capital: None,
            territories: Vec::new(),
            technologies,
            resources,
            battles,
            help_popups,
        })
    }

    /// 9565E0 (the owner is appended to its kingdom list here: 95A850).
    fn territory(&mut self) -> Result<TerritoryImage> {
        self.r.version(0)?;
        let index = self.r.u32()?;
        let def_ids = self.r.wstr()?;
        let definition = map_territory(self.map()?, &def_ids);
        let visual = self.r.u32()?;
        let position = self.image().territories.len();
        let mut owner = None;
        if self.r.u8()? != 0 {
            let owner_ids = self.r.wstr()?;
            let k = self
                .find_kingdom(&owner_ids)
                .ok_or_else(|| null("owner kingdom (938BC0 found none; 95A850 on null)"))?;
            // 95A850 appends; 954CF0 recounts the still empty army list
            self.image_mut().kingdoms[k].territories.push(position);
            owner = Some(k);
        }
        // 956570 clears the (empty) list; references 954860 + 9549C0
        let mut armies = Vec::new();
        for _ in 0..self.r.count()? {
            let handle = self.r.u32()?;
            let slot = self.r.i32()?;
            let snapshot = self.snapshot()?;
            armies.push(ArmyRefImage { handle, slot, snapshot });
        }
        let enemy_army_count = self.r.i32()?;
        // 93A65E: the hash insert reads [[t+0x0C]+4]
        let definition = definition.ok_or_else(|| null("map territory (95D900 found none)"))?;
        Ok(TerritoryImage { index, definition, visual, owner, armies, enemy_army_count })
    }

    /// 9409E0 (map lookups 938970/95D900 through B49FA0 = the loading state).
    fn snapshot(&mut self) -> Result<ArmySnapshot> {
        self.r.version(1)?;
        let fought = self.r.i32()?;
        let t_ids = self.r.wstr()?;
        let territory = map_territory(self.map()?, &t_ids);
        let mut path = Vec::new();
        for _ in 0..self.r.count()? {
            let ids = self.r.wstr()?;
            path.push(map_territory(self.map()?, &ids));
        }
        let mut divisions = Vec::new();
        for _ in 0..self.r.count()? {
            let ids = self.r.wstr()?;
            let found = read_ids_ref(&ids, &self.data.divisions, |d| d.ids.as_str(), CLASS_DIVISION)?;
            divisions.push(found.map(|(i, _)| DivisionRef(i as u32)));
        }
        let owner_name = self.r.wstr()?;
        // 4BDBA0(name, 0x2B): 427BD0, then Thing vtable+0x38 must hold
        // (type 0x2B accepts every Thing type)
        let def_ids = self.r.wstr()?;
        let definition = match registry(&self.registries.things, &def_ids, |t| t.0.as_str())?.filter(|(_, t)| t.1) {
            None => None,
            Some((_, (ids, _))) => Some(self.data.armies.iter().position(|a| same(&a.ids, ids)).ok_or_else(|| {
                Error::Unsupported(format!("snapshot Thing {ids} is not an army type (not representable)"))
            })?),
        };
        Ok(ArmySnapshot { fought, territory, path, divisions, owner_name: Some(owner_name), definition })
    }

    /// 95F100.
    fn record(&mut self) -> Result<RecordImage> {
        self.r.version(0)?;
        let t_ids = self.r.wstr()?;
        // 939B60: the MGState territory hash, keyed by map territory IDS
        let territory = if self.image().territories.is_empty() {
            None
        } else {
            let map = self.map()?;
            let mut hits = self
                .image()
                .territories
                .iter()
                .enumerate()
                .filter(|(_, t)| same(&map.territories[t.definition].ids, &t_ids));
            match (hits.next(), hits.next()) {
                (Some((i, _)), None) => Some(i),
                (None, _) => None,
                _ => {
                    return Err(Error::Unsupported(
                        "two territories share a map definition (hash chain order not modelled)".into(),
                    ))
                }
            }
        };
        let attacker_ids = self.r.wstr()?;
        let attacker = self.find_kingdom(&attacker_ids);
        let defender_ids = self.r.wstr()?;
        let defender = self.find_kingdom(&defender_ids);
        let attacker_won = self.r.u8()?;
        Ok(RecordImage { territory, attacker, defender, attacker_won })
    }
}

// ---------------------------------------------------------------- army component record

/// Army MetaGame component record (93EEA0 / 940740), written inside the army
/// actor record of `DYNAMIC_THINGS`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArmyComponentImage {
    /// `+0x10` battles fought.
    pub fought: i32,
    /// `+0x20` territory position (written as its index field, -1 for null).
    pub territory: Option<usize>,
    /// `+0x24` path, `+0x30` waypoint list (territory positions).
    pub path: Vec<usize>,
    pub waypoints: Vec<usize>,
    /// `+0x14` divisions (`None` = null entry, written empty).
    pub divisions: Vec<Option<DivisionRef>>,
    /// Updatable id of the component's `+0x08` updatable (50BC10): the save
    /// session id when the updatable is registered (`+0x0C` set), -1
    /// otherwise; `None` when a record with updatable version 0 was read.
    pub updatable_id: Option<i32>,
}

impl ArmyComponentImage {
    /// `updatable_id`: the id the engine save session assigns (50BC10 takes
    /// the session counter `[[B36534]+0x280]` and post-increments it) when the
    /// component's updatable is registered, -1 when it is not.
    pub fn from_army(army: &MgArmy, updatable_id: i32) -> Self {
        ArmyComponentImage {
            fought: army.fought,
            territory: army.territory,
            path: army.path.clone(),
            waypoints: army.waypoints.clone(),
            divisions: army.divisions.iter().map(|&d| Some(d)).collect(),
            updatable_id: Some(updatable_id),
        }
    }

    /// 93EEA0 (territory index fields are the Rust positions).
    pub fn write(&self, data: &MetaGameData) -> Result<Vec<u8>> {
        let mut w = Writer::default();
        w.version(1);
        w.i32(self.fought);
        w.i32(self.territory.map_or(-1, |t| t as i32));
        w.u32(self.path.len() as u32);
        for &t in &self.path {
            w.u32(t as u32);
        }
        w.u32(self.waypoints.len() as u32);
        for &t in &self.waypoints {
            w.u32(t as u32);
        }
        w.u32(self.divisions.len() as u32);
        for d in &self.divisions {
            w.wstr(d.map(|d| data.divisions[d.0 as usize].ids.as_str()).unwrap_or(""));
        }
        // 50BC10
        w.version(1);
        w.i32(self
            .updatable_id
            .ok_or_else(|| Error::Unsupported("50BC10 always writes an updatable id".into()))?);
        Ok(w.out)
    }

    /// 940740 with territory indices resolved by 9389F0 against a state of
    /// `territory_count` territories (no bounds check natively: an index
    /// outside the vector, including the -1 written for a null territory, is
    /// refused). Returns the image and the bytes consumed.
    pub fn parse(bytes: &[u8], data: &MetaGameData, territory_count: usize) -> Result<(Self, usize)> {
        let mut r = Reader { bytes, pos: 0 };
        let territory = |v: u32| -> Result<usize> {
            let i = v as usize;
            if (v as i32) < 0 || i >= territory_count {
                return Err(Error::Unsupported(format!(
                    "component territory index {} outside the {territory_count} territories (9389F0 reads past the vector)",
                    v as i32
                )));
            }
            Ok(i)
        };
        r.version(1)?;
        let fought = r.i32()?;
        let current = territory(r.u32()?)?;
        let mut path = Vec::new();
        for _ in 0..r.count()? {
            path.push(territory(r.u32()?)?);
        }
        let mut waypoints = Vec::new();
        for _ in 0..r.count()? {
            waypoints.push(territory(r.u32()?)?);
        }
        let mut divisions = Vec::new();
        for _ in 0..r.count()? {
            let ids = r.wstr()?;
            let found = read_ids_ref(&ids, &data.divisions, |d| d.ids.as_str(), CLASS_DIVISION)?;
            divisions.push(found.map(|(i, _)| DivisionRef(i as u32)));
        }
        // 50BCD0: version 0 reads no id
        let updatable_id = if r.version(1)? == 0 { None } else { Some(r.i32()?) };
        Ok((
            ArmyComponentImage { fought, territory: Some(current), path, waypoints, divisions, updatable_id },
            r.pos,
        ))
    }

    /// Apply a loaded record to the army's metagame fields.
    pub fn apply_to(&self, army: &mut MgArmy) -> Result<()> {
        army.fought = self.fought;
        army.territory = self.territory;
        army.path = self.path.clone();
        army.waypoints = self.waypoints.clone();
        army.divisions = self
            .divisions
            .iter()
            .map(|d| d.ok_or_else(|| Error::Unsupported("null division in an army component".into())))
            .collect::<Result<Vec<_>>>()?;
        Ok(())
    }
}
