//! Original film (`.RFM`) reader.
//!
//! Container: a TGCK v2 chunk file (aa-metagame `chunk_file` documents the
//! layout), read with `aa_mapworld::MapArchive` (zlib chunks included). A
//! film written by 0x76C310 has five chunks: `MAP` (a zero word), `MAP_HEADER`
//! (59EA60: header and polymorphic creator), `MAPSTRINGS`, `SessionPlayers`
//! and `Orders`.
//!
//! Orders: one `[u32 LE length][packet]` record per world tick. Film record
//! 0x74D640 writes the mirror stream at the start of every world tick
//! 0x752AE0, before playback and before GWorld update 0x4F9970. Record `r`
//! therefore holds the decrees executed in the admin ticks between world
//! updates `r` and `r + 1`; a checkpoint in record `r` is the S state after
//! `r` world updates. Playback 0x751A40 runs record `r` right before update
//! `r + 1`. Each packet is decoded with the native descriptor registry
//! (9-bit ids) and the aa-orders payload schemas. The film ordering player
//! (Admin+0x284) persists across records and changes on
//! `TGC_SetPlayerOrderDecree`, as in `AdminNode::play_film_record`.
use crate::decree::{Decree, id};
use crate::registries::Registries;
use crate::{Error, Result};
use aa_mapworld::header::{DerivedCreator, MapHeader, WorldParameters};
use aa_mapworld::{MapArchive, SourceIdentity};
use aa_orders::payload::Field;
use serde::Serialize;
use std::collections::BTreeMap;

/// Decoding guard for one chunk (not a game limit).
const CHUNK_BUDGET: u64 = 1 << 30;

#[derive(Clone, Debug, Serialize)]
pub struct ChunkInfo {
    pub name: String,
    pub packed_bytes: u64,
    pub decoded_bytes: u64,
    pub zlib: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct KingdomInfo {
    pub id: String,
    pub nation: String,
    pub faction: String,
    pub color: String,
    pub handicap: String,
    /// The kingdom's team id (`string_14`, e.g. "team_1").
    pub team: String,
    /// `byte_10`: a map independent kingdom (cities), not a player slot.
    pub independent: bool,
}
impl KingdomInfo {
    fn list(parameters: &WorldParameters) -> Vec<Self> {
        parameters
            .kingdoms
            .iter()
            .map(|k| KingdomInfo {
                id: text(&k.id),
                nation: text(&k.nation),
                faction: text(&k.faction),
                color: text(&k.color),
                handicap: text(&k.handicap),
                team: text(&k.string_14),
                independent: k.byte_10 != 0,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct FilmHeader {
    /// The decoded MAP_HEADER; `map_header.creator` is the base creator
    /// payload (512A00), whose `rng_seed` seeds the world RNG at 0x76AA90.
    pub map_header: MapHeader,
    /// Derived creator data (Load source IDS, Random settings).
    pub creator: DerivedCreator,
    pub creator_class: String,
    pub seed: u32,
    /// Kingdom parameters of the world creator (WP::CreateKingdomsAndTeams
    /// input), in stored order.
    pub creator_kingdoms: Vec<KingdomInfo>,
    /// Kingdom parameters of the header itself.
    pub header_kingdoms: Vec<KingdomInfo>,
    /// `rmc` of a random-map creator.
    pub rmc: Option<String>,
}

/// One SessionPlayers row (76C310): player+0x20 id, player+0x24 name and the
/// bound kingdom's Kingdom+0x188 index (`None` = no kingdom, written as
/// `u32::MAX`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SessionPlayer {
    pub id: u32,
    pub name: String,
    pub kingdom_index: Option<u32>,
}

/// One decoded decree of a world-tick record.
#[derive(Clone, Debug, Serialize)]
pub struct FilmDecree {
    /// Native descriptor ordinal.
    pub descriptor: u16,
    pub name: &'static str,
    /// The film ordering player when this decree runs (after a SetPlayer:
    /// its own player).
    pub player: Option<u32>,
    /// Absolute bit position of the payload in the packet.
    pub payload_bit: usize,
    pub decree: Decree,
    pub fields: Vec<Field>,
}

/// The decode stopped inside a record. The decrees before it are kept.
#[derive(Clone, Debug, Serialize)]
pub struct DecodeFailure {
    pub message: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct TickRecord {
    /// Film record index = world tick (see the module documentation).
    pub tick: u32,
    /// Packet body bits (0 for an empty tick).
    pub body_bits: usize,
    pub decrees: Vec<FilmDecree>,
    pub error: Option<DecodeFailure>,
}

/// A `KKC_SetSimSyncValueOrderDecree` checkpoint of the simulation sync log
/// S (`[0xB36560]`): `count` is the S last index (`end - 1`, 0x74E670), which
/// equals the number of S entries since the 0x77D900(0, 0) reset of session
/// launch 0x775BF0; `hash` is the rolling hash after that entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Checkpoint {
    pub tick: u32,
    pub count: i32,
    pub hash: i32,
}

#[derive(Clone, Debug, Serialize)]
pub struct Film {
    pub source: SourceIdentity,
    pub chunks: Vec<ChunkInfo>,
    pub header: FilmHeader,
    pub players: Vec<SessionPlayer>,
    pub map_strings_bytes: usize,
    pub ticks: Vec<TickRecord>,
    pub checkpoints: Vec<Checkpoint>,
}

fn text(units: &[u16]) -> String {
    String::from_utf16_lossy(units)
}
fn chunk_error(chunk: &'static str, message: impl Into<String>) -> Error {
    Error::Chunk {
        chunk,
        message: message.into(),
    }
}

/// SessionPlayers chunk (76C310).
pub fn decode_session_players(bytes: &[u8]) -> Result<Vec<SessionPlayer>> {
    let mut at = 0usize;
    let mut take = |n: usize| -> Result<&[u8]> {
        let end = at
            .checked_add(n)
            .filter(|&e| e <= bytes.len())
            .ok_or_else(|| chunk_error("SessionPlayers", format!("truncated at byte {at}")))?;
        let out = &bytes[at..end];
        at = end;
        Ok(out)
    };
    let word = |b: &[u8]| u32::from_le_bytes(b.try_into().expect("four bytes"));
    let count = word(take(4)?);
    let mut players = Vec::new();
    for _ in 0..count {
        let id = word(take(4)?);
        let units = u16::from_le_bytes(take(2)?.try_into().expect("two bytes")) as usize;
        let name: Vec<u16> = take(units * 2)?
            .chunks_exact(2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect();
        let kingdom = word(take(4)?);
        players.push(SessionPlayer {
            id,
            name: text(&name),
            kingdom_index: (kingdom != u32::MAX).then_some(kingdom),
        });
    }
    if at != bytes.len() {
        return Err(chunk_error("SessionPlayers", "unconsumed bytes"));
    }
    Ok(players)
}

/// Split the Orders chunk into its per-world-tick packets.
pub fn split_orders(bytes: &[u8]) -> Result<Vec<&[u8]>> {
    let mut at = 0usize;
    let mut out = Vec::new();
    while at < bytes.len() {
        let len = bytes
            .get(at..at + 4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")) as usize)
            .ok_or_else(|| {
                chunk_error("Orders", format!("truncated record length at byte {at}"))
            })?;
        let body = bytes.get(at + 4..at + 4 + len).ok_or_else(|| {
            chunk_error("Orders", format!("record at byte {at} exceeds the chunk"))
        })?;
        out.push(body);
        at += 4 + len;
    }
    Ok(out)
}

/// Decode one world-tick packet. `player` is the persistent film ordering
/// player.
pub fn decode_record(
    tick: u32,
    body: &[u8],
    registry: &aa_orders::Registry,
    registries: &Registries,
    player: &mut Option<u32>,
) -> TickRecord {
    let mut decrees = Vec::new();
    let mut error = None;
    let body_bits = match aa_orders::Packet::new(body.to_vec()) {
        Err(e) => {
            error = Some(DecodeFailure {
                message: format!("invalid packet: {e}"),
            });
            0
        }
        Ok(packet) => {
            let bits = aa_orders::net::admin::body_bits(&packet).max(0) as usize;
            let counts = |r| registries.count(r).map(|(n, _)| n);
            let mut gate = aa_orders::Gate::default();
            let result = aa_orders::dispatch(
                Some(&packet),
                None,
                registry,
                false,
                &mut gate,
                |descriptor, cursor, _gate| {
                    let payload_bit = cursor.position().absolute().map_err(|e| e.to_string())?;
                    let fields = aa_orders::payload::decode(descriptor, cursor, registries)
                        .map_err(|e| e.to_string())?;
                    let decree = Decree::from_fields(descriptor, &fields, &counts);
                    if let Decree::SetPlayer { player: p } = decree {
                        *player = (p != 0).then_some(p);
                    }
                    decrees.push(FilmDecree {
                        descriptor: descriptor as u16,
                        name: aa_orders::native::DESCRIPTORS[descriptor].name,
                        player: *player,
                        payload_bit,
                        decree,
                        fields,
                    });
                    Ok(())
                },
            );
            if let Err(e) = result {
                error = Some(DecodeFailure {
                    message: e.to_string(),
                });
            }
            bits
        }
    };
    TickRecord {
        tick,
        body_bits,
        decrees,
        error,
    }
}

impl Film {
    pub fn read(path: &std::path::Path, registries: &Registries) -> Result<Self> {
        let bytes = std::fs::read(path).map_err(|source| Error::Io {
            path: path.display().to_string(),
            source,
        })?;
        Self::from_bytes(bytes, registries)
    }

    pub fn from_bytes(bytes: Vec<u8>, registries: &Registries) -> Result<Self> {
        let archive = MapArchive::from_bytes(bytes)?;
        let mut chunks = Vec::new();
        let mut decoded = BTreeMap::new();
        for row in archive.directory() {
            let chunk = archive.decode_chunk(&row.name, CHUNK_BUDGET)?;
            chunks.push(ChunkInfo {
                name: row.name.clone(),
                packed_bytes: row.packed_bytes,
                decoded_bytes: row.decoded_bytes,
                zlib: chunk.encoding == aa_mapworld::archive::Encoding::Zlib,
            });
            decoded.insert(row.name.clone(), chunk);
        }
        let get = |name: &'static str| {
            decoded
                .get(name)
                .ok_or_else(|| chunk_error(name, "chunk missing"))
        };

        let (map_header, creator) = MapHeader::decode_any_creator(get("MAP_HEADER")?)?;
        let rmc = match &creator {
            DerivedCreator::Random(s) => Some(text(&s.rmc)),
            _ => None,
        };
        let header = FilmHeader {
            creator_class: text(&map_header.creator.class_name),
            seed: map_header.creator.rng_seed,
            creator_kingdoms: KingdomInfo::list(&map_header.creator.parameters),
            header_kingdoms: KingdomInfo::list(&map_header.parameters),
            rmc,
            creator,
            map_header,
        };
        let players = decode_session_players(&get("SessionPlayers")?.bytes)?;
        let map_strings_bytes = decoded.get("MAPSTRINGS").map_or(0, |c| c.bytes.len());

        let registry = aa_orders::native::registry()?;
        let mut player = None;
        let ticks: Vec<TickRecord> = split_orders(&get("Orders")?.bytes)?
            .into_iter()
            .enumerate()
            .map(|(tick, body)| {
                decode_record(tick as u32, body, &registry, registries, &mut player)
            })
            .collect();
        let checkpoints = ticks
            .iter()
            .flat_map(|t| {
                t.decrees.iter().filter_map(move |d| match d.decree {
                    Decree::SimSyncValue { index, hash } => Some(Checkpoint {
                        tick: t.tick,
                        count: index,
                        hash,
                    }),
                    _ => None,
                })
            })
            .collect();
        Ok(Self {
            source: archive.source().clone(),
            chunks,
            header,
            players,
            map_strings_bytes,
            ticks,
            checkpoints,
        })
    }

    /// Decrees of world tick `tick` (empty for an empty or absent record).
    pub fn decrees_at(&self, tick: u32) -> &[FilmDecree] {
        self.ticks.get(tick as usize).map_or(&[], |t| &t.decrees)
    }
    pub fn world_ticks(&self) -> usize {
        self.ticks.len()
    }
    pub fn non_empty_ticks(&self) -> usize {
        self.ticks.iter().filter(|t| t.body_bits > 0).count()
    }
    pub fn decree_count(&self) -> usize {
        self.ticks.iter().map(|t| t.decrees.len()).sum()
    }
    /// Records whose decode stopped early.
    pub fn failures(&self) -> impl Iterator<Item = &TickRecord> {
        self.ticks.iter().filter(|t| t.error.is_some())
    }
    /// Decrees per descriptor (`"<id> <name>"`).
    pub fn histogram(&self) -> BTreeMap<String, usize> {
        let mut out = BTreeMap::new();
        for d in self.ticks.iter().flat_map(|t| &t.decrees) {
            *out.entry(format!("{:3} {}", d.descriptor, d.name))
                .or_insert(0) += 1;
        }
        out
    }
    /// The creator kingdom bound to a session player, via the SessionPlayers
    /// Kingdom+0x188 index. Taking that index as the position in the creator
    /// kingdom list is inferred from the GKingdom::Init order of the
    /// original log (side01, side02, ...).
    pub fn player_kingdom(&self, player: u32) -> Option<&KingdomInfo> {
        let index = self
            .players
            .iter()
            .find(|p| p.id == player)?
            .kingdom_index?;
        self.header.creator_kingdoms.get(index as usize)
    }
    /// The header's own kingdom parameters with the same id as the
    /// player's creator kingdom (they carry the chosen nation and general).
    pub fn player_header_kingdom(&self, player: u32) -> Option<&KingdomInfo> {
        let id = &self.player_kingdom(player)?.id;
        self.header.header_kingdoms.iter().find(|k| &k.id == id)
    }
    /// SetPlayer decrees are bookkeeping; this counts the gameplay decrees.
    pub fn is_bookkeeping(descriptor: usize) -> bool {
        matches!(
            descriptor,
            id::SET_PLAYER | id::SIM_SYNC_VALUE | id::SYNC_VALUE | id::VERSION_ACK
        )
    }
}
