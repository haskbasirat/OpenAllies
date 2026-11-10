//! Writing a saved game, the counterpart of [`crate::saved_world`]: the Save Game
//! task (`KKC_ResourceManagerTaskSaveGame::SaveGame` 0x5A0500 -> SaveFull 0x5A0380
//! -> the resource's Save 0x5B26B0 / 0x5B1C70, which writes the map resource with
//! its virtual +0x50) and the world's chunks, written by GWorld's Save 0x4F2DB0.
//! 0x5A0500 calls SaveFull with 0, which skips SaveFull's verify and repair of the
//! map status (GWorld+44); only the task's Run 0x5A0BA0 in its mode 1 calls
//! SaveFull(1), and repairs only when the byte `[B39940]+0x280` is set. A game saved
//! from the game menu so keeps the map status the world had.
//!
//! The file is the original's chunk file ("TGCK" version 2, the container of
//! `aa_metagame::chunk_file`) with the chunks of [`CHUNKS`], in that order, every one
//! zlib-compressed but `MAP` when the save is compressed (the byte 4F2DB0 passes to
//! each chunk's open 0x859150, read through `[B39940]+0x28C`): the directory of every
//! original saved game has this order and these encodings.
//!
//! In part. What is ported here, each proven byte for byte against the original's
//! own saved games (`tests::original_saved_games_re_encode_exactly`):
//! - the chunk order and encodings ([`CHUNKS`]);
//! - `GWORLD` ([`GworldWords`]): 4F2DB0's version word (0x8589B0 with 28), then the
//!   world's +D4 stream, +DC tick, +E0 begin, +E4 end, +E8 step, +140, +184, +194,
//!   +198, +44 and +48, 48 bytes, named as the world's sync dump 0x4F6280 names
//!   them. [`GworldWords::of_world`] takes each from where `aa-play` keeps it (see
//!   there); the words no battle code of `aa-play` changes come from the GWORLD the
//!   battle was loaded from;
//! - `IDMGR` ([`IdmgrWords`]): the GTID manager's Save 0x4EFD30, version 0, the next
//!   slot (+60008), the highest slot (+6000C), the capacity 0x10000 and the
//!   generations of slots 0..highest (+4, `highest * 2` bytes; the load 0x4F0230
//!   reads that many). [`IdmgrWords::of_world`] writes them from `crate::ids`;
//! - `MISC_DATA` ([`MiscData`]), the tail of 4F2DB0, read and written again; `aa-play`
//!   keeps none of it yet;
//! - the record writes ([`Out`]): 0x4F1B90's head (definition, class, GTID) and
//!   GThing's part, and with them `TEAMS` ([`TeamsChunk`]: the count +170, a
//!   presence byte and a record for each team) and `KINGDOMS` ([`KingdomsChunk`]: the
//!   word +158, the count +14C, a presence byte and a record for each kingdom), each
//!   the inverse of `aa_mapworld`'s reader;
//! - the frame of `DYNAMIC_THINGS` and its index ([`ThingsChunk`], 0x4F24C0: a word 0
//!   and the version 11, the records, then the index's word 0, the count and each
//!   record's length);
//! - a thing's GActor tail ([`thing_tail`], [`Out::component`]): the masks, every
//!   saved component in the Load's order, an organization with its members' records
//!   ([`Out::organization`], [`Out::walked_record`]), then the actor's own words,
//!   each the inverse of `aa_mapworld`'s component readers and saved-world walk
//!   (`tests::original_saved_things_re_encode_from_the_walk`, `components_read_back`);
//! - `UPDATABLES` ([`UpdatablesChunk`]): the Updatable table's count and the five
//!   world queues.
//!
//! Written from the world (`of_world`), the rest of each chunk as the battle was
//! loaded: `GWORLD`, `IDMGR`; in `KINGDOMS` the stockpiles and the Commander ([`KingdomsChunk::of_world`]); in `DYNAMIC_THINGS` which things
//! are still live and where they stand ([`ThingsChunk::of_world`]). Not written from
//! the world yet: the components' contents (health, supply, queues), the Minds, the
//! actor state, a thing made since the load, a researched property, `TEAMS`,
//! `UPDATABLES` (`crate::scheduler`'s queues) and `MISC_DATA`.
//!
//! Not written at all yet, each by the original's writer named: `MAP` and
//! `MAP_HEADER` (the map resource), `RULES` (GWorld+0C), `TERRAIN` (+30),
//! `HEIGHT_MAP` (+34), `MOUNTAIN_HEIGHT_MAP` (+38, then the byte +275 in the same
//! chunk), `RESTRICTION_GROUPS` (0x503FA0), `OBJECTIVES` (count +254 and each
//! objective's Save, then the word +25C), `USED_REQUIRED_OBJECTS` with its index,
//! `TRIGGERS` (0x54C830), `DENIZEN_GROUPS` (0x56A2A0), `SUN` (+4C). A campaign
//! battle (rules mode 4) also writes `METAGAME` after `KINGDOMS` (0x938960). So a
//! battle cannot be saved from `aa-play` yet: these chunks taken from the map would
//! load as the battle's start, not as it stands.

use aa_mapworld::actor_payload::{ActorPayload, ActorTail, CountedList, PropertiesPayload, UpdatablePayload};
use aa_mapworld::inventory::ObjectState;
use aa_mapworld::saved_components::{SavedComponent, StructurePayload};
use aa_mapworld::saved_world::{DamageRecord, SavedOrganization, SavedProperty, SavedQueueItem, SavedThing};
use aa_mapworld::kingdoms::{
    AbilityState, Commander, KingdomPayload, Kingdoms, PropertyState, ReferenceWord, StrategicState, TeamRecord, Teams,
};
use aa_mapworld::recruitment::{ItemState, Recruitment};
use aa_mapworld::{Chunk, IdManagerState, Inventory, MapArchive, Placement, ThingState, WorldState};

/// The chunks of a saved battle outside the campaign in the order the original
/// writes them, and whether each is compressed in a compressed save (`MAP`, the
/// resource's own word, never). A campaign battle (rules mode 4) has `METAGAME`
/// after `KINGDOMS` as well; no campaign save has been checked.
pub const CHUNKS: [(&str, bool); 21] = [
    ("MAP", false),
    ("MAP_HEADER", true),
    ("GWORLD", true),
    ("RULES", true),
    ("TERRAIN", true),
    ("HEIGHT_MAP", true),
    ("MOUNTAIN_HEIGHT_MAP", true),
    ("IDMGR", true),
    ("RESTRICTION_GROUPS", true),
    ("TEAMS", true),
    ("OBJECTIVES", true),
    ("KINGDOMS", true),
    ("USED_REQUIRED_OBJECTS", true),
    ("USED_REQUIRED_OBJECTS_INDEX", true),
    ("DYNAMIC_THINGS", true),
    ("DYNAMIC_THINGS_INDEX", true),
    ("TRIGGERS", true),
    ("DENIZEN_GROUPS", true),
    ("SUN", true),
    ("UPDATABLES", true),
    ("MISC_DATA", true),
];

/// The version 4F2DB0 writes first (0x8589B0 with 0x1C).
pub const GWORLD_VERSION: u32 = 0x1C;

/// GWorld+48 `m_mission_state` once StartMission 0x4FDCE0 has run.
pub const MISSION_RUNNING: u32 = 1;
/// GWorld+48 once 0x4F4DA0 has settled the mission's result.
pub const MISSION_ENDED: u32 = 2;

/// The words of the `GWORLD` chunk, in 4F2DB0's order, with the names the world's
/// sync dump 0x4F6280 gives them.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GworldWords {
    /// +D4 `m_rng`: the world's random stream.
    pub stream: u32,
    /// +DC `m_game_ticks`.
    pub tick: u32,
    /// +E0, +E4, +E8 `m_game_time_begin_tick`, `m_game_time_end_tick`,
    /// `m_game_time_per_tick`, as bits.
    pub begin: u32,
    pub end: u32,
    pub step: u32,
    /// +140 `m_next_gplayer_id`. Nothing but the constructor 0x4F7D00 (0) and the
    /// load writes it that has been found; 0 on every original save.
    pub next_gplayer_id: u32,
    /// +184 `m_next_camera_id`, raised by the camera's virtual 0x4DBFB0 as it names
    /// a new camera `camera_%d`.
    pub next_camera_id: u32,
    /// +194 `m_next_area_id`, raised by the area's virtual 0x4DB890 (`area_%d`).
    pub next_area_id: u32,
    /// +198 `m_next_structure_queue_item_id`: the KKC_StructureQueueItem
    /// constructors 0x5895B0, 0x5896E0, 0x589830 and 0x589920 raise it and take the
    /// new value as the item's id.
    pub next_structure_queue_item_id: u32,
    /// +44 `m_map_status`: StartMission 0x4FDCE0 reports
    /// `start_mission_map_not_verified` below 2 (`KKE_MAP_STATUS_REPAIRED`), and the
    /// verify 0x4F8FE0 raises it to 2. 2 on every shipped map and original save.
    pub map_status: u32,
    /// +48 `m_mission_state`: 0 from the constructor 0x4F7D00, [`MISSION_RUNNING`]
    /// from StartMission, [`MISSION_ENDED`] from 0x4F4DA0. The other writes found
    /// are the load and 0x4F51F0, which copies +44 and +48 from another world.
    pub mission_state: u32,
}

impl GworldWords {
    /// The words a saved chunk holds (a version 28 one holds every word).
    pub fn of_state(state: &WorldState) -> Option<Self> {
        (state.version == GWORLD_VERSION && state.trailing.is_empty()).then_some(())?;
        Some(Self {
            stream: state.rng_state,
            tick: state.word_dc,
            begin: state.begin_time_bits,
            end: state.word_e4,
            step: state.delta_bits,
            next_gplayer_id: state.word_140,
            next_camera_id: state.word_184?,
            next_area_id: state.word_194?,
            next_structure_queue_item_id: state.word_198?,
            map_status: state.word_44?,
            mission_state: state.word_48?,
        })
    }

    /// The world as it stands, `loaded` the GWORLD of the map or saved game the
    /// battle was loaded from:
    /// - the stream and the clock are the world's;
    /// - +198 is `Books::next_item`, which aa-play's queue item creation raises as
    ///   the constructors do (`production::native` `next_item_id`);
    /// - +48: in a battle the mission state only rises, 0 -> 1 -> 2 (no write of 0
    ///   but the constructor's has been found), so it is the higher of the loaded one
    ///   and what the world has reached since: 1 once the session started
    ///   (`World::session_started`), 2 once a winner was settled (`World::winner`,
    ///   which aa-play's 0x4E9720 / 0x4F4DA0 ports set where 0x4F4DA0 sets 2);
    /// - +140, +184, +194 and +44 are the loaded ones: aa-play has no GPlayer, camera
    ///   or area creation, and saving from the game menu does not repair the map.
    ///   Whether the original creates a camera or an area in a battle is not settled.
    pub fn of_world(world: &crate::world::World, loaded: &Self) -> Self {
        let reached = if world.winner.is_some() {
            MISSION_ENDED
        } else if world.session_started {
            MISSION_RUNNING
        } else {
            0
        };
        Self {
            stream: world.native_rng.state,
            tick: world.clock.tick,
            begin: world.clock.begin.0,
            end: world.clock.end.0,
            step: world.clock.step.0,
            next_structure_queue_item_id: world.books.next_item,
            mission_state: loaded.mission_state.max(reached),
            ..*loaded
        }
    }

    /// The chunk's bytes.
    pub fn encode(&self) -> Vec<u8> {
        let words = [
            GWORLD_VERSION,
            self.stream,
            self.tick,
            self.begin,
            self.end,
            self.step,
            self.next_gplayer_id,
            self.next_camera_id,
            self.next_area_id,
            self.next_structure_queue_item_id,
            self.map_status,
            self.mission_state,
        ];
        words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}

/// The words of the `IDMGR` chunk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdmgrWords {
    pub next_slot: u32,
    pub highest_slot: u32,
    /// The generations of slots 0..highest_slot.
    pub generations: Vec<u16>,
}

/// The capacity word of `IDMGR`: the manager's 0x10000 slots.
pub const IDMGR_CAPACITY: u32 = 0x1_0000;

impl IdmgrWords {
    pub fn of_state(state: &IdManagerState) -> Self {
        Self { next_slot: state.next_slot, highest_slot: state.highest_slot, generations: state.serialized_generations.clone() }
    }

    /// The world's GTID manager as 0x4EFD30 writes it: the next-fit cursor, the
    /// highest slot ever taken, and the generation of every slot below it, live or
    /// free.
    pub fn of_world(world: &crate::world::World) -> Self {
        let highest = world.ids.highest_slot().max(0) as u32;
        Self {
            next_slot: u32::from(world.ids.next_slot()),
            highest_slot: highest,
            generations: (0..highest).map(|s| world.ids.generation_at(s as u16)).collect(),
        }
    }

    /// The chunk's bytes: version 0, next, highest, capacity, then `highest`
    /// generations. `None` unless there are exactly `highest` generations (what
    /// the load 0x4F0230 reads).
    pub fn encode(&self) -> Option<Vec<u8>> {
        (self.generations.len() == self.highest_slot as usize).then_some(())?;
        let mut out = Vec::with_capacity(16 + self.generations.len() * 2);
        for w in [0, self.next_slot, self.highest_slot, IDMGR_CAPACITY] {
            out.extend(w.to_le_bytes());
        }
        for g in &self.generations {
            out.extend(g.to_le_bytes());
        }
        Some(out)
    }
}

/// The `MISC_DATA` chunk, the last 4F2DB0 writes: the count +26C and each entry of
/// +268 (its text and its word +4; on the original's saves a `namelist_*` id and a
/// number), the word +278, the text +290 (empty when there is none), the count of
/// the list +294 and each of its texts, then a byte, set when +27C is, and its two
/// words. A text is a 16-bit length and that many UTF-16 units. `aa-play` keeps
/// none of this yet, so only a decoded chunk is written again.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MiscData {
    pub entries: Vec<(Vec<u16>, u32)>,
    pub word_278: u32,
    pub text_290: Vec<u16>,
    pub texts_294: Vec<Vec<u16>>,
    pub pair_27c: Option<[u32; 2]>,
}

impl MiscData {
    pub fn encode(&self) -> Vec<u8> {
        // The string write puts the 16-bit length, then `(length & 0xFFFF) * 2` bytes.
        fn text(out: &mut Vec<u8>, t: &[u16]) {
            let t = &t[..t.len() & 0xFFFF];
            out.extend((t.len() as u16).to_le_bytes());
            out.extend(t.iter().flat_map(|u| u.to_le_bytes()));
        }
        let mut out = Vec::new();
        out.extend((self.entries.len() as u32).to_le_bytes());
        for (t, w) in &self.entries {
            text(&mut out, t);
            out.extend(w.to_le_bytes());
        }
        out.extend(self.word_278.to_le_bytes());
        text(&mut out, &self.text_290);
        out.extend((self.texts_294.len() as u32).to_le_bytes());
        for t in &self.texts_294 {
            text(&mut out, t);
        }
        out.push(u8::from(self.pair_27c.is_some()));
        for w in self.pair_27c.iter().flatten() {
            out.extend(w.to_le_bytes());
        }
        out
    }

    /// The chunk read in the same order; `None` unless it ends exactly there.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        fn take<'a>(bytes: &'a [u8], at: &mut usize, n: usize) -> Option<&'a [u8]> {
            let b = bytes.get(*at..at.checked_add(n)?)?;
            *at += n;
            Some(b)
        }
        fn word(bytes: &[u8], at: &mut usize) -> Option<u32> {
            take(bytes, at, 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        }
        fn text(bytes: &[u8], at: &mut usize) -> Option<Vec<u16>> {
            let n = take(bytes, at, 2).map(|b| u16::from_le_bytes([b[0], b[1]]))? as usize;
            Some(take(bytes, at, n * 2)?.chunks_exact(2).map(|u| u16::from_le_bytes([u[0], u[1]])).collect())
        }
        let at = &mut 0usize;
        let count = word(bytes, at)?;
        let mut entries = Vec::new();
        for _ in 0..count {
            let t = text(bytes, at)?;
            entries.push((t, word(bytes, at)?));
        }
        let word_278 = word(bytes, at)?;
        let text_290 = text(bytes, at)?;
        let count = word(bytes, at)?;
        let mut texts_294 = Vec::new();
        for _ in 0..count {
            texts_294.push(text(bytes, at)?);
        }
        let pair_27c = match take(bytes, at, 1)?[0] {
            0 => None,
            _ => Some([word(bytes, at)?, word(bytes, at)?]),
        };
        (*at == bytes.len()).then_some(Self { entries, word_278, text_290, texts_294, pair_27c })
    }
}

/// The stream writes the record Saves make: little-endian words and bytes, and a
/// text as its 16-bit length and `length & 0xFFFF` UTF-16 units.
#[derive(Clone, Debug, Default)]
pub struct Out(pub Vec<u8>);

impl Out {
    pub fn word(&mut self, w: u32) {
        self.0.extend(w.to_le_bytes());
    }
    pub fn byte(&mut self, b: u8) {
        self.0.push(b);
    }
    pub fn text(&mut self, t: &[u16]) {
        let t = &t[..t.len() & 0xFFFF];
        self.0.extend((t.len() as u16).to_le_bytes());
        self.0.extend(t.iter().flat_map(|u| u.to_le_bytes()));
    }
    /// A count word and that many words.
    pub fn words(&mut self, v: &[u32]) {
        self.word(v.len() as u32);
        for &w in v {
            self.word(w);
        }
    }
    /// The head 0x4F1B90 writes for every team and kingdom (and 0x4F24C0 for every
    /// thing): the definition id, the class, the GTID; the thing's own Save follows.
    pub fn record_head(&mut self, definition: &[u16], class: u32, gtid: u32) {
        self.text(definition);
        self.word(class);
        self.word(gtid);
    }
    /// GThing's part of a thing's Save, what `ThingState::decode_prefix` reads: the
    /// version, the name, +1C, the position and facing, the flags, and the extra word
    /// of version 1.
    pub fn thing(&mut self, t: &ThingState) {
        self.word(t.version);
        self.text(&t.name_utf16);
        self.word(t.word_1c);
        for &w in t.position_bits.iter().chain(&t.facing_bits) {
            self.word(w);
        }
        self.word(t.serialized_flags);
        if let Some(w) = t.legacy_version_one_word {
            self.word(w);
        }
    }
}

/// The class word of a TEAMS record.
pub const TEAM_CLASS: u32 = 5;

/// A team as a saved game holds it: what `aa_mapworld`'s TEAMS reader keeps, and
/// the display text 0x49AD70 writes after a non-empty key in a saved game (the
/// reader passes over it).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedTeam {
    pub record: TeamRecord,
    pub display_text: Option<Vec<u16>>,
}

/// The `TEAMS` chunk: the count +170, then for each team a presence byte and, when
/// it is set, 0x4F1B90's record (head, GThing, then the team's version, slot, key
/// (and text), the lists +4C and +58 and, from version 1, the text +64).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TeamsChunk {
    pub slots: Vec<(u8, Option<SavedTeam>)>,
}

impl TeamsChunk {
    /// A saved game's TEAMS, through `aa_mapworld`'s reader; the display texts are
    /// read where the writer puts them.
    pub fn decode(chunk: &Chunk) -> Option<Self> {
        let teams = Teams::decode_stream(chunk, true).ok()?;
        let mut slots = Vec::new();
        for slot in teams.slots {
            let team = match slot.record {
                None => None,
                Some(record) => {
                    let display_text = if record.display_key_utf16.is_empty() {
                        None
                    } else {
                        let mut head = Out::default();
                        Self::put_head(&mut head, &record);
                        let at = record.source_span.offset + head.0.len();
                        let n = usize::from(u16::from_le_bytes([*chunk.bytes.get(at)?, *chunk.bytes.get(at + 1)?]));
                        let units = chunk.bytes.get(at + 2..at + 2 + n * 2)?;
                        Some(units.chunks_exact(2).map(|u| u16::from_le_bytes([u[0], u[1]])).collect())
                    };
                    Some(SavedTeam { record, display_text })
                }
            };
            slots.push((slot.source_presence_byte, team));
        }
        Some(Self { slots })
    }

    /// The record up to its key.
    fn put_head(out: &mut Out, r: &TeamRecord) {
        out.record_head(&r.definition_id_utf16, TEAM_CLASS, r.serialized_id);
        out.thing(&r.thing);
        out.word(r.version);
        out.word(r.slot_index);
        out.text(&r.display_key_utf16);
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Out::default();
        out.word(self.slots.len() as u32);
        for (presence, team) in &self.slots {
            out.byte(*presence);
            let Some(SavedTeam { record: r, display_text }) = team else { continue };
            Self::put_head(&mut out, r);
            if !r.display_key_utf16.is_empty() {
                out.text(display_text.as_deref().unwrap_or_default());
            }
            out.words(&r.words_4c);
            out.words(&r.words_58);
            if let Some(t) = &r.string_64_utf16 {
                out.text(t);
            }
        }
        out.0
    }
}

/// The class word of a KINGDOMS record.
pub const KINGDOM_CLASS: u32 = 4;

impl Out {
    fn counted<T>(&mut self, list: &CountedList<T>, mut each: impl FnMut(&mut Self, &T)) {
        self.word(list.serialized_count);
        for v in &list.values {
            each(self, v);
        }
    }
    fn reference_word(&mut self, r: &ReferenceWord) {
        self.text(&r.reference_utf16);
        self.word(r.word);
    }
    fn some_word(&mut self, w: Option<u32>) {
        if let Some(w) = w {
            self.word(w);
        }
    }
    fn some_text(&mut self, t: &Option<Vec<u16>>) {
        if let Some(t) = t {
            self.text(t);
        }
    }

    /// GObject's part (the kingdom's, from world version 19): version, scale and the
    /// words its version adds.
    pub fn object(&mut self, o: &ObjectState) {
        self.word(o.version);
        self.word(o.scale_multiplier_bits);
        self.some_word(o.variant_index.map(|v| v as u32));
        self.some_word(o.associated_object_id);
        if let Some(w) = o.legacy_word {
            self.0.extend(w.to_le_bytes());
        }
        self.some_word(o.nested_count);
    }

    pub fn updatable(&mut self, u: &UpdatablePayload) {
        self.word(u.version);
        self.some_word(u.serialized_id.map(|v| v as u32));
    }

    /// GActor's Save as `ActorPayload` reads it, with the Properties component the
    /// only saved one (slot 6; the reader refuses any other).
    pub fn actor(&mut self, a: &ActorPayload) -> Option<()> {
        let (p, t) = (&a.prefix, &a.tail);
        self.word(p.version);
        self.word(p.owner_handle);
        self.word(p.state_low);
        self.some_word(p.state_high);
        self.word(t.default_components_mask);
        self.word(t.saved_components_mask);
        if t.saved_components_mask & !(1 << 6) != 0 {
            return None;
        }
        if t.saved_components_mask & (1 << 6) != 0 {
            let props = t.properties.as_ref()?;
            self.updatable(&props.updatable);
            self.word(props.version);
            self.words(&props.property_classes);
        }
        self.actor_tail_end(t);
        Some(())
    }

    /// GActor's Save after its components, what `ActorTail` reads after them: the
    /// current component, +158, the words and texts its version adds, the four lists
    /// and +174.
    pub fn actor_tail_end(&mut self, t: &ActorTail) {
        self.word(t.current_component as u32);
        self.some_word(t.word_158);
        self.some_word(t.legacy_word_15c);
        if let Some(s) = &t.word_160_source {
            self.byte(s.presence);
            self.some_text(&s.utf16);
        }
        if let Some(w) = &t.word_164 {
            self.byte(w.presence);
            self.some_word(w.value);
        }
        self.some_text(&t.localized_name_key_utf16);
        self.some_text(&t.localized_name_text_utf16);
        self.some_word(t.word_168);
        self.some_word(t.word_16c);
        self.some_word(t.word_170);
        if let Some(l) = &t.lists {
            for list in [&l.list_118, &l.list_120, &l.list_128] {
                self.counted(list, |o, v| o.word(v[0]));
            }
            self.counted(&l.list_130, |o, v| v.iter().for_each(|&w| o.word(w)));
        }
        self.some_word(t.word_174);
    }

    fn recruitment(&mut self, r: &Recruitment) {
        self.word(r.version);
        self.word(r.serialized_count);
        for item in &r.items {
            self.word(u32::from(matches!(item.state, ItemState::Group { .. })));
            self.word(item.base.version);
            self.text(&item.base.property_id);
            self.byte(item.base.enabled);
            match &item.state {
                ItemState::Group { version, word_0c } => {
                    self.word(*version);
                    self.word(*word_0c);
                }
                ItemState::Single(s) => {
                    self.word(s.version);
                    self.word(s.word_0c);
                    if s.version > 0 {
                        self.word(s.word_10);
                    }
                    self.word(s.word_14);
                    if let Some(b) = s.legacy_v0_byte {
                        self.byte(b);
                    }
                    self.word(s.word_1c);
                    if s.version >= 2 {
                        self.word(s.word_18);
                    }
                }
            }
        }
        self.some_word(r.word_24);
    }

    fn ability(&mut self, a: &AbilityState) {
        self.word(a.version);
        self.word(a.word_0c);
        self.word(a.word_10);
        self.word(a.word_14);
        self.byte(a.byte_18);
        if let Some(w) = &a.words {
            self.counted(w, |o, &v| o.word(v));
        }
    }

    fn commander(&mut self, c: &Commander) {
        if let Some(p) = c.presence {
            self.byte(p);
        }
        self.some_text(&c.reference_utf16);
        if let Some(s) = &c.state {
            self.word(s.version);
            self.word(s.word_0c);
            self.counted(&s.abilities, Self::ability);
        }
    }

    fn property(&mut self, p: &PropertyState) {
        self.word(p.version);
        self.word(p.word_04);
        self.some_word(p.word_08);
        self.text(&p.reference_utf16);
        if let Some(b) = p.byte_10 {
            self.byte(b);
        }
        self.some_word(p.word_14);
    }

    fn strategic(&mut self, s: &StrategicState) {
        self.word(s.version);
        s.words.iter().for_each(|&w| self.word(w));
    }

    /// The kingdom's own Save, what `KingdomPayload` reads (`saved`: the display text
    /// follows a non-empty key).
    pub fn kingdom(&mut self, k: &KingdomPayload) {
        self.word(k.version);
        self.word(k.word_188);
        self.word(k.word_18c);
        self.word(k.word_190);
        self.text(&k.display_key_utf16);
        self.some_text(&k.display_text_utf16);
        self.text(&k.color_reference_utf16);
        self.counted(&k.words_1d0, |o, &v| o.word(v));
        self.counted(&k.resources_1c4, Self::reference_word);
        self.some_word(k.team_handle);
        if let Some(l) = &k.words_1dc {
            self.counted(l, |o, &v| o.word(v));
        }
        if let Some(b) = k.byte_23a {
            self.byte(b);
        }
        if let Some(r) = &k.recruitment {
            self.recruitment(r);
        }
        if let Some(c) = &k.commander {
            self.commander(c);
        }
        self.some_text(&k.nation_reference_utf16);
        if let Some(l) = &k.named_words {
            self.counted(l, Self::reference_word);
        }
        self.some_word(k.word_19c);
        if let Some(l) = &k.references_24c {
            self.counted(l, |o, t| o.text(t));
        }
        if let Some(l) = &k.properties {
            self.counted(l, Self::property);
        }
        if let Some(b) = k.bytes_23b_23c {
            self.0.extend(b);
        }
        self.some_text(&k.string_240_utf16);
        self.some_text(&k.string_348_utf16);
        if let Some(ai) = &k.ai {
            if let Some(t) = &ai.tactical {
                self.word(t.version);
                for g in &t.definition_groups {
                    self.counted(g, Self::reference_word);
                }
                for g in &t.handle_groups {
                    self.counted(g, |o, v| v.iter().for_each(|&w| o.word(w)));
                }
            }
            if k.version > 10 {
                self.strategic(&ai.strategic);
            }
            self.counted(&ai.entries, |o, e| {
                o.word(e.version);
                o.strategic(&e.state);
                e.words_10c.iter().for_each(|&w| o.word(w));
            });
            if k.version <= 10 {
                self.strategic(&ai.strategic);
            }
            self.word(ai.word_334);
        }
        if let Some(n) = k.named_states_count {
            self.word(n);
            for s in &k.named_states {
                self.word(s.version);
                self.text(&s.name_utf16);
                self.text(&s.text_utf16);
                self.words(&s.members);
                self.byte(s.byte_18);
                self.word(s.word_1c);
            }
        }
        if let Some(l) = &k.references_35c {
            self.counted(l, Self::reference_word);
        }
        if let Some(l) = &k.words_36c {
            self.counted(l, |o, &v| o.word(v));
        }
        for b in [k.byte_238, k.byte_239, k.byte_314].into_iter().flatten() {
            self.byte(b);
        }
        if let Some(l) = &k.resources_258 {
            self.counted(l, Self::reference_word);
        }
    }
}

/// The `KINGDOMS` chunk: the word +158 (the major kingdoms), the count +14C, then for
/// each kingdom a presence byte and, when it is set, 0x4F1B90's record: the head,
/// GThing, from world version 19 GObject and GActor, above 21 the Updatable, then
/// the kingdom's own Save. What `aa_mapworld`'s KINGDOMS reader keeps is all of it.
#[derive(Clone, Debug)]
pub struct KingdomsChunk(pub Kingdoms);

impl KingdomsChunk {
    /// A saved game's KINGDOMS (`world_version` from its GWORLD).
    pub fn decode(chunk: &Chunk, world_version: u32) -> Option<Self> {
        Kingdoms::decode_stream(chunk, world_version, true).ok().map(Self)
    }

    /// The kingdoms of `world` as they stand, `self` the KINGDOMS of the saved game the
    /// battle was restored from (`crate::saved_world::restore`, which numbers the
    /// kingdoms as here: the majors, +190 == 0, in slot order, then the others). What
    /// `aa-play` keeps of a kingdom is written into its record, the rest is the record
    /// as loaded:
    /// - each stockpile of the list +1C4, from `KingdomBooks::stockpiled` (the restore
    ///   takes it from there);
    /// - the byte +23B stays as loaded: 0x4E6DB0 sets it to 1 at mission start and
    ///   every kingdom record of the original's saves has 1, beaten ones included (a
    ///   win or loss is +0x19C through 0x4E9720, not written from the world yet);
    /// - the Commander's balance +0xC and each ability's last use +10 (-FLT_MAX for
    ///   never, 0x4C8650), from `special_ops::Commander`.
    ///
    /// `Err` when the world holds what the record cannot say yet: a technology the
    /// loaded properties list lacks, or a general the loaded record has no state for.
    pub fn of_world(&self, world: &crate::world::World) -> Result<Self, String> {
        let mut out = self.clone();
        let majors = out.0.slots.iter().filter_map(|s| s.record.as_ref()).filter(|r| r.payload.word_190 == 0).count();
        let (mut major, mut other) = (0u32, majors as u32);
        let data = world.data.clone();
        for slot in &mut out.0.slots {
            let Some(r) = slot.record.as_mut() else { continue };
            let k = if r.payload.word_190 == 0 {
                major += 1;
                major - 1
            } else {
                other += 1;
                other - 1
            };
            let name = String::from_utf16_lossy(&r.thing.name_utf16);
            world.kingdoms.get(k as usize).filter(|w| w.id == k).ok_or_else(|| format!("kingdom {name}: not in the world"))?;
            let books = world.books.kingdom(k);
            for res in &mut r.payload.resources_1c4.values {
                let id = String::from_utf16_lossy(&res.reference_utf16);
                if let (Some(i), Some(b)) = (data.resource_index(&id), books)
                    && let Some(v) = b.stockpiled.get(i)
                {
                    res.word = v.to_bits();
                }
            }
            let saved: Vec<String> = r
                .payload
                .properties
                .iter()
                .flat_map(|l| l.values.iter())
                .map(|p| String::from_utf16_lossy(&p.reference_utf16))
                .filter(|p| data.property(p).is_some())
                .collect();
            let held = books.map_or(&[][..], |b| &b.technologies[..]);
            if held.len() != saved.len() || held.iter().zip(&saved).any(|(a, b)| !a.eq_ignore_ascii_case(b)) {
                return Err(format!("kingdom {name}: its technologies {held:?} are not the loaded {saved:?} (a property record is not written yet)"));
            }
            if let Some(c) = world.special_ops.commanders.get(k as usize).filter(|c| c.general.is_some()) {
                let state = r.payload.commander.as_mut().and_then(|c| c.state.as_mut()).ok_or_else(|| format!("kingdom {name}: a general without a saved Commander"))?;
                state.word_0c = c.experience.to_bits();
                for (a, last) in state.abilities.values.iter_mut().zip(&c.last_use) {
                    a.word_10 = last.map_or((-f32::MAX).to_bits(), f32::to_bits);
                }
            }
        }
        Ok(out)
    }

    /// `None` for a kingdom the writer cannot write yet (a saved component other
    /// than Properties, which the reader refuses too).
    pub fn encode(&self) -> Option<Vec<u8>> {
        let k = &self.0;
        let mut out = Out::default();
        out.word(k.major_count);
        out.word(k.slots.len() as u32);
        for slot in &k.slots {
            out.byte(slot.source_presence_byte);
            let Some(r) = &slot.record else { continue };
            out.record_head(&r.definition_id_utf16, KINGDOM_CLASS, r.serialized_id);
            out.thing(&r.thing);
            if k.world_version >= 19 {
                out.object(r.object.as_ref()?);
                out.actor(r.actor.as_ref()?)?;
            }
            if k.world_version > 21 {
                out.updatable(r.updatable.as_ref()?);
            }
            out.kingdom(&r.payload);
        }
        Some(out.0)
    }
}

impl Out {
    /// A thing record as `aa_mapworld`'s inventory frames it: the head, GThing, the
    /// GObject part (classes 2, 3, 6, 7, 9 and a group), the GActor prefix, then the
    /// rest of the thing's Save, still as the saved bytes.
    pub fn placement(&mut self, p: &Placement) {
        self.record_head(&p.thing_id_utf16, p.class_code, p.serialized_id);
        self.thing(&p.thing);
        if let Some(o) = &p.object {
            self.object(o);
        }
        if let Some(a) = &p.actor {
            self.word(a.version);
            self.word(a.owner_handle);
            self.word(a.state_low);
            self.some_word(a.state_high);
        }
        self.0.extend(&p.opaque_tail);
    }
}

/// `DYNAMIC_THINGS` and `DYNAMIC_THINGS_INDEX` as 0x4F24C0 writes them: the chunk's
/// word 0 and version word (11 on the original's saves), the records one after the
/// other, then the index's word 0, the count and each record's length. Each record
/// is written back from `aa_mapworld`'s inventory frame ([`Out::placement`]); the
/// part after the GActor prefix (the components, the Mind) is still the saved
/// bytes, and a record the inventory does not frame is its saved bytes whole.
#[derive(Clone, Debug)]
pub struct ThingsChunk(pub Inventory);

impl ThingsChunk {
    pub fn decode(archive: &MapArchive) -> Option<Self> {
        Inventory::decode(archive, 1 << 30).ok().map(Self)
    }

    /// The chunk and its index.
    pub fn encode(&self) -> (Vec<u8>, Vec<u8>) {
        let mut records: Vec<(usize, Vec<u8>)> = self
            .0
            .placements
            .iter()
            .map(|p| {
                let mut out = Out::default();
                out.placement(p);
                (p.record_ordinal, out.0)
            })
            .chain(self.0.unsupported.iter().map(|u| (u.record_ordinal, u.raw.clone())))
            .collect();
        records.sort_by_key(|r| r.0);
        let (mut data, mut index) = (Out::default(), Out::default());
        data.word(0);
        data.word(self.0.data_header_word);
        index.word(0);
        index.word(records.len() as u32);
        for (_, bytes) in &records {
            index.word(bytes.len() as u32);
            data.0.extend(bytes);
        }
        (data.0, index.0)
    }
}

/// `DYNAMIC_THINGS` and its index written from a world ([`ThingsChunk::of_world`]),
/// with what was not.
#[derive(Clone, Debug, Default)]
pub struct WrittenThings {
    pub data: Vec<u8>,
    pub index: Vec<u8>,
    /// Records whose thing has left the world's GTID manager (and organization members
    /// that have), left out: 0x4F24C0 writes the live things of the manager.
    pub dropped: usize,
    /// Organizations whose tail is not written yet, by GTID with [`thing_tail`]'s
    /// reason: their members stand where the save had them.
    pub members_as_saved: Vec<(u32, String)>,
    /// Live actors and organizations the loaded chunk has no record of (made since
    /// the load): not written.
    pub not_written: Vec<u32>,
}

impl ThingsChunk {
    /// The things of `world` as they stand, `self` the DYNAMIC_THINGS of the saved
    /// game it was restored from (`crate::saved_world::restore`, which gives each
    /// restored thing its record's GTID). Each record whose GTID is still live is
    /// written again, the rest of it as loaded, with where aa-play has the thing: the
    /// x and y of GThing's position (+0x10, +0x14; the z is the saved one, which the
    /// restore does not read: it takes the ground), of an actor its `pos`, of an
    /// organization its `anchor`, of an organization's members theirs (its tail
    /// written again by [`Out::walked_record`]).
    ///
    /// In part: the components (health, supply, queues), the Minds and the actor
    /// state are the saved ones, and a thing made since the load is not written
    /// ([`WrittenThings::not_written`]).
    pub fn of_world(&self, world: &crate::world::World, world_version: u32) -> WrittenThings {
        use crate::ids::Thing;
        let mut out = WrittenThings::default();
        let mut written = std::collections::BTreeSet::new();
        let at = |thing: Thing| match thing {
            Thing::Actor(a) => Some(world.actors[a as usize].pos.truncate()),
            Thing::Org(o) => Some(world.orgs[o as usize].anchor),
            _ => None,
        };
        let place = |t: &mut ThingState, xy: Option<glam::Vec2>| {
            if let Some(xy) = xy {
                t.position_bits[0] = xy.x.to_bits();
                t.position_bits[1] = xy.y.to_bits();
            }
        };
        let mut records: Vec<(usize, Vec<u8>)> = Vec::new();
        for p in &self.0.placements {
            let Some(thing) = world.ids.thing(p.serialized_id) else {
                out.dropped += 1;
                continue;
            };
            written.insert(p.serialized_id);
            let mut p = p.clone();
            place(&mut p.thing, at(thing));
            let mut bytes = Out::default();
            let whole = matches!(thing, Thing::Org(_)).then(|| -> Result<Out, String> {
                let mut t = aa_mapworld::saved_world::walk(p.clone(), world_version, 1, &mut crate::saved_world::Minds).map_err(|e| e.to_string())?;
                if let Some(e) = &t.partial {
                    return Err(e.clone());
                }
                // A member that has left the world is written as an empty slot.
                for (_, m) in t.organization.iter_mut().flat_map(|o| o.members.iter_mut()) {
                    match world.ids.thing(m.record.serialized_id) {
                        Some(member) => {
                            written.insert(m.record.serialized_id);
                            place(&mut m.record.thing, at(member));
                        }
                        None => out.dropped += 1,
                    }
                }
                let mut b = Out::default();
                b.record_head(&p.thing_id_utf16, p.class_code, p.serialized_id);
                b.thing(&p.thing);
                if let Some(o) = &p.object {
                    b.object(o);
                }
                // The kingdoms' memories of it and its GActor prefix, which the walk
                // reads past the object, are written as saved.
                let drained = t.record.opaque_tail_span.offset - p.opaque_tail_span.offset;
                if drained > 0 {
                    b.0.extend(&p.opaque_tail[..drained]);
                } else {
                    let a = p.actor.as_ref().ok_or("no GActor prefix")?;
                    b.word(a.version);
                    b.word(a.owner_handle);
                    b.word(a.state_low);
                    b.some_word(a.state_high);
                }
                let keep = |m: &SavedThing| world.ids.thing(m.record.serialized_id).is_some();
                b.0.extend(thing_tail_over(&t, world_version, Over { saved: p.opaque_tail.get(drained..), keep: &keep })?);
                Ok(b)
            });
            match whole {
                Some(Ok(b)) => bytes = b,
                Some(Err(e)) => {
                    out.members_as_saved.push((p.serialized_id, e));
                    bytes.placement(&p);
                }
                None => bytes.placement(&p),
            }
            records.push((p.record_ordinal, bytes.0));
        }
        for u in &self.0.unsupported {
            // A group or a record the inventory does not frame, as saved while its
            // thing lives.
            let gtid = aa_mapworld::inventory::decode_group_record(&u.raw, u.record_ordinal).ok().map(|g| g.serialized_id);
            if gtid.is_some_and(|g| world.ids.thing(g).is_none()) {
                out.dropped += 1;
                continue;
            }
            written.extend(gtid);
            records.push((u.record_ordinal, u.raw.clone()));
        }
        records.sort_by_key(|r| r.0);
        let (mut data, mut index) = (Out::default(), Out::default());
        data.word(0);
        data.word(self.0.data_header_word);
        index.word(0);
        index.word(records.len() as u32);
        for (_, bytes) in &records {
            index.word(bytes.len() as u32);
            data.0.extend(bytes);
        }
        out.not_written = world
            .ids
            .iter()
            .filter(|(id, thing)| matches!(thing, Thing::Actor(_) | Thing::Org(_)) && !written.contains(&id.raw()))
            .map(|(id, _)| id.raw())
            .collect();
        (out.data, out.index) = (data.0, index.0);
        out
    }
}

/// The order GActor's Load 0x4D8CF0 reads the saved components in (`ActorTail`):
/// slot 15, the others up from 0, the Mind (slot 4) last.
pub fn component_order() -> impl Iterator<Item = u8> {
    std::iter::once(15).chain((0..31).filter(|s| *s != 15 && *s != 4)).chain(std::iter::once(4))
}

fn units(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

impl Out {
    /// A saved component as `SavedComponent::decode_prefix` reads it, each word where
    /// that reader takes it. `None` for what that reader refuses too: a Properties
    /// component with property objects, an Element with a damage history, a
    /// Structure with a queue, a Denizen with commands (the walk's own readers keep
    /// those: [`thing_tail`]). The Mind, the Transport, the Organization and the HQ are
    /// their saved bytes.
    pub fn component(&mut self, c: &SavedComponent) -> Option<()> {
        match c {
            SavedComponent::Hq(v) => self.0.extend(&v.bytes),
            SavedComponent::Mind(v) => self.0.extend(&v.bytes),
            SavedComponent::Transport(v) => self.0.extend(&v.bytes),
            SavedComponent::Organization(v) => self.0.extend(&v.bytes),
            SavedComponent::Body(b) => {
                self.word(b.version);
                if let Some(u) = &b.updatable {
                    self.updatable(u);
                }
                self.some_word(b.dying_started_bits);
                self.word(b.health_multiplier_bits);
                self.some_word(b.sink_duration_bits);
                self.some_word(b.last_heal_time_bits);
            }
            SavedComponent::Character(v) => {
                self.updatable(&v.updatable);
                self.word(v.version);
                self.word(v.word_28);
                self.some_word(v.legacy_word);
                self.word(v.animation_kind as u32);
                self.word(v.animation_index as u32);
                self.word(v.animation_version);
                self.word(v.elapsed_bits);
                self.word(v.resumed_at_bits);
                self.word(v.rate_bits);
                self.byte(v.active_byte);
                self.some_word(v.variant.map(|w| w as u32));
            }
            SavedComponent::Construction(v) => {
                self.updatable(&v.updatable);
                self.word(v.version);
                self.counted(&v.words_10, |o, &w| o.word(w));
                if let Some(l) = &v.words_1c {
                    self.counted(l, |o, &w| o.word(w));
                }
                self.some_text(&v.reference_28_utf16);
                for w in v.legacy_upgrade_words.iter().flatten() {
                    self.word(*w);
                }
            }
            SavedComponent::Performer(v) => {
                self.updatable(&v.updatable);
                self.word(v.version);
                self.word(v.word_10);
                self.word(v.multiplier_20);
                self.word(v.word_28);
                v.words_2c_30.iter().for_each(|&w| self.word(w));
                self.word(v.ability_index as u32);
                v.words_3c_40.iter().for_each(|&w| self.word(w));
                self.counted(&v.states, |o, s| {
                    s.words_00_04.iter().for_each(|&w| o.word(w));
                    o.some_word(s.word_08);
                    if let Some(b) = s.present_byte {
                        o.byte(b);
                    }
                    for w in s.words_10_14.iter().flatten() {
                        o.word(*w);
                    }
                    o.some_word(s.word_18);
                });
                self.word(v.word_58);
                self.some_word(v.word_44);
                self.counted(&v.abilities, Self::ability);
                if let Some(b) = v.byte_5c {
                    self.byte(b);
                }
            }
            SavedComponent::Mover(v) => {
                self.updatable(&v.updatable);
                self.word(v.version);
                v.vector_10.iter().for_each(|&w| self.word(w));
                self.word(v.word_20);
                self.word(v.word_24);
                self.some_word(v.speed_multiplier_bits);
                for w in v.legacy_vector.iter().flatten() {
                    self.word(*w);
                }
                self.word(v.word_44);
                self.word(v.word_48);
                for b in [v.byte_4c, v.byte_54].into_iter().flatten() {
                    self.byte(b);
                }
                for w in v.vector_38.iter().flatten() {
                    self.word(*w);
                }
                self.some_word(v.word_50);
                if let Some(b) = v.byte_56 {
                    self.byte(b);
                }
                self.some_word(v.word_58);
                if let Some(b) = v.byte_5c {
                    self.byte(b);
                }
                for w in v.vector_60.iter().flatten() {
                    self.word(*w);
                }
                if let Some(b) = v.byte_6c {
                    self.byte(b);
                }
                for w in v.vector_70.iter().chain(&v.vector_7c).flatten() {
                    self.word(*w);
                }
            }
            SavedComponent::Denizen(v) => {
                if v.command_count.is_some_and(|n| n as i32 > 0) {
                    return None;
                }
                self.updatable(&v.updatable);
                self.word(v.version);
                for b in [v.byte_20, v.byte_21].into_iter().flatten() {
                    self.byte(b);
                }
                if let Some(l) = &v.occupants {
                    self.counted(l, |o, p| p.iter().for_each(|&w| o.word(w)));
                }
                self.some_word(v.word_34);
                if let Some(l) = &v.handles {
                    self.counted(l, |o, &w| o.word(w));
                }
                self.some_word(v.command_count);
            }
            SavedComponent::Supply(v) => {
                self.word(v.version);
                if let Some(u) = &v.updatable {
                    self.updatable(u);
                }
                self.some_word(v.amount_multiplier_bits);
                self.some_word(v.word_1c);
            }
            SavedComponent::Recruit(v) => {
                self.updatable(&v.updatable);
                self.word(v.version);
                if let Some(b) = v.byte_1c {
                    self.byte(b);
                }
                for w in v.words_14.iter().flatten() {
                    self.word(*w);
                }
                if let Some(l) = &v.legacy {
                    self.word(l.serialized_count);
                    for (name, flag) in &l.items {
                        self.text(name);
                        self.byte(*flag);
                    }
                    self.word(l.word_after);
                    if let Some(b) = l.byte_v2 {
                        self.byte(b);
                    }
                }
            }
            SavedComponent::Defender(v) => {
                self.updatable(&v.updatable);
                self.word(v.version);
                if let Some(l) = &v.abilities {
                    self.counted(l, Self::ability);
                }
                if let Some(l) = &v.pairs {
                    self.counted(l, |o, p| p.iter().for_each(|&w| o.word(w)));
                }
            }
            SavedComponent::Research(v) => {
                self.updatable(&v.updatable);
                self.word(v.version);
                self.counted(&v.references, |o, t| o.text(t));
            }
            SavedComponent::Lair { updatable, version, words, word_1c, word_20 } => {
                self.updatable(updatable);
                self.word(*version);
                self.counted(words, |o, &w| o.word(w));
                self.word(*word_1c);
                self.word(*word_20);
            }
            SavedComponent::City { updatable, version, words } => {
                self.updatable(updatable);
                self.word(*version);
                if let Some(l) = words {
                    self.counted(l, |o, &w| o.word(w));
                }
            }
            SavedComponent::Structure(v) => return self.structure(v, &[]).ok(),
            SavedComponent::Properties(v) => return self.properties(v, &[]).ok(),
            SavedComponent::UpdatableOnly { updatable, version } => {
                self.updatable(updatable);
                self.word(*version);
            }
            SavedComponent::Experience { version, word_08 } => {
                self.word(*version);
                self.word(*word_08);
            }
            SavedComponent::Element { .. } => return self.element(c, &[], GWORLD_VERSION).ok(),
            SavedComponent::MobileDeployment { updatable, version, word_10, word_14, discarded_resources } => {
                self.updatable(updatable);
                self.word(*version);
                self.word(*word_10);
                self.some_word(*word_14);
                if let Some(v) = discarded_resources {
                    self.resources(v);
                }
            }
        }
        Some(())
    }

    /// A ResourceVector (4CD3F0): version 0, the count, the values.
    fn resources(&mut self, v: &[u32]) {
        self.word(0);
        self.words(v);
    }

    /// The Properties component (581AE0) with its property objects (503B10, and a
    /// version 0 word more of a class 1 object from version 3, 503BC0), what the walk
    /// reads: a class word each, -1 for none.
    pub fn properties(&mut self, p: &PropertiesPayload, objects: &[SavedProperty]) -> Result<(), String> {
        self.updatable(&p.updatable);
        self.word(p.version);
        self.word(p.property_classes.len() as u32);
        let mut objects = objects.iter();
        for &class in &p.property_classes {
            self.word(class);
            if class == u32::MAX {
                continue;
            }
            let o = objects.next().filter(|o| o.class == class).ok_or("slot 6: a property class without its object")?;
            self.property(&o.state);
            if class == 1 && o.state.version > 2 {
                self.word(0);
                self.word(o.word_18.ok_or("slot 6: a modifier property without its word")?);
            }
        }
        if objects.next().is_some() {
            return Err("slot 6: more property objects than classes".into());
        }
        Ok(())
    }

    /// A damage record (4C0BA0) at world version 20 or later (below that a byte or a
    /// word the walk passes over).
    fn damage(&mut self, words: [u32; 2], damage_type: &str, more: [u32; 2], word_14: Option<u32>, word_18: Option<u32>, world_version: u32) -> Result<(), String> {
        if world_version < 0x14 {
            return Err(format!("a damage record of world version {world_version}"));
        }
        words.iter().for_each(|&w| self.word(w));
        self.text(&units(damage_type));
        more.iter().for_each(|&w| self.word(w));
        self.word(word_14.ok_or("a damage record without +14")?);
        self.word(word_18.ok_or("a damage record without +18")?);
        Ok(())
    }

    /// The Element component (570FA0) with its damage history.
    pub fn element(&mut self, c: &SavedComponent, history: &[DamageRecord], world_version: u32) -> Result<(), String> {
        let SavedComponent::Element { version, updatable, legacy_byte, history_count } = c else {
            return Err("slot 8: not an Element".into());
        };
        self.word(*version);
        if let Some(u) = updatable {
            self.updatable(u);
        }
        if let Some(b) = legacy_byte {
            self.byte(*b);
        }
        if let Some(n) = history_count {
            if (*n as i32).max(0) as usize != history.len() {
                return Err("slot 8: the history is not the saved count".into());
            }
            self.word(*n);
            for d in history {
                self.damage([d.word_00, d.word_04], &d.damage_type, [d.word_0c, d.word_10], d.word_14, d.word_18, world_version)?;
            }
        }
        Ok(())
    }

    /// The Structure component (58A770) with its queue items (58A470): the item's id
    /// from version 13, its kind, the recruit's ActorData or the research's property,
    /// the location of kind 2, then what was paid, asked and held.
    pub fn structure(&mut self, s: &StructurePayload, queue: &[SavedQueueItem]) -> Result<(), String> {
        self.structure_over(s, queue, None)
    }

    /// [`Out::structure`] over its saved bytes (`saved`, from the component's start):
    /// the 4C3050 reference before an item's own layout, which the walk passes over,
    /// is the saved one.
    pub fn structure_over(&mut self, s: &StructurePayload, queue: &[SavedQueueItem], saved: Option<&[u8]>) -> Result<(), String> {
        let start = self.0.len();
        self.word(s.version);
        if let Some(u) = &s.updatable {
            self.updatable(u);
        }
        self.word(s.discarded_object_handle);
        if let Some(n) = s.legacy_reference_count {
            if n as i32 > 0 {
                return Err("slot 13: a version 0 reference list (the walk passes over it)".into());
            }
            self.word(n);
        }
        if let Some(n) = s.queue_count {
            if (n as i32).max(0) as usize != queue.len() {
                return Err("slot 13: the queue is not the saved count".into());
            }
            self.word(n);
        }
        for item in queue {
            if s.version == 9 {
                return Err("slot 13: a version 9 queue item (its last word is passed over)".into());
            }
            if s.version > 12 {
                self.word(item.id.ok_or("slot 13: an item without its id")?);
            }
            self.word(item.kind);
            match item.kind {
                0 | 2 => {
                    self.text(&units(item.actor.as_deref().ok_or("slot 13: a recruit without its ActorData")?));
                    match &item.layout {
                        None => self.byte(0),
                        Some((id, parameters)) => {
                            let saved = saved.ok_or("slot 13: an item with its own layout (the walk keeps no reference)")?;
                            // The saved presence byte and reference.
                            let at = self.0.len() - start;
                            let n = saved.get(at + 1..at + 3).map(|b| usize::from(u16::from_le_bytes([b[0], b[1]]))).ok_or("slot 13: the saved reference")?;
                            self.0.extend(saved.get(at..at + 3 + n * 2).ok_or("slot 13: the saved reference")?);
                            // The layout's Load 4C3180: version 0, its IDS, the parameters.
                            self.word(0);
                            self.text(&units(id));
                            self.word(parameters.len() as u32);
                            for p in parameters {
                                self.text(&units(p));
                            }
                        }
                    }
                    if item.kind == 2 {
                        item.location.ok_or("slot 13: a kind 2 item without a location")?.iter().for_each(|&w| self.word(w));
                    }
                }
                1 => self.text(&units(item.research.as_deref().ok_or("slot 13: research without its property")?)),
                k => return Err(format!("slot 13: item kind {k}")),
            }
            if s.version > 3 {
                self.resources(&item.paid);
                self.resources(&item.requested);
            }
            if s.version > 7 {
                self.resources(&item.current);
            }
        }
        for w in [s.word_20, s.word_2c, s.word_24, s.word_28, s.word_34].into_iter().flatten() {
            self.word(w);
        }
        Ok(())
    }

    /// An Organization (57D130): its Updatable, version, layout (4C3180), the slots
    /// with each member's record, then its words. The walk keeps a member's slot word
    /// and passes over an empty slot's; each slot is written as its index, or -1 when
    /// empty (on the original's saves every organization reads back so:
    /// `tests::original_saved_things_re_encode_from_the_walk`).
    pub fn organization(&mut self, o: &SavedOrganization, world_version: u32) -> Result<(), String> {
        self.organization_over(o, world_version, Over::walk()).map(|_| ())
    }

    /// [`Out::organization`] over its saved bytes (`over.saved`, from the
    /// Organization's start): a member's part the walk does not keep is taken from
    /// there, and a member `over.keep` refuses is written as an empty slot. Returns how
    /// many bytes further on the saved Organization ends than the written one.
    pub fn organization_over(&mut self, o: &SavedOrganization, world_version: u32, over: Over<'_>) -> Result<isize, String> {
        let start = self.0.len();
        let mut shift = 0isize;
        match o.updatable_id {
            Some(id) => {
                self.word(1);
                self.word(id as u32);
            }
            None => self.word(0),
        }
        self.word(o.version);
        self.word(0);
        self.text(&units(&o.layout));
        self.word(o.parameters.len() as u32);
        for p in &o.parameters {
            self.text(&units(p));
        }
        self.word(o.slots);
        let mut members = o.members.iter().peekable();
        for i in 0..o.slots as i32 {
            match members.peek() {
                Some((slot, _)) if *slot == i => {
                    let (_, m) = members.next().ok_or("slot 7: member")?;
                    if !(over.keep)(m) {
                        // The saved slot word and record, written as an empty slot.
                        self.word(u32::MAX);
                        shift += m.consumed as isize;
                        continue;
                    }
                    self.word(i as u32);
                    let at = (self.0.len() - start) as isize + shift;
                    let saved = over.saved.and_then(|s| s.get(at as usize..));
                    let before = self.0.len();
                    self.walked_record_over(m, world_version, Over { saved, keep: over.keep })?;
                    shift += m.consumed as isize - (self.0.len() - before) as isize;
                }
                _ => self.word(u32::MAX),
            }
        }
        if members.next().is_some() {
            return Err("slot 7: a member whose slot word is not its index".into());
        }
        if (2..=6).contains(&o.version) || o.version == 0 {
            return Err(format!("slot 7: organization version {} (the walk passes over words)", o.version));
        }
        for s in &o.recruit_singles {
            self.text(&units(s));
        }
        self.word(o.leader as u32);
        self.word(o.morale_bits);
        self.word(o.formation as u32);
        self.some_word(o.secondary_formation.map(|w| w as u32));
        for off in &o.offsets {
            off.iter().for_each(|&w| self.word(w));
        }
        for &w in &o.words_50 {
            self.word(w);
        }
        if o.version >= 4 {
            self.word(o.word_84.ok_or("slot 7: no +84")?);
            self.0.extend(o.bytes_8c.ok_or("slot 7: no +8C")?);
            let (w90, w94, ids, w9c, wa0) = o.damage.as_ref().ok_or("slot 7: no damage record")?;
            self.damage([*w90, *w94], ids, [*w9c, *wa0], o.word_a4, o.word_a8, world_version)?;
        }
        self.some_word(o.word_88);
        self.some_word(o.created_at_bits);
        for w in o.position_bits.iter().flatten() {
            self.word(*w);
        }
        self.some_word(o.word_d0);
        self.some_word(o.word_d4);
        Ok(shift)
    }

    /// A walked record whole (an organization's member, or a top-level record): the
    /// head, GThing, GObject, an all-absent list of the kingdoms' memories (one 0 byte
    /// each from object version 5), the GActor prefix, then [`thing_tail`].
    pub fn walked_record(&mut self, t: &SavedThing, world_version: u32) -> Result<(), String> {
        self.walked_record_over(t, world_version, Over::walk())
    }

    /// [`Out::walked_record`] of an organization's member over its saved bytes
    /// (`over.saved`, from the member's start, where the walk counts its spans from):
    /// what the walk reads past the object, the kingdoms' memories of it and its
    /// GActor prefix, is written as saved, and its tail by [`thing_tail_over`].
    pub fn walked_record_over(&mut self, t: &SavedThing, world_version: u32, over: Over<'_>) -> Result<(), String> {
        let p = &t.record;
        let start = self.0.len();
        self.record_head(&p.thing_id_utf16, p.class_code, p.serialized_id);
        self.thing(&p.thing);
        if let Some(o) = &p.object {
            self.object(o);
        }
        let head = self.0.len() - start;
        if let Some(o) = p.object.as_ref().filter(|o| o.nested_count.is_some_and(|n| n != 0)) {
            if let Some(saved) = over.saved {
                let drained = p.opaque_tail_span.offset.checked_sub(head).ok_or("the memories' span")?;
                self.0.extend(saved.get(head..head + drained).ok_or("the saved memories")?);
                let tail = thing_tail_over(t, world_version, Over { saved: saved.get(head + drained..), keep: over.keep })?;
                self.0.extend(tail);
                return Ok(());
            }
            if !t.sightings.is_empty() {
                return Err("the kingdoms' memories of it (the walk passes over two words of each)".into());
            }
            if o.version < 5 {
                return Err(format!("a memory list of object version {}", o.version));
            }
            self.0.extend(std::iter::repeat_n(0u8, (o.nested_count.unwrap_or(0) as i32).max(0) as usize));
        }
        let a = p.actor.as_ref().ok_or("no GActor prefix")?;
        self.word(a.version);
        self.word(a.owner_handle);
        self.word(a.state_low);
        self.some_word(a.state_high);
        let at = self.0.len() - start;
        let tail = thing_tail_over(t, world_version, Over { saved: over.saved.and_then(|s| s.get(at..)), keep: over.keep })?;
        self.0.extend(tail);
        Ok(())
    }
}

/// What [`thing_tail_over`] may take from the saved bytes it writes again.
#[derive(Clone, Copy)]
pub struct Over<'a> {
    /// The saved bytes the thing was walked from, from where the writing starts.
    pub saved: Option<&'a [u8]>,
    /// Whether an organization's member is written; one that is not is written as an
    /// empty slot (-1).
    pub keep: &'a dyn Fn(&SavedThing) -> bool,
}

fn keep_all(_: &SavedThing) -> bool {
    true
}

impl Over<'static> {
    /// Nothing but what the walk keeps, every member.
    pub fn walk() -> Self {
        Over { saved: None, keep: &keep_all }
    }
}

/// A walked thing's GActor tail (`aa_mapworld::saved_world::walk`): the two masks,
/// each saved component in [`component_order`], then [`Out::actor_tail_end`]. `Err`
/// names what the walk does not keep: a Denizen's (slot 10), an aircraft's (23) or a
/// Transport's (24) words, or a part the component writers refuse.
pub fn thing_tail(t: &SavedThing, world_version: u32) -> Result<Vec<u8>, String> {
    thing_tail_over(t, world_version, Over::walk())
}

/// [`thing_tail`] over the saved tail (`over.saved`, from the tail's start): a
/// Denizen (slot 10) is read there with `SavedComponent::decode_prefix` and written
/// again, a Structure item's layout reference is the saved one
/// ([`Out::structure_over`]), and an organization's members are written by
/// [`Out::organization_over`].
pub fn thing_tail_over(t: &SavedThing, world_version: u32, over: Over<'_>) -> Result<Vec<u8>, String> {
    let tail = t.tail.as_ref().ok_or("no GActor tail")?;
    let mut out = Out::default();
    let mut shift = 0isize;
    out.word(tail.default_components_mask);
    out.word(tail.saved_components_mask);
    let mut components = t.components.iter();
    for slot in component_order() {
        if tail.saved_components_mask & (1 << slot) == 0 {
            continue;
        }
        let saved = over.saved.and_then(|s| s.get((out.0.len() as isize + shift) as usize..));
        match slot {
            4 => out.0.extend(t.mind.as_ref().ok_or("slot 4: no Mind")?),
            7 => shift += out.organization_over(t.organization.as_ref().ok_or("slot 7: no Organization")?, world_version, Over { saved, keep: over.keep })?,
            10 if saved.is_some() => {
                let s = saved.unwrap_or_default();
                let (c, n) = SavedComponent::decode_prefix(10, s).map_err(|e| format!("slot 10: {e}"))?;
                let before = out.0.len();
                out.component(&c).ok_or("slot 10: refused")?;
                if out.0[before..] != s[..n] {
                    return Err("slot 10: not written as saved".into());
                }
            }
            10 | 23 | 24 => return Err(format!("slot {slot}: the walk keeps only part of it")),
            _ => {
                let (s, c) = components.next().ok_or_else(|| format!("slot {slot}: not walked"))?;
                if *s != slot {
                    return Err(format!("slot {slot}: the walk holds slot {s}"));
                }
                match c {
                    SavedComponent::Properties(p) => out.properties(p, &t.properties)?,
                    SavedComponent::Element { .. } => out.element(c, &t.element_history, world_version)?,
                    SavedComponent::Structure(s) => out.structure_over(s, &t.queue, saved)?,
                    _ => out.component(c).ok_or_else(|| format!("slot {slot}: refused"))?,
                }
            }
        }
    }
    out.actor_tail_end(tail);
    Ok(out.0)
}

/// One world queue as its Save writes it: version 0, the current bucket, the bucket
/// count, then per bucket its count word and that many (Updatable table id, data)
/// pairs. A bucket keeps its count word as saved (a negative count is an empty one).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueueWords {
    pub current: u32,
    pub buckets: Vec<(u32, Vec<[u32; 2]>)>,
}

/// The `UPDATABLES` chunk 4F2DB0 writes: the count of the Updatable table (the word
/// at `*(+280)`), then the queues +1E8 and +208 (the deferred queues 3 and 4) and
/// +1AC, +1C0, +1D4 (the periodic queues 0, 1, 2), what aa-runtime's
/// `saved_queues::decode` and `map::saved_periodic` read. Written again from a decoded
/// chunk; `crate::scheduler`'s queues are not written into it yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UpdatablesChunk {
    pub table_count: u32,
    /// In file order: 3, 4, 0, 1, 2.
    pub queues: Vec<QueueWords>,
}

impl UpdatablesChunk {
    /// A world version 22 or later chunk (five queues); `None` unless it ends
    /// exactly after the last.
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let mut at = 0usize;
        let mut word = || -> Option<u32> {
            let w = u32::from_le_bytes(bytes.get(at..at.checked_add(4)?)?.try_into().ok()?);
            at += 4;
            Some(w)
        };
        let table_count = word()?;
        let mut queues = Vec::new();
        for _ in 0..5 {
            (word()? == 0).then_some(())?;
            let current = word()?;
            let count = word()? as i32;
            (count > 0 && (current as i32) >= 0 && (current as i32) < count).then_some(())?;
            let mut buckets = Vec::new();
            for _ in 0..count {
                let n = word()?;
                let mut pairs = Vec::new();
                for _ in 0..(n as i32).max(0) {
                    pairs.push([word()?, word()?]);
                }
                buckets.push((n, pairs));
            }
            queues.push(QueueWords { current, buckets });
        }
        (at == bytes.len()).then_some(Self { table_count, queues })
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Out::default();
        out.word(self.table_count);
        for q in &self.queues {
            out.word(0);
            out.word(q.current);
            out.word(q.buckets.len() as u32);
            for (n, pairs) in &q.buckets {
                out.word(*n);
                pairs.iter().flatten().for_each(|&w| out.word(w));
            }
        }
        out.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aa_mapworld::{Chunk, ChunkDirectory, MapArchive, SourceIdentity, archive::Encoding};

    fn chunk(name: &str, bytes: Vec<u8>) -> Chunk {
        Chunk {
            map_source: SourceIdentity::of(&[]),
            directory: ChunkDirectory {
                name: name.into(),
                directory_ordinal: 0,
                packed_offset: 12,
                packed_bytes: bytes.len() as u64,
                source_timestamp: 0,
                decoded_bytes: bytes.len() as u64,
            },
            payload_source: SourceIdentity::of(&bytes),
            encoding: Encoding::Raw,
            bytes,
        }
    }

    fn sample() -> GworldWords {
        GworldWords {
            stream: 0x9ea4_d5b9,
            tick: 0x0001_45a0,
            begin: 0x45a2_cf80,
            end: 0x45a2_d000,
            step: 0x3d80_0000,
            next_gplayer_id: 0,
            next_camera_id: 0,
            next_area_id: 0,
            next_structure_queue_item_id: 0x2bc,
            map_status: 2,
            mission_state: MISSION_RUNNING,
        }
    }

    /// What is written reads back with the loader's port as the same words.
    #[test]
    fn gworld_reads_back() {
        let words = sample();
        let bytes = words.encode();
        assert_eq!(bytes.len(), 48);
        let state = WorldState::decode(&chunk("GWORLD", bytes)).unwrap();
        assert_eq!(state.version, GWORLD_VERSION);
        assert!(state.trailing.is_empty());
        assert_eq!(GworldWords::of_state(&state), Some(words));
    }

    #[test]
    fn idmgr_reads_back() {
        let words = IdmgrWords { next_slot: 0x1e01, highest_slot: 3, generations: vec![0, 2, 0] };
        let state = IdManagerState::decode(&chunk("IDMGR", words.encode().unwrap())).unwrap();
        assert_eq!(state.capacity, IDMGR_CAPACITY);
        assert_eq!(IdmgrWords::of_state(&state), words);
        let short = IdmgrWords { generations: vec![0, 2], ..words };
        assert_eq!(short.encode(), None);
    }

    #[test]
    fn misc_data_reads_back() {
        let units = |s: &str| s.encode_utf16().collect::<Vec<u16>>();
        let data = MiscData {
            entries: vec![(units("namelist_northafrica"), 72), (units("namelist_northafrica"), 63)],
            word_278: 5,
            text_290: units("x"),
            texts_294: vec![units("a"), Vec::new()],
            pair_27c: Some([1, 2]),
        };
        let bytes = data.encode();
        assert_eq!(MiscData::decode(&bytes), Some(data));
        assert_eq!(MiscData::decode(&bytes[..bytes.len() - 1]), None);
        let empty = MiscData::default().encode();
        assert_eq!(empty, [0u8; 15]);
        assert_eq!(MiscData::decode(&empty), Some(MiscData::default()));
    }

    /// Every component writer against `aa_mapworld`'s reader of the same slot, at the
    /// newest version the reader takes (where the most optional words are present) and
    /// at an older one where a version adds or drops words.
    #[test]
    fn components_read_back() {
        use aa_mapworld::saved_components::*;
        let u = |id| UpdatablePayload { version: 1, serialized_id: Some(id) };
        let words = |v: &[u32]| CountedList { serialized_count: v.len() as u32, values: v.to_vec() };
        let ability = aa_mapworld::kingdoms::AbilityState { version: 1, word_0c: 1, word_10: 0xff7f_ffff, word_14: 3, byte_18: 1, words: Some(words(&[7, 8])) };
        let cases: Vec<(u8, SavedComponent)> = vec![
            (0, SavedComponent::Body(BodyPayload { version: 4, updatable: Some(u(9)), dying_started_bits: Some(0xbf80_0000), health_multiplier_bits: 0x3f40_0000, sink_duration_bits: Some(5), last_heal_time_bits: Some(6) })),
            (0, SavedComponent::Body(BodyPayload { version: 1, updatable: None, dying_started_bits: Some(1), health_multiplier_bits: 0x3f80_0000, sink_duration_bits: None, last_heal_time_bits: None })),
            (
                3,
                SavedComponent::Mover(MoverPayload {
                    updatable: u(3),
                    version: 8,
                    vector_10: [1, 2, 3],
                    word_20: 4,
                    word_24: 5,
                    speed_multiplier_bits: Some(0x3f80_0000),
                    legacy_vector: None,
                    word_44: 6,
                    word_48: 7,
                    byte_4c: None,
                    byte_54: Some(1),
                    vector_38: Some([8, 9, 10]),
                    word_50: Some(11),
                    byte_56: Some(0),
                    word_58: Some(12),
                    byte_5c: Some(1),
                    vector_60: Some([13, 14, 15]),
                    byte_6c: Some(0),
                    vector_70: Some([16, 17, 18]),
                    vector_7c: Some([19, 20, 21]),
                }),
            ),
            (
                3,
                SavedComponent::Mover(MoverPayload {
                    updatable: UpdatablePayload { version: 0, serialized_id: None },
                    version: 3,
                    vector_10: [1, 2, 3],
                    word_20: 4,
                    word_24: 5,
                    speed_multiplier_bits: None,
                    legacy_vector: Some([6, 7, 8]),
                    word_44: 9,
                    word_48: 10,
                    byte_4c: Some(1),
                    byte_54: Some(0),
                    vector_38: Some([11, 12, 13]),
                    word_50: Some(14),
                    byte_56: None,
                    word_58: None,
                    byte_5c: None,
                    vector_60: None,
                    byte_6c: None,
                    vector_70: None,
                    vector_7c: None,
                }),
            ),
            (
                2,
                SavedComponent::Character(CharacterPayload {
                    updatable: u(4),
                    version: 2,
                    word_28: 1,
                    legacy_word: None,
                    animation_kind: -1,
                    animation_index: 3,
                    animation_version: 0,
                    elapsed_bits: 5,
                    resumed_at_bits: 6,
                    rate_bits: 7,
                    active_byte: 1,
                    variant: Some(-1),
                }),
            ),
            (
                15,
                SavedComponent::Construction(ConstructionPayload {
                    updatable: u(5),
                    version: 1,
                    words_10: words(&[1, 2, 3]),
                    words_1c: Some(words(&[4])),
                    reference_28_utf16: Some(Vec::new()),
                    legacy_upgrade_words: None,
                }),
            ),
            (
                5,
                SavedComponent::Performer(PerformerPayload {
                    updatable: u(6),
                    version: 9,
                    word_10: 1,
                    multiplier_20: 2,
                    word_28: 3,
                    words_2c_30: [4, 5],
                    ability_index: -1,
                    words_3c_40: [6, 7],
                    states: CountedList {
                        serialized_count: 1,
                        values: vec![PerformerStateEntry { words_00_04: [8, 9], word_08: Some(10), present_byte: Some(1), words_10_14: Some([11, 12]), word_18: Some(13) }],
                    },
                    word_58: 14,
                    word_44: Some(15),
                    abilities: CountedList { serialized_count: 1, values: vec![ability.clone()] },
                    byte_5c: Some(0),
                }),
            ),
            (
                10,
                SavedComponent::Denizen(DenizenPayload {
                    updatable: u(7),
                    version: 4,
                    byte_20: Some(1),
                    byte_21: Some(0),
                    occupants: Some(CountedList { serialized_count: 1, values: vec![[1, 2]] }),
                    word_34: Some(3),
                    handles: Some(words(&[4, 5])),
                    command_count: Some(0),
                }),
            ),
            (11, SavedComponent::Supply(SupplyPayload { version: 2, updatable: Some(u(8)), amount_multiplier_bits: Some(0x3f00_0000), word_1c: Some(1) })),
            (11, SavedComponent::Supply(SupplyPayload { version: 0, updatable: None, amount_multiplier_bits: None, word_1c: None })),
            (16, SavedComponent::Recruit(RecruitPayload { updatable: u(9), version: 3, byte_1c: Some(1), words_14: Some([1, 2]), legacy: None })),
            (
                16,
                SavedComponent::Recruit(RecruitPayload {
                    updatable: u(9),
                    version: 2,
                    byte_1c: Some(0),
                    words_14: None,
                    legacy: Some(LegacyRecruitQueue { serialized_count: 1, items: vec![(units("a"), 0)], word_after: 3, byte_v2: Some(1) }),
                }),
            ),
            (
                20,
                SavedComponent::Defender(DefenderPayload {
                    updatable: u(10),
                    version: 2,
                    abilities: Some(CountedList { serialized_count: 1, values: vec![ability] }),
                    pairs: Some(CountedList { serialized_count: 1, values: vec![[1, 2]] }),
                }),
            ),
            (21, SavedComponent::Research(ResearchPayload { updatable: u(11), version: 0, references: CountedList { serialized_count: 1, values: vec![units("prop")] } })),
            (25, SavedComponent::Lair { updatable: u(12), version: 0, words: words(&[1]), word_1c: 2, word_20: 3 }),
            (29, SavedComponent::City { updatable: u(13), version: 1, words: Some(words(&[1, 2])) }),
            (
                13,
                SavedComponent::Structure(StructurePayload {
                    version: 13,
                    updatable: Some(u(14)),
                    discarded_object_handle: 0,
                    legacy_reference_count: None,
                    queue_count: Some(0),
                    word_20: Some(1),
                    word_2c: Some(2),
                    word_24: Some(0x7f7f_ffff),
                    word_28: Some(u32::MAX),
                    word_34: Some(3),
                }),
            ),
            (6, SavedComponent::Properties(PropertiesPayload { updatable: u(15), version: 1, property_classes: vec![u32::MAX, u32::MAX] })),
            (9, SavedComponent::UpdatableOnly { updatable: u(16), version: 0 }),
            (17, SavedComponent::Experience { version: 0, word_08: 0x4000_0000 }),
            (8, SavedComponent::Element { version: 3, updatable: Some(u(17)), legacy_byte: None, history_count: Some(0) }),
            (8, SavedComponent::Element { version: 1, updatable: Some(u(17)), legacy_byte: Some(1), history_count: None }),
            (19, SavedComponent::MobileDeployment { updatable: u(18), version: 2, word_10: 1, word_14: Some(2), discarded_resources: Some(vec![3, 4]) }),
        ];
        for (slot, c) in cases {
            let mut out = Out::default();
            out.component(&c).unwrap_or_else(|| panic!("slot {slot} refused"));
            let (back, n) = SavedComponent::decode_prefix(slot, &out.0).unwrap_or_else(|e| panic!("slot {slot}: {e}"));
            assert_eq!(n, out.0.len(), "slot {slot}");
            assert_eq!(back, c, "slot {slot}");
        }
        // What the reader refuses is refused here too.
        let busy = SavedComponent::Element { version: 3, updatable: None, legacy_byte: None, history_count: Some(1) };
        assert_eq!(Out::default().component(&busy), None);
        assert_eq!(Out::default().component(&SavedComponent::Properties(PropertiesPayload { updatable: u(1), version: 1, property_classes: vec![0] })), None);
    }

    /// A chunk file of uncompressed chunks (the reader takes a chunk whose bytes
    /// are not a zlib stream as stored).
    fn container(chunks: &[(String, Vec<u8>)]) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend(b"TGCK");
        out.extend(2u32.to_le_bytes());
        out.extend((chunks.len() as u32).to_le_bytes());
        let mut rows = Vec::new();
        for (name, bytes) in chunks {
            rows.push((name, out.len() as u64, bytes.len() as u64));
            out.extend(bytes);
        }
        for (name, offset, len) in rows {
            let mut field = [0u16; 32];
            for (i, u) in name.encode_utf16().enumerate() {
                field[i] = u;
            }
            out.extend(field.iter().flat_map(|u| u.to_le_bytes()));
            for w in [offset, len, 0, len] {
                out.extend(w.to_le_bytes());
            }
        }
        out
    }

    /// The game data and an original saved game (`AA_SAVED_GAME`, read only).
    struct Saved {
        data: std::sync::Arc<crate::data::GameData>,
        generals: std::sync::Arc<crate::special_ops::GeneralData>,
        bytes: Vec<u8>,
    }

    impl Saved {
        fn open() -> Self {
            use std::sync::Arc;
            let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
            let path = std::path::PathBuf::from(std::env::var_os("AA_SAVED_GAME").expect("AA_SAVED_GAME"));
            let bytes = std::fs::read(&path).unwrap();
            let data = Arc::new(crate::data::GameData::load(&root).unwrap());
            let generals = Arc::new(crate::special_ops::GeneralData::load(&root, &data).unwrap());
            Self { data, generals, bytes }
        }

        fn restore(&self, bytes: &[u8]) -> crate::world::World {
            self.try_restore(bytes).unwrap()
        }

        fn try_restore(&self, bytes: &[u8]) -> anyhow::Result<crate::world::World> {
            let human = |_: &str| crate::world::Controller::Human;
            let map = crate::map::GameMap::load_file("saved/round-trip", bytes.to_vec(), &self.data, None)?;
            Ok(crate::saved_world::restore(bytes, self.data.clone(), self.generals.clone(), std::sync::Arc::new(map), &human)?.world)
        }
    }

    fn gworld_of(bytes: &[u8]) -> GworldWords {
        let archive = MapArchive::from_bytes(bytes.to_vec()).unwrap();
        GworldWords::of_state(&WorldState::decode(&archive.decode_chunk("GWORLD", 1 << 30).unwrap()).unwrap()).expect("a version 28 GWORLD")
    }

    fn idmgr_of(bytes: &[u8]) -> Vec<u8> {
        MapArchive::from_bytes(bytes.to_vec()).unwrap().decode_chunk("IDMGR", 1 << 30).unwrap().bytes
    }

    /// A saved game's KINGDOMS, decoded, and its bytes.
    fn kingdoms_of(bytes: &[u8]) -> (KingdomsChunk, Vec<u8>) {
        let chunk = MapArchive::from_bytes(bytes.to_vec()).unwrap().decode_chunk("KINGDOMS", 1 << 30).unwrap();
        (KingdomsChunk::decode(&chunk, GWORLD_VERSION).expect("KINGDOMS"), chunk.bytes)
    }

    /// A general's balance and the last use of each ability, as bits.
    type GeneralBits = Option<(u32, Vec<Option<u32>>)>;

    /// What a restore keeps of each kingdom, as bits: the stockpiles, whether it is out
    /// of the game, its general.
    fn kingdom_state(world: &crate::world::World) -> Vec<(Vec<u32>, bool, GeneralBits)> {
        world
            .kingdoms
            .iter()
            .map(|k| {
                let stock = world.books.kingdom(k.id).map_or_else(Vec::new, |b| b.stockpiled.iter().map(|v| v.to_bits()).collect());
                let general = world
                    .special_ops
                    .commanders
                    .get(k.id as usize)
                    .filter(|c| c.general.is_some())
                    .map(|c| (c.experience.to_bits(), c.last_use.iter().map(|t| t.map(f32::to_bits)).collect()));
                (stock, k.defeated, general)
            })
            .collect()
    }

    /// `bytes`' chunks with `written` in place of the chunks of the same names.
    fn with_chunks(bytes: &[u8], written: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let archive = MapArchive::from_bytes(bytes.to_vec()).unwrap();
        let chunks: Vec<(String, Vec<u8>)> = CHUNKS
            .iter()
            .map(|(name, _)| {
                let b = match written.iter().find(|(n, _)| n == name) {
                    Some((_, b)) => b.clone(),
                    None => archive.decode_chunk(name, 1 << 30).unwrap().bytes,
                };
                (name.to_string(), b)
            })
            .collect();
        container(&chunks)
    }

    /// The GWORLD words that differ, by offset.
    fn differences(a: &GworldWords, b: &GworldWords) -> Vec<String> {
        const NAMES: [&str; 12] = ["version", "+D4", "+DC", "+E0", "+E4", "+E8", "+140", "+184", "+194", "+198", "+44", "+48"];
        let (x, y) = (a.encode(), b.encode());
        let word = |v: &[u8], i: usize| u32::from_le_bytes([v[i * 4], v[i * 4 + 1], v[i * 4 + 2], v[i * 4 + 3]]);
        (0..12).filter(|&i| word(&x, i) != word(&y, i)).map(|i| format!("{} {} vs {}", NAMES[i], word(&x, i), word(&y, i))).collect()
    }

    /// A battle restored from an original saved game and written at once: its IDMGR
    /// is the file's byte for byte, and so is its GWORLD but for the stream +D4 (the
    /// saved one drawn on by the load's record pass) and +198 where the restore does
    /// not put GWORLD's +198 in `Books::next_item` (printed).
    #[test]
    #[ignore = "needs the game data (AA_GAME_ROOT) and an original saved game (AA_SAVED_GAME)"]
    fn a_restored_world_writes_the_saved_words() {
        let saved = Saved::open();
        let world = saved.restore(&saved.bytes);
        let loaded = gworld_of(&saved.bytes);
        assert_eq!(IdmgrWords::of_world(&world).encode().unwrap(), idmgr_of(&saved.bytes), "IDMGR");
        let written = GworldWords::of_world(&world, &loaded);
        // The stream the restore leaves is the saved one after the load's record
        // pass draws (`saved_world`), as the original's load leaves it.
        let pass = draws(loaded.stream, written.stream).expect("the restored stream is the saved one, drawn on");
        let differ = differences(&written, &loaded);
        eprintln!("GWORLD written from the restored world: the stream {pass} draws on; differs in {differ:?}");
        assert!(differ.iter().all(|d| d.starts_with("+198 ") || d.starts_with("+D4 ")), "{differ:?}");
    }

    /// How many draws of the world stream lead from `from` to `to`.
    fn draws(from: u32, to: u32) -> Option<u32> {
        let mut rng = aa_simulation::NativeRng::new(from);
        (0..1u32 << 22).find(|_| {
            let at = rng.state == to;
            rng.next_u32();
            at
        })
    }

    /// A battle restored from an original saved game, played on for 64 ticks, its
    /// GWORLD and IDMGR written and put in place of the saved ones (the other chunks
    /// are the original's): the restore of that file stands at the played clock and
    /// stream, its GTID manager is the played one, and it writes the same IDMGR and
    /// GWORLD again (+198 aside, as above).
    #[test]
    #[ignore = "needs the game data (AA_GAME_ROOT) and an original saved game (AA_SAVED_GAME)"]
    fn a_played_world_written_and_restored_again() {
        let saved = Saved::open();
        let mut world = saved.restore(&saved.bytes);
        let loaded = gworld_of(&saved.bytes);
        for _ in 0..64 {
            world.step(Vec::new());
        }
        let gworld = GworldWords::of_world(&world, &loaded);
        let idmgr = IdmgrWords::of_world(&world);
        let mut chunks = vec![("GWORLD", gworld.encode()), ("IDMGR", idmgr.encode().unwrap())];
        // A kingdom that finished a research since the save holds a property the
        // writer has no record for yet: then the saved KINGDOMS stays (printed).
        let kingdoms = match kingdoms_of(&saved.bytes).0.of_world(&world) {
            Ok(k) => k.encode(),
            Err(e) => {
                eprintln!("KINGDOMS not written from the world: {e}");
                None
            }
        };
        chunks.extend(kingdoms.clone().map(|k| ("KINGDOMS", k)));
        let file = with_chunks(&saved.bytes, &chunks);
        assert_eq!(gworld_of(&file), gworld);
        let restored = saved.restore(&file);
        if let Some(kingdoms) = kingdoms {
            // The kingdoms restore as they were played.
            assert_eq!(kingdom_state(&restored), kingdom_state(&world), "the kingdoms of the written file");
            assert_eq!(kingdoms_of(&file).0.of_world(&restored).unwrap().encode().unwrap(), kingdoms, "KINGDOMS written again");
        }
        assert_eq!(restored.clock, world.clock);
        assert_eq!(restored.tick, world.tick);
        assert_eq!(restored.time.to_bits(), world.time.to_bits());
        assert_eq!(restored.clock.tick, loaded.tick + 64);
        // The restore read the played stream and drew on it as many times as its
        // record pass draws on the original's file (the same things).
        let first = draws(loaded.stream, saved.restore(&saved.bytes).native_rng.state).expect("first pass");
        let pass = draws(world.native_rng.state, restored.native_rng.state).expect("the played stream, drawn on");
        assert_eq!(pass, first);
        assert_eq!(IdmgrWords::of_world(&restored), idmgr);
        let again = GworldWords::of_world(&restored, &gworld_of(&file));
        let differ = differences(&again, &gworld);
        eprintln!(
            "restored at tick {} (saved {}), the stream {pass} draws on, next slot {}, highest {}, {} live ids (played {}); GWORLD differs in {differ:?}",
            restored.clock.tick,
            loaded.tick,
            restored.ids.next_slot(),
            restored.ids.highest_slot(),
            restored.ids.live(),
            world.ids.live()
        );
        assert!(differ.iter().all(|d| d.starts_with("+198 ") || d.starts_with("+D4 ")), "{differ:?}");
    }

    /// Where each thing stands, by GTID: an actor's x and y, an organization's anchor.
    fn positions(world: &crate::world::World) -> std::collections::BTreeMap<u32, [u32; 2]> {
        use crate::ids::Thing;
        world
            .ids
            .iter()
            .filter_map(|(id, thing)| {
                let xy = match thing {
                    Thing::Actor(a) => world.actors[a as usize].pos.truncate(),
                    Thing::Org(o) => world.orgs[o as usize].anchor,
                    _ => return None,
                };
                Some((id.raw(), [xy.x.to_bits(), xy.y.to_bits()]))
            })
            .collect()
    }

    /// A battle restored from an original saved game and played on for 64 ticks, its
    /// GWORLD, IDMGR, KINGDOMS and DYNAMIC_THINGS written from the world: the restore
    /// of that file has every thing the played world has (by GTID) where the played
    /// world has it, the things that moved since the save included.
    #[test]
    #[ignore = "needs the game data (AA_GAME_ROOT) and an original saved game (AA_SAVED_GAME)"]
    fn a_played_world_restores_where_its_things_stood() {
        let saved = Saved::open();
        let start = positions(&saved.restore(&saved.bytes));
        let mut world = saved.restore(&saved.bytes);
        let loaded = gworld_of(&saved.bytes);
        for _ in 0..64 {
            world.step(Vec::new());
        }
        let archive = MapArchive::from_bytes(saved.bytes.clone()).unwrap();
        let things = ThingsChunk::decode(&archive).unwrap().of_world(&world, GWORLD_VERSION);
        let mut chunks = vec![
            ("GWORLD", GworldWords::of_world(&world, &loaded).encode()),
            ("IDMGR", IdmgrWords::of_world(&world).encode().unwrap()),
            ("DYNAMIC_THINGS", things.data.clone()),
            ("DYNAMIC_THINGS_INDEX", things.index.clone()),
        ];
        match kingdoms_of(&saved.bytes).0.of_world(&world) {
            Ok(k) => chunks.push(("KINGDOMS", k.encode().unwrap())),
            Err(e) => eprintln!("KINGDOMS not written from the world: {e}"),
        }
        let file = with_chunks(&saved.bytes, &chunks);
        let restored = saved.restore(&file);
        let (played, back) = (positions(&world), positions(&restored));
        let moved = played.iter().filter(|(id, xy)| start.get(id).is_some_and(|s| s != *xy)).count();
        let differ: Vec<_> = played.iter().filter(|(id, xy)| back.get(id).is_some_and(|b| b != *xy)).map(|(id, _)| format!("{id:#x}")).collect();
        let missing: Vec<_> = played.keys().filter(|id| !back.contains_key(id)).map(|id| format!("{id:#x}")).collect();
        let extra: Vec<_> = back.keys().filter(|id| !played.contains_key(id)).map(|id| format!("{id:#x}")).collect();
        eprintln!(
            "{} things played, {moved} moved since the save; restored: {} differ, {} missing, {} extra; written: {} dropped, {} organizations with members as saved {:?}, {} not written",
            played.len(),
            differ.len(),
            missing.len(),
            extra.len(),
            things.dropped,
            things.members_as_saved.len(),
            things.members_as_saved.iter().take(3).collect::<Vec<_>>(),
            things.not_written.len()
        );
        assert!(differ.is_empty(), "positions differ: {differ:?}");
        let unwritten: Vec<String> = things.not_written.iter().map(|id| format!("{id:#x}")).collect();
        assert!(missing.iter().all(|m| unwritten.contains(m)), "missing {missing:?}, not written {unwritten:?}");
        assert!(extra.is_empty(), "restored but not played: {extra:?}");
    }

    /// What a played world written and restored still lacks, printed, not asserted:
    /// by GTID, the things that stand elsewhere or differ in health from the unsaved
    /// world's, at the restore and after 32 more ticks of both, and the streams.
    #[test]
    #[ignore = "needs the game data (AA_GAME_ROOT) and an original saved game (AA_SAVED_GAME)"]
    fn a_played_world_and_its_restore_played_on() {
        use crate::ids::Thing;
        let saved = Saved::open();
        let mut world = saved.restore(&saved.bytes);
        let loaded = gworld_of(&saved.bytes);
        for _ in 0..64 {
            world.step(Vec::new());
        }
        let archive = MapArchive::from_bytes(saved.bytes.clone()).unwrap();
        let things = ThingsChunk::decode(&archive).unwrap().of_world(&world, GWORLD_VERSION);
        let mut chunks = vec![
            ("GWORLD", GworldWords::of_world(&world, &loaded).encode()),
            ("IDMGR", IdmgrWords::of_world(&world).encode().unwrap()),
            ("DYNAMIC_THINGS", things.data),
            ("DYNAMIC_THINGS_INDEX", things.index),
        ];
        if let Ok(k) = kingdoms_of(&saved.bytes).0.of_world(&world) {
            chunks.push(("KINGDOMS", k.encode().unwrap()));
        }
        let mut restored = saved.restore(&with_chunks(&saved.bytes, &chunks));
        let standing = |w: &crate::world::World| -> std::collections::BTreeMap<u32, ([u32; 2], Option<u32>)> {
            w.ids
                .iter()
                .filter_map(|(id, thing)| match thing {
                    Thing::Actor(a) => {
                        let a = &w.actors[a as usize];
                        Some((id.raw(), ([a.pos.x.to_bits(), a.pos.y.to_bits()], Some(a.health.to_bits()))))
                    }
                    Thing::Org(o) => Some((id.raw(), ([w.orgs[o as usize].anchor.x.to_bits(), w.orgs[o as usize].anchor.y.to_bits()], None))),
                    _ => None,
                })
                .collect()
        };
        let report = |a: &crate::world::World, b: &crate::world::World| {
            let (x, y) = (standing(a), standing(b));
            let elsewhere = x.iter().filter(|(id, s)| y.get(id).is_some_and(|t| t.0 != s.0)).count();
            let health = x.iter().filter(|(id, s)| y.get(id).is_some_and(|t| t.1 != s.1)).count();
            let missing = x.keys().filter(|id| !y.contains_key(id)).count();
            let extra = y.keys().filter(|id| !x.contains_key(id)).count();
            format!(
                "tick {} / {}, streams {}, {} things: {elsewhere} stand elsewhere, {health} differ in health, {missing} missing, {extra} extra",
                a.clock.tick,
                b.clock.tick,
                if a.native_rng.state == b.native_rng.state { "equal" } else { "differ" },
                x.len()
            )
        };
        eprintln!("at the restore: {}", report(&world, &restored));
        for _ in 0..32 {
            world.step(Vec::new());
            restored.step(Vec::new());
        }
        eprintln!("32 ticks on: {}", report(&world, &restored));
    }

    /// Every saved game in `AA_SAVED_GAMES` that `saved_world` restores, written at
    /// once: the IDMGR is the file's byte for byte, and the GWORLD is the file's but
    /// for the stream (the saved one drawn on by the load's record pass) and +198
    /// (which the restore does not take yet; counted).
    #[test]
    #[ignore = "needs the game data (AA_GAME_ROOT) and a folder of the original's saved games (AA_SAVED_GAMES)"]
    fn every_restored_save_writes_its_idmgr_and_gworld() {
        use std::sync::Arc;
        let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
        let data = Arc::new(crate::data::GameData::load(&root).unwrap());
        let generals = Arc::new(crate::special_ops::GeneralData::load(&root, &data).unwrap());
        let saved = Saved { data, generals, bytes: Vec::new() };
        let folder = std::env::var_os("AA_SAVED_GAMES").expect("AA_SAVED_GAMES");
        let mut files: Vec<_> = std::fs::read_dir(folder)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("RSG")))
            .collect();
        files.sort();
        let (mut restored, mut refused, mut next_item_differs) = (0usize, Vec::new(), 0usize);
        let (mut things_exact, mut things_dropped, mut members_as_saved) = (0usize, 0usize, 0usize);
        for path in &files {
            let bytes = std::fs::read(path).unwrap();
            if MapArchive::from_bytes(bytes.clone()).is_err() {
                continue;
            }
            let world = match saved.try_restore(&bytes) {
                Ok(w) => w,
                Err(e) => {
                    refused.push(format!("{}: {e}", path.display()));
                    continue;
                }
            };
            restored += 1;
            assert_eq!(IdmgrWords::of_world(&world).encode().unwrap(), idmgr_of(&bytes), "{} IDMGR", path.display());
            let loaded = gworld_of(&bytes);
            let written = GworldWords::of_world(&world, &loaded);
            assert!(draws(loaded.stream, written.stream).is_some(), "{} stream", path.display());
            let differ = differences(&written, &loaded);
            assert!(differ.iter().all(|d| d.starts_with("+198 ") || d.starts_with("+D4 ")), "{} {differ:?}", path.display());
            next_item_differs += usize::from(differ.iter().any(|d| d.starts_with("+198 ")));
            let (kingdoms, saved_bytes) = kingdoms_of(&bytes);
            let written = kingdoms.of_world(&world).unwrap_or_else(|e| panic!("{} KINGDOMS: {e}", path.display()));
            assert_eq!(written.encode(), Some(saved_bytes), "{} KINGDOMS from the world", path.display());
            // DYNAMIC_THINGS from the world: the saved chunk but for the records of the
            // things the restore does not make (dropped, counted).
            let archive = MapArchive::from_bytes(bytes.clone()).unwrap();
            let things = ThingsChunk::decode(&archive).unwrap().of_world(&world, GWORLD_VERSION);
            if things.dropped == 0 {
                assert_eq!(things.data, archive.decode_chunk("DYNAMIC_THINGS", 1 << 30).unwrap().bytes, "{} DYNAMIC_THINGS", path.display());
                assert_eq!(things.index, archive.decode_chunk("DYNAMIC_THINGS_INDEX", 1 << 30).unwrap().bytes, "{} DYNAMIC_THINGS_INDEX", path.display());
                things_exact += 1;
            }
            things_dropped += things.dropped;
            members_as_saved += things.members_as_saved.len();
        }
        eprintln!(
            "{restored} saved games restored and written: IDMGR and KINGDOMS exact, GWORLD exact but the stream; +198 differs in {next_item_differs}; \
             DYNAMIC_THINGS exact in {things_exact} (the others drop {things_dropped} records of things not restored), {members_as_saved} organizations with members as saved; refused {refused:?}"
        );
        assert!(restored > 0, "no saved game restored");
    }

    fn saved_files() -> Vec<std::path::PathBuf> {
        let folder = std::env::var_os("AA_SAVED_GAMES").expect("AA_SAVED_GAMES");
        let mut files: Vec<_> = std::fs::read_dir(folder)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("RSG")))
            .collect();
        files.sort();
        files
    }

    /// Every DYNAMIC_THINGS record of every saved game in `AA_SAVED_GAMES`, walked
    /// whole (`aa_mapworld::saved_world::walk`, an organization's members inside it)
    /// and written again from what the walk keeps ([`thing_tail`],
    /// [`Out::walked_record`]): each tail the writers take is the saved tail byte for
    /// byte, and each whole record is the saved record. The records they do not take
    /// are counted by the reason.
    #[test]
    #[ignore = "reads the original's saved games: set AA_SAVED_GAMES to a folder of them"]
    fn original_saved_things_re_encode_from_the_walk() {
        let mut reasons: std::collections::BTreeMap<String, usize> = Default::default();
        let mut still: std::collections::BTreeMap<String, usize> = Default::default();
        let (mut files, mut tails, mut records, mut slots, mut over) = (0usize, 0usize, 0usize, [0usize; 31], 0usize);
        for path in saved_files() {
            let Ok(archive) = MapArchive::from_bytes(std::fs::read(&path).unwrap()) else { continue };
            files += 1;
            let version = WorldState::decode(&archive.decode_chunk("GWORLD", 1 << 30).unwrap()).unwrap().version;
            let inventory = Inventory::decode(&archive, 1 << 30).unwrap();
            let groups = inventory.unsupported.iter().filter_map(|u| aa_mapworld::inventory::decode_group_record(&u.raw, u.record_ordinal).ok());
            for p in inventory.placements.iter().cloned().chain(groups) {
                let mut whole = Out::default();
                whole.placement(&p);
                let (raw_tail, raw_offset) = (p.opaque_tail.clone(), p.opaque_tail_span.offset);
                let thing = match aa_mapworld::saved_world::walk(p, version, 1, &mut crate::saved_world::Minds) {
                    Ok(t) => t,
                    Err(e) => {
                        *reasons.entry(format!("walk: {}", e.to_string().chars().take(60).collect::<String>())).or_default() += 1;
                        continue;
                    }
                };
                if let Some(e) = &thing.partial {
                    *reasons.entry(format!("walk: {}", e.chars().take(60).collect::<String>())).or_default() += 1;
                    continue;
                }
                if thing.tail.is_none() {
                    *reasons.entry("no GActor".into()).or_default() += 1;
                    continue;
                }
                match thing_tail(&thing, version) {
                    Ok(tail) => {
                        // A group's own words follow its GActor tail.
                        if thing.record.class_code == 8 {
                            assert!(thing.record.opaque_tail.starts_with(&tail), "{} group {}", path.display(), thing.record.record_ordinal);
                        } else {
                            assert_eq!(tail, thing.record.opaque_tail, "{} record {}", path.display(), thing.record.record_ordinal);
                        }
                        tails += 1;
                        let mask = thing.tail.as_ref().unwrap().saved_components_mask;
                        (0..31).filter(|s| mask & (1 << s) != 0).for_each(|s| slots[s] += 1);
                        if thing.record.class_code != 8 && thing.sightings.is_empty() {
                            let mut again = Out::default();
                            again.walked_record(&thing, version).unwrap();
                            assert_eq!(again.0, whole.0, "{} whole record {}", path.display(), thing.record.record_ordinal);
                            records += 1;
                        }
                    }
                    Err(e) => {
                        *reasons.entry(e).or_default() += 1;
                        // Over the saved tail, as `ThingsChunk::of_world` writes an
                        // organization: what the walk does not keep is taken from it.
                        let drained = thing.record.opaque_tail_span.offset - raw_offset;
                        match thing_tail_over(&thing, version, Over { saved: raw_tail.get(drained..), keep: &keep_all }) {
                            Ok(tail) => {
                                assert!(thing.record.opaque_tail.starts_with(&tail), "{} record {} over the saved tail", path.display(), thing.record.record_ordinal);
                                if thing.record.class_code != 8 {
                                    assert_eq!(tail.len(), thing.record.opaque_tail.len(), "{} record {}", path.display(), thing.record.record_ordinal);
                                }
                                over += 1;
                            }
                            Err(e) => *still.entry(e).or_default() += 1,
                        }
                    }
                }
            }
        }
        eprintln!("{files} saved games: {tails} GActor tails written again exactly ({records} whole records); saved components in them by slot {slots:?}");
        for (why, n) in &reasons {
            eprintln!("  not written: {n} x {why}");
        }
        eprintln!("of those, written over the saved tail exactly: {over}");
        for (why, n) in &still {
            eprintln!("  still not written: {n} x {why}");
        }
        assert!(tails > 0);
    }

    /// Every saved game in `AA_SAVED_GAMES` (a folder of the original's `.RSG`
    /// files, read only): its chunks are [`CHUNKS`] in order with their encodings,
    /// and its GWORLD, IDMGR, TEAMS, KINGDOMS, DYNAMIC_THINGS (with its index),
    /// UPDATABLES and MISC_DATA, read and written again, are the original's bytes. A file whose
    /// directory is not a chunk directory is counted and named, not checked.
    #[test]
    #[ignore = "reads the original's saved games: set AA_SAVED_GAMES to a folder of them"]
    fn original_saved_games_re_encode_exactly() {
        let folder = std::env::var_os("AA_SAVED_GAMES").expect("AA_SAVED_GAMES");
        let mut files: Vec<_> = std::fs::read_dir(folder)
            .unwrap()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("RSG")))
            .collect();
        files.sort();
        let (mut checked, mut unreadable, mut framed, mut whole) = (0usize, 0usize, 0usize, 0usize);
        for path in &files {
            // A file whose directory is not a chunk directory (a save cut short) is
            // not a saved game the original can open either.
            let Ok(archive) = MapArchive::from_bytes(std::fs::read(path).unwrap()) else {
                eprintln!("not a chunk file: {}", path.display());
                unreadable += 1;
                continue;
            };
            checked += 1;
            let names: Vec<&str> = archive.directory().iter().map(|d| d.name.as_str()).collect();
            let expected: Vec<&str> = CHUNKS.iter().map(|c| c.0).collect();
            assert_eq!(names, expected, "{}", path.display());
            for (name, compressed) in CHUNKS {
                let c = archive.decode_chunk(name, 1 << 30).unwrap();
                assert_eq!(c.encoding == Encoding::Zlib, compressed, "{} {name}", path.display());
            }
            let gworld = archive.decode_chunk("GWORLD", 1 << 30).unwrap();
            let words = GworldWords::of_state(&WorldState::decode(&gworld).unwrap()).expect("a version 28 GWORLD");
            assert_eq!(words.encode(), gworld.bytes, "{} GWORLD", path.display());
            let idmgr = archive.decode_chunk("IDMGR", 1 << 30).unwrap();
            let ids = IdmgrWords::of_state(&IdManagerState::decode(&idmgr).unwrap());
            assert_eq!(ids.encode().as_ref(), Some(&idmgr.bytes), "{} IDMGR", path.display());
            let teams = archive.decode_chunk("TEAMS", 1 << 30).unwrap();
            let decoded = TeamsChunk::decode(&teams).unwrap_or_else(|| panic!("{} TEAMS", path.display()));
            assert_eq!(decoded.encode(), teams.bytes, "{} TEAMS", path.display());
            let kingdoms = archive.decode_chunk("KINGDOMS", 1 << 30).unwrap();
            let decoded = KingdomsChunk::decode(&kingdoms, GWORLD_VERSION).unwrap_or_else(|| panic!("{} KINGDOMS", path.display()));
            assert_eq!(decoded.encode().as_ref(), Some(&kingdoms.bytes), "{} KINGDOMS", path.display());
            let things = ThingsChunk::decode(&archive).unwrap_or_else(|| panic!("{} DYNAMIC_THINGS", path.display()));
            let (data, index) = things.encode();
            assert_eq!(data, archive.decode_chunk("DYNAMIC_THINGS", 1 << 30).unwrap().bytes, "{} DYNAMIC_THINGS", path.display());
            assert_eq!(index, archive.decode_chunk("DYNAMIC_THINGS_INDEX", 1 << 30).unwrap().bytes, "{} DYNAMIC_THINGS_INDEX", path.display());
            framed += things.0.placements.len();
            whole += things.0.unsupported.len();
            let updatables = archive.decode_chunk("UPDATABLES", 1 << 30).unwrap();
            let decoded = UpdatablesChunk::decode(&updatables.bytes).unwrap_or_else(|| panic!("{} UPDATABLES", path.display()));
            assert_eq!(decoded.encode(), updatables.bytes, "{} UPDATABLES", path.display());
            let misc = archive.decode_chunk("MISC_DATA", 1 << 30).unwrap();
            let data = MiscData::decode(&misc.bytes).unwrap_or_else(|| panic!("{} MISC_DATA", path.display()));
            assert_eq!(data.encode(), misc.bytes, "{} MISC_DATA", path.display());
        }
        assert!(checked > 0, "no saved games");
        eprintln!(
            "{checked} saved games re-encode exactly ({unreadable} files not chunk files); {framed} thing records framed, {whole} written back whole"
        );
    }
}
