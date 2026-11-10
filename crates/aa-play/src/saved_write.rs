//! Writing a saved game, the counterpart of [`crate::saved_world`]: the Save Game
//! task (`KKC_ResourceManagerTaskSaveGame::SaveGame` 0x5A0500 -> SaveFull 0x5A0380
//! -> the resource's Save 0x5B26B0 / 0x5B1C70, which writes the map resource with
//! its virtual +0x50) and the world's chunks, written by GWorld's Save 0x4F2DB0.
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
//!   +198, +44 and +48, 48 bytes. `aa-play` holds the stream and the clock
//!   ([`GworldWords::of_world`]); the other six words it has no place for are carried
//!   from the GWORLD the world was loaded from (its `carried` argument), a rendition;
//! - `IDMGR` ([`IdmgrWords`]): version 0, the next slot, the highest slot, the
//!   capacity 0x10000 and the generations of slots 0..highest (the load reads that
//!   many, 0x4F0230). The generations of free slots are private to `crate::ids`, so
//!   a world's IDMGR is not written yet;
//! - `MISC_DATA` ([`MiscData`]), the tail of 4F2DB0, read and written again; `aa-play`
//!   keeps none of it yet.
//!
//! Not written yet, each by the original's writer named: `MAP` and `MAP_HEADER` (the
//! map resource), `RULES` (GWorld+0C), `TERRAIN` (+30), `HEIGHT_MAP` (+34),
//! `MOUNTAIN_HEIGHT_MAP` (+38, then the byte +275 in the same chunk), `RESTRICTION_GROUPS`
//! (0x503FA0), `TEAMS` (count +170, a presence byte and 0x4F1B90 for each team),
//! `OBJECTIVES` (count +254 and each objective's Save, then the word +25C), `KINGDOMS`
//! (the word +158, count +14C, a presence byte and 0x4F1B90 for each kingdom),
//! `USED_REQUIRED_OBJECTS` and `DYNAMIC_THINGS` with their indexes (0x4F24C0, every
//! thing's Save), `TRIGGERS` (0x54C830), `DENIZEN_GROUPS` (0x56A2A0), `SUN` (+4C),
//! `UPDATABLES` (the word at *+280, then the lists +1E8, +208, +1AC, +1C0, +1D4).
//! A campaign battle (rules mode 4) also writes `METAGAME` after
//! `KINGDOMS` (0x938960). So a battle cannot be saved from `aa-play` yet: a file
//! with these chunks taken from the map would load as the battle's start, not as it
//! stands.

use aa_mapworld::{IdManagerState, WorldState};

/// The chunks of a saved game in the order the original writes them, and whether
/// each is compressed in a compressed save (`MAP`, the resource's own word, never).
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

/// The words of the `GWORLD` chunk, in 4F2DB0's order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GworldWords {
    /// +D4: the world's random stream.
    pub stream: u32,
    /// +DC: the tick.
    pub tick: u32,
    /// +E0, +E4, +E8: the clock's begin, end and step, as bits.
    pub begin: u32,
    pub end: u32,
    pub step: u32,
    /// +140, +184, +194, +198, +44, +48: words `aa-play` does not keep. +44 is the
    /// map status, which SaveFull 0x5A0380 verifies and repairs to
    /// `KKE_MAP_STATUS_REPAIRED` (2) before the world is written; +198 grows with
    /// the battle's length; the others are 0, 11 (+184) and 1 or 2 (+48) on the
    /// original's saves.
    pub word_140: u32,
    pub word_184: u32,
    pub word_194: u32,
    pub word_198: u32,
    pub word_44: u32,
    pub word_48: u32,
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
            word_140: state.word_140,
            word_184: state.word_184?,
            word_194: state.word_194?,
            word_198: state.word_198?,
            word_44: state.word_44?,
            word_48: state.word_48?,
        })
    }

    /// The world as it stands: its stream and its clock, the other words carried
    /// from `carried` (what the world was loaded from).
    pub fn of_world(world: &crate::world::World, carried: &Self) -> Self {
        Self {
            stream: world.native_rng.state,
            tick: world.clock.tick,
            begin: world.clock.begin.0,
            end: world.clock.end.0,
            step: world.clock.step.0,
            ..*carried
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
            self.word_140,
            self.word_184,
            self.word_194,
            self.word_198,
            self.word_44,
            self.word_48,
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

    /// The chunk's bytes: version 0, next, highest, capacity, then `highest`
    /// generations.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(16 + self.generations.len() * 2);
        for w in [0, self.next_slot, self.highest_slot, IDMGR_CAPACITY] {
            out.extend(w.to_le_bytes());
        }
        for g in &self.generations {
            out.extend(g.to_le_bytes());
        }
        out
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
        fn text(out: &mut Vec<u8>, t: &[u16]) {
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
            word_140: 0,
            word_184: 0,
            word_194: 0,
            word_198: 0x2bc,
            word_44: 2,
            word_48: 1,
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
        let state = IdManagerState::decode(&chunk("IDMGR", words.encode())).unwrap();
        assert_eq!(state.capacity, IDMGR_CAPACITY);
        assert_eq!(IdmgrWords::of_state(&state), words);
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

    /// A battle restored from an original saved game (`AA_SAVED_GAME`, read only),
    /// played on for 64 ticks, its GWORLD written and put in place of the saved one:
    /// the restore of that file stands at the played clock, with the same kingdoms,
    /// actors and organizations (the other chunks are the original's).
    #[test]
    #[ignore = "needs the game data (AA_GAME_ROOT) and an original saved game (AA_SAVED_GAME)"]
    fn a_played_world_written_and_restored_again() {
        use crate::data::GameData;
        use crate::map::GameMap;
        use crate::special_ops::GeneralData;
        use std::sync::Arc;
        let root = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
        let path = std::path::PathBuf::from(std::env::var_os("AA_SAVED_GAME").expect("AA_SAVED_GAME"));
        let bytes = std::fs::read(&path).unwrap();
        let data = Arc::new(GameData::load(&root).unwrap());
        let generals = Arc::new(GeneralData::load(&root, &data).unwrap());
        let human = |_: &str| crate::world::Controller::Human;
        let restore = |bytes: &[u8]| {
            let map = GameMap::load_file("saved/round-trip", bytes.to_vec(), &data, None).unwrap();
            crate::saved_world::restore(bytes, data.clone(), generals.clone(), Arc::new(map), &human).unwrap().world
        };
        let mut world = restore(&bytes);
        for _ in 0..64 {
            world.step(Vec::new());
        }
        let archive = MapArchive::from_bytes(bytes.clone()).unwrap();
        let saved = WorldState::decode(&archive.decode_chunk("GWORLD", 1 << 30).unwrap()).unwrap();
        let carried = GworldWords::of_state(&saved).expect("a version 28 GWORLD");
        let written = GworldWords::of_world(&world, &carried);
        let chunks: Vec<(String, Vec<u8>)> = CHUNKS
            .iter()
            .map(|(name, _)| {
                let b = if *name == "GWORLD" { written.encode() } else { archive.decode_chunk(name, 1 << 30).unwrap().bytes };
                (name.to_string(), b)
            })
            .collect();
        let file = container(&chunks);
        let again = MapArchive::from_bytes(file.clone()).unwrap();
        let state = WorldState::decode(&again.decode_chunk("GWORLD", 1 << 30).unwrap()).unwrap();
        assert_eq!(GworldWords::of_state(&state), Some(written));
        assert_eq!(state.rng_state, world.native_rng.state);
        let restored = restore(&file);
        assert_eq!(restored.clock, world.clock);
        assert_eq!(restored.tick, world.tick);
        assert_eq!(restored.time.to_bits(), world.time.to_bits());
        assert_eq!(restored.clock.tick, saved.word_dc + 64);
        let first = restore(&bytes);
        assert_eq!(restored.kingdoms.len(), first.kingdoms.len());
        assert_eq!(restored.actors.len(), first.actors.len());
        assert_eq!(restored.orgs.len(), first.orgs.len());
        eprintln!(
            "restored at tick {} (saved {}), {} kingdoms, {} actors, {} organizations",
            restored.clock.tick,
            saved.word_dc,
            restored.kingdoms.len(),
            restored.actors.len(),
            restored.orgs.len()
        );
    }

    /// Every saved game in `AA_SAVED_GAMES` (a folder of the original's `.RSG`
    /// files, read only): its chunks are [`CHUNKS`] in order with their encodings,
    /// and its GWORLD and IDMGR, read and written again, are the original's bytes.
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
        let (mut checked, mut unreadable) = (0usize, 0usize);
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
            assert_eq!(words.word_44, 2, "{}: the map status SaveFull leaves", path.display());
            let idmgr = archive.decode_chunk("IDMGR", 1 << 30).unwrap();
            let ids = IdmgrWords::of_state(&IdManagerState::decode(&idmgr).unwrap());
            assert_eq!(ids.encode(), idmgr.bytes, "{} IDMGR", path.display());
            let misc = archive.decode_chunk("MISC_DATA", 1 << 30).unwrap();
            let data = MiscData::decode(&misc.bytes).unwrap_or_else(|| panic!("{} MISC_DATA", path.display()));
            assert_eq!(data.encode(), misc.bytes, "{} MISC_DATA", path.display());
        }
        assert!(checked > 0, "no saved games");
        eprintln!("{checked} saved games re-encode exactly ({unreadable} files not chunk files)");
    }
}
