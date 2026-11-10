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
//!   the inverse of `aa_mapworld`'s reader. These write a decoded chunk again; what
//!   `aa-play` keeps of a team or kingdom is not written into them yet (a team's +4C
//!   and +58 are its relations to every kingdom and every team; a kingdom's record
//!   holds its stockpiles, recruitment, AI and the rest);
//! - the frame of `DYNAMIC_THINGS` and its index ([`ThingsChunk`]): each record's
//!   head, GThing, GObject and GActor prefix are written from `aa_mapworld`'s
//!   inventory, the rest of each record (components, Mind) is still the saved bytes;
//! - `UPDATABLES` ([`UpdatablesChunk`]): the Updatable table's count and the five
//!   world queues, written again from a decoded chunk.
//!
//! Not written yet, each by the original's writer named: `MAP` and `MAP_HEADER` (the
//! map resource), `RULES` (GWorld+0C), `TERRAIN` (+30), `HEIGHT_MAP` (+34),
//! `MOUNTAIN_HEIGHT_MAP` (+38, then the byte +275 in the same chunk), `RESTRICTION_GROUPS`
//! (0x503FA0), `OBJECTIVES` (count +254 and each objective's Save, then the word
//! +25C), `USED_REQUIRED_OBJECTS` and `DYNAMIC_THINGS` with their indexes (0x4F24C0:
//! a word 0 and the version 11, then 0x4F1B90 for every live thing in the GTID
//! manager's slot order that its virtuals +B0 / +A8 / +AC / +B4 select, then the
//! `_INDEX` chunk with a word 0, the count and each record's length; on the
//! original's saves the lengths and the 8 head bytes add up to the chunk), `TRIGGERS` (0x54C830),
//! `DENIZEN_GROUPS` (0x56A2A0), `SUN` (+4C), `UPDATABLES` (the word at *+280, then
//! the lists +1E8, +208, +1AC, +1C0, +1D4). A campaign battle (rules mode 4) also
//! writes `METAGAME` after `KINGDOMS` (0x938960). So a battle cannot be saved from
//! `aa-play` yet: a file with these chunks taken from the map would load as the
//! battle's start, not as it stands.

use aa_mapworld::actor_payload::{ActorPayload, CountedList, UpdatablePayload};
use aa_mapworld::inventory::ObjectState;
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
        Some(())
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
    /// is the file's byte for byte, and so is its GWORLD but for +198, which the
    /// restore does not put in `Books::next_item` yet (printed).
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
        let file = with_chunks(&saved.bytes, &[("GWORLD", gworld.encode()), ("IDMGR", idmgr.encode().unwrap())]);
        assert_eq!(gworld_of(&file), gworld);
        let restored = saved.restore(&file);
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
        }
        eprintln!("{restored} saved games restored and written: IDMGR exact, GWORLD exact but the stream; +198 differs in {next_item_differs}; refused {refused:?}");
        assert!(restored > 0, "no saved game restored");
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
