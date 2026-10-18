use crate::{Chunk, Error, Inventory, MapArchive, Result, SourceIdentity, inventory::Reader};
use aa_actors::ActorId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IdManagerState {
    pub source: SourceIdentity,
    pub next_slot: u32,
    pub highest_slot: u32,
    pub capacity: u32,
    /// Original loader reads exactly highest_slot u16s, not highest_slot+1.
    /// Remaining generations keep the native Reset zero value.
    pub serialized_generations: Vec<u16>,
}
impl IdManagerState {
    /// Exact empty, pre-registration native ID-manager state. Unlike creation,
    /// subsequent restoration must use register_saved without reseeding IDs.
    pub fn restore_empty_registry<T>(
        &self,
    ) -> std::result::Result<aa_actors::IdentityRegistry<T>, aa_actors::registry::RegistryError>
    {
        aa_actors::IdentityRegistry::from_load_metadata(
            self.next_slot,
            self.highest_slot,
            &self.serialized_generations,
        )
    }
    pub fn decode(chunk: &Chunk) -> Result<Self> {
        chunk.verify()?;
        if chunk.directory.name != "IDMGR" {
            return Err(Error::Format("incorrect IDMGR chunk identity".into()));
        }
        let mut r = Reader::new(&chunk.bytes);
        r.version(0)?;
        let next_slot = r.u32()?;
        let highest_slot = r.u32()?;
        let capacity = r.u32()?;
        if capacity != 65536 || !(1..65536).contains(&next_slot) || highest_slot > 65535 {
            return Err(Error::Format("invalid native ID-manager dimensions".into()));
        }
        let serialized_generations = r
            .take(highest_slot as usize * 2)?
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        if r.at != chunk.bytes.len() {
            return Err(Error::Format("IDMGR trailing bytes".into()));
        }
        Ok(Self {
            source: chunk.payload_source.clone(),
            next_slot,
            highest_slot,
            capacity,
            serialized_generations,
        })
    }
    /// The exact pre-construction generation guard from 4FAE80. This alone does
    /// not prove the object is registered, owned, initialized or currently alive.
    pub fn generation_matches(&self, raw: u32) -> bool {
        let id = ActorId::from_raw(raw);
        self.serialized_generations
            .get(id.slot() as usize)
            .copied()
            .unwrap_or(0)
            == id.generation()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WorldState {
    pub source: SourceIdentity,
    pub version: u32,
    pub rng_state: u32,
    pub word_dc: u32,
    pub begin_time_bits: u32,
    pub word_e4: u32,
    pub delta_bits: u32,
    pub word_140: u32,
    pub word_184: Option<u32>,
    pub word_194: Option<u32>,
    pub word_198: Option<u32>,
    pub word_44: Option<u32>,
    pub word_48: Option<u32>,
    pub trailing: Vec<u8>,
}
impl WorldState {
    pub fn decode(chunk: &Chunk) -> Result<Self> {
        chunk.verify()?;
        if chunk.directory.name != "GWORLD" {
            return Err(Error::Format("incorrect GWORLD chunk identity".into()));
        }
        let mut r = Reader::new(&chunk.bytes);
        let version = r.version(28)?;
        Ok(Self {
            source: chunk.payload_source.clone(),
            version,
            rng_state: r.u32()?,
            word_dc: r.u32()?,
            begin_time_bits: r.u32()?,
            word_e4: r.u32()?,
            delta_bits: r.u32()?,
            word_140: r.u32()?,
            word_184: if version >= 4 { Some(r.u32()?) } else { None },
            word_194: if version >= 4 { Some(r.u32()?) } else { None },
            word_198: if version >= 27 { Some(r.u32()?) } else { None },
            word_44: if version >= 5 { Some(r.u32()?) } else { None },
            word_48: if version >= 7 { Some(r.u32()?) } else { None },
            trailing: chunk.bytes[r.at..].to_vec(),
        })
    }
}
#[derive(Clone, Debug, Serialize)]
pub struct OpaqueSection {
    pub name: String,
    pub source: SourceIdentity,
    pub reason: String,
    pub bytes: Vec<u8>,
}
#[derive(Clone, Debug, Serialize)]
pub struct WorldInventory {
    pub map_source: SourceIdentity,
    pub header: Option<crate::header::MapHeader>,
    pub header_error: Option<String>,
    pub teams: Option<crate::kingdoms::Teams>,
    pub teams_error: Option<String>,
    pub team_generation_mismatches: Vec<u32>,
    pub kingdoms: Option<crate::kingdoms::Kingdoms>,
    pub kingdoms_error: Option<String>,
    pub kingdom_generation_mismatches: Vec<u32>,
    pub ownership: Option<crate::ownership::OwnershipInventory>,
    pub world: WorldState,
    pub id_manager: IdManagerState,
    pub dynamic_things: Inventory,
    pub generation_mismatches: Vec<usize>,
    pub opaque_initialization: Vec<OpaqueSection>,
    pub absent_sections: Vec<String>,
}
impl WorldInventory {
    pub fn decode(map: &MapArchive, max_chunk_bytes: u64) -> Result<Self> {
        let world = WorldState::decode(&map.decode_chunk("GWORLD", max_chunk_bytes)?)?;
        let id_manager = IdManagerState::decode(&map.decode_chunk("IDMGR", max_chunk_bytes)?)?;
        let dynamic_things = Inventory::decode(map, max_chunk_bytes)?;
        let generation_mismatches = dynamic_things
            .placements
            .iter()
            .filter(|p| !id_manager.generation_matches(p.serialized_id))
            .map(|p| p.record_ordinal)
            .collect();
        let mut opaque_initialization = Vec::new();
        let mut absent_sections = Vec::new();
        let header_chunk = map.decode_chunk("MAP_HEADER", max_chunk_bytes)?;
        let (header, header_error) = match crate::header::MapHeader::decode(&header_chunk) {
            Ok(header) => (Some(header), None),
            Err(error) => {
                let reason = format!("MAP_HEADER decoding is unsupported: {error}");
                opaque_initialization.push(OpaqueSection {
                    name: "MAP_HEADER".into(),
                    source: header_chunk.payload_source,
                    reason: reason.clone(),
                    bytes: header_chunk.bytes,
                });
                (None, Some(reason))
            }
        };
        let teams_chunk = map.decode_chunk("TEAMS", max_chunk_bytes)?;
        let (teams, teams_error) = match crate::kingdoms::Teams::decode(&teams_chunk) {
            Ok(teams) => (Some(teams), None),
            Err(error) => {
                let reason = format!("TEAMS decoding is unsupported: {error}");
                opaque_initialization.push(OpaqueSection {
                    name: "TEAMS".into(),
                    source: teams_chunk.payload_source,
                    reason: reason.clone(),
                    bytes: teams_chunk.bytes,
                });
                (None, Some(reason))
            }
        };
        let team_generation_mismatches = teams
            .iter()
            .flat_map(|teams| &teams.slots)
            .filter_map(|slot| slot.record.as_ref())
            .map(|record| record.serialized_id)
            .filter(|id| !id_manager.generation_matches(*id))
            .collect();
        let kingdoms_chunk = map.decode_chunk("KINGDOMS", max_chunk_bytes)?;
        let (kingdoms, kingdoms_error) =
            match crate::kingdoms::Kingdoms::decode(&kingdoms_chunk, world.version) {
                Ok(kingdoms) => (Some(kingdoms), None),
                Err(error) => {
                    let reason = format!("KINGDOMS decoding is unsupported: {error}");
                    opaque_initialization.push(OpaqueSection {
                        name: "KINGDOMS".into(),
                        source: kingdoms_chunk.payload_source,
                        reason: reason.clone(),
                        bytes: kingdoms_chunk.bytes,
                    });
                    (None, Some(reason))
                }
            };
        let kingdom_generation_mismatches = kingdoms
            .iter()
            .flat_map(|kingdoms| &kingdoms.slots)
            .filter_map(|slot| slot.record.as_ref())
            .map(|record| record.serialized_id)
            .filter(|id| !id_manager.generation_matches(*id))
            .collect();
        let ownership = kingdoms.as_ref().map(|kingdoms| {
            crate::ownership::OwnershipInventory::from_sources(
                kingdoms,
                &id_manager,
                &dynamic_things,
            )
        });
        for (name, reason) in [
            (
                "RULES",
                "Original WorldRules stream retained for mode-aware 512420/512200 SVar binding",
            ),
            (
                "USED_REQUIRED_OBJECTS",
                "Required-object load order retained pending subtype decoding",
            ),
            (
                "USED_REQUIRED_OBJECTS_INDEX",
                "Required-object record extents retained pending subtype decoding",
            ),
        ] {
            if !map.directory().iter().any(|entry| entry.name == name) {
                absent_sections.push(name.to_owned());
                continue;
            }
            let chunk = map.decode_chunk(name, max_chunk_bytes)?;
            opaque_initialization.push(OpaqueSection {
                name: name.into(),
                source: chunk.payload_source,
                reason: reason.into(),
                bytes: chunk.bytes,
            });
        }
        Ok(Self {
            map_source: map.source().clone(),
            header,
            header_error,
            teams,
            teams_error,
            team_generation_mismatches,
            kingdoms,
            kingdoms_error,
            kingdom_generation_mismatches,
            ownership,
            world,
            id_manager,
            dynamic_things,
            generation_mismatches,
            opaque_initialization,
            absent_sections,
        })
    }
}
