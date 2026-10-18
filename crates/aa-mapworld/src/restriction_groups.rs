//! Serialized RESTRICTION_GROUPS, original505600 /5049C0.
//! This is input decoding only. Typed FindActor/Property resolution and the
//! historical allowed-list complements belong to the loaded source owner.
use crate::{
    Chunk, Error, Result, SourceIdentity,
    inventory::{ByteSpan, Reader},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocalizedStorage {
    ///49AD70: stream virtual48=true and59E520=false (authored RMP loading).
    KeyOnly,
    ///49AD70: otherwise a nonempty key is followed by its stored value.
    KeyAndValue,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileRecord {
    pub span: ByteSpan,
    pub version: u32,
    pub name: Vec<u16>,
    /// Native profile+0, distinct from the serialization version.
    pub word_0: u32,
    pub display_key: Vec<u16>,
    pub display_value: Option<Vec<u16>>,
    pub legacy_discarded: Option<Vec<u16>>,
    pub allowed_actors: Vec<Vec<u16>>,
    pub allowed_properties: Vec<Vec<u16>>,
    /// None is the version0/1 complement encoding, never an empty restriction.
    pub restricted_actors: Option<Vec<Vec<u16>>>,
    pub restricted_properties: Option<Vec<Vec<u16>>>,
}

#[derive(Clone, Debug)]
pub struct RestrictionGroups {
    map_source: SourceIdentity,
    payload_source: SourceIdentity,
    version: u32,
    word_0: u32,
    storage: LocalizedStorage,
    profiles: Vec<ProfileRecord>,
}

fn strings(r: &mut Reader<'_>) -> Result<Vec<Vec<u16>>> {
    let count = r.u32()? as i32;
    // Native signed loops execute zero times for a negative count. Every
    // positive string requires at least its two-byte length in this stream.
    let count = count.max(0) as usize;
    if count > (r.bytes.len() - r.at) / 2 {
        return Err(Error::Format(
            "restriction string count exceeds remaining payload".into(),
        ));
    }
    let mut out = Vec::new();
    out.try_reserve(count).map_err(|_| Error::Allocation)?;
    for _ in 0..count {
        out.push(r.string()?);
    }
    Ok(out)
}

impl RestrictionGroups {
    pub fn decode(chunk: &Chunk, storage: LocalizedStorage) -> Result<Self> {
        chunk.verify()?;
        if chunk.directory.name != "RESTRICTION_GROUPS" {
            return Err(Error::Format(
                "incorrect RESTRICTION_GROUPS chunk identity".into(),
            ));
        }
        let mut r = Reader::new(&chunk.bytes);
        let version = r.version(0)?;
        let word_0 = r.u32()?;
        let count = (r.u32()? as i32).max(0) as usize;
        if count > (r.bytes.len() - r.at) / 20 {
            return Err(Error::Format(
                "restriction profile count exceeds remaining payload".into(),
            ));
        }
        let mut profiles = Vec::new();
        profiles.try_reserve(count).map_err(|_| Error::Allocation)?;
        for _ in 0..count {
            let start = r.at;
            let version = r.version(4)?;
            let name = r.string()?;
            let word_0 = r.u32()?;
            let display_key = r.string()?;
            let display_value = if version >= 3
                && !display_key.is_empty()
                && storage == LocalizedStorage::KeyAndValue
            {
                Some(r.string()?)
            } else {
                None
            };
            let legacy_discarded = if version == 0 {
                Some(r.string()?)
            } else {
                None
            };
            let allowed_actors = strings(&mut r)?;
            let allowed_properties = strings(&mut r)?;
            let (restricted_actors, restricted_properties) = if version >= 2 {
                (Some(strings(&mut r)?), Some(strings(&mut r)?))
            } else {
                (None, None)
            };
            profiles.push(ProfileRecord {
                span: ByteSpan {
                    offset: start,
                    length: r.at - start,
                },
                version,
                name,
                word_0,
                display_key,
                display_value,
                legacy_discarded,
                allowed_actors,
                allowed_properties,
                restricted_actors,
                restricted_properties,
            });
        }
        if r.at != chunk.bytes.len() {
            return Err(Error::Format(format!(
                "restriction groups trailing bytes at {}",
                r.at
            )));
        }
        Ok(Self {
            map_source: chunk.map_source.clone(),
            payload_source: chunk.payload_source.clone(),
            version,
            word_0,
            storage,
            profiles,
        })
    }
    pub fn map_source(&self) -> &SourceIdentity {
        &self.map_source
    }
    pub fn payload_source(&self) -> &SourceIdentity {
        &self.payload_source
    }
    pub fn version(&self) -> u32 {
        self.version
    }
    pub fn word_0(&self) -> u32 {
        self.word_0
    }
    pub fn storage(&self) -> LocalizedStorage {
        self.storage
    }
    pub fn profiles(&self) -> &[ProfileRecord] {
        &self.profiles
    }
}
