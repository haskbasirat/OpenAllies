//! Latest native header/creator serialization:59E7E0,5116F0,50D730,512A00.
//! This writes a real captured film-header object; creating that object from
//! current GWorld fields (5A26E0/5A17D0) is a separate, mandatory producer.
use aa_mapworld::header::{MapHeader, WorldCreator, WorldParameters, WorldRules};

pub type Result<T> = std::result::Result<T, String>;
pub trait Sink {
    fn write(&mut self, bytes: &[u8]) -> Result<()>;
    /// Actual49AA30 in MapResource key-only mode calls5A1120 AFTER key writes.
    /// The implementation retains the real current localized entry reference.
    /// MAPSTRINGS itself is the separate actual GameData+5B4 TextData list;
    /// these retained references do not define that list's serialization order.
    fn register_map_string(&mut self, key: &[u16]) -> Result<()>;
    ///An actual current localized pair carries both CString words. Key-only
    ///DTO diagnostics may use the legacy required key service; a real pair
    ///must bind its creation/fallback path instead of dropping its text.
    fn register_localized_pair(&mut self, key: &[u16], text: Option<&[u16]>) -> Result<()> {
        if text.is_some() {
            return Err(
                "current film localized pair requires actual pair-aware MapString service".into(),
            );
        }
        self.register_map_string(key)
    }
}
pub(crate) fn word(sink: &mut impl Sink, value: u32) -> Result<()> {
    sink.write(&value.to_le_bytes())
}
pub(crate) fn byte(sink: &mut impl Sink, value: u8) -> Result<()> {
    sink.write(&[value])
}
pub(crate) fn count(sink: &mut impl Sink, count: usize) -> Result<()> {
    word(
        sink,
        u32::try_from(count).map_err(|_| "film count exceeds DWORD wire representation")?,
    )
}
pub fn string(sink: &mut impl Sink, value: &[u16]) -> Result<()> {
    let len = value.len() as u16;
    sink.write(&len.to_le_bytes())?;
    let bytes: Vec<u8> = value[..len as usize]
        .iter()
        .flat_map(|u| u.to_le_bytes())
        .collect();
    sink.write(&bytes)
}
pub fn rules(sink: &mut impl Sink, value: &WorldRules) -> Result<()> {
    word(sink, 4)?;
    word(sink, value.mode)?;
    string(sink, &value.game_type)?;
    count(sink, value.rules.len())?;
    for row in &value.rules {
        string(sink, &row.key)?;
        byte(sink, row.enabled)?;
        word(sink, row.kind)?;
        match row.kind {
            0 => byte(sink, row.value as u8)?,
            1..=3 => word(sink, row.value)?,
            _ => return Err("film WorldRule kind has no admitted native producer".into()),
        }
    }
    Ok(())
}
pub fn parameters(sink: &mut impl Sink, value: &WorldParameters) -> Result<()> {
    crate::film_parameter_serialization::write(
        sink,
        &crate::film_parameter_serialization::Decoded(value),
    )
}
///59E7E0 always writes version7, regardless of the original saved input version.
pub fn header(sink: &mut impl Sink, value: &MapHeader) -> Result<()> {
    crate::film_current_header::write(sink, &crate::film_current_header::Decoded(value))
}
///59E530 class tag and actual512A00 WorldCreator version3 payload.
pub fn creator(sink: &mut impl Sink, value: &WorldCreator) -> Result<()> {
    if value.class_name != "KKC_WorldCreator".encode_utf16().collect::<Vec<_>>() {
        return Err("film polymorphic creator requires actual derived serializer".into());
    }
    string(sink, &value.class_name)?;
    creator_base_payload(sink, value)
}
///512A00 is also called after512C30's derived version0 word. This function
///emits the base payload only, never a second polymorphic class tag.
pub fn creator_base_payload(sink: &mut impl Sink, value: &WorldCreator) -> Result<()> {
    word(sink, 3)?;
    rules(sink, &value.rules)?;
    parameters(sink, &value.parameters)?;
    for extent in value.world_extents_bits {
        word(sink, extent)?;
    }
    string(sink, &value.biome)?;
    string(sink, &value.weather)?;
    word(sink, value.rng_seed)
}
