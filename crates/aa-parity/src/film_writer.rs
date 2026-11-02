//! The bytes of a film (`.RFM`), the inverse of [`crate::film`]: what
//! OpenFilmForRecording 0x76C310 and the record step 0x74D640 write.
//!
//! * `MAP_HEADER` is 59EA60: the header 59E7E0, then the polymorphic creator
//!   59E530 (`KKC_WorldCreator` 512A00, `KKC_WorldCreatorLoad` 512C30,
//!   `KKC_WorldCreatorRandom`). The field order is aa-runtime `film_header`
//!   and `film_parameter_serialization`, written over the decoded
//!   [`MapHeader`]; aa-parity does not link aa-runtime, which needs the whole
//!   native catalog.
//! * `SessionPlayers` is the loop of 0x76C310.
//! * `Orders` is one `[u32 LE length][packet]` per world tick; a packet is the
//!   finalized mirror stream (0x74D640, `aa_orders::net::transport::finalize`).
//! * `MAPSTRINGS` and the `.TGI` beside the film are the map's text table
//!   (59FDA0, 4738C0).
//!
//! `tests/film_writer.rs` proves every piece against the original's own bytes:
//! each chunk of every real film, decoded and written again, is the same chunk.
//! The chunk container and its zlib (level 6; deflater byte identity is not
//! claimed) are `aa_metagame::chunk_file`.
use crate::film::SessionPlayer;
use aa_mapworld::header::{DerivedCreator, MapHeader, WorldCreator, WorldParameters, WorldRules};
use aa_metagame::chunk_file::ChunkFileWriter;
use aa_orders::Position;
use aa_orders::commands::WireWriter;
use aa_orders::payload::{self, Field, SchemaContext};
use std::path::{Path, PathBuf};

/// `.RFM`, the extension constant 0xA4772C.
pub const FILM_EXTENSION: &str = "RFM";
/// The text table beside a film and inside its `MAPSTRINGS` chunk.
pub const TEXT_EXTENSION: &str = "TGI";

fn word(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn byte(out: &mut Vec<u8>, value: u8) {
    out.push(value);
}
/// A CString: the 16-bit unit count, then the units.
fn string(out: &mut Vec<u8>, units: &[u16]) {
    out.extend_from_slice(&(units.len() as u16).to_le_bytes());
    for u in units {
        out.extend_from_slice(&u.to_le_bytes());
    }
}

/// WorldRules 0x512420's stream, version word first.
fn rules(out: &mut Vec<u8>, value: &WorldRules) {
    word(out, value.version);
    word(out, value.mode);
    string(out, &value.game_type);
    word(out, value.rules.len() as u32);
    for row in &value.rules {
        string(out, &row.key);
        byte(out, row.enabled);
        word(out, row.kind);
        if row.kind == 0 {
            byte(out, row.value as u8);
        } else {
            word(out, row.value);
        }
    }
}

/// WorldParameters 50D730.
fn parameters(out: &mut Vec<u8>, value: &WorldParameters) {
    word(out, value.version);
    word(out, value.teams.len() as u32);
    for t in &value.teams {
        string(out, &t.id);
        string(out, &t.display_key);
    }
    word(out, value.kingdoms.len() as u32);
    for k in &value.kingdoms {
        string(out, &k.id);
        byte(out, k.byte_10);
        string(out, &k.nation);
        string(out, &k.faction);
        string(out, &k.color);
        string(out, &k.handicap);
        string(out, &k.display_key);
        if value.version >= 12 {
            string(out, k.string_0c.as_deref().unwrap_or(&[]));
        }
        string(out, &k.string_2c);
        byte(out, k.byte_29);
        byte(out, k.byte_28);
        string(out, &k.string_14);
        word(out, k.localized_entries.len() as u32);
        for e in &k.localized_entries {
            string(out, e);
        }
        word(out, k.definitions.len() as u32);
        for e in &k.definitions {
            string(out, e);
        }
        string(out, &k.string_58);
        word(out, k.properties.len() as u32);
        for e in &k.properties {
            string(out, e);
        }
        byte(out, k.byte_31);
        byte(out, k.byte_32);
        if value.version >= 13 {
            word(out, k.resources.len() as u32);
            for (id, bits) in &k.resources {
                string(out, id);
                word(out, *bits);
            }
        }
    }
}

/// 512A00 after its class tag: version, rules, parameters, extents, biome,
/// weather, seed.
fn creator_base(out: &mut Vec<u8>, value: &WorldCreator) {
    word(out, value.version);
    rules(out, &value.rules);
    parameters(out, &value.parameters);
    for extent in value.world_extents_bits {
        word(out, extent);
    }
    string(out, &value.biome);
    string(out, &value.weather);
    word(out, value.rng_seed);
}

/// The `MAP_HEADER` chunk 59EA60: header, then polymorphic creator.
pub fn map_header_bytes(header: &MapHeader, creator: &DerivedCreator) -> Vec<u8> {
    let mut out = Vec::new();
    word(&mut out, header.version);
    rules(&mut out, &header.rules);
    parameters(&mut out, &header.parameters);
    for v in header.words_0c_10 {
        word(&mut out, v);
    }
    string(&mut out, &header.name_key);
    string(&mut out, &header.description_key);
    if let Some(extents) = header.display_extents_bits {
        word(&mut out, extents[0]);
        word(&mut out, extents[1]);
    }
    word(&mut out, header.word_50);
    word(&mut out, header.word_54);
    string(&mut out, &header.string_58);
    string(&mut out, &header.string_5c);
    string(&mut out, &header.string_60);
    byte(&mut out, header.byte_64);
    string(&mut out, &header.string_38);
    string(&mut out, &header.string_3c);
    if let Some(v) = header.word_40 {
        word(&mut out, v);
    }
    for v in header.words_18_1c {
        word(&mut out, v);
    }
    for entry in &header.entries {
        byte(&mut out, 1);
        byte(&mut out, entry.flag);
        string(&mut out, &entry.string_20);
        string(&mut out, &entry.string_18);
        if let Some(flag) = entry.flag_v5 {
            byte(&mut out, flag);
        }
    }
    byte(&mut out, 0);
    let base = &header.creator;
    string(&mut out, &base.class_name);
    match creator {
        DerivedCreator::Base => creator_base(&mut out, base),
        DerivedCreator::Load {
            derived_version,
            source_resource_ids,
        } => {
            word(&mut out, *derived_version);
            creator_base(&mut out, base);
            string(&mut out, source_resource_ids);
        }
        DerivedCreator::Random(settings) => {
            creator_base(&mut out, base);
            word(&mut out, settings.version);
            string(&mut out, &settings.rmc);
            word(&mut out, settings.terrain_weights.len() as u32);
            for (id, bits) in &settings.terrain_weights {
                string(&mut out, id);
                word(&mut out, *bits);
            }
            word(&mut out, settings.total_terrain_weight_bits);
            word(&mut out, settings.object_counts.len() as u32);
            for (id, bits) in &settings.object_counts {
                string(&mut out, id);
                word(&mut out, *bits);
            }
            word(&mut out, settings.num_heroes);
        }
    }
    out
}

/// The `SessionPlayers` chunk (the loop of 0x76C310): count, then per player
/// its id, name and Kingdom+0x188 index, `u32::MAX` for no kingdom.
pub fn session_players_bytes(players: &[SessionPlayer]) -> Vec<u8> {
    let mut out = Vec::new();
    word(&mut out, players.len() as u32);
    for p in players {
        word(&mut out, p.id);
        let units: Vec<u16> = p.name.encode_utf16().collect();
        string(&mut out, &units);
        word(&mut out, p.kingdom_index.unwrap_or(u32::MAX));
    }
    out
}

/// The map text table 4738C0 writes, in the head-first order the caller gives
/// (natively the field hash walk of `film_text`). Values are plain text.
pub fn map_strings_bytes(name: &str, entries: &[(String, String)]) -> Vec<u8> {
    let mut out = b"[Text language=Default]\r\n{\r\n\tids = map_strings_table\r\n\tname = \"".to_vec();
    out.extend_from_slice(&latin1(name));
    out.extend_from_slice(b"\"\r\n\r\n");
    for (key, text) in entries {
        out.push(b'\t');
        out.extend_from_slice(&latin1(key));
        out.extend_from_slice(b" = \"");
        out.extend_from_slice(&latin1(text));
        out.extend_from_slice(b"\"\r\n");
    }
    out.extend_from_slice(b"}\r\n");
    out
}

/// A DisplayString value. The native parser takes Latin-1 text; the characters
/// its text format reserves are left out of the value, never escaped by guess.
fn latin1(text: &str) -> Vec<u8> {
    text.chars()
        .filter(|&c| c as u32 >= 32 && (c as u32) < 256 && !matches!(c, '"' | '&' | '\\'))
        .map(|c| c as u32 as u8)
        .collect()
}

/// One decree of a record: its descriptor and its fields in schema order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WireDecree {
    pub descriptor: usize,
    pub fields: Vec<Field>,
}

/// The mirror stream of one record (Admin+0x278 / +0x27C) and the `Orders`
/// chunk bytes so far.
pub struct OrdersWriter {
    mirror: WireWriter,
    records: u32,
}

impl OrdersWriter {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            mirror: WireWriter::new(Position::PACKET_START).map_err(|e| e.to_string())?,
            records: 0,
        })
    }
    /// The mirror write of one executed decree: the 9-bit descriptor and its
    /// payload (flow's mirror step, schema writers).
    pub fn decree(&mut self, decree: &WireDecree, context: &dyn SchemaContext) -> Result<(), String> {
        self.mirror
            .write_uint(decree.descriptor as u32, aa_orders::flow::DESCRIPTOR_BITS)
            .map_err(|e| e.to_string())?;
        payload::encode(decree.descriptor, &decree.fields, &mut self.mirror, context)
            .map_err(|e| e.to_string())
    }
    /// 0x74D640: the finalized mirror as `[u32 length][bytes]`; the mirror
    /// starts over.
    pub fn record(&mut self) -> Result<Vec<u8>, String> {
        let bytes = aa_orders::net::transport::finalize(&self.mirror).map_err(|e| e.to_string())?;
        self.mirror = WireWriter::new(Position::PACKET_START).map_err(|e| e.to_string())?;
        self.records += 1;
        let mut out = Vec::with_capacity(4 + bytes.len());
        out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
        out.extend_from_slice(&bytes);
        Ok(out)
    }
    /// Records written so far: the number of the next one.
    pub fn records(&self) -> u32 {
        self.records
    }
}

/// A film being written: the open file, chunks so far, and where it goes.
///
/// The file holds the bare chunk file header from the open on, as
/// 859690 / 859380 leave a new file, and the writer owns it until
/// [`FilmFile::close`], so a second opener sees a sharing failure the way the
/// film dialog does. The chunk directory comes at the close (859330 / 859090):
/// a game that dies before it leaves a film the reader refuses, natively too.
pub struct FilmFile {
    path: PathBuf,
    writer: ChunkFileWriter,
    file: std::fs::File,
}

impl FilmFile {
    /// 0x5A3250 / 0x5A00E0 with 0x76C310 up to the open `Orders` chunk.
    pub fn create(
        path: &Path,
        map_header: &[u8],
        map_strings: &[u8],
        session_players: &[u8],
    ) -> std::io::Result<Self> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        let mut writer = ChunkFileWriter::create(&path.to_string_lossy());
        let fail = |e: aa_metagame::Error| std::io::Error::other(e.to_string());
        // 59EA60: `MAP` is one raw zero word, the rest the compressed chunks.
        writer.open_chunk("MAP", false).map_err(fail)?;
        writer.write(&0u32.to_le_bytes()).map_err(fail)?;
        writer.open_chunk("MAP_HEADER", true).map_err(fail)?;
        writer.write(map_header).map_err(fail)?;
        writer.open_chunk("MAPSTRINGS", true).map_err(fail)?;
        writer.write(map_strings).map_err(fail)?;
        writer.open_chunk("SessionPlayers", true).map_err(fail)?;
        writer.write(session_players).map_err(fail)?;
        writer.open_chunk("Orders", true).map_err(fail)?;
        let mut out = Self {
            path: path.to_owned(),
            writer,
            file,
        };
        use std::io::Write;
        out.file.write_all(&aa_metagame::chunk_file::MAGIC.to_le_bytes())?;
        out.file.write_all(&aa_metagame::chunk_file::VERSION.to_le_bytes())?;
        out.file.write_all(&0u32.to_le_bytes())?;
        Ok(out)
    }
    /// One record of the `Orders` chunk.
    pub fn write_record(&mut self, record: &[u8]) -> std::io::Result<()> {
        self.writer
            .write(record)
            .map_err(|e| std::io::Error::other(e.to_string()))
    }
    pub fn path(&self) -> &Path {
        &self.path
    }
    /// 0x74B310 and the stream's close: the finished chunk file on disk.
    pub fn close(self) -> std::io::Result<PathBuf> {
        use std::io::{Seek, SeekFrom, Write};
        let Self {
            path,
            writer,
            mut file,
        } = self;
        let bytes = writer
            .close()
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        file.seek(SeekFrom::Start(0))?;
        file.set_len(0)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        Ok(path)
    }
}
