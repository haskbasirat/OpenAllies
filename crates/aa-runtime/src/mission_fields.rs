//! The sole mutable World+44/+48 fields, distinct from GWORLD source bytes.
//! Ctor4F7D73/76, current-map Load4FC76C..DF, StartMission4FDFF1.
pub struct State {mode44:u32,started48:u32,loaded:bool}
impl State {
    pub fn from_world_constructor()->Self {Self{mode44:0,started48:0,loaded:false}}
    /// Validated current-map field installation. Versions16..28 use these
    /// unconditional four-byte reads; legacy verification needs its real
    /// resource-mode warning service and is not silently omitted.
    pub fn load_from_archive(&mut self,map:&aa_mapworld::MapArchive,saved:&aa_mapworld::world::WorldState)->Result<(),String> {
        if self.loaded {return Err("World44/48 fields have already loaded".into());}
        let chunk=map.decode_chunk("GWORLD",u64::MAX).map_err(|e|e.to_string())?;
        let actual=aa_mapworld::world::WorldState::decode(&chunk).map_err(|e|e.to_string())?;
        if actual.source!=saved.source || actual.version!=saved.version
            || actual.word_44!=saved.word_44 || actual.word_48!=saved.word_48 {
            return Err("World44/48 decoded view differs from its actual GWORLD bytes".into());
        }
        if !(16..=28).contains(&actual.version) {
            return Err("legacy World44/48 Load requires original verification services".into());
        }
        self.mode44=actual.word_44.ok_or("World44 current-map Load field missing")?;
        self.started48=actual.word_48.ok_or("World48 current-map Load field missing")?;
        self.loaded=true;
        Ok(())
    }
    pub fn require_loaded(&self)->Result<(),String> {
        if self.loaded {Ok(())} else {Err("actual World44/48 Load has not completed".into())}
    }
    pub fn mode44(&self)->u32 {self.mode44}
    pub fn started48(&self)->u32 {self.started48}
    ///4FDFF1, only after the preceding real mission startup effects.
    pub(crate) fn mark_started(&mut self)->Result<(),String> {
        self.require_loaded()?;self.started48=1;Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_constructor_load_order_and_start_store() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!(
            "../../../research/development10/mission-fields/native-mission-fields.json")).unwrap();
        assert_eq!(fixture["rows"].as_array().unwrap().len(),27);
        for row in fixture["rows"].as_array().unwrap() {
            let mut state=State::from_world_constructor();
            assert_eq!([state.mode44(),state.started48()],[0,0]);
            assert!(state.mark_started().is_err());
            //Explicit decoded-field fixture. Real caller validates archive.
            state.mode44=row["input"]["word44"].as_u64().unwrap() as u32;
            state.started48=row["input"]["word48"].as_u64().unwrap() as u32;
            state.loaded=true;
            assert_eq!(serde_json::json!([state.mode44(),state.started48()]),row["output"]["loaded"]);
            state.mark_started().unwrap();assert_eq!(serde_json::json!(state.started48()),row["output"]["started"]);
            assert_eq!(row["output"]["reads"],serde_json::json!([0x44,0x48]));
        }
    }
}
