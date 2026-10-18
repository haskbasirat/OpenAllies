//! Reached target MapResource+90 fields after5A2A40: saved-state/film gate,
//! recording-map name+38 and the height-then-width raw Creator extent stores.
//! This owns only that loaded field slice, not the fresh film Header or a
//! substitute mutable Rules/Parameters owner.
//! Name overwrite reads the explicit sourceResource ARGUMENT's+38 (EBP loaded
//! at5A2B04), not the target `this` Resource's constructor IDS. The two are
//! deliberately distinct in the native fixture and current owner binding.
use aa_mapworld::{MapArchive, SourceIdentity, header::MapHeader};

///5A28D1 loads literalA44368, then5A29D8..5A29E3 passes the SAME CString to
///5A27A0 and5AA890 copies it to target Resource+38. No caller raw-string ctor.
pub(crate) struct ConstructorIds {
    words: Vec<u16>,
}
impl ConstructorIds {
    pub(crate) fn from_fresh_factory_5a28b0() -> Self {
        Self {
            words: vec![110, 101, 119],
        }
    }
    pub(crate) fn words(&self) -> &[u16] {
        &self.words
    }
}
pub struct State {
    source: SourceIdentity,
    loaded_mode: u32,
    name_38: Option<Vec<u16>>,
    extents: [u32; 2],
}
impl State {
    pub(crate) fn load(
        map: &MapArchive,
        parsed_creator: &crate::resource_creator::Creator,
        source_ids: Option<&[u16]>,
    ) -> Result<Self, String> {
        let chunk = map
            .decode_chunk("MAP_HEADER", u64::MAX)
            .map_err(|e| e.to_string())?;
        let header = MapHeader::decode(&chunk).map_err(|e| e.to_string())?;
        let mode = header.rules.mode;
        Ok(Self {
            source: header.source,
            loaded_mode: mode,
            name_38: post_read_name(mode, header.string_38, source_ids)?,
            extents: parsed_creator.extents_bits(),
        })
    }
    pub fn source(&self) -> &SourceIdentity {
        &self.source
    }
    pub fn loaded_mode(&self) -> u32 {
        self.loaded_mode
    }
    pub fn recording_name(&self) -> Result<&[u16], String> {
        self.name_38.as_deref().ok_or_else(|| {
            "5A2A40 target Header38 requires actual explicit sourceResource argument IDS".into()
        })
    }
    pub fn extents_bits(&self) -> [u32; 2] {
        self.extents
    }
}
fn post_read_name(
    mode: u32,
    loaded: Vec<u16>,
    ids: Option<&[u16]>,
) -> Result<Option<Vec<u16>>, String> {
    let saved = crate::map_resource_mode::world_rule_mode(mode)?.saved_state;
    Ok(if !saved && mode != 3 {
        ids.map(<[u16]>::to_vec)
    } else {
        Some(loaded)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_fresh_factory_ids_and_header_postread_fields() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../../research/skirmish-20261017-film/native-target-header.json"
        ))
        .unwrap();
        let ids = ConstructorIds::from_fresh_factory_5a28b0();
        assert_eq!(
            serde_json::to_value(ids.words()).unwrap(),
            fixture["factory_ids"]
        );
        assert_eq!(fixture["factory_executions"], 6);
        for row in fixture["cases"].as_array().unwrap() {
            let mode = row["input"]["mode"].as_u64().unwrap() as u32;
            let before: Vec<u16> = row["input"]["name"]
                .as_str()
                .unwrap()
                .encode_utf16()
                .collect();
            let source_ids: Vec<u16> = row["input"]["source_ids"]
                .as_str()
                .unwrap()
                .encode_utf16()
                .collect();
            let actual = post_read_name(mode, before, Some(&source_ids))
                .unwrap()
                .unwrap();
            assert_eq!(serde_json::to_value(actual).unwrap(), row["output"]["name"]);
            let bits: [u32; 2] = serde_json::from_value(row["input"]["extents"].clone()).unwrap();
            assert_eq!(
                serde_json::to_value(bits).unwrap(),
                row["output"]["extents"]
            );
        }
        assert!(post_read_name(0, vec![65], None).unwrap().is_none());
        assert_eq!(post_read_name(2, vec![65], None).unwrap(), Some(vec![65]));
        assert_eq!(post_read_name(3, vec![65], None).unwrap(), Some(vec![65]));
    }
}
