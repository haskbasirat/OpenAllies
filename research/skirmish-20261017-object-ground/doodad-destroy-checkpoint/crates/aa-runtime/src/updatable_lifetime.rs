//!World+280 save hash and+284 temporary saved-ID table lifetimes.
//!Only fresh constructor/load operations are supported here. A retained
//!diagnostic Table is not evidence that either native pointer remains live.
#[derive(Debug, PartialEq, Eq)]
enum SavedTable {
    ConstructorNull,
    Bound { count: usize },
    ReturnedNull,
}
#[derive(Debug)]
pub(crate) struct State {
    saved: SavedTable,
}
impl State {
    ///4F804E/54 store EBX=0 from4F7D35. The save hash remains constructor
    ///null: actual4F2DB0 save/enumeration is not implemented by this owner.
    pub(crate) fn from_world_constructor() -> Self {
        Self {
            saved: SavedTable::ConstructorNull,
        }
    }
    ///The caller is actual4FC600 setup before the first saved record. The
    ///count must match both the original UPDATABLES chunk and its actual table.
    pub(crate) fn bind_saved_table(
        &mut self,
        chunk: &aa_mapworld::Chunk,
        actual_count: usize,
    ) -> Result<(), String> {
        if self.saved != SavedTable::ConstructorNull {
            return Err("World284 saved table already bound/returned".into());
        }
        chunk.verify().map_err(|v| v.to_string())?;
        if chunk.directory.name != "UPDATABLES" {
            return Err("50B470 requires actual UPDATABLES source".into());
        }
        let count = i32::from_le_bytes(
            chunk
                .bytes
                .get(..4)
                .ok_or("50B470 missing source count")?
                .try_into()
                .unwrap(),
        );
        if count < 0 || actual_count != count as usize {
            return Err("50B470 actual saved-table/source count differs".into());
        }
        self.saved = SavedTable::Bound {
            count: actual_count,
        };
        Ok(())
    }
    ///4FC600 epilogue invokes50B500 only after the actual conversion loop.
    ///Its first store clears the table's retained World284 backpointer. Root
    ///calls this at that same boundary; errors before it retain Bound.
    pub(crate) fn finish_map_load_50b500(&mut self) -> Result<(), String> {
        if self.saved == SavedTable::ReturnedNull {
            return Err("4FC600 saved-table return already retired".into());
        }
        self.saved = SavedTable::ReturnedNull;
        Ok(())
    }
    pub(crate) fn saved_table_present(&self) -> bool {
        matches!(self.saved, SavedTable::Bound { .. })
    }
    pub(crate) fn require_live_table_count(&self, count: usize) -> Result<(), String> {
        match self.saved {
            SavedTable::Bound { count: actual } if count != actual => {
                Err("World284 current table allocation count differs from bound source".into())
            }
            _ => Ok(()),
        }
    }
    pub(crate) fn save_hash_present(&self) -> bool {
        false
    }
    pub(crate) fn require_returned(&self) -> Result<(), String> {
        if self.saved == SavedTable::ReturnedNull {
            Ok(())
        } else {
            Err("native map-load epilogue has not retired World284".into())
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_epilogue_capability_is_distinct_from_retained_diagnostic_table() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-map-replacement-drain/native-updatable-lifetime.json")).unwrap();
        let mut covered = 0;
        for row in fixture["rows"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["input"]["save_hash"] != true)
        {
            let count = row["input"]["count"].as_u64().unwrap() as usize;
            let mut state = State::from_world_constructor();
            assert_eq!(row["output"]["constructor"], serde_json::json!([0, 0]));
            assert!(!state.saved_table_present());
            assert!(!state.save_hash_present());
            assert!(state.require_returned().is_err());
            let bytes = (count as i32).to_le_bytes().to_vec();
            let identity = aa_mapworld::SourceIdentity::of(&bytes);
            let chunk = aa_mapworld::Chunk {
                map_source: identity.clone(),
                payload_source: identity,
                directory: aa_mapworld::ChunkDirectory {
                    name: "UPDATABLES".into(),
                    directory_ordinal: 0,
                    packed_offset: 12,
                    packed_bytes: 4,
                    source_timestamp: 0,
                    decoded_bytes: 4,
                },
                encoding: aa_mapworld::archive::Encoding::Raw,
                bytes,
            };
            state.bind_saved_table(&chunk, count).unwrap();
            if row["input"]["returned"] == true {
                state.finish_map_load_50b500().unwrap();
                assert!(state.require_returned().is_ok());
                assert!(state.finish_map_load_50b500().is_err());
            } else {
                assert!(state.require_returned().is_err());
            }
            assert_eq!(
                state.saved_table_present(),
                row["output"]["saved_table_present"].as_bool().unwrap()
            );
            assert_eq!(
                state.save_hash_present(),
                row["output"]["save_hash_present"].as_bool().unwrap()
            );
            covered += 1;
        }
        assert_eq!(covered, 60); //positive actual save-hash lifetime is not exposed
    }
}
