//! Original MAP_HEADER records and appended KKC_WorldCreator inputs.
//!
//! Decodes the native key-only localization stream, WorldRules v4 (the verified
//! corpus has world mode0; that is independent of the stream localization flag),
//! WorldParameters v11..13, MapHeader v4..7 and WorldCreator v3. Definition
//! strings remain IDs, not guessed registry indices or live kingdom owners.

use crate::{Chunk, Error, Result, SourceIdentity, inventory::Reader};
use serde::{Deserialize, Serialize};

pub type Utf16 = Vec<u16>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rule {
    pub key: Utf16,
    pub enabled: u8,
    /// Native type0 stores one byte; types1..3 store one raw word.
    pub kind: u32,
    pub value: u32,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldRules {
    pub version: u32,
    pub mode: u32,
    pub game_type: Utf16,
    pub rules: Vec<Rule>,
}
impl WorldRules {
    /// The standalone RULES chunk uses the same original 512420 stream as
    /// the embedded header/creator record. Reject trailing data explicitly.
    pub fn decode_bytes(bytes: &[u8]) -> Result<Self> {
        let mut reader = Reader::new(bytes);
        let value = rules(&mut reader)?;
        if reader.at != bytes.len() {
            return Err(Error::Format("unconsumed RULES bytes".into()));
        }
        Ok(value)
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamParameter {
    pub id: Utf16,
    pub display_key: Utf16,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct KingdomParameter {
    pub id: Utf16,
    pub byte_10: u8,
    pub nation: Utf16,
    pub faction: Utf16,
    pub color: Utf16,
    pub handicap: Utf16,
    pub display_key: Utf16,
    pub string_0c: Option<Utf16>,
    pub string_2c: Utf16,
    pub byte_29: u8,
    pub byte_28: u8,
    pub string_14: Utf16,
    pub localized_entries: Vec<Utf16>,
    pub definitions: Vec<Utf16>,
    pub string_58: Utf16,
    pub properties: Vec<Utf16>,
    pub byte_31: u8,
    pub byte_32: u8,
    pub resources: Vec<(Utf16, u32)>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldParameters {
    pub version: u32,
    pub teams: Vec<TeamParameter>,
    pub kingdoms: Vec<KingdomParameter>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HeaderEntry {
    pub flag: u8,
    pub string_20: Utf16,
    pub string_18: Utf16,
    pub flag_v5: Option<u8>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorldCreator {
    pub class_name: Utf16,
    pub version: u32,
    pub rules: WorldRules,
    pub parameters: WorldParameters,
    /// Native m_width/+3C and m_height/+40. These exist in all supported maps,
    /// including old MapHeader versions with no duplicated display dimensions.
    pub world_extents_bits: [u32; 2],
    pub biome: Utf16,
    pub weather: Utf16,
    pub rng_seed: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapHeader {
    pub source: SourceIdentity,
    pub version: u32,
    pub rules: WorldRules,
    pub parameters: WorldParameters,
    pub words_0c_10: [u32; 2],
    pub name_key: Utf16,
    pub description_key: Utf16,
    pub display_extents_bits: Option<[u32; 2]>,
    pub word_50: u32,
    pub word_54: u32,
    pub string_58: Utf16,
    pub string_5c: Utf16,
    pub string_60: Utf16,
    pub byte_64: u8,
    pub string_38: Utf16,
    pub string_3c: Utf16,
    pub word_40: Option<u32>,
    pub words_18_1c: [u32; 2],
    pub entries: Vec<HeaderEntry>,
    pub creator_offset: usize,
    pub creator: WorldCreator,
}

fn byte(r: &mut Reader<'_>) -> Result<u8> {
    Ok(r.take(1)?[0])
}
fn count(r: &mut Reader<'_>, minimum: usize) -> Result<usize> {
    let count = r.u32()? as usize;
    if count > i32::MAX as usize
        || count
            .checked_mul(minimum)
            .is_none_or(|size| size > r.bytes.len() - r.at)
    {
        return Err(Error::Format(
            "header count exceeds its source extent".into(),
        ));
    }
    Ok(count)
}
fn many<T>(
    r: &mut Reader<'_>,
    minimum: usize,
    mut read: impl FnMut(&mut Reader<'_>) -> Result<T>,
) -> Result<Vec<T>> {
    let count = count(r, minimum)?;
    let mut items = Vec::new();
    items.try_reserve(count).map_err(|_| Error::Allocation)?;
    for _ in 0..count {
        items.push(read(r)?);
    }
    Ok(items)
}
fn rules(r: &mut Reader<'_>) -> Result<WorldRules> {
    let version = r.version(4)?;
    if version != 4 {
        return Err(Error::Format(
            "WorldRules version requires an older decoder".into(),
        ));
    }
    let mode = r.u32()?;
    if mode != 0 {
        return Err(Error::Format(
            "nonzero world mode is outside the verified original-map corpus".into(),
        ));
    }
    let game_type = r.string()?;
    let rules = many(r, 8, |r| {
        let key = r.string()?;
        let enabled = byte(r)?;
        let kind = r.u32()?;
        let value = match kind {
            0 => u32::from(byte(r)?),
            1..=3 => r.u32()?,
            _ => return Err(Error::Format(format!("unknown native rule kind {kind}"))),
        };
        Ok(Rule {
            key,
            enabled,
            kind,
            value,
        })
    })?;
    Ok(WorldRules {
        version,
        mode,
        game_type,
        rules,
    })
}
fn parameters(r: &mut Reader<'_>) -> Result<WorldParameters> {
    let version = r.version(13)?;
    if version < 11 {
        return Err(Error::Format(
            "WorldParameters version requires an older decoder".into(),
        ));
    }
    let teams = many(r, 4, |r| {
        Ok(TeamParameter {
            id: r.string()?,
            display_key: r.string()?,
        })
    })?;
    //Even an empty record has length prefixes, four count fields and three
    //bytes. Reject impossible counts before reserving typed record storage.
    let minimum = 35 + usize::from(version >= 12) * 2 + usize::from(version >= 13) * 4;
    let kingdoms = many(r, minimum, |r| {
        let id = r.string()?;
        let byte_10 = byte(r)?;
        let nation = r.string()?;
        let faction = r.string()?;
        let color = r.string()?;
        let handicap = r.string()?;
        let display_key = r.string()?;
        let string_0c = if version >= 12 {
            Some(r.string()?)
        } else {
            None
        };
        let string_2c = r.string()?;
        let byte_29 = byte(r)?;
        let byte_28 = byte(r)?;
        let string_14 = r.string()?;
        let localized_entries = many(r, 2, |r| r.string())?;
        let definitions = many(r, 2, |r| r.string())?;
        let string_58 = r.string()?;
        let properties = many(r, 2, |r| r.string())?;
        let byte_31 = byte(r)?;
        let byte_32 = byte(r)?;
        let resources = if version >= 13 {
            many(r, 6, |r| Ok((r.string()?, r.u32()?)))?
        } else {
            Vec::new()
        };
        Ok(KingdomParameter {
            id,
            byte_10,
            nation,
            faction,
            color,
            handicap,
            display_key,
            string_0c,
            string_2c,
            byte_29,
            byte_28,
            string_14,
            localized_entries,
            definitions,
            string_58,
            properties,
            byte_31,
            byte_32,
            resources,
        })
    })?;
    Ok(WorldParameters {
        version,
        teams,
        kingdoms,
    })
}
impl MapHeader {
    pub fn decode(chunk: &Chunk) -> Result<Self> {
        chunk.verify()?;
        if chunk.directory.name != "MAP_HEADER" {
            return Err(Error::Format("incorrect MAP_HEADER chunk identity".into()));
        }
        let mut r = Reader::new(&chunk.bytes);
        let version = r.version(7)?;
        if version < 4 {
            return Err(Error::Format(
                "MapHeader version requires an older decoder".into(),
            ));
        }
        let rules = rules(&mut r)?;
        let parameters = parameters(&mut r)?;
        let words_0c_10 = [r.u32()?, r.u32()?];
        let name_key = r.string()?;
        let description_key = r.string()?;
        let display_extents_bits = if version >= 7 {
            Some([r.u32()?, r.u32()?])
        } else {
            None
        };
        let word_50 = r.u32()?;
        let word_54 = r.u32()?;
        let string_58 = r.string()?;
        let string_5c = r.string()?;
        let string_60 = r.string()?;
        let byte_64 = byte(&mut r)?;
        let string_38 = r.string()?;
        let string_3c = r.string()?;
        let word_40 = if version >= 6 { Some(r.u32()?) } else { None };
        let words_18_1c = [r.u32()?, r.u32()?];
        let mut entries = Vec::new();
        while byte(&mut r)? != 0 {
            entries.try_reserve(1).map_err(|_| Error::Allocation)?;
            entries.push(HeaderEntry {
                flag: byte(&mut r)?,
                string_20: r.string()?,
                string_18: r.string()?,
                flag_v5: if version >= 5 {
                    Some(byte(&mut r)?)
                } else {
                    None
                },
            });
        }
        let creator_offset = r.at;
        let class_name = r.string()?;
        if class_name != "KKC_WorldCreator".encode_utf16().collect::<Vec<_>>() {
            return Err(Error::Format(
                "unsupported polymorphic world creator".into(),
            ));
        }
        let creator_version = r.version(3)?;
        if creator_version != 3 {
            return Err(Error::Format(
                "WorldCreator version requires an older decoder".into(),
            ));
        }
        let creator = WorldCreator {
            class_name,
            version: creator_version,
            rules: self::rules(&mut r)?,
            parameters: self::parameters(&mut r)?,
            world_extents_bits: [r.u32()?, r.u32()?],
            biome: r.string()?,
            weather: r.string()?,
            rng_seed: r.u32()?,
        };
        if r.at != chunk.bytes.len() {
            return Err(Error::Format("unconsumed MAP_HEADER bytes".into()));
        }
        Ok(Self {
            source: chunk.payload_source.clone(),
            version,
            rules,
            parameters,
            words_0c_10,
            name_key,
            description_key,
            display_extents_bits,
            word_50,
            word_54,
            string_58,
            string_5c,
            string_60,
            byte_64,
            string_38,
            string_3c,
            word_40,
            words_18_1c,
            entries,
            creator_offset,
            creator,
        })
    }
}
